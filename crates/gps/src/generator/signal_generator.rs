use std::path::PathBuf;

use constants::*;
use geometry::Ecef;

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
    /// Receiver positions in ECEF coordinates (one per 100ms time step)
    pub positions: Vec<Ecef>,
    /// Total number of motion steps to simulate
    pub simulation_step_count: usize,
    /// Current GPS time at the receiver
    pub receiver_gps_time: GpsTime,
    /// Signal gain values for each channel
    pub antenna_gains: [i32; MAX_CHAN],
    /// Antenna gain pattern lookup table (by elevation angle)
    pub antenna_pattern: [f64; 37],
    /// Simulation mode (static or dynamic position)
    pub mode: MotionMode,
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
            receiver_gps_time: GpsTime::default(),
            antenna_gains: [0; MAX_CHAN],
            antenna_pattern: [0.0; 37],
            mode: MotionMode::Static,
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
            MotionMode::Static => eprintln!("Using static location mode."),
            MotionMode::Dynamic => eprintln!("Using dynamic location mode."),
        }

        eprintln!(
            "xyz = {}, {}, {}",
            self.positions[0].x, self.positions[0].y, self.positions[0].z,
        );
        let gps_time_start = self.receiver_gps_time.clone();
        let date_time_start = DateTime::from(&gps_time_start);
        eprintln!(
            "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
            date_time_start.y,
            date_time_start.m,
            date_time_start.d,
            date_time_start.hh,
            date_time_start.mm,
            date_time_start.sec,
            gps_time_start.week,
            gps_time_start.sec,
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
        // Initial reception time
        self.receiver_gps_time = self.receiver_gps_time.add_secs(0.0);
        // Allocate visible satellites
        self.allocate_channel(self.positions[0]);
        Self::print_channel_status(&self.channels);

        ////////////////////////////////////////////////////////////
        // Receiver antenna gain pattern
        ////////////////////////////////////////////////////////////
        // for i in 0..37 {
        for (i, item) in self.antenna_pattern.iter_mut().take(37).enumerate() {
            *item = 10.0f64.powf(-ANT_PAT_DB[i] / 20.0);
        }

        self.iq_buffer_size =
            (self.sample_frequency * self.sample_rate).floor() as usize;
        self.writer = match &self.output_file {
            Some(file) => Some(IQWriter::new(
                file,
                self.data_format,
                self.iq_buffer_size,
            )?),
            None => None,
        };
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
                &self.receiver_gps_time,
                &xyz,
                self.elevation_mask,
            ) {
                visible_satellite_count += 1; // Number of visible satellites
                if self.allocated_satellite[sv] == -1 {
                    // Visible but not allocated
                    //
                    // Allocated new satellite
                    let mut channel_index = 0;
                    for (i, ichan) in
                        self.channels.iter_mut().take(MAX_CHAN).enumerate()
                    {
                        if ichan.prn == 0 {
                            // Initialize channel
                            ichan.update_for_satellite(
                                sv + 1,
                                eph,
                                &self.ionoutc,
                                &self.receiver_gps_time,
                                &xyz,
                                azel,
                            );
                            break;
                        }
                        channel_index = i + 1;
                    }
                    // Set satellite allocation channel
                    if channel_index < MAX_CHAN {
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
    fn generate_and_write_samples(&mut self) -> Result<(), Error> {
        let sampling_period = self.sample_frequency.recip();
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::msg("IQWriter not initialized"))?;
        let buffer_size = writer.buffer_size;
        for isamp in 0..buffer_size {
            let mut i_acc: i32 = 0;
            let mut q_acc: i32 = 0;
            // Step 1: Accumulate signal components from all channels
            for i in 0..MAX_CHAN {
                if self.channels[i].prn != 0 {
                    let (ip, qp) = self.channels[i]
                        .generate_iq_contribution(self.antenna_gains[i]);
                    // Accumulate for all visible satellites
                    // Add to total signal accumulation
                    i_acc += ip;
                    q_acc += qp;
                    // Update code phase
                    // Update code phase (C/A code sequence control)
                    self.channels[i].update_navigation_bits(sampling_period);
                }
            }

            // Step 2: Quantize and store I/Q samples
            // Scaled by 2^7
            // i_acc = (i_acc + 64) >> 7;
            // q_acc = (q_acc + 64) >> 7;
            // Store I/Q samples into buffer
            writer.buffer[isamp * 2] = ((i_acc + 64) >> 7) as i16; // 8-bit quantization (with rounding)
            writer.buffer[isamp * 2 + 1] = ((q_acc + 64) >> 7) as i16;
        }

        // Step 3: Write I/Q data to output file (handling different formats)
        writer.write_samples()?;
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
    fn update_channel_parameters(&mut self, current_location: Ecef) {
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
                    self.sample_rate,
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

    /// Handles periodic tasks that occur at regular intervals during
    /// simulation.
    ///
    /// This method performs tasks that need to happen periodically (every 30
    /// seconds):
    /// - Updates the navigation message for all active channels
    /// - Refreshes the ephemeris data set if a newer one is available
    /// - Updates the navigation subframes if the ephemeris set changed
    /// - Reallocates satellite channels based on current visibility
    ///
    /// These periodic updates ensure that the simulation accurately reflects
    /// the changing satellite positions and navigation data over time.
    ///
    /// # Arguments
    /// * `current_location` - The current receiver position in ECEF coordinates
    fn handle_periodic_tasks(&mut self, current_location: Ecef) {
        let current_step_index =
            (self.receiver_gps_time.sec * 10.0 + 0.5) as i32;
        if current_step_index % 300 == 0 {
            // Every 30 seconds
            // 1. Update Nav Msg for active channels
            for ichan in self.channels.iter_mut().take(MAX_CHAN) {
                if ichan.prn != 0 {
                    ichan.generate_nav_msg(&self.receiver_gps_time, false);
                }
            }
            // 2. Refresh ephemeris index and subframes if necessary
            // Refresh ephemeris and subframes
            // Quick and dirty fix. Need more elegant way.
            let mut refreshed_eph = false;
            let next_ephemeris_set_index = self.valid_ephemerides_index + 1;
            // Check if next ephemeris set is valid and timely
            if next_ephemeris_set_index < self.ephemerides.len()
                && self.ephemerides[next_ephemeris_set_index]
                    .iter()
                    .take(MAX_SAT)
                    .any(|eph| eph.vflg)
            {
                // Find the earliest ToC in the next set (or check a specific
                // SV) Simplified check: Assume SV 0's ToC is
                // representative
                if self.ephemerides[next_ephemeris_set_index][0].vflg {
                    let dt = self.ephemerides[next_ephemeris_set_index][0]
                        .toc
                        .diff_secs(&self.receiver_gps_time);
                    // If the next ephemeris is close (e.g., within an hour),
                    // switch to it
                    if dt.abs() < SECONDS_IN_HOUR {
                        // Use absolute diff
                        self.valid_ephemerides_index = next_ephemeris_set_index;
                        refreshed_eph = true;
                        eprintln!(
                            "\nSwitched to ephemeris set index \
                             {next_ephemeris_set_index}"
                        );
                    }
                }
            }

            // If ephemeris refreshed, update subframes for active channels
            if refreshed_eph {
                let current_ephemeris_set_index = self.valid_ephemerides_index;
                for ichan in self
                    .channels
                    .iter_mut()
                    .take(MAX_CHAN)
                    .filter(|ch| ch.prn != 0)
                {
                    let sv = ichan.prn - 1;
                    ichan.generate_navigation_subframes(
                        &self.ephemerides[current_ephemeris_set_index][sv],
                        &self.ionoutc,
                    );
                    // self.ephemerides[current_ephemeris_set_index][sv]
                    //     .generate_navigation_subframes(
                    //         &self.ionoutc,
                    //         &mut ichan.sbf,
                    //     );
                    // Maybe need to regenerate nav message bits immediately?
                    // ichan.generate_nav_msg(&self.receiver_gps_time, false);
                    // // Already done above, maybe redundant
                }
            }
            // Update channel allocation
            self.allocate_channel(current_location);

            // Show details about simulated channels
            if self.verbose {
                Self::print_channel_status(&self.channels);
            }
        }
    }

    /// Runs the GPS signal simulation and generates baseband I/Q samples.
    ///
    /// This is the main simulation method that:
    /// 1. Determines the number of simulation steps based on mode and duration
    /// 2. For each time step:
    ///    - Determines the current receiver position (static or from motion
    ///      file)
    ///    - Step 1: Updates satellite parameters (pseudorange, phase, and gain)
    ///    - Step 2: Generates baseband I/Q sample data
    ///    - Step 3: Periodically updates navigation data (every 30 seconds)
    ///    - Step 4: Updates simulation time and displays progress
    ///
    /// The method must be called after `initialize()`.
    ///
    /// # Returns
    /// * `Ok(())` - If the simulation completes successfully
    /// * `Err(Error)` - If there's an error during simulation
    ///
    /// # Errors
    /// * Returns an error if the generator was not initialized
    /// * Returns an error if there's an issue generating or writing samples
    pub fn run_simulation(&mut self) -> Result<(), Error> {
        if !self.initialized {
            return Err(Error::msg("Not initialized!"));
        }
        // Determine the total number of simulation steps
        let num_steps = match self.mode {
            MotionMode::Static => self.simulation_step_count.max(1), /* Ensure at least one step for static */
            MotionMode::Dynamic => self.simulation_step_count,
        };

        if num_steps == 0 {
            eprintln!("Warning: No simulation steps requested.");
            return Ok(());
        }

        eprintln!("Starting signal generation for {num_steps} steps...");
        // Generate baseband signals
        self.receiver_gps_time =
            self.receiver_gps_time.add_secs(self.sample_rate);
        let time_start = std::time::Instant::now();
        // Main loop: Iterate through each time interval (0.1 seconds)
        // From 1..num_steps, because step 0 was done in initiallize.
        for step_index in 1..num_steps {
            // Select receiver position based on static/dynamic mode
            let current_location = match self.mode {
                MotionMode::Static => self.positions[0],
                MotionMode::Dynamic => self
                    .positions
                    .get(step_index)
                    .copied()
                    .unwrap_or(self.positions[0]),
            };
            // Step 1: Update satellite parameters (pseudorange, phase, and
            // gain)
            self.update_channel_parameters(current_location);

            // Step 2: Generate baseband I/Q sample data
            self.generate_and_write_samples()?;
            // Update navigation message and channel allocation every 30 seconds
            // Step 3: Periodically update navigation data (every 30 seconds)
            self.handle_periodic_tasks(current_location);

            // Step 4: Update simulation time and display progress
            // Update receiver time
            self.receiver_gps_time =
                self.receiver_gps_time.add_secs(self.sample_rate);
            eprint!(
                "\rTime into run = {:4.1}\0",
                (step_index + 1) as f64 / 10.0
            );
        }

        eprintln!("\nDone!");
        eprintln!(
            "Process time = {:.1} [sec]",
            time_start.elapsed().as_secs_f32()
        );
        Ok(())
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
    fn print_channel_status(channels: &[Channel; MAX_CHAN]) {
        eprintln!("PRN Az(deg) El(deg)  Range(m) Iono(m)");
        for ichan in channels.iter().filter(|ch| ch.prn != 0) {
            eprintln!(
                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                ichan.prn,
                ichan.azel().az * R2D,
                ichan.azel().el * R2D,
                ichan.rho0().distance, /* Using rho0 which is updated in
                                        * channel.update_state */
                ichan.rho0().iono_delay,
            );
        }
    }
}
