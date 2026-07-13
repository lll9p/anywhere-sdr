use std::path::PathBuf;

use constants::*;
use geometry::Ecef;

use super::{motion_control::RuntimeMotionControl, timeline::SampleTimeline};
use crate::{
    Error,
    channel::Channel,
    datetime::{DateTime, GpsTime},
    ephemeris::Ephemeris,
    generator::utils::MotionMode,
    io::{DataFormat, IQWriter},
    ionoutc::IonoUtc,
    propagation::compute_range,
    table::ANT_PAT_DB,
};

/// Finite and runtime-controlled sample emission loops.
mod run;

/// Main class for GPS signal generation and simulation.
///
/// This struct contains all the state needed to simulate GPS signals:
/// - Satellite ephemeris data and parameters
/// - Receiver position and motion information
/// - Channel allocation and tracking state
/// - Signal generation parameters
/// - I/O configuration for sample output
///
/// The typical usage flow is:
/// 1. Create a `SignalGeneratorBuilder` and configure simulation parameters
/// 2. Call `build()` to create a `SignalGenerator`
/// 3. Call `initialize()` to set up the simulation
/// 4. Call `run_simulation()` to generate the GPS signals
pub struct SignalGenerator {
    /// Satellite ephemeris data organized in hourly sets
    pub ephemerides: Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
    /// Index of the currently active ephemeris set
    pub valid_ephemerides_index: usize,
    /// Array of satellite signal channels being tracked
    pub channels: [Channel; MAX_CHAN],
    /// Ionospheric and UTC parameters
    pub ionoutc: IonoUtc,
    /// Tracking which satellites are allocated to which channels (-1 = not
    /// allocated)
    pub allocated_satellite: [i32; MAX_SAT],
    /// Receiver positions in ECEF coordinates (one per configured update step)
    pub positions: Vec<Ecef>,
    /// Total number of emitted intervals to simulate
    pub simulation_step_count: usize,
    /// Requested emitted waveform duration in seconds
    pub duration_seconds: Option<f64>,
    /// Current GPS time at the receiver
    pub receiver_gps_time: GpsTime,
    /// Signal gain values for each channel
    pub antenna_gains: [i32; MAX_CHAN],
    /// Antenna gain pattern lookup table (by elevation angle)
    pub antenna_pattern: [f64; 37],
    /// Simulation mode (static or dynamic position)
    pub mode: MotionMode,
    /// Optional runtime motion controller (`UserControl` mode)
    pub runtime_motion_control: Option<RuntimeMotionControl>,
    /// Elevation mask angle in radians (satellites below this are not visible)
    pub elevation_mask: f64,
    /// Sampling frequency in Hz (typically 2.6MHz)
    pub sample_frequency: f64,
    /// Time step between samples in seconds (typically 0.1s)
    pub sample_rate: f64,
    /// I/Q data format for output
    pub data_format: DataFormat,
    /// Optional fixed gain value (when Some, path loss is disabled)
    pub fixed_gain: Option<i32>,
    /// Size of I/Q sample buffer
    pub iq_buffer_size: usize,
    /// Output file path
    pub output_file: Option<PathBuf>,
    /// I/Q sample writer
    pub writer: Option<IQWriter>,
    /// Whether the generator has been initialized
    pub initialized: bool,
    /// Authoritative emitted-sample timeline
    pub(super) timeline: Option<SampleTimeline>,
    /// Outcome controlling whether a finite timeline may restart
    pub(super) finite_run_state: run::FiniteRunState,
    /// Whether to show detailed channel status
    pub verbose: bool,
}
impl Default for SignalGenerator {
    fn default() -> Self {
        Self {
            ephemerides: Box::default(),
            valid_ephemerides_index: usize::default(),
            channels: std::array::from_fn(|_| Channel::default()),
            ionoutc: IonoUtc::default(),
            allocated_satellite: [0; MAX_SAT],
            positions: Vec::new(),
            simulation_step_count: usize::default(),
            duration_seconds: None,
            receiver_gps_time: GpsTime::default(),
            antenna_gains: [0; MAX_CHAN],
            antenna_pattern: [0.0; 37],
            mode: MotionMode::Static,
            runtime_motion_control: None,
            elevation_mask: f64::default(),
            sample_frequency: 0.0,
            sample_rate: 0.0,
            data_format: DataFormat::Bits8,
            fixed_gain: None,
            iq_buffer_size: 0,
            // iq_buffer: Vec::new(),
            output_file: None,
            writer: None,
            initialized: false,
            timeline: None,
            finite_run_state: run::FiniteRunState::default(),
            verbose: true,
        }
    }
}
impl SignalGenerator {
    /// Initializes the signal generator before simulation.
    ///
    /// This method performs the necessary setup steps before running the
    /// simulation:
    /// - Displays the simulation mode and initial position
    /// - Sets up the receiver time
    /// - Allocates satellite channels based on visibility
    /// - Initializes the antenna gain pattern
    /// - Sets up the I/Q sample buffer and writer
    ///
    /// This method must be called before `run_simulation()`.
    ///
    /// # Returns
    /// * `Ok(())` - If initialization is successful
    /// * `Err(Error)` - If there's an error during initialization
    ///
    /// # Errors
    /// * Returns an error if the output file cannot be opened or if there's an
    ///   issue with the I/Q writer
    pub fn initialize(&mut self) -> Result<(), Error> {
        // Initialize channels
        match self.mode {
            MotionMode::Static => {
                tracing::info!("using static location mode");
            }
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

        if let Some(first_position) = self.positions.first() {
            tracing::info!(
                x = first_position.x,
                y = first_position.y,
                z = first_position.z,
                "initial receiver position (ECEF)"
            );
        }
        let gps_time_start = self.receiver_gps_time.clone();
        let date_time_start = DateTime::from(&gps_time_start);
        tracing::info!(
            year = date_time_start.y,
            month = date_time_start.m,
            day = date_time_start.d,
            hour = date_time_start.hh,
            minute = date_time_start.mm,
            second = date_time_start.sec,
            gps_week = gps_time_start.week,
            gps_seconds = gps_time_start.sec,
            "start time"
        );
        // Clear all channels
        self.channels
            .iter_mut()
            .take(MAX_CHAN)
            .for_each(|ch| ch.prn = 0);
        // Clear satellite allocation flag
        self.allocated_satellite
            .iter_mut()
            .take(MAX_SAT)
            .for_each(|s| *s = -1);
        // Allocate visible satellites at the initial state epoch.
        self.allocate_channel(self.positions[0]);
        if self.verbose {
            Self::log_channel_status(&self.channels);
        }

        ////////////////////////////////////////////////////////////
        // Receiver antenna gain pattern
        ////////////////////////////////////////////////////////////
        // for i in 0..37 {
        for (i, item) in self.antenna_pattern.iter_mut().take(37).enumerate() {
            *item = 10.0f64.powf(-ANT_PAT_DB[i] / 20.0);
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
        self.iq_buffer_size = timeline.maximum_block_samples()?;
        self.writer = match &self.output_file {
            Some(file) => Some(IQWriter::new(
                file,
                self.data_format,
                self.iq_buffer_size,
            )?),
            None => None,
        };
        self.timeline = Some(timeline);
        self.finite_run_state = run::FiniteRunState::Ready;
        self.initialized = true;
        Ok(())
    }

    /// Allocates satellite channels based on visibility from the current
    /// position.
    ///
    /// This method determines which satellites are visible from the given
    /// position, allocates channels to visible satellites, and deallocates
    /// channels for satellites that are no longer visible.
    ///
    /// # Arguments
    /// * `xyz` - The current receiver position in ECEF coordinates
    ///
    /// # Returns
    /// * The number of visible satellites
    pub fn allocate_channel(&mut self, xyz: Ecef) -> i32 {
        let receiver_gps_time = self.receiver_gps_time.clone();
        self.allocate_channel_at(xyz, &receiver_gps_time)
    }

    /// Allocates channels using the supplied exact receiver epoch.
    fn allocate_channel_at(
        &mut self, xyz: Ecef, receiver_gps_time: &GpsTime,
    ) -> i32 {
        let mut visible_satellite_count: i32 = 0;
        // let ref_0: [f64; 3] = [0., 0., 0.];
        // #[allow(unused_variables)]
        // let mut r_ref: f64 = 0.;
        // #[allow(unused_variables)]
        // let mut r_xyz: f64;
        for (sv, eph) in self.ephemerides[self.valid_ephemerides_index]
            .iter()
            .enumerate()
            .take(MAX_SAT)
        {
            if let Some((azel, true)) = eph.check_visibility(
                receiver_gps_time,
                &xyz,
                self.elevation_mask,
            ) {
                visible_satellite_count += 1; // Number of visible satellites
                if self.allocated_satellite[sv] == -1 {
                    // Visible but not allocated
                    //
                    // Allocated new satellite
                    let mut allocated_channel_index: Option<usize> = None;
                    for (channel_index, channel) in
                        self.channels.iter_mut().take(MAX_CHAN).enumerate()
                    {
                        if channel.prn == 0 {
                            // Initialize channel
                            channel.update_for_satellite(
                                sv + 1,
                                eph,
                                &self.ionoutc,
                                receiver_gps_time,
                                &xyz,
                                azel,
                            );
                            allocated_channel_index = Some(channel_index);
                            break;
                        }
                    }
                    // Set satellite allocation channel
                    if let Some(channel_index) = allocated_channel_index {
                        self.allocated_satellite[sv] = channel_index as i32;
                    }
                }
            } else if self.allocated_satellite[sv] >= 0 {
                // Not visible but allocated
                // Clear channel
                self.channels[self.allocated_satellite[sv] as usize].prn = 0;
                // Clear satellite allocation flag
                self.allocated_satellite[sv] = -1;
            }
        }
        visible_satellite_count
    }

    /// Generates I/Q samples for all active channels and writes them to the
    /// output file.
    ///
    /// This method performs the following steps:
    /// 1. Accumulates signal components from all active satellite channels
    /// 2. Quantizes and stores the combined I/Q samples in the buffer
    /// 3. Writes the I/Q data to the output file
    ///
    /// # Returns
    /// * `Ok(())` - If sample generation and writing is successful
    /// * `Err(Error)` - If there's an error during sample generation or writing
    ///
    /// # Errors
    /// * Returns an error if the I/Q writer is not initialized
    /// * Returns an error if writing to the output file fails
    #[inline]
    fn generate_and_write_samples(
        &mut self, complex_sample_count: usize,
    ) -> Result<(), Error> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::msg("IQWriter not initialized"))?;
        writer.buffer_size = complex_sample_count;
        writer.buffer.resize(2 * complex_sample_count, 0);
        Self::generate_samples_into(
            &mut self.channels,
            &self.antenna_gains,
            &mut writer.buffer,
        )?;
        writer.write_samples()?;
        Ok(())
    }

    /// Generates interleaved i16 I/Q samples into `out`.
    #[inline]
    fn generate_samples_into(
        channels: &mut [Channel; MAX_CHAN], antenna_gains: &[i32; MAX_CHAN],
        out: &mut [i16],
    ) -> Result<(), Error> {
        if !out.len().is_multiple_of(2) {
            return Err(Error::msg("I/Q buffer length must be even"));
        }

        // Build a compact list of active channels once per block.
        let mut active: [(usize, i32); MAX_CHAN] = [(0, 0); MAX_CHAN];
        let mut active_count: usize = 0;
        for channel_index in 0..MAX_CHAN {
            if channels[channel_index].prn != 0 {
                active[active_count] =
                    (channel_index, antenna_gains[channel_index]);
                active_count += 1;
            }
        }

        for iq in out.chunks_exact_mut(2) {
            let mut i_acc: i32 = 0;
            let mut q_acc: i32 = 0;

            // Step 1: Accumulate signal components from all active channels.
            for &(channel_index, antenna_gain) in &active[..active_count] {
                let (ip, qp) = channels[channel_index]
                    .generate_iq_contribution(antenna_gain);
                i_acc += ip;
                q_acc += qp;
                channels[channel_index].update_navigation_bits();
            }

            // Step 2: Quantize and store I/Q samples.
            // Scaled by 2^7.
            iq[0] = ((i_acc + 64) >> 7) as i16;
            iq[1] = ((q_acc + 64) >> 7) as i16;
        }

        Ok(())
    }

    /// Updates pseudorange, Doppler shift, and signal gain for all active
    /// channels.
    ///
    /// This method calculates the current signal parameters for each active
    /// satellite channel:
    /// - Computes the current pseudorange (distance) to each satellite
    /// - Updates the code and carrier phase based on the pseudorange change
    /// - Calculates the signal gain based on path loss and antenna pattern
    ///
    /// The gain calculation depends on whether fixed gain mode is enabled:
    /// - If fixed gain is set, all satellites use the same constant gain
    /// - Otherwise, gain is calculated based on distance and elevation angle
    ///
    /// # Arguments
    /// * `current_location` - The current receiver position in ECEF coordinates
    fn update_channel_parameters(
        &mut self, current_location: Ecef, elapsed_seconds: f64,
    ) {
        let ephemeris_set_index = self.valid_ephemerides_index;
        let sampling_period = self.sample_frequency.recip();
        for i in 0..MAX_CHAN {
            // Only process channels with assigned satellites
            if self.channels[i].prn != 0 {
                // Convert satellite PRN to array index
                let sv = self.channels[i].prn - 1;
                let eph = &self.ephemerides[ephemeris_set_index][sv];
                // Calculate current pseudorange (propagation delay)
                // Refresh code phase and data bit counters

                // Current pseudorange
                let rho = compute_range(
                    eph,
                    &self.ionoutc,
                    &self.receiver_gps_time,
                    &current_location,
                );
                self.channels[i].update_state(
                    &rho,
                    elapsed_seconds,
                    sampling_period,
                );

                // Calculate signal gain (considering path loss and antenna
                // pattern) Signal gain
                // Apply gain mode selection
                let gain = if let Some(fixed_gain) = self.fixed_gain {
                    // Fixed gain mode
                    fixed_gain // hold the power level constant
                } else {
                    // With path loss compensation
                    // Path loss
                    let path_loss = 20_200_000.0 / rho.distance;
                    // Receiver antenna gain
                    let boresight_angle_index =
                        ((90.0 - rho.azel.el * R2D) / 5.0) as usize; // covert elevation to boresight
                    let ant_gain = self.antenna_pattern[boresight_angle_index];
                    (path_loss * ant_gain * 128.0) as i32 // scaled by 2^7
                };
                // Store gain for IQ generation phase
                self.antenna_gains[i] = gain; // hold the power level constant
            }
        }
    }

    /// Prints detailed status information about active satellite channels.
    ///
    /// This method displays a table of information for each active channel,
    /// including:
    /// - PRN number (satellite identifier)
    /// - Azimuth angle in degrees
    /// - Elevation angle in degrees
    /// - Range (distance) to the satellite in meters
    /// - Ionospheric delay in meters
    ///
    /// This information is useful for debugging and monitoring the simulation.
    ///
    /// # Arguments
    /// * `channels` - Array of satellite channels
    fn log_channel_status(channels: &[Channel; MAX_CHAN]) {
        use std::fmt::Write as _;

        let mut output = String::new();
        if writeln!(&mut output, "PRN Az(deg) El(deg)  Range(m) Iono(m)")
            .is_err()
        {
            tracing::warn!("failed to format channel status header");
            return;
        }
        for ichan in channels.iter().filter(|ch| ch.prn != 0) {
            if writeln!(
                &mut output,
                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                ichan.prn,
                ichan.azel().az * R2D,
                ichan.azel().el * R2D,
                ichan.rho0().distance,
                ichan.rho0().iono_delay,
            )
            .is_err()
            {
                tracing::warn!("failed to format channel status row");
                return;
            }
        }

        tracing::info!("channel status\n{output}");
    }
}
