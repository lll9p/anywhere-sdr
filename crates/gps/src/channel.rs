use constants::{
    CA_SEQ_LEN, CA_SEQ_LEN_FLOAT, CARR_TO_CODE, CODE_FREQ, LAMBDA_L1_INV,
    N_DWRD, N_DWRD_SBF, N_SBF, PI, POW2_M5, POW2_M19, POW2_M24, POW2_M27,
    POW2_M29, POW2_M30, POW2_M31, POW2_M33, POW2_M43, POW2_M50, POW2_M55,
    SPEED_OF_LIGHT_INV,
};
use geometry::{Azel, Ecef};

use crate::{
    datetime::{GpsTime, TimeRange},
    ephemeris::Ephemeris,
    ionoutc::IonoUtc,
    propagation::compute_range,
    table::*,
};
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
    /// C/A code sequence chips for this satellite (1023 chips)
    ca_sequence: [i32; CA_SEQ_LEN],
    /// Current carrier frequency with Doppler shift (Hz)
    carrier_frequency: f64,
    /// Current code frequency with Doppler effect (Hz)
    code_frequency: f64,
    /// Current carrier phase accumulator (fixed-point representation)
    carrier_phase: u32,
    /// Carrier phase step per sample (fixed-point representation)
    carrier_phase_step: i32,
    /// Current code phase position within C/A sequence (0.0 to 1022.999...)
    code_phase: f64,
    /// GPS time at the start of the navigation message frame
    nav_message_start_time: GpsTime,
    /// Navigation message subframes (5 subframes of 10 words each)
    subframes: [[u32; N_DWRD_SBF]; 5],
    /// Complete navigation message data words (50 words total)
    data_words: [u32; N_DWRD],
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
    ) {
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
        let rho = compute_range(eph, ionoutc, receiver_gps_time, xyz);
        self.rho0 = rho;
        // Initialize carrier phase
        // r_xyz = rho.range;
        // below line does nothing
        // let _rho =
        //     compute_range(&eph[sv], ionoutc, grx,
        // &ref_0); r_ref = rho.
        // range;
        // Initialize carrier phase (using a fixed or random value initially)
        // A random initial phase is often more realistic unless specific
        // alignment is needed.
        let mut phase_ini: f64 = 0.0; // TODO: Must initialize properly
        //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
        // #ifdef FLOAT_CARR_PHASE
        //                         self.carrier_phase =
        // phase_ini - floor(phase_ini);
        // #else
        phase_ini -= phase_ini.floor();
        self.carrier_phase = (512.0 * 65536.0 * phase_ini) as u32;
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
        self.carrier_phase_step = (512.0
            * 65536.0
            * self.carrier_frequency
            * sampling_period)
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
            self.ca_sequence[self.code_phase as usize] * 2 - 1;
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

        let mut j = CA_SEQ_LEN - delay[self.prn - 1];
        for (ica, ig1) in self.ca_sequence.iter_mut().zip(g1) {
            *ica = (1 - ig1 * g2[j % CA_SEQ_LEN]) / 2;
            j += 1;
        }
    }

    /// Generates the full navigation message data words (DWRD) for a 30-second
    /// cycle.
    ///
    /// Populates the `self.dwrd` array with the 5 subframes (50 words total).
    /// It inserts the Time of Week (TOW) count and Week Number (WN) into the
    /// appropriate words and computes the parity checksum for each word.
    ///
    /// # Arguments
    /// * `time` - The current GPS time used to calculate TOW and WN.
    /// * `init` - Flag indicating if this is the initial generation (handles
    ///   subframe 5 differently).]
    pub fn generate_nav_msg(&mut self, time: &GpsTime, init: bool) {
        let mut time_init = GpsTime::default();
        let mut sbfwrd: u32;
        let mut prevwrd: u32 = 0;
        let mut nib: i32;
        time_init.week = time.week;
        time_init.sec =
            f64::from(((time.sec + 0.5) as u32).wrapping_div(30)) * 30.0; // Align with the full frame length = 30 sec

        let wn = (time_init.week % 1024) as u32;
        let mut tow = (time_init.sec as u32).wrapping_div(6);
        self.nav_message_start_time = time_init; // Data bit reference time

        if init {
            // Initialize subframe 5
            prevwrd = 0;
            for iwrd in 0..N_DWRD_SBF {
                sbfwrd = self.subframes[4][iwrd];
                // Add TOW-count message into HOW
                if iwrd == 1 {
                    sbfwrd |= (tow & 0x1ffff) << 13;
                }
                // Compute checksum
                sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
                nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
                self.data_words[iwrd] = Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.data_words[iwrd];
            }
        } else {
            // Save subframe 5
            for iwrd in 0..N_DWRD_SBF {
                self.data_words[iwrd] =
                    self.data_words[N_DWRD_SBF * N_SBF + iwrd];
                prevwrd = self.data_words[iwrd];
            }
            /*
            // Sanity check
            if (((chan->dwrd[1])&(0x1FFFFUL<<13)) != ((tow&0x1FFFFUL)<<13))
            {
                fprintf(stderr, "\nWARNING: Invalid TOW in subframe 5.\n");
                return(0);
            }
            */
        }
        for isbf in 0..N_SBF {
            tow = tow.wrapping_add(1);

            for iwrd in 0..N_DWRD_SBF {
                sbfwrd = self.subframes[isbf][iwrd];
                // Add transmission week number to Subframe 1
                if isbf == 0 && iwrd == 2 {
                    sbfwrd |= (wn & 0x3ff) << 20;
                }
                // Add TOW-count message into HOW
                if iwrd == 1 {
                    sbfwrd |= (tow & 0x1ffff) << 13;
                }
                // Compute checksum
                sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
                nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
                self.data_words[(isbf + 1) * N_DWRD_SBF + iwrd] =
                    Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.data_words[(isbf + 1) * N_DWRD_SBF + iwrd];
            }
        }
    }

    /// Computes the 6-bit parity checksum for a 30-bit navigation message word.
    ///
    /// Implements the parity algorithm defined in IS-GPS-200, using the
    /// previous word's last two bits (D29*, D30*) and the current word's 24
    /// data bits. Handles non-information bearing bits (NIB) adjustments
    /// for specific words.
    ///
    /// # Arguments
    /// * `source` - The 32-bit input word containing data bits (29-6) and
    ///   previous parity bits (31-30).
    /// * `nib` - Flag indicating if the word contains non-information-bearing
    ///   bits (usually words 2 and 10).
    ///
    /// # Returns
    /// The 32-bit word with the computed 6 parity bits (5-0) inserted.
    #[allow(non_snake_case)]
    // #[inline]
    pub fn compute_checksum(source: u32, nib: i32) -> u32 {
        /*
        Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
        Bits 29 to  6 = Source data bits, d1, d2, ..., d24
        Bits  5 to  0 = Empty parity bits
        */

        /*
        Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
        Bits 29 to  6 = Data bits transmitted by the SV, D1, D2, ..., D24
        Bits  5 to  0 = Computed parity bits, D25, D26, ..., D30
        */

        /*
                          1            2           3
        bit    12 3456 7890 1234 5678 9012 3456 7890
        ---    -------------------------------------
        D25    11 1011 0001 1111 0011 0100 1000 0000
        D26    01 1101 1000 1111 1001 1010 0100 0000
        D27    10 1110 1100 0111 1100 1101 0000 0000
        D28    01 0111 0110 0011 1110 0110 1000 0000
        D29    10 1011 1011 0001 1111 0011 0100 0000
        D30    00 1011 0111 1010 1000 1001 1100 0000
        */
        let bmask: [u32; 6] = [
            0x3b1f_3480,
            0x1d8f_9a40,
            0x2ec7_cd00,
            0x1763_e680,
            0x2bb1_f340,
            0x0b7a_89c0,
        ];
        let mut D: u32;
        let mut d: u32 = source & 0x3fff_ffc0;
        let D29: u32 = source >> 31 & 0x1;
        let D30: u32 = source >> 30 & 0x1;
        if nib != 0 {
            // Non-information bearing bits for word 2 and 10
            /*
            Solve bits 23 and 24 to preserve parity check
            with zeros in bits 29 and 30.
            */
            if D30
                .wrapping_add((bmask[4] & d).count_ones())
                .wrapping_rem(2)
                != 0
            {
                d ^= 0x1 << 6;
            }
            if D29
                .wrapping_add((bmask[5] & d).count_ones())
                .wrapping_rem(2)
                != 0
            {
                d ^= 0x1 << 7;
            }
        }
        D = d;
        if D30 != 0 {
            D ^= 0x3fff_ffc0;
        }
        D |= D29
            .wrapping_add((bmask[0] & d).count_ones())
            .wrapping_rem(2)
            << 5;
        D |= D30
            .wrapping_add((bmask[1] & d).count_ones())
            .wrapping_rem(2)
            << 4;
        D |= D29
            .wrapping_add((bmask[2] & d).count_ones())
            .wrapping_rem(2)
            << 3;
        D |= D30
            .wrapping_add((bmask[3] & d).count_ones())
            .wrapping_rem(2)
            << 2;
        D |= D30
            .wrapping_add((bmask[4] & d).count_ones())
            .wrapping_rem(2)
            << 1;
        D |= D29
            .wrapping_add((bmask[5] & d).count_ones())
            .wrapping_rem(2);
        D &= 0x3fff_ffff;

        //D |= (source & 0xC0000000UL); // Add D29* and D30* from source data
        // bits
        D
    }

    /// Updates the navigation bit state based on the elapsed sampling period.
    ///
    /// Increments the code phase. If a code epoch rolls over (every 1ms),
    /// it increments the code counter (`icode`). If the code counter rolls over
    /// (every 20ms), it increments the bit counter (`ibit`) and updates the
    /// current data bit (`data_bit`). If the bit counter rolls over (every
    /// 600ms), it increments the word counter (`iword`). It also updates
    /// the current C/A code chip (`code_ca`) and carrier phase
    /// (`carr_phase`).
    ///
    /// # Arguments
    /// * `sampling_period` - The receiver sampling period in seconds.
    pub fn update_navigation_bits(&mut self, sampling_period: f64) {
        // Update code phase
        // Step 4: Update code phase (C/A code sequence control)
        // Increment phase by instantaneous freq * dt
        self.code_phase += self.code_frequency * sampling_period;

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
        // Update current C/A code chip
        // Set current code chip
        // this is slower: self.current_code_chip =
        // self.ca_sequence[self.code_phase as usize] * 2 - 1;
        self.current_code_chip =
            self.ca_sequence[self.code_phase as i32 as usize] * 2 - 1;

        // Update carrier phase
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

        self.carrier_phase =
            (self.carrier_phase).wrapping_add(self.carrier_phase_step as u32);
        // self.carrier_phase += self.carrier_phase_step as u32;
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

    /// Constructs the GPS navigation message subframes from ephemeris and UTC
    /// parameters.
    ///
    /// This method implements the detailed bit-level formatting of the GPS
    /// navigation message as specified in the Interface Specification
    /// IS-GPS-200. It converts the satellite ephemeris and ionospheric/UTC
    /// parameters into the binary format transmitted by GPS satellites.
    ///
    /// The navigation message consists of 5 subframes, each containing 10 words
    /// of 30 bits:
    /// - Subframe 1: Satellite clock parameters, GPS week number, and satellite
    ///   health
    /// - Subframe 2: Ephemeris parameters (first part)
    /// - Subframe 3: Ephemeris parameters (second part)
    /// - Subframe 4: Almanac, ionospheric model, UTC parameters (uses page 18)
    /// - Subframe 5: Almanac for other satellites (uses page 25)
    ///
    /// Each parameter is scaled according to the GPS ICD specifications and
    /// placed in the appropriate bit positions within each word. The method
    /// also handles special cases such as:
    /// - Proper scaling of floating-point values to integer representations
    /// - Handling of sign bits for signed parameters
    /// - Inclusion of preamble and telemetry words
    /// - Formatting of Time of Week (TOW) counters
    ///
    /// # Arguments
    /// * `eph` - Satellite ephemeris containing orbital parameters and clock
    ///   corrections
    /// * `ionoutc` - Ionospheric delay model and UTC time conversion parameters
    ///
    /// # Implementation Details
    /// - Subframes 1-3 contain the fundamental ephemeris and clock correction
    ///   data needed for precise positioning
    /// - Subframe 4 page 18 includes:
    ///   - Ionospheric α/β coefficients (Klobuchar model parameters)
    ///   - UTC parameters (`A0`, `A1`, `ΔtLS`)
    ///   - Leap second transition parameters
    /// - Subframe 5 page 25 is reserved (zero-filled in this implementation)
    /// - All value conversions follow GPS-ICD-defined scaling factors and
    ///   bit-field layouts
    /// - The constructed subframes are stored in the channel's `subframes`
    ///   field
    #[allow(clippy::too_many_lines)]
    pub fn generate_navigation_subframes(
        &mut self, eph: &Ephemeris, ionoutc: &IonoUtc,
    ) {
        let ura = 0;
        let data_id = 1;
        let sbf4_page25_sv_id = 63;
        let sbf5_page25_sv_id = 51;
        let wnlsf;
        let dtlsf;
        let dn;
        let sbf4_page18_sv_id = 56;

        // FIXED: This has to be the "transmission" week number, not for the
        // ephemeris reference time wn = (unsigned long)(self.toe.week%1024);
        let wn = 0;
        let toe = (eph.toe.sec / 16.0) as u32;
        let toc = (eph.toc.sec / 16.0) as u32;
        let iode = eph.iode as u32;
        let iodc = eph.iodc as u32;
        let deltan = (eph.deltan / POW2_M43 / PI) as i32;
        let cuc = (eph.cuc / POW2_M29) as i32;
        let cus = (eph.cus / POW2_M29) as i32;
        let cic = (eph.cic / POW2_M29) as i32;
        let cis = (eph.cis / POW2_M29) as i32;
        let crc = (eph.crc / POW2_M5) as i32;
        let crs = (eph.crs / POW2_M5) as i32;
        let ecc = (eph.ecc / POW2_M33) as u32;
        let sqrta = (eph.sqrta / POW2_M19) as u32;
        let m0 = (eph.m0 / POW2_M31 / PI) as i32;
        let omg0 = (eph.omg0 / POW2_M31 / PI) as i32;
        let inc0 = (eph.inc0 / POW2_M31 / PI) as i32;
        let aop = (eph.aop / POW2_M31 / PI) as i32;
        let omgdot = (eph.omgdot / POW2_M43 / PI) as i32;
        let idot = (eph.idot / POW2_M43 / PI) as i32;
        let af0 = (eph.af0 / POW2_M31) as i32;
        let af1 = (eph.af1 / POW2_M43) as i32;
        let af2 = (eph.af2 / POW2_M55) as i32;
        let tgd = (eph.tgd / POW2_M31) as i32;
        let svhlth = eph.svhlth as u32 as i32;

        #[allow(non_snake_case)]
        let codeL2 = eph.codeL2 as u32 as i32;
        let wna = (eph.toe.week % 256) as u32;
        let toa = (eph.toe.sec / 4096.0) as u32;
        let alpha0 = (ionoutc.alpha0 / POW2_M30).round() as i32;
        let alpha1 = (ionoutc.alpha1 / POW2_M27).round() as i32;
        let alpha2 = (ionoutc.alpha2 / POW2_M24).round() as i32;
        let alpha3 = (ionoutc.alpha3 / POW2_M24).round() as i32;
        let beta0 = (ionoutc.beta0 / 2048.0).round() as i32;
        let beta1 = (ionoutc.beta1 / 16384.0).round() as i32;
        let beta2 = (ionoutc.beta2 / 65536.0).round() as i32;
        let beta3 = (ionoutc.beta3 / 65536.0).round() as i32;

        #[allow(non_snake_case)]
        let A0 = (ionoutc.A0 / POW2_M30).round() as i32;

        #[allow(non_snake_case)]
        let A1 = (ionoutc.A1 / POW2_M50).round() as i32;
        let dtls = ionoutc.dtls;
        let tot = (ionoutc.tot / 4096) as u32;
        let week_number = (ionoutc.week_number % 256) as u32;
        // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
        // Days are counted from 1 to 7 (Sunday is 1).
        if ionoutc.leapen == 1 {
            wnlsf = (ionoutc.wnlsf % 256) as u32;
            dn = ionoutc.day_number as u32;
            dtlsf = ionoutc.dtlsf as u32;
        } else {
            wnlsf = (1929 % 256) as u32;
            dn = 7;
            dtlsf = 18;
        }
        // Subframe 1
        self.subframes[0] = [
            0x008b_0000 << 6,
            0x1 << 8,
            (wn & 0x3ff) << 20
                | (codeL2 as u32 & 0x3) << 18
                | (ura & 0xf) << 14
                | (svhlth as u32 & 0x3f) << 8
                | (iodc >> 8 & 0x3) << 6,
            0,
            0,
            0,
            (tgd as u32 & 0xff) << 6,
            (iodc & 0xff) << 22 | (toc & 0xffff) << 6,
            (af2 as u32 & 0xff) << 22 | (af1 as u32 & 0xffff) << 6,
            (af0 as u32 & 0x003f_ffff) << 8,
        ];
        // Subframe 2
        self.subframes[1] = [
            0x008b_0000 << 6,
            0x2 << 8,
            (iode & 0xff) << 22 | (crs as u32 & 0xffff) << 6,
            (deltan as u32 & 0xffff) << 14 | ((m0 >> 24) as u32 & 0xff) << 6,
            (m0 as u32 & 0x00ff_ffff) << 6,
            (cuc as u32 & 0xffff) << 14 | (ecc >> 24 & 0xff) << 6,
            (ecc & 0x00ff_ffff) << 6,
            (cus as u32 & 0xffff) << 14 | (sqrta >> 24 & 0xff) << 6,
            (sqrta & 0x00ff_ffff) << 6,
            (toe & 0xffff) << 14,
        ];
        // Subframe 3
        self.subframes[2] = [
            0x008b_0000 << 6,
            0x3 << 8,
            (cic as u32 & 0xffff) << 14 | ((omg0 >> 24) as u32 & 0xff) << 6,
            (omg0 as u32 & 0x00ff_ffff) << 6,
            (cis as u32 & 0xffff) << 14 | ((inc0 >> 24) as u32 & 0xff) << 6,
            (inc0 as u32 & 0x00ff_ffff) << 6,
            (crc as u32 & 0xffff) << 14 | ((aop >> 24) as u32 & 0xff) << 6,
            (aop as u32 & 0x00ff_ffff) << 6,
            (omgdot as u32 & 0x00ff_ffff) << 6,
            (iode & 0xff) << 22 | (idot as u32 & 0x3fff) << 8,
        ];
        if ionoutc.vflg {
            // Subframe 4, page 18
            self.subframes[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28
                    | sbf4_page18_sv_id << 22
                    | (alpha0 as u32 & 0xff) << 14
                    | (alpha1 as u32 & 0xff) << 6,
                (alpha2 as u32 & 0xff) << 22
                    | (alpha3 as u32 & 0xff) << 14
                    | (beta0 as u32 & 0xff) << 6,
                (beta1 as u32 & 0xff) << 22
                    | (beta2 as u32 & 0xff) << 14
                    | (beta3 as u32 & 0xff) << 6,
                (A1 as u32 & 0x00ff_ffff) << 6,
                ((A0 >> 8) as u32 & 0x00ff_ffff) << 6,
                (A0 as u32 & 0xff) << 22
                    | (tot & 0xff) << 14
                    | (week_number & 0xff) << 6,
                (dtls as u32 & 0xff) << 22
                    | (wnlsf & 0xff) << 14
                    | (dn & 0xff) << 6,
                (dtlsf & 0xff) << 22,
            ];
        } else {
            // Subframe 4, page 25
            self.subframes[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28 | sbf4_page25_sv_id << 22,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
        }
        // Subframe 5, page 25
        self.subframes[4] = [
            0x008b_0000 << 6,
            0x5 << 8,
            data_id << 28
                | sbf5_page25_sv_id << 22
                | (toa & 0xff) << 14
                | (wna & 0xff) << 6,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
    }
}
