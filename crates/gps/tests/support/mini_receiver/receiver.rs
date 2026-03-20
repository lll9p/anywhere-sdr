use std::f64::consts::PI;

use constants::{
    CA_SEQ_LEN_FLOAT, CARR_TO_CODE, CODE_FREQ, LAMBDA_L1_INV, MAX_SAT,
    SECONDS_IN_WEEK, SPEED_OF_LIGHT_INV,
};
use geometry::Ecef;
use gps::{BroadcastEphemeris, Error, GpsTime, IonoUtc, compute_range};

use super::{AcquisitionMetric, FixedScenario, VisibleSatellite, ca_code};

const ACQUISITION_WINDOW_MS: usize = 4;
const ACQUISITION_CODE_WINDOW_CHIPS: i32 = 32;
const ACQUISITION_DOPPLER_BINS: [f64; 11] = [
    -250.0, -200.0, -150.0, -100.0, -50.0, 0.0, 50.0, 100.0, 150.0, 200.0,
    250.0,
];

#[derive(Clone, Debug)]
pub struct TrackingAssist {
    pub prn: usize,
    pub predicted_carrier_hz: f64,
    pub predicted_code_phase_chips: f64,
}

#[derive(Clone, Debug)]
pub struct PromptEpoch {
    pub start_time: GpsTime,
    pub value_re: f64,
    pub value_im: f64,
}

#[derive(Clone, Debug)]
pub struct TrackedSatellite {
    pub prn: usize,
    pub prompt_epochs: Vec<PromptEpoch>,
}

struct TrackerState {
    prn: usize,
    ephemeris: BroadcastEphemeris,
    acquisition: AcquisitionMetric,
    ca_code: [i8; constants::CA_SEQ_LEN],
    code_phase: f64,
    code_phase_step: f64,
    carrier_re: f64,
    carrier_im: f64,
    carrier_step_re: f64,
    carrier_step_im: f64,
    epoch_accum_re: f64,
    epoch_accum_im: f64,
    current_epoch_start_sample: f64,
    prompt_epochs: Vec<PromptEpoch>,
}

struct TrackingContext {
    receiver_position: Ecef,
    ionoutc: IonoUtc,
    start_time: GpsTime,
    sample_frequency_hz: f64,
    sample_rate_seconds: f64,
}

impl TrackerState {
    fn new(
        scenario: &FixedScenario, acquisition: AcquisitionMetric,
    ) -> Result<Self, Error> {
        let acquired_code_phase_chips = acquisition.acquired_code_phase_chips;
        let ephemeris = scenario
            .expected_ephemeris(acquisition.prn)
            .cloned()
            .ok_or(Error::NoEphemeris)?;
        let ca_code = ca_code(acquisition.prn).map_err(Error::msg)?;
        let initial_code_period_samples = scenario.sample_frequency_hz()
            * acquired_code_phase_chips
            / CODE_FREQ;

        Ok(Self {
            prn: acquisition.prn,
            ephemeris,
            acquisition,
            ca_code,
            code_phase: acquired_code_phase_chips,
            code_phase_step: 0.0,
            carrier_re: 1.0,
            carrier_im: 0.0,
            carrier_step_re: 1.0,
            carrier_step_im: 0.0,
            epoch_accum_re: 0.0,
            epoch_accum_im: 0.0,
            current_epoch_start_sample: -initial_code_period_samples,
            prompt_epochs: Vec::new(),
        })
    }

    fn update_tracking_rates(
        &mut self, context: &TrackingContext, block_time: &GpsTime,
    ) {
        let block_end = block_time.add_secs(context.sample_rate_seconds);
        let range_now = compute_range(
            &self.ephemeris,
            &context.ionoutc,
            block_time,
            &context.receiver_position,
        );
        let range_next = compute_range(
            &self.ephemeris,
            &context.ionoutc,
            &block_end,
            &context.receiver_position,
        );
        let range_rate =
            (range_next.range - range_now.range) / context.sample_rate_seconds;
        let carrier_hz = -range_rate * LAMBDA_L1_INV
            + (self.acquisition.acquired_carrier_hz
                - self.acquisition.predicted_carrier_hz);
        let code_hz = CODE_FREQ + carrier_hz * CARR_TO_CODE;
        let carrier_phase_step =
            -2.0 * PI * carrier_hz / context.sample_frequency_hz;

        self.code_phase_step = code_hz / context.sample_frequency_hz;
        self.carrier_step_re = carrier_phase_step.cos();
        self.carrier_step_im = carrier_phase_step.sin();
    }

    fn process_block(
        &mut self, context: &TrackingContext, block_index: usize,
        block_time: &GpsTime, samples: &[i16],
    ) {
        self.update_tracking_rates(context, block_time);
        let capture_start = &context.start_time;
        let samples_per_complex = 2usize;
        let complex_sample_count = samples.len() / samples_per_complex;
        let block_start_sample = block_index * complex_sample_count;

        for sample_index in 0..complex_sample_count {
            let raw_i = f64::from(samples[sample_index * 2]);
            let raw_q = f64::from(samples[sample_index * 2 + 1]);
            let code = f64::from(self.ca_code[self.code_phase as usize]);
            let mixed_re = raw_i * self.carrier_re - raw_q * self.carrier_im;
            let mixed_im = raw_q * self.carrier_re + raw_i * self.carrier_im;

            self.epoch_accum_re += mixed_re * code;
            self.epoch_accum_im += mixed_im * code;

            let next_carrier_re = self.carrier_re * self.carrier_step_re
                - self.carrier_im * self.carrier_step_im;
            let next_carrier_im = self.carrier_re * self.carrier_step_im
                + self.carrier_im * self.carrier_step_re;
            self.carrier_re = next_carrier_re;
            self.carrier_im = next_carrier_im;

            self.code_phase += self.code_phase_step;
            if self.code_phase >= CA_SEQ_LEN_FLOAT {
                self.code_phase -= CA_SEQ_LEN_FLOAT;
                let epoch_start_sample = self.current_epoch_start_sample;
                if epoch_start_sample >= 0.0 {
                    let epoch_start_time = add_secs_precise(
                        capture_start,
                        epoch_start_sample / context.sample_frequency_hz,
                    );
                    self.prompt_epochs.push(PromptEpoch {
                        start_time: epoch_start_time,
                        value_re: self.epoch_accum_re,
                        value_im: self.epoch_accum_im,
                    });
                }

                self.epoch_accum_re = 0.0;
                self.epoch_accum_im = 0.0;
                self.current_epoch_start_sample =
                    (block_start_sample + sample_index + 1) as f64;
            }
        }
    }
}

pub fn build_tracking_assists(
    scenario: &FixedScenario, visible_satellites: &[VisibleSatellite],
) -> Result<Vec<TrackingAssist>, Error> {
    let mut assists = Vec::with_capacity(visible_satellites.len());
    let receiver_position = scenario.receiver_position();
    let navigation_start_time = aligned_navigation_start(scenario.start_time());
    let next_block_time = scenario
        .start_time()
        .add_secs(scenario.sample_rate_seconds());

    for satellite in visible_satellites {
        let ephemeris = scenario
            .expected_ephemeris(satellite.prn)
            .ok_or(Error::NoEphemeris)?;
        let range_now = compute_range(
            ephemeris,
            scenario.ionoutc(),
            scenario.start_time(),
            &receiver_position,
        );
        let range_next = compute_range(
            ephemeris,
            scenario.ionoutc(),
            &next_block_time,
            &receiver_position,
        );
        let range_rate = (range_next.range - range_now.range)
            / scenario.sample_rate_seconds();
        let predicted_carrier_hz = -range_rate * LAMBDA_L1_INV;
        let predicted_code_phase_chips = predicted_code_phase(
            scenario.start_time(),
            &navigation_start_time,
            range_now.range,
        );

        assists.push(TrackingAssist {
            prn: satellite.prn,
            predicted_carrier_hz,
            predicted_code_phase_chips,
        });
    }

    Ok(assists)
}

pub fn assisted_acquisition(
    first_block: &[i16], sample_frequency_hz: f64, assists: &[TrackingAssist],
) -> Result<Vec<AcquisitionMetric>, Error> {
    let samples_per_ms = (sample_frequency_hz / 1000.0).round() as usize;
    let usable_complex_samples =
        ACQUISITION_WINDOW_MS.saturating_mul(samples_per_ms);
    let available_complex_samples = first_block.len() / 2;
    let search_complex_samples =
        available_complex_samples.min(usable_complex_samples);
    if search_complex_samples == 0 {
        return Err(Error::msg("acquisition input block is empty"));
    }

    let mut metrics = Vec::with_capacity(assists.len());
    for assist in assists {
        let code = ca_code(assist.prn).map_err(Error::msg)?;
        let mut best_metric = f64::MIN;
        let mut best_frequency = assist.predicted_carrier_hz;
        let mut best_code_phase = assist.predicted_code_phase_chips;
        let mut all_metrics = Vec::new();

        for doppler_offset_hz in ACQUISITION_DOPPLER_BINS {
            let candidate_frequency =
                assist.predicted_carrier_hz + doppler_offset_hz;
            let carrier_phase_step =
                2.0 * PI * candidate_frequency / sample_frequency_hz;

            for code_offset in
                -ACQUISITION_CODE_WINDOW_CHIPS..=ACQUISITION_CODE_WINDOW_CHIPS
            {
                let mut code_phase =
                    assist.predicted_code_phase_chips + f64::from(code_offset);
                while code_phase < 0.0 {
                    code_phase += CA_SEQ_LEN_FLOAT;
                }
                while code_phase >= CA_SEQ_LEN_FLOAT {
                    code_phase -= CA_SEQ_LEN_FLOAT;
                }

                let mut lo_phase = 0.0f64;
                let mut current_code_phase = code_phase;
                let mut metric = 0.0;
                let mut sum_re = 0.0;
                let mut sum_im = 0.0;

                for sample_index in 0..search_complex_samples {
                    let i_sample = f64::from(first_block[sample_index * 2]);
                    let q_sample = f64::from(first_block[sample_index * 2 + 1]);
                    let carrier_cos = lo_phase.cos();
                    let carrier_sin = lo_phase.sin();
                    let mixed_re =
                        i_sample * carrier_cos + q_sample * carrier_sin;
                    let mixed_im =
                        q_sample * carrier_cos - i_sample * carrier_sin;
                    let code_chip =
                        f64::from(code[current_code_phase as usize]);

                    sum_re += mixed_re * code_chip;
                    sum_im += mixed_im * code_chip;
                    lo_phase += carrier_phase_step;
                    current_code_phase += CODE_FREQ / sample_frequency_hz;
                    if current_code_phase >= CA_SEQ_LEN_FLOAT {
                        current_code_phase -= CA_SEQ_LEN_FLOAT;
                    }

                    if (sample_index + 1) % samples_per_ms == 0 {
                        metric += sum_re * sum_re + sum_im * sum_im;
                        sum_re = 0.0;
                        sum_im = 0.0;
                    }
                }

                all_metrics.push(metric);
                if metric > best_metric {
                    best_metric = metric;
                    best_frequency = candidate_frequency;
                    best_code_phase = code_phase;
                }
            }
        }

        let noise_floor = if all_metrics.len() <= 1 {
            best_metric
        } else {
            let total: f64 = all_metrics.iter().copied().sum();
            (total - best_metric) / (all_metrics.len() - 1) as f64
        };
        let peak_ratio = if noise_floor > 0.0 {
            best_metric / noise_floor
        } else {
            f64::INFINITY
        };

        metrics.push(AcquisitionMetric {
            prn: assist.prn,
            predicted_carrier_hz: assist.predicted_carrier_hz,
            predicted_code_phase_chips: assist.predicted_code_phase_chips,
            acquired_carrier_hz: best_frequency,
            acquired_code_phase_chips: best_code_phase,
            peak_ratio,
        });
    }

    metrics.sort_by(|left, right| left.prn.cmp(&right.prn));
    Ok(metrics)
}

pub fn track_satellites(
    scenario: &mut FixedScenario, acquisitions: &[AcquisitionMetric],
) -> Result<Vec<TrackedSatellite>, Error> {
    let tracking_context = TrackingContext {
        receiver_position: scenario.receiver_position(),
        ionoutc: scenario.ionoutc().clone(),
        start_time: scenario.start_time().clone(),
        sample_frequency_hz: scenario.sample_frequency_hz(),
        sample_rate_seconds: scenario.sample_rate_seconds(),
    };
    let mut tracker_states: [Option<TrackerState>; MAX_SAT] =
        std::array::from_fn(|_| None);
    for acquisition in acquisitions {
        let state = TrackerState::new(scenario, acquisition.clone())?;
        tracker_states[acquisition.prn - 1] = Some(state);
    }

    scenario.run_streaming::<_, Error>(|block_index, block_time, iq| {
        for tracker_state in tracker_states.iter_mut().flatten() {
            tracker_state.process_block(
                &tracking_context,
                block_index,
                block_time,
                iq,
            );
        }
        Ok(())
    })?;

    let mut tracked = Vec::new();
    for tracker_state in tracker_states.into_iter().flatten() {
        tracked.push(TrackedSatellite {
            prn: tracker_state.prn,
            prompt_epochs: tracker_state.prompt_epochs,
        });
    }
    tracked.sort_by(|left, right| left.prn.cmp(&right.prn));
    Ok(tracked)
}

fn aligned_navigation_start(time: &GpsTime) -> GpsTime {
    let aligned_seconds = f64::from((time.sec + 0.5) as u32 / 30) * 30.0;
    GpsTime {
        week: time.week,
        sec: aligned_seconds,
    }
}

fn predicted_code_phase(
    current_time: &GpsTime, navigation_start: &GpsTime, pseudorange_m: f64,
) -> f64 {
    let milliseconds = (current_time.diff_secs(navigation_start) + 6.0
        - pseudorange_m * SPEED_OF_LIGHT_INV)
        * 1000.0;
    let mut code_phase = milliseconds.fract() * CA_SEQ_LEN_FLOAT;
    while code_phase < 0.0 {
        code_phase += CA_SEQ_LEN_FLOAT;
    }
    code_phase
}

fn add_secs_precise(time: &GpsTime, dt: f64) -> GpsTime {
    let mut week = time.week;
    let mut sec = time.sec + dt;

    while sec >= SECONDS_IN_WEEK {
        sec -= SECONDS_IN_WEEK;
        week += 1;
    }
    while sec < 0.0 {
        sec += SECONDS_IN_WEEK;
        week -= 1;
    }

    GpsTime { week, sec }
}

#[cfg(test)]
mod tests {
    use gps::GpsTime;

    use super::add_secs_precise;

    #[test]
    fn add_secs_precise_preserves_sample_scale_offsets() {
        let time = GpsTime {
            week: 2190,
            sec: 123_456.0,
        };
        let sample_offset_seconds = 1_000.0 / 2_600_000.0;

        let rounded = time.add_secs(sample_offset_seconds);
        let precise = add_secs_precise(&time, sample_offset_seconds);

        assert!(
            (rounded.diff_secs(&time) - sample_offset_seconds).abs() > 1.0e-4,
            "GpsTime::add_secs unexpectedly preserved sample-scale precision"
        );
        assert!(
            (precise.diff_secs(&time) - sample_offset_seconds).abs() < 1.0e-12,
            "add_secs_precise lost sample-scale precision"
        );
    }
}
