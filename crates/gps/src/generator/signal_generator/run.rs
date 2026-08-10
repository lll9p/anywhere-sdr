use constants::MAX_CHAN;
use geometry::Ecef;

use super::SignalGenerator;
use crate::{
    Error, GpsTime, IqBlockSizing,
    error::resolve_with_finalization,
    generator::{
        MotionMode, motion_control::MotionIntegrator, timeline::TimelineBlock,
        utils::ephemeris_set_matches_time,
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
    /// Direct output failed and cannot safely continue on the same stream.
    OutputFailed,
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

        let next_ephemeris_set_index = self
            .valid_ephemerides_index
            .checked_add(1)
            .filter(|&index| {
                self.ephemerides.get(index).is_some_and(|set| {
                    ephemeris_set_matches_time(set, deadline)
                })
            });

        if let Some(next_ephemeris_set_index) = next_ephemeris_set_index {
            self.valid_ephemerides_index = next_ephemeris_set_index;
            tracing::info!(
                next_ephemeris_set_index,
                "switched to ephemeris set"
            );

            let current_ephemerides = self
                .ephemerides
                .get(next_ephemeris_set_index)
                .ok_or_else(|| Error::msg("invalid ephemeris set index"))?;
            for channel in self
                .channels
                .iter_mut()
                .take(MAX_CHAN)
                .filter(|channel| channel.prn != 0)
            {
                let satellite_index = channel.prn - 1;
                if let Some(ephemeris) = current_ephemerides
                    .get(satellite_index)
                    .filter(|ephemeris| ephemeris.vflg)
                {
                    channel.generate_navigation_subframes(
                        ephemeris,
                        &self.ionoutc,
                    );
                }
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
            FiniteRunState::OutputFailed => {
                return Err(Error::FiniteRunOutputFailed);
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

    /// Interpolates one ECEF endpoint along a linear segment.
    fn interpolate_ecef(start: &Ecef, end: &Ecef, fraction: f64) -> Ecef {
        Ecef::new(
            start.x + (end.x - start.x) * fraction,
            start.y + (end.y - start.y) * fraction,
            start.z + (end.z - start.z) * fraction,
        )
    }

    /// Resolves a sample-derived endpoint against timestamped motion knots.
    fn timestamped_block_location(
        &self, block: &TimelineBlock, elapsed_knots: &[f64],
    ) -> Result<Ecef, Error> {
        if elapsed_knots.len() != self.positions.len()
            || elapsed_knots.is_empty()
        {
            return Err(Error::wrong_positions());
        }

        let end_index = elapsed_knots
            .partition_point(|elapsed| *elapsed <= block.end_elapsed_seconds);
        if end_index == 0 {
            return self
                .positions
                .first()
                .copied()
                .ok_or_else(Error::wrong_positions);
        }
        if end_index == elapsed_knots.len() {
            return self
                .positions
                .last()
                .copied()
                .ok_or_else(Error::wrong_positions);
        }
        let end_elapsed = *elapsed_knots
            .get(end_index)
            .ok_or_else(Error::wrong_positions)?;
        let start_index = end_index - 1;
        let start_elapsed = *elapsed_knots
            .get(start_index)
            .ok_or_else(Error::wrong_positions)?;
        let fraction = (block.end_elapsed_seconds - start_elapsed)
            / (end_elapsed - start_elapsed);
        let start = self
            .positions
            .get(start_index)
            .ok_or_else(Error::wrong_positions)?;
        let end = self
            .positions
            .get(end_index)
            .ok_or_else(Error::wrong_positions)?;
        Ok(Self::interpolate_ecef(start, end, fraction))
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
                if let Some(elapsed_knots) = &self.motion_elapsed_seconds {
                    return self
                        .timestamped_block_location(block, elapsed_knots);
                }

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
                Ok(Self::interpolate_ecef(
                    start,
                    end,
                    block.step_endpoint_fraction(),
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

    /// Emits every remaining direct-file timeline block.
    fn run_direct_blocks(
        &mut self, emitted_blocks: &mut usize,
    ) -> Result<(), Error> {
        while let Some(block) = self.next_timeline_block()? {
            let current_location = self.finite_block_location(&block)?;
            self.prepare_block(&block, current_location)?;
            self.generate_and_write_samples(block.sample_count)?;
            *emitted_blocks += 1;
            if self.verbose && emitted_blocks.is_multiple_of(100) {
                tracing::debug!(
                    emitted_blocks = *emitted_blocks,
                    gps_week = self.receiver_gps_time.week,
                    gps_seconds = self.receiver_gps_time.sec,
                    "simulation progress"
                );
            }
            self.finish_block(block, current_location)?;
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
        if self.writer.is_none() {
            return Err(Error::IQWriterNotInitialized);
        }
        self.begin_finite_run()?;

        tracing::info!(
            requested_intervals = self.simulation_step_count,
            "starting signal generation"
        );
        let time_start = std::time::Instant::now();
        let mut emitted_blocks = 0usize;
        let primary_result = self.run_direct_blocks(&mut emitted_blocks);
        let finalization_result = self
            .writer
            .as_mut()
            .ok_or(Error::IQWriterNotInitialized)
            .and_then(crate::IQWriter::finish);

        match resolve_with_finalization(primary_result, finalization_result) {
            Ok(()) => {
                self.complete_finite_run();
                tracing::info!(emitted_blocks, "done");
                tracing::info!(
                    process_seconds = time_start.elapsed().as_secs_f32(),
                    "process time"
                );
                Ok(())
            }
            Err(error) => {
                self.finite_run_state = FiniteRunState::OutputFailed;
                Err(error)
            }
        }
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
