use constants::{MAX_CHAN, MAX_SAT, STATIC_MAX_DURATION};

use super::{SignalGenerator, run};
use crate::{
    Error,
    generator::{
        MotionMode,
        timeline::{SampleTimeline, validate_duration, validate_update_step},
    },
    io::IQWriter,
    table::ANT_PAT_DB,
};

impl SignalGenerator {
    /// Initializes the configured timeline, channels, and optional file writer.
    pub fn initialize(&mut self) -> Result<(), Error> {
        match self.mode {
            MotionMode::Static => tracing::info!("using static location mode"),
            MotionMode::Dynamic => {
                tracing::info!("using dynamic location mode");
            }
            MotionMode::UserControl => {
                tracing::info!("using runtime motion control mode");
                if self.runtime_motion_control.is_none() {
                    return Err(Error::msg(
                        "runtime motion control not configured",
                    ));
                }
            }
        }

        validate_update_step(self.sample_rate)?;
        validate_duration(self.duration_seconds)?;
        if matches!(self.mode, MotionMode::Static)
            && self
                .duration_seconds
                .is_some_and(|duration| duration > STATIC_MAX_DURATION as f64)
        {
            return Err(Error::invalid_duration());
        }

        let interval_limit = match self.mode {
            MotionMode::Static if self.duration_seconds.is_none() => {
                Some(self.simulation_step_count)
            }
            MotionMode::Dynamic => Some(self.simulation_step_count),
            MotionMode::Static | MotionMode::UserControl => None,
        };
        let duration_seconds = if matches!(self.mode, MotionMode::UserControl) {
            None
        } else {
            self.duration_seconds
        };
        let timeline = SampleTimeline::new(
            self.receiver_gps_time.clone(),
            self.sample_frequency,
            self.sample_rate,
            duration_seconds,
            interval_limit,
        )?;
        let block_sizing = timeline.maximum_block_sizing()?;

        if let Some(first_position) = self.positions.first() {
            tracing::info!(
                x = first_position.x,
                y = first_position.y,
                z = first_position.z,
                "initial receiver position (ECEF)"
            );
        }
        let gps_calendar_start = self.receiver_gps_time.to_gps_calendar()?;
        tracing::info!(
            year = gps_calendar_start.year(),
            month = gps_calendar_start.month(),
            day = gps_calendar_start.day(),
            hour = gps_calendar_start.hour(),
            minute = gps_calendar_start.minute(),
            second = gps_calendar_start.second(),
            gps_week = self.receiver_gps_time.week,
            gps_seconds = self.receiver_gps_time.sec,
            "start time (GPS calendar)"
        );

        self.channels
            .iter_mut()
            .take(MAX_CHAN)
            .for_each(|channel| channel.prn = 0);
        self.allocated_satellite
            .iter_mut()
            .take(MAX_SAT)
            .for_each(|satellite| *satellite = -1);
        let initial_position = self
            .positions
            .first()
            .copied()
            .ok_or_else(Error::wrong_positions)?;
        self.allocate_channel(initial_position)?;
        if self.verbose {
            Self::log_channel_status(&self.channels);
        }

        for (index, gain) in self.antenna_pattern.iter_mut().enumerate() {
            *gain = 10.0f64.powf(-ANT_PAT_DB[index] / 20.0);
        }

        let writer = match &self.output_file {
            Some(file) => Some(IQWriter::new(
                file,
                self.data_format,
                block_sizing.complex_samples(),
            )?),
            None => None,
        };
        self.iq_buffer_size = block_sizing.complex_samples();
        self.writer = writer;
        self.timeline = Some(timeline);
        self.finite_run_state = run::FiniteRunState::Ready;
        self.initialized = true;
        Ok(())
    }
}
