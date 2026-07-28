use constants::{MAX_CHAN, MAX_SAT, SECONDS_IN_HOUR};
use geometry::Ecef;

use super::SignalGenerator;
use crate::{
    Error, GpsTime, IqBlockSizing,
    generator::{
        MotionMode, motion_control::MotionIntegrator, timeline::TimelineBlock,
    },
};

/// Outcome of the most recent finite run invocation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::generator) enum FiniteRunState {
    /// Initialized timeline has not completed an invocation.
    #[default]
    Ready,
    /// An invocation returned before successful timeline exhaustion.
    Interrupted,
    /// An invocation successfully exhausted the finite timeline.
    Completed,
}

impl SignalGenerator {
    /// Applies navigation and allocation work at an exact frame deadline.
    fn handle_periodic_tasks(
        &mut self, current_location: Ecef, deadline: &GpsTime,
    ) -> Result<(), Error> {
        for channel in self.channels.iter_mut().take(MAX_CHAN) {
            if channel.prn != 0 {
                channel.generate_nav_msg(deadline, false);
            }
        }

        let mut refreshed_ephemerides = false;
        let next_ephemeris_set_index = self.valid_ephemerides_index + 1;
        if next_ephemeris_set_index < self.ephemerides.len()
            && self.ephemerides[next_ephemeris_set_index]
                .iter()
                .take(MAX_SAT)
                .any(|ephemeris| ephemeris.vflg)
            && self.ephemerides[next_ephemeris_set_index][0].vflg
        {
            let difference = self.ephemerides[next_ephemeris_set_index][0]
                .toc
                .diff_secs(deadline);
            if difference.abs() < SECONDS_IN_HOUR {
                self.valid_ephemerides_index = next_ephemeris_set_index;
                refreshed_ephemerides = true;
                tracing::info!(
                    next_ephemeris_set_index,
                    "switched to ephemeris set"
                );
            }
        }

        if refreshed_ephemerides {
            let current_ephemeris_set_index = self.valid_ephemerides_index;
            for channel in self
                .channels
                .iter_mut()
                .take(MAX_CHAN)
                .filter(|channel| channel.prn != 0)
            {
                let satellite_index = channel.prn - 1;
                channel.generate_navigation_subframes(
                    &self.ephemerides[current_ephemeris_set_index]
                        [satellite_index],
                    &self.ionoutc,
                );
            }
        }

        self.allocate_channel_at(current_location, deadline)?;
        if self.verbose {
            Self::log_channel_status(&self.channels);
        }
        Ok(())
    }

    /// Returns the next non-empty block from the initialized timeline.
    fn next_timeline_block(&mut self) -> Result<Option<TimelineBlock>, Error> {
        self.timeline
            .as_mut()
            .ok_or_else(|| Error::msg("sample timeline not initialized"))?
            .next_block()
    }

    /// Starts or resumes one finite run invocation without replaying errors.
    fn begin_finite_run(&mut self) -> Result<(), Error> {
        match self.finite_run_state {
            FiniteRunState::Completed => {
                let restarted = self
                    .timeline
                    .as_ref()
                    .ok_or_else(|| {
                        Error::msg("sample timeline not initialized")
                    })?
                    .restarted_if_exhausted(self.receiver_gps_time.clone())?
                    .ok_or_else(|| {
                        Error::msg("completed finite timeline is not exhausted")
                    })?;
                self.timeline = Some(restarted);
            }
            FiniteRunState::Interrupted => {
                let exhausted = self
                    .timeline
                    .as_ref()
                    .ok_or_else(|| {
                        Error::msg("sample timeline not initialized")
                    })?
                    .is_exhausted();
                if exhausted {
                    return Err(Error::FiniteRunInterruptedAtEnd);
                }
            }
            FiniteRunState::Ready => {}
        }
        self.finite_run_state = FiniteRunState::Interrupted;
        Ok(())
    }

    /// Records successful exhaustion of the active finite invocation.
    fn complete_finite_run(&mut self) {
        self.finite_run_state = FiniteRunState::Completed;
    }

    /// Resolves the receiver position used at a finite block endpoint.
    fn finite_block_location(
        &self, block: &TimelineBlock,
    ) -> Result<Ecef, Error> {
        match self.mode {
            MotionMode::Static => self
                .positions
                .first()
                .copied()
                .ok_or_else(Error::wrong_positions),
            MotionMode::Dynamic => {
                let start_index = block
                    .step_index
                    .checked_sub(1)
                    .ok_or_else(Error::wrong_positions)?;
                let start = self
                    .positions
                    .get(start_index)
                    .ok_or_else(Error::wrong_positions)?;
                let end = self
                    .positions
                    .get(block.step_index)
                    .ok_or_else(Error::wrong_positions)?;
                let fraction = block.step_endpoint_fraction();
                Ok(Ecef::new(
                    start.x + (end.x - start.x) * fraction,
                    start.y + (end.y - start.y) * fraction,
                    start.z + (end.z - start.z) * fraction,
                ))
            }
            MotionMode::UserControl => Err(Error::msg(
                "finite block location requested in runtime motion mode",
            )),
        }
    }

    /// Advances GPS and channel state to a block endpoint.
    fn prepare_block(
        &mut self, block: &TimelineBlock, current_location: Ecef,
    ) -> Result<(), Error> {
        self.receiver_gps_time = block.end_time.clone();
        self.update_channel_parameters(
            current_location,
            block.duration_seconds,
        )?;
        Ok(())
    }

    /// Runs every exact frame deadline reached by the emitted block.
    fn finish_block(
        &mut self, block: TimelineBlock, current_location: Ecef,
    ) -> Result<(), Error> {
        for deadline in &block.frame_deadlines {
            self.handle_periodic_tasks(current_location, deadline)?;
        }
        Ok(())
    }

    /// Runs the GPS signal simulation and writes every emitted sample.
    pub fn run_simulation(&mut self) -> Result<(), Error> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }
        if matches!(self.mode, MotionMode::UserControl) {
            return Err(Error::msg(
                "run_simulation is not supported in runtime motion control \
                 mode",
            ));
        }
        self.begin_finite_run()?;

        tracing::info!(
            requested_intervals = self.simulation_step_count,
            "starting signal generation"
        );
        let time_start = std::time::Instant::now();
        let mut emitted_blocks = 0usize;

        while let Some(block) = self.next_timeline_block()? {
            let current_location = self.finite_block_location(&block)?;
            self.prepare_block(&block, current_location)?;
            self.generate_and_write_samples(block.sample_count)?;
            emitted_blocks += 1;
            if self.verbose && emitted_blocks.is_multiple_of(100) {
                tracing::debug!(
                    emitted_blocks,
                    gps_week = self.receiver_gps_time.week,
                    gps_seconds = self.receiver_gps_time.sec,
                    "simulation progress"
                );
            }
            self.finish_block(block, current_location)?;
        }
        if let Some(writer) = &mut self.writer {
            writer.finish_packing()?;
        }
        self.complete_finite_run();

        tracing::info!(emitted_blocks, "done");
        tracing::info!(
            process_seconds = time_start.elapsed().as_secs_f32(),
            "process time"
        );
        Ok(())
    }

    /// Streams interleaved I/Q blocks on the emitted-sample timeline.
    pub fn run_streaming<F, E>(&mut self, mut on_block: F) -> Result<(), E>
    where
        F: FnMut(&[i16]) -> Result<(), E>,
        E: From<Error>,
    {
        if !self.initialized {
            return Err(Error::NotInitialized.into());
        }
        if matches!(self.mode, MotionMode::UserControl) {
            return Err(Error::msg(
                "run_streaming is not supported in runtime motion control mode",
            )
            .into());
        }
        self.begin_finite_run().map_err(E::from)?;

        let buffer_capacity = IqBlockSizing::new(self.iq_buffer_size)
            .map_err(E::from)?
            .interleaved_i16_len();
        let mut iq_buffer = Vec::with_capacity(buffer_capacity);
        while let Some(block) = self.next_timeline_block().map_err(E::from)? {
            let current_location =
                self.finite_block_location(&block).map_err(E::from)?;
            self.prepare_block(&block, current_location)
                .map_err(E::from)?;
            let block_len = IqBlockSizing::new(block.sample_count)
                .map_err(E::from)?
                .interleaved_i16_len();
            iq_buffer.resize(block_len, 0);
            Self::generate_samples_into(
                &mut self.channels,
                &self.antenna_gains,
                &mut iq_buffer,
            )
            .map_err(E::from)?;
            self.finish_block(block, current_location)
                .map_err(E::from)?;
            on_block(&iq_buffer)?;
        }
        self.complete_finite_run();
        Ok(())
    }

    /// Runs streaming with runtime motion control until the callback stops it.
    pub fn run_streaming_user_control<F, E>(
        &mut self, mut on_block: F,
    ) -> Result<(), E>
    where
        F: FnMut(&[i16]) -> Result<(), E>,
        E: From<Error>,
    {
        if !self.initialized {
            return Err(Error::NotInitialized.into());
        }
        if !matches!(self.mode, MotionMode::UserControl) {
            return Err(Error::msg(
                "generator is not in runtime motion control mode",
            )
            .into());
        }
        let Some(control) = self.runtime_motion_control.clone() else {
            return Err(
                Error::msg("runtime motion control not configured").into()
            );
        };
        let initial_position = self
            .positions
            .first()
            .copied()
            .ok_or_else(Error::wrong_positions)
            .map_err(E::from)?;
        let mut integrator = MotionIntegrator::new(initial_position);
        let buffer_capacity = IqBlockSizing::new(self.iq_buffer_size)
            .map_err(E::from)?
            .interleaved_i16_len();
        let mut iq_buffer = Vec::with_capacity(buffer_capacity);

        loop {
            let block = self
                .next_timeline_block()
                .map_err(E::from)?
                .ok_or_else(|| {
                    E::from(Error::msg(
                        "runtime motion timeline ended unexpectedly",
                    ))
                })?;
            let current_location = integrator
                .step(block.duration_seconds, &control)
                .map_err(E::from)?;
            self.prepare_block(&block, current_location)
                .map_err(E::from)?;
            let block_len = IqBlockSizing::new(block.sample_count)
                .map_err(E::from)?
                .interleaved_i16_len();
            iq_buffer.resize(block_len, 0);
            Self::generate_samples_into(
                &mut self.channels,
                &self.antenna_gains,
                &mut iq_buffer,
            )
            .map_err(E::from)?;
            on_block(&iq_buffer)?;
            self.finish_block(block, current_location)
                .map_err(E::from)?;
        }
    }
}
