use constants::{
    CA_SEQ_LEN, CA_SEQ_LEN_FLOAT, CARR_TO_CODE, CODE_FREQ, LAMBDA_L1,
    LAMBDA_L1_INV, N_DWRD, N_DWRD_SBF, SPEED_OF_LIGHT_INV,
};
use geometry::{Azel, Ecef};

use crate::{
    Error,
    datetime::{GpsTime, TimeRange},
    ephemeris::Ephemeris,
    ionoutc::IonoUtc,
    propagation::compute_range,
    table::*,
};

/// Navigation message timing, TOW insertion, and parity generation.
mod nav_message;
/// Broadcast ephemeris and ionosphere subframe encoding.
mod navigation_subframes;

/// Number of effective bits in the fixed-point carrier accumulator.
const CARRIER_PHASE_BITS: u32 = 25;
/// Fixed-point accumulator counts per carrier cycle.
const CARRIER_PHASE_SCALE: f64 = (1_u32 << CARRIER_PHASE_BITS) as f64;
/// Mask that keeps carrier accumulator state modulo one cycle.
const CARRIER_PHASE_MASK: u32 = (1_u32 << CARRIER_PHASE_BITS) - 1;

/// Converts modeled pseudorange into a deterministic simulator phase reference.
///
/// This reference is zero cycles at zero modeled range; it is not a calibrated
/// absolute satellite oscillator phase.
fn carrier_phase_from_range(range_meters: f64) -> u32 {
    let cycles = (-range_meters / LAMBDA_L1).rem_euclid(1.0);
    ((CARRIER_PHASE_SCALE * cycles) as u32) & CARRIER_PHASE_MASK
}

/// Represents a single GPS satellite channel being tracked by the receiver.
///
/// This structure maintains the complete state of a satellite signal channel,
/// including signal generation parameters, navigation message data, and
/// tracking information. Each Channel instance corresponds to one satellite
/// (identified by PRN) that is being simulated.
///
/// The Channel is responsible for:
/// - Generating the satellite-specific C/A code sequence (1023 chips)
/// - Maintaining carrier and code phase information
/// - Constructing and managing the navigation message data
/// - Tracking pseudorange and geometric information
/// - Generating I/Q samples for the satellite signal
///
/// During simulation, the channel state is updated at each time step to
/// accurately model the changing satellite-receiver geometry and signal
/// characteristics.
pub struct Channel {
    /// Satellite PRN (Pseudorandom Noise) number (1-32)
    pub prn: usize,

    /// Current carrier phase accumulator (fixed-point representation)
    carrier_phase: u32,
    /// Carrier phase step per sample (fixed-point representation)
    carrier_phase_step: i32,

    /// Current code phase position within C/A sequence (0.0 to 1022.999...)
    code_phase: f64,
    /// Code phase step per sample (chips/sample)
    code_phase_step: f64,

    /// Current word index in navigation message (0-49)
    word_index: i32,
    /// Current bit index within the current word (0-29)
    bit_index: i32,
    /// Current code epoch index within the current bit (0-19)
    code_epoch_index: i32,

    /// Current navigation data bit value (+1 or -1)
    current_data_bit: i32,
    /// Current C/A code chip value (+1 or -1)
    current_code_chip: i32,

    /// C/A code sequence chips for this satellite (1023 chips), stored as
    /// -1/+1.
    ca_sequence: [i8; CA_SEQ_LEN],

    /// Current carrier frequency with Doppler shift (Hz)
    carrier_frequency: f64,
    /// Current code frequency with Doppler effect (Hz)
    code_frequency: f64,

    /// GPS time at the start of the navigation message frame
    nav_message_start_time: GpsTime,
    /// Navigation message subframes (5 subframes of 10 words each)
    subframes: [[u32; N_DWRD_SBF]; 5],
    /// Complete navigation message data words (50 words total)
    data_words: [u32; N_DWRD],

    /// Satellite azimuth and elevation angles
    azel: Azel,
    /// Previous pseudorange measurement and associated data
    rho0: TimeRange,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            prn: 0,
            ca_sequence: [0; CA_SEQ_LEN],
            carrier_frequency: 0.0,
            code_frequency: 0.0,
            code_phase_step: 0.0,
            carrier_phase: 0,
            carrier_phase_step: 0,
            code_phase: 0.0,
            nav_message_start_time: GpsTime { week: 0, sec: 0. },
            subframes: [[0; N_DWRD_SBF]; 5],
            data_words: [0; N_DWRD],
            word_index: 0,
            bit_index: 0,
            code_epoch_index: 0,
            current_data_bit: 0,
            current_code_chip: 0,
            azel: Azel::default(),
            rho0: TimeRange::default(),
        }
    }
}
impl Channel {
    /// Returns a reference to the initial pseudorange information.
    pub fn rho0(&self) -> &TimeRange {
        &self.rho0
    }

    /// Returns a reference to the satellite's azimuth and elevation.
    pub fn azel(&self) -> &Azel {
        &self.azel
    }

    /// Initializes or updates the channel state for a specific satellite.
    ///
    /// This involves setting the PRN, generating C/A code and navigation
    /// subframes, initializing pseudorange, and setting the initial carrier
    /// phase.
    ///
    /// # Arguments
    /// * `prn` - The PRN number of the satellite.
    /// * `eph` - The ephemeris data for the satellite.
    /// * `ionoutc` - Ionospheric and UTC parameters.
    /// * `receiver_gps_time` - The current GPS time at the receiver.
    /// * `xyz` - The receiver's position in ECEF coordinates.
    /// * `azel` - The satellite's azimuth and elevation as seen from the
    ///   receiver.
    pub fn update_for_satellite(
        &mut self, prn: usize, eph: &Ephemeris, ionoutc: &IonoUtc,
        receiver_gps_time: &GpsTime, xyz: &Ecef, azel: Azel,
    ) -> Result<(), Error> {
        // Initialize channel
        self.prn = prn;
        self.azel = azel;
        // C/A code generation
        self.codegen();
        // Generate subframe
        self.generate_navigation_subframes(eph, ionoutc);
        // Generate navigation message
        // Populate the first full navigation message cycle (30 seconds / 5
        // subframes)
        self.generate_nav_msg(receiver_gps_time, true);
        // Initialize pseudorange
        let rho = compute_range(eph, ionoutc, receiver_gps_time, xyz)?;
        self.carrier_phase = carrier_phase_from_range(rho.range);
        self.rho0 = rho;
        Ok(())
    }

    /// Updates the channel's state based on new pseudorange information and
    /// time delta.
    ///
    /// Calculates the new code phase and carrier phase step based on the change
    /// in pseudorange over the sampling period.
    ///
    /// # Arguments
    /// * `rho1` - The new pseudorange measurement and associated time/azel
    ///   data.
    /// * `dt` - The time difference since the last pseudorange measurement
    ///   (`rho0`).
    /// * `sampling_period` - The receiver's sampling period in seconds.
    pub fn update_state(
        &mut self, rho1: &TimeRange, dt: f64, sampling_period: f64,
    ) {
        // Update azimuth/elevation information
        // Update code phase and data bit counters
        self.azel = rho1.azel;
        // Calculate code phase (C/A code offset)
        self.compute_code_phase(rho1, dt);
        self.code_phase_step = self.code_frequency * sampling_period;
        self.carrier_phase_step =
            (CARRIER_PHASE_SCALE * self.carrier_frequency * sampling_period)
                .round() as i32;
    }

    ///  \brief Compute the code phase for a given channel (satellite)
    ///  \param chan Channel on which we operate (is updated)
    ///  \param[in] rho1 Current range, after \a dt has expired
    ///  \param[in dt delta-t (time difference) in seconds
    /// Computes the code phase for the channel based on pseudorange rate.
    ///
    /// Updates carrier and code frequencies, calculates initial code phase,
    /// word/bit/code counters, and sets the initial C/A code and data bit
    /// values.
    ///
    /// # Arguments
    /// * `rho1` - Current range information.
    /// * `dt` - Time difference since the last range measurement (`rho0`).
    #[inline]
    pub fn compute_code_phase(&mut self, rho1: &TimeRange, dt: f64) {
        // Pseudorange rate.
        let rhorate = (rho1.range - self.rho0.range) / dt;
        // Carrier and code frequency.
        self.carrier_frequency = -rhorate * LAMBDA_L1_INV;
        self.code_frequency = CODE_FREQ + self.carrier_frequency * CARR_TO_CODE;
        // Initial code phase and data bit counters.
        let ms = (self.rho0.time.diff_secs(&self.nav_message_start_time) + 6.0
            - self.rho0.range * SPEED_OF_LIGHT_INV)
            * 1000.0;
        let mut ims = ms as i32;
        self.code_phase = ms.fract() * CA_SEQ_LEN_FLOAT; // in chip
        self.word_index = ims / 600; // 1 word = 30 bits = 600 ms
        ims -= self.word_index * 600;
        self.bit_index = ims / 20; // 1 bit = 20 code = 20 ms
        ims -= self.bit_index * 20;
        self.code_epoch_index = ims; // 1 code = 1 ms
        self.current_code_chip =
            i32::from(self.ca_sequence[self.code_phase as usize]);
        self.current_data_bit = (self.data_words[self.word_index as usize]
            >> (29 - self.bit_index)
            & 0x1) as i32
            * 2
            - 1;
        // Save current pseudorange
        self.rho0 = rho1.clone();
    }

    /// Generates the C/A (Coarse/Acquisition) code sequence for this satellite
    /// channel.
    ///
    /// This method implements the GPS C/A code generation algorithm as
    /// specified in the GPS Interface Control Document (ICD-GPS-200). Each
    /// satellite has a unique C/A code sequence that allows receivers to
    /// distinguish between different satellite signals.
    ///
    /// The algorithm uses:
    /// 1. Two 10-bit Linear Feedback Shift Registers (LFSRs), G1 and G2
    /// 2. A satellite-specific delay value for the G2 register
    /// 3. A modulo-2 addition (XOR) of specific taps from each register
    ///
    /// The resulting sequence has the following properties:
    /// - Length: 1023 chips (repeats every 1 millisecond at 1.023 MHz)
    /// - Balanced: Contains 512 zeros and 511 ones
    /// - Low cross-correlation with other satellite codes
    /// - Good autocorrelation properties for signal acquisition
    ///
    /// The generated sequence is stored in the channel's `ca_sequence` field
    /// and is used for spreading the navigation data bits during signal
    /// generation.
    #[inline]
    pub fn codegen(&mut self) {
        let delay: [usize; 32] = [
            5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257,
            258, 469, 470, 471, 472, 473, 474, 509, 512, 513, 514, 515, 516,
            859, 860, 861, 862,
        ];
        let mut g1: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
        let mut g2: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
        let mut r1: [i32; N_DWRD_SBF] = [-1; N_DWRD_SBF];
        let mut r2: [i32; N_DWRD_SBF] = [-1; N_DWRD_SBF];
        // if !(self.prn <= 32 || self.prn >= 1) {
        //     return;
        // }
        if !(1..=32).contains(&self.prn) {
            return;
        }
        for i in 0..CA_SEQ_LEN {
            g1[i] = r1[9];
            g2[i] = r2[9];
            let c1 = r1[2] * r1[9];
            let c2 = r2[1] * r2[2] * r2[5] * r2[7] * r2[8] * r2[9];
            for j in (1..N_DWRD_SBF).rev() {
                r1[j] = r1[j - 1];
                r2[j] = r2[j - 1];
            }
            r1[0] = c1;
            r2[0] = c2;
        }

        let sequence_start = CA_SEQ_LEN - delay[self.prn - 1];
        for (sequence_index, (ca_chip, g1_chip)) in
            (sequence_start..).zip(self.ca_sequence.iter_mut().zip(g1))
        {
            *ca_chip = (-(g1_chip * g2[sequence_index % CA_SEQ_LEN])) as i8;
        }
    }

    /// Advances this channel by exactly one complex sample.
    ///
    /// This is on the hot path: it updates the C/A code phase/chip, the
    /// navigation message counters (1ms code epochs -> 20ms nav bits -> 600ms
    /// nav words), and the carrier phase accumulator.
    ///
    /// Notes:
    /// - `code_phase_step` and `carrier_phase_step` are per-sample increments
    ///   computed in `update_state(..., sampling_period)` to keep this method
    ///   free of floating point multiplications.
    /// - If the sample rate (and thus `sampling_period`) changes at runtime,
    ///   callers must refresh the step values before calling this again.
    pub fn update_navigation_bits(&mut self) {
        // Advance one sample worth of C/A code phase.
        self.code_phase += self.code_phase_step;

        // --- Handle Code Epoch Rollover (every 1ms / 1023 chips) ---
        if self.code_phase >= CA_SEQ_LEN_FLOAT {
            self.code_phase -= CA_SEQ_LEN_FLOAT; // Wrap code phase
            self.code_epoch_index += 1; // Increment ms counter

            // Check for code rollover (20 codes per bit)
            // 20 C/A codes = 1 navigation data bit
            // Process navigation data bit (every 20 C/A code periods)
            if self.code_epoch_index >= 20 {
                self.code_epoch_index = 0;
                self.bit_index += 1;

                // Check for bit rollover (30 bits per word)
                // Process navigation word (every 30 data bits)
                if self.bit_index >= 30 {
                    // 30 navigation data bits = 1 word
                    self.bit_index = 0;
                    self.word_index += 1;
                    // if (chan[i].word_index>=N_DWRD)
                    // fprintf(stderr, "\nWARNING: Subframe word buffer
                    // overflow.\n");
                }

                // Extract current navigation data bit
                // Update data bit based on new word/bit index
                // Set new navigation data bit
                let word_idx = self.word_index as usize;
                self.current_data_bit = (self.data_words[word_idx]
                    >> (29 - self.bit_index)
                    & 0x1) as i32
                    * 2
                    - 1;
            }
        }
        // Update current C/A code chip.
        // `ca_sequence` stores -1/+1 chips as i8 so we can just widen here.
        self.current_code_chip =
            i32::from(self.ca_sequence[self.code_phase as i32 as usize]);

        // Advance one sample worth of carrier phase (fixed-point accumulator).
        // #ifdef FLOAT_CARR_PHASE
        //                     chan[i].carrier_phase +=
        // chan[i].carrier_frequency
        // * sampling_period;
        //
        //                     if (chan[i].carrier_phase >= 1.0)
        //                         chan[i].carrier_phase -= 1.0;
        //                     else if (chan[i].carrier_phase<0.0)
        //                         chan[i].carrier_phase += 1.0;
        // #else
        // Step 5: Update carrier phase (using phase accumulator)

        self.carrier_phase = self
            .carrier_phase
            .wrapping_add(self.carrier_phase_step as u32)
            & CARRIER_PHASE_MASK;
    }

    /// Generates the In-phase (I) and Quadrature (Q) signal contributions for
    /// this channel.
    ///
    /// Calculates the I/Q components based on the current carrier phase (using
    /// a pre-computed sine/cosine lookup table), the current C/A code chip,
    /// the current navigation data bit, and the antenna gain.
    ///
    /// # Arguments
    /// * `antenna_gain` - The gain factor applied to the signal.
    ///
    /// # Returns
    /// A tuple `(ip, qp)` representing the I and Q components.]]>
    pub fn generate_iq_contribution(&self, antenna_gain: i32) -> (i32, i32) {
        // #ifdef FLOAT_CARR_PHASE
        //                     iTable =
        // (int)floor(chan[i].carrier_phase*512.0);
        // #else
        // Use precomputed sine/cosine tables to generate carrier
        let i_table = (self.carrier_phase >> 16 & 0x1ff) as usize; // 9-bit index
        // Generate I/Q components (considering navigation data bit and C/A
        // code)
        let scaled_gain =
            self.current_data_bit * self.current_code_chip * antenna_gain;
        let ip = scaled_gain * COS_TABLE512[i_table];
        let qp = scaled_gain * SIN_TABLE512[i_table];
        (ip, qp)
    }
}

#[cfg(test)]
#[path = "channel/phase_tests.rs"]
mod phase_tests;
