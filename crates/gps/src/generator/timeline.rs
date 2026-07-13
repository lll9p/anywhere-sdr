use constants::SECONDS_IN_WEEK;

use crate::{Error, GpsTime};

/// Largest denominator retained when recovering a configured decimal ratio.
const MAX_RATIONAL_DENOMINATOR: u128 = 1_000_000_000;
/// Duration of one complete LNAV frame.
const FRAME_SECONDS: u64 = 30;

/// Positive reduced rational used for exact sample-boundary calculations.
#[derive(Clone, Copy, Debug)]
struct Rational {
    /// Ratio numerator.
    numerator: u128,
    /// Ratio denominator.
    denominator: u128,
}

impl Rational {
    /// Recovers a bounded rational representation of a positive finite value.
    fn from_positive_f64(value: f64, name: &str) -> Result<Self, Error> {
        if !value.is_finite() || value <= 0.0 {
            return Err(Error::msg(format!(
                "{name} must be finite and greater than zero"
            )));
        }

        let mut remainder = value;
        let mut previous_numerator = 0u128;
        let mut numerator = 1u128;
        let mut previous_denominator = 1u128;
        let mut denominator = 0u128;

        loop {
            let whole = remainder.floor() as u128;
            let Some(next_numerator) = whole
                .checked_mul(numerator)
                .and_then(|term| term.checked_add(previous_numerator))
            else {
                break;
            };
            let Some(next_denominator) = whole
                .checked_mul(denominator)
                .and_then(|term| term.checked_add(previous_denominator))
            else {
                break;
            };
            if next_denominator > MAX_RATIONAL_DENOMINATOR {
                break;
            }

            previous_numerator = numerator;
            numerator = next_numerator;
            previous_denominator = denominator;
            denominator = next_denominator;

            let approximation = numerator as f64 / denominator as f64;
            let tolerance = f64::EPSILON * value.abs().max(1.0) * 8.0;
            if (approximation - value).abs() <= tolerance {
                break;
            }

            let fractional = remainder - whole as f64;
            if fractional <= f64::EPSILON {
                break;
            }
            remainder = fractional.recip();
        }

        if denominator == 0 {
            return Err(Error::msg(format!(
                "could not represent {name} as a rational value"
            )));
        }

        let divisor = greatest_common_divisor(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Multiplies and rounds to the nearest whole unit.
    fn rounded_product(self, multiplier: u128) -> Result<u64, Error> {
        let scaled =
            self.numerator.checked_mul(multiplier).ok_or_else(|| {
                Error::msg("sample timeline multiplication overflow")
            })?;
        let rounded = scaled
            .checked_add(self.denominator / 2)
            .ok_or_else(|| Error::msg("sample timeline rounding overflow"))?
            / self.denominator;
        u64::try_from(rounded)
            .map_err(|_| Error::msg("sample count exceeds supported range"))
    }

    /// Multiplies and rounds upward to a whole unit.
    fn ceiling_product(self, multiplier: u128) -> Result<u64, Error> {
        let scaled =
            self.numerator.checked_mul(multiplier).ok_or_else(|| {
                Error::msg("sample timeline multiplication overflow")
            })?;
        let ceiling = divide_ceiling(scaled, self.denominator)?;
        u64::try_from(ceiling)
            .map_err(|_| Error::msg("sample count exceeds supported range"))
    }
}

/// Returns the greatest common divisor for ratio reduction.
fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

/// Divides positive integers while rounding upward.
fn divide_ceiling(numerator: u128, denominator: u128) -> Result<u128, Error> {
    numerator
        .checked_add(denominator.saturating_sub(1))
        .map(|value| value / denominator)
        .ok_or_else(|| Error::msg("sample timeline ceiling overflow"))
}

/// Returns the number of configured intervals covering a duration.
pub(super) fn planned_interval_count(
    duration_seconds: f64, step_seconds: f64,
) -> Result<usize, Error> {
    if duration_seconds == 0.0 {
        return Ok(0);
    }
    let duration = Rational::from_positive_f64(duration_seconds, "duration")?;
    let step = Rational::from_positive_f64(step_seconds, "sample rate")?;
    let numerator = duration
        .numerator
        .checked_mul(step.denominator)
        .ok_or_else(|| Error::msg("simulation interval count overflow"))?;
    let denominator = duration
        .denominator
        .checked_mul(step.numerator)
        .ok_or_else(|| Error::msg("simulation interval count overflow"))?;
    let count = divide_ceiling(numerator, denominator)?;
    usize::try_from(count).map_err(|_| {
        Error::msg("simulation interval count exceeds supported range")
    })
}

/// One non-empty contiguous range of emitted complex samples.
#[derive(Debug)]
pub(super) struct TimelineBlock {
    /// Complex samples emitted by this block.
    pub sample_count: usize,
    /// Configured update-step endpoint associated with this block.
    pub step_index: usize,
    /// Sample offset of the configured step's starting epoch.
    pub step_start_sample: u64,
    /// Sample offset of the full configured step's ending epoch.
    pub step_end_sample: u64,
    /// Sample offset of this block's endpoint.
    pub end_sample: u64,
    /// Exact sample-derived duration of this block.
    pub duration_seconds: f64,
    /// GPS epoch after the final sample in this block.
    pub end_time: GpsTime,
    /// Exact LNAV frame deadlines reached at this block endpoint.
    pub frame_deadlines: Vec<GpsTime>,
}

impl TimelineBlock {
    /// Returns this endpoint's fraction of its full configured update step.
    pub fn step_endpoint_fraction(&self) -> f64 {
        let step_samples = self.step_end_sample - self.step_start_sample;
        let elapsed_samples = self.end_sample - self.step_start_sample;
        elapsed_samples as f64 / step_samples as f64
    }
}

/// One configured step retained while a frame deadline splits its samples.
#[derive(Clone, Copy, Debug)]
struct PendingStep {
    /// Sample offset of the step's starting epoch.
    start_sample: u64,
    /// Sample offset of the full, unclipped configured step endpoint.
    full_end_sample: u64,
    /// Sample offset at which this finite run stops processing the step.
    emitted_end_sample: u64,
}

/// Authoritative rational plan and integer emitted-sample cursor.
#[derive(Clone, Debug)]
pub(super) struct SampleTimeline {
    /// GPS epoch before any waveform sample is emitted.
    start_time: GpsTime,
    /// Integer complex-sample frequency.
    sample_frequency_hz: u64,
    /// Configured state-update interval.
    step: Rational,
    /// Finite emitted-sample target, or no target for runtime control.
    total_samples: Option<u64>,
    /// Number of complex samples emitted so far.
    emitted_samples: u64,
    /// Next configured update-step index.
    next_step_index: u64,
    /// Configured step retained when an exact frame deadline splits a block.
    pending_step: Option<PendingStep>,
    /// Sample offset of the next exact LNAV frame deadline.
    next_frame_sample: u64,
    /// GPS time of the next exact LNAV frame deadline.
    next_frame_time: GpsTime,
}

impl SampleTimeline {
    /// Builds a timeline from generator configuration and optional limits.
    pub fn new(
        start_time: GpsTime, sample_frequency_hz: f64, step_seconds: f64,
        duration_seconds: Option<f64>, interval_limit: Option<usize>,
    ) -> Result<Self, Error> {
        let sample_frequency_hz = sample_frequency_hz.round() as u64;
        if sample_frequency_hz == 0 {
            return Err(Error::msg(
                "sample frequency must be greater than zero",
            ));
        }
        let step = Rational::from_positive_f64(step_seconds, "sample rate")?;
        let start_seconds = if start_time.sec == 0.0 {
            Rational {
                numerator: 0,
                denominator: 1,
            }
        } else {
            Rational::from_positive_f64(start_time.sec, "GPS seconds")?
        };

        let duration_samples = duration_seconds
            .map(|duration| {
                if duration == 0.0 {
                    Ok(0)
                } else {
                    Rational::from_positive_f64(duration, "duration")?
                        .rounded_product(u128::from(sample_frequency_hz))
                }
            })
            .transpose()?;
        let limited_samples = interval_limit
            .map(|limit| {
                step.rounded_product(
                    u128::from(sample_frequency_hz)
                        .checked_mul(limit as u128)
                        .ok_or_else(|| {
                            Error::msg("sample timeline overflow")
                        })?,
                )
            })
            .transpose()?;
        let total_samples = match (duration_samples, limited_samples) {
            (Some(duration), Some(limit)) => Some(duration.min(limit)),
            (Some(duration), None) => Some(duration),
            (None, Some(limit)) => Some(limit),
            (None, None) => None,
        };

        let (next_frame_sample, next_frame_time) = first_frame_deadline(
            &start_time,
            start_seconds,
            sample_frequency_hz,
        )?;

        Ok(Self {
            start_time,
            sample_frequency_hz,
            step,
            total_samples,
            emitted_samples: 0,
            next_step_index: 0,
            pending_step: None,
            next_frame_sample,
            next_frame_time,
        })
    }

    /// Returns a fresh finite plan after this timeline has been fully emitted.
    pub fn restarted_if_exhausted(
        &self, start_time: GpsTime,
    ) -> Result<Option<Self>, Error> {
        if !self.is_exhausted() {
            return Ok(None);
        }

        let start_seconds = if start_time.sec == 0.0 {
            Rational {
                numerator: 0,
                denominator: 1,
            }
        } else {
            Rational::from_positive_f64(start_time.sec, "GPS seconds")?
        };
        let (next_frame_sample, next_frame_time) = first_frame_deadline(
            &start_time,
            start_seconds,
            self.sample_frequency_hz,
        )?;
        let mut restarted = self.clone();
        restarted.start_time = start_time;
        restarted.emitted_samples = 0;
        restarted.next_step_index = 0;
        restarted.pending_step = None;
        restarted.next_frame_sample = next_frame_sample;
        restarted.next_frame_time = next_frame_time;
        Ok(Some(restarted))
    }

    /// Returns whether every finite target sample has been emitted.
    pub fn is_exhausted(&self) -> bool {
        self.total_samples
            .is_some_and(|total| self.emitted_samples >= total)
    }

    /// Returns the capacity needed by any unsplit configured update block.
    pub fn maximum_block_samples(&self) -> Result<usize, Error> {
        let samples = self
            .step
            .ceiling_product(u128::from(self.sample_frequency_hz))?;
        usize::try_from(samples)
            .map_err(|_| Error::msg("I/Q block size exceeds supported range"))
    }

    /// Advances to the next duration-, step-, or frame-bounded sample block.
    pub fn next_block(&mut self) -> Result<Option<TimelineBlock>, Error> {
        if self.is_exhausted() {
            return Ok(None);
        }

        let pending_step = loop {
            if let Some(step) = self.pending_step {
                break step;
            }
            self.next_step_index = self
                .next_step_index
                .checked_add(1)
                .ok_or_else(|| Error::msg("sample timeline step overflow"))?;
            let previous_step_index = self.next_step_index - 1;
            let start_sample = self.step.rounded_product(
                u128::from(self.sample_frequency_hz)
                    .checked_mul(u128::from(previous_step_index))
                    .ok_or_else(|| Error::msg("sample timeline overflow"))?,
            )?;
            let full_end_sample = self.step.rounded_product(
                u128::from(self.sample_frequency_hz)
                    .checked_mul(u128::from(self.next_step_index))
                    .ok_or_else(|| Error::msg("sample timeline overflow"))?,
            )?;
            let emitted_end_sample = self
                .total_samples
                .map_or(full_end_sample, |total| full_end_sample.min(total));
            if emitted_end_sample > self.emitted_samples {
                let step = PendingStep {
                    start_sample,
                    full_end_sample,
                    emitted_end_sample,
                };
                self.pending_step = Some(step);
                break step;
            }
            if self.is_exhausted() {
                return Ok(None);
            }
        };

        let start_sample = self.emitted_samples;
        let end_sample =
            pending_step.emitted_end_sample.min(self.next_frame_sample);
        self.emitted_samples = end_sample;
        if end_sample == pending_step.emitted_end_sample {
            self.pending_step = None;
        }
        let mut frame_deadlines = Vec::new();
        if end_sample == self.next_frame_sample {
            frame_deadlines.push(self.next_frame_time.clone());
            self.next_frame_sample = self
                .next_frame_sample
                .checked_add(
                    FRAME_SECONDS
                        .checked_mul(self.sample_frequency_hz)
                        .ok_or_else(|| {
                            Error::msg("GPS frame sample overflow")
                        })?,
                )
                .ok_or_else(|| Error::msg("GPS frame sample overflow"))?;
            self.next_frame_time = next_frame_time(&self.next_frame_time);
        }

        let sample_count = end_sample - start_sample;
        Ok(Some(TimelineBlock {
            sample_count: usize::try_from(sample_count).map_err(|_| {
                Error::msg("I/Q block size exceeds supported range")
            })?,
            step_index: usize::try_from(self.next_step_index).map_err(
                |_| Error::msg("simulation step exceeds supported range"),
            )?,
            step_start_sample: pending_step.start_sample,
            step_end_sample: pending_step.full_end_sample,
            end_sample,
            duration_seconds: sample_count as f64
                / self.sample_frequency_hz as f64,
            end_time: self.time_at_sample(end_sample),
            frame_deadlines,
        }))
    }

    /// Converts an emitted-sample offset to its GPS endpoint.
    fn time_at_sample(&self, sample: u64) -> GpsTime {
        self.start_time
            .add_secs(sample as f64 / self.sample_frequency_hz as f64)
    }
}

/// Finds the first exact LNAV frame deadline after a timeline start epoch.
fn first_frame_deadline(
    start_time: &GpsTime, start_seconds: Rational, sample_frequency_hz: u64,
) -> Result<(u64, GpsTime), Error> {
    let frame_denominator = start_seconds
        .denominator
        .checked_mul(u128::from(FRAME_SECONDS))
        .ok_or_else(|| Error::msg("GPS frame calculation overflow"))?;
    let next_frame_seconds = (start_seconds.numerator / frame_denominator + 1)
        .checked_mul(u128::from(FRAME_SECONDS))
        .ok_or_else(|| Error::msg("GPS frame calculation overflow"))?;
    let difference_numerator = next_frame_seconds
        .checked_mul(start_seconds.denominator)
        .and_then(|value| value.checked_sub(start_seconds.numerator))
        .ok_or_else(|| Error::msg("GPS frame calculation overflow"))?;
    let next_frame_sample = divide_ceiling(
        difference_numerator
            .checked_mul(u128::from(sample_frequency_hz))
            .ok_or_else(|| Error::msg("GPS frame calculation overflow"))?,
        start_seconds.denominator,
    )?;
    let next_frame_sample = u64::try_from(next_frame_sample)
        .map_err(|_| Error::msg("GPS frame sample exceeds supported range"))?;
    let next_frame_time = if next_frame_seconds >= SECONDS_IN_WEEK as u128 {
        GpsTime {
            week: start_time.week + 1,
            sec: 0.0,
        }
    } else {
        GpsTime {
            week: start_time.week,
            sec: next_frame_seconds as f64,
        }
    };
    Ok((next_frame_sample, next_frame_time))
}

/// Advances an exact frame deadline across GPS-week rollover.
fn next_frame_time(time: &GpsTime) -> GpsTime {
    let seconds = time.sec + FRAME_SECONDS as f64;
    if seconds >= SECONDS_IN_WEEK {
        GpsTime {
            week: time.week + 1,
            sec: seconds - SECONDS_IN_WEEK,
        }
    } else {
        GpsTime {
            week: time.week,
            sec: seconds,
        }
    }
}

#[cfg(test)]
#[path = "timeline_tests.rs"]
mod tests;
