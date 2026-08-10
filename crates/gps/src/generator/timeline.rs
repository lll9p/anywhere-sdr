use constants::SECONDS_IN_WEEK;

use super::rational::{Rational, divide_ceiling};
use crate::{Error, GpsTime, IqBlockSizing};

/// Duration of one complete LNAV frame.
const FRAME_SECONDS: u64 = 30;

/// Validates the configured state-update step.
pub(super) fn validate_update_step(step_seconds: f64) -> Result<(), Error> {
    if !step_seconds.is_finite() || step_seconds <= 0.0 {
        return Err(Error::invalid_update_step(step_seconds));
    }
    Ok(())
}

/// Validates the common duration domain before mode-specific planning.
pub(super) fn validate_duration(
    duration_seconds: Option<f64>,
) -> Result<(), Error> {
    if duration_seconds
        .is_some_and(|duration| !duration.is_finite() || duration < 0.0)
    {
        return Err(Error::invalid_duration());
    }
    Ok(())
}

/// Returns the number of configured intervals covering a duration.
pub(super) fn planned_interval_count(
    duration_seconds: f64, step_seconds: f64, interval_limit: Option<usize>,
) -> Result<usize, Error> {
    validate_duration(Some(duration_seconds))?;
    validate_update_step(step_seconds)?;
    if duration_seconds == 0.0 || interval_limit == Some(0) {
        return Ok(0);
    }

    let ratio = duration_seconds / step_seconds;
    if let Some(limit) = interval_limit
        && (!ratio.is_finite() || ratio >= limit as f64)
    {
        return Ok(limit);
    }
    if !ratio.is_finite() {
        return Err(Error::unsupported_workload(
            "simulation interval count exceeds supported range",
        ));
    }
    if ratio == 0.0 {
        return Ok(1);
    }

    let ratio = Rational::from_positive_f64(ratio, "duration/step")?;
    let count = divide_ceiling(ratio.numerator, ratio.denominator)?;
    let count = usize::try_from(count).map_err(|_| {
        Error::unsupported_workload(
            "simulation interval count exceeds supported range",
        )
    })?;
    Ok(interval_limit.map_or(count, |limit| count.min(limit)))
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
    /// Exact sample-derived elapsed time at this block endpoint.
    pub end_elapsed_seconds: f64,
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
        validate_duration(duration_seconds)?;
        validate_update_step(step_seconds)?;
        IqBlockSizing::validate_update_step(sample_frequency_hz, step_seconds)?;
        let sample_frequency_hz = sample_frequency_hz.round() as u64;
        let step = Rational::from_positive_f64(step_seconds, "sample rate")?;
        if step.numerator == 0 {
            return Err(Error::unsupported_workload(
                "configured update step is below supported timeline resolution",
            ));
        }
        let start_seconds = if start_time.sec == 0.0 {
            Rational {
                numerator: 0,
                denominator: 1,
            }
        } else {
            Rational::from_positive_f64(start_time.sec, "GPS seconds")?
        };

        let limited_samples = interval_limit
            .map(|limit| {
                let limit = u128::try_from(limit).map_err(|_| {
                    Error::unsupported_workload(
                        "simulation interval limit exceeds supported range",
                    )
                })?;
                step.rounded_product(
                    u128::from(sample_frequency_hz)
                        .checked_mul(limit)
                        .ok_or_else(|| {
                            Error::unsupported_workload(
                                "sample timeline interval limit overflow",
                            )
                        })?,
                )
            })
            .transpose()?;
        let duration_samples = duration_seconds
            .map(|duration| {
                if duration == 0.0 {
                    return Ok(0);
                }
                if let (Some(limit), Some(limited_samples)) =
                    (interval_limit, limited_samples)
                    && (duration / step_seconds >= limit as f64)
                {
                    return Ok(limited_samples);
                }
                Rational::from_positive_f64(duration, "duration")?
                    .rounded_product(u128::from(sample_frequency_hz))
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

    /// Returns checked dimensions for any unsplit configured update block.
    pub fn maximum_block_sizing(&self) -> Result<IqBlockSizing, Error> {
        let samples = self
            .step
            .ceiling_product(u128::from(self.sample_frequency_hz))?;
        let samples = usize::try_from(samples).map_err(|_| {
            Error::unsupported_workload(
                "I/Q block size exceeds supported range",
            )
        })?;
        IqBlockSizing::new(samples)
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
            self.next_step_index =
                self.next_step_index.checked_add(1).ok_or_else(|| {
                    Error::unsupported_workload("sample timeline step overflow")
                })?;
            let previous_step_index = self.next_step_index - 1;
            let start_sample = self.step.rounded_product(
                u128::from(self.sample_frequency_hz)
                    .checked_mul(u128::from(previous_step_index))
                    .ok_or_else(|| {
                        Error::unsupported_workload(
                            "sample timeline step multiplication overflow",
                        )
                    })?,
            )?;
            let full_end_sample = self.step.rounded_product(
                u128::from(self.sample_frequency_hz)
                    .checked_mul(u128::from(self.next_step_index))
                    .ok_or_else(|| {
                        Error::unsupported_workload(
                            "sample timeline step multiplication overflow",
                        )
                    })?,
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
                            Error::unsupported_workload(
                                "GPS frame sample multiplication overflow",
                            )
                        })?,
                )
                .ok_or_else(|| {
                    Error::unsupported_workload(
                        "GPS frame sample offset overflow",
                    )
                })?;
            self.next_frame_time = next_frame_time(&self.next_frame_time);
        }

        let sample_count = end_sample - start_sample;
        let sample_count = usize::try_from(sample_count).map_err(|_| {
            Error::unsupported_workload(
                "I/Q block size exceeds supported range",
            )
        })?;
        let sample_count = IqBlockSizing::new(sample_count)?.complex_samples();
        Ok(Some(TimelineBlock {
            sample_count,
            step_index: usize::try_from(self.next_step_index).map_err(
                |_| {
                    Error::unsupported_workload(
                        "simulation step exceeds supported range",
                    )
                },
            )?,
            step_start_sample: pending_step.start_sample,
            step_end_sample: pending_step.full_end_sample,
            end_sample,
            end_elapsed_seconds: end_sample as f64
                / self.sample_frequency_hz as f64,
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
    let frame_overflow =
        || Error::unsupported_workload("GPS frame sample calculation overflow");
    let frame_denominator = start_seconds
        .denominator
        .checked_mul(u128::from(FRAME_SECONDS))
        .ok_or_else(&frame_overflow)?;
    let next_frame_seconds = (start_seconds.numerator / frame_denominator + 1)
        .checked_mul(u128::from(FRAME_SECONDS))
        .ok_or_else(&frame_overflow)?;
    let difference_numerator = next_frame_seconds
        .checked_mul(start_seconds.denominator)
        .and_then(|value| value.checked_sub(start_seconds.numerator))
        .ok_or_else(&frame_overflow)?;
    let next_frame_sample = divide_ceiling(
        difference_numerator
            .checked_mul(u128::from(sample_frequency_hz))
            .ok_or_else(&frame_overflow)?,
        start_seconds.denominator,
    )?;
    let next_frame_sample = u64::try_from(next_frame_sample).map_err(|_| {
        Error::unsupported_workload("GPS frame sample exceeds supported range")
    })?;
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
