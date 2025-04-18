use crate::{
    constants::*,
    datetime::{GpsTime, TimeRange},
};

///  Structure representing a Channel
#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Channel {
    /// PRN Number(Pseudorandom Noise)
    pub prn: usize,
    /// C/A Sequence
    pub ca: [i32; CA_SEQ_LEN],
    /// Carrier frequency
    pub f_carr: f64,
    /// Code frequency
    pub f_code: f64,
    /* #ifdef FLOAT_CARR_PHASE
        double carr_phase;
    #endif */
    /// Carrier phase
    pub carr_phase: u32,
    /// Carrier phasestep
    pub carr_phasestep: i32,
    /// Code phase
    pub code_phase: f64,
    /// GPS time at start
    pub g0: GpsTime,
    /// current subframe
    pub sbf: [[u32; N_DWRD_SBF]; 5],
    /// Data words of sub-frame
    pub dwrd: [u32; N_DWRD],
    /// initial word
    pub iword: i32,
    /// initial bit
    pub ibit: i32,
    /// initial code
    pub icode: i32,
    ///  current data bit
    pub dataBit: i32,
    ///  current C/A code
    pub codeCA: i32,
    pub azel: [f64; 2],
    pub rho0: TimeRange,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            prn: 0,
            ca: [0; CA_SEQ_LEN],
            f_carr: 0.0,
            f_code: 0.0,
            carr_phase: 0,
            carr_phasestep: 0,
            code_phase: 0.0,
            g0: GpsTime { week: 0, sec: 0. },
            sbf: [[0; N_DWRD_SBF]; 5],
            dwrd: [0; N_DWRD],
            iword: 0,
            ibit: 0,
            icode: 0,
            dataBit: 0,
            codeCA: 0,
            azel: [0.0; 2],
            rho0: TimeRange::default(),
        }
    }
}
impl Channel {
    ///  \brief Compute the code phase for a given channel (satellite)
    ///  \param chan Channel on which we operate (is updated)
    ///  \param[in] rho1 Current range, after \a dt has expired
    ///  \param[in dt delta-t (time difference) in seconds
    #[inline]
    pub fn compute_code_phase(&mut self, rho1: &TimeRange, dt: f64) {
        // Pseudorange rate.
        let rhorate = (rho1.range - self.rho0.range) / dt;
        // Carrier and code frequency.
        self.f_carr = -rhorate / LAMBDA_L1;
        self.f_code = CODE_FREQ + self.f_carr * CARR_TO_CODE;
        // Initial code phase and data bit counters.
        let ms = (self.rho0.time.diff_secs(&self.g0) + 6.0
            - self.rho0.range / SPEED_OF_LIGHT)
            * 1000.0;
        let mut ims = ms as i32;
        self.code_phase = (ms - f64::from(ims)) * CA_SEQ_LEN as f64; // in chip
        self.iword = ims / 600; // 1 word = 30 bits = 600 ms
        ims -= self.iword * 600;
        self.ibit = ims / 20; // 1 bit = 20 code = 20 ms
        ims -= self.ibit * 20;
        self.icode = ims; // 1 code = 1 ms
        self.codeCA = self.ca[self.code_phase as usize] * 2 - 1;
        self.dataBit = (self.dwrd[self.iword as usize] >> (29 - self.ibit)
            & 0x1) as i32
            * 2
            - 1;
        // Save current pseudorange
        self.rho0 = rho1.clone();
    }

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
        self.g0 = time_init; // Data bit reference time

        if init {
            // Initialize subframe 5
            prevwrd = 0;
            for iwrd in 0..N_DWRD_SBF {
                sbfwrd = self.sbf[4][iwrd];
                // Add TOW-count message into HOW
                if iwrd == 1 {
                    sbfwrd |= (tow & 0x1ffff) << 13;
                }
                // Compute checksum
                sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
                nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
                self.dwrd[iwrd] = Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.dwrd[iwrd];
            }
        } else {
            // Save subframe 5
            for iwrd in 0..N_DWRD_SBF {
                self.dwrd[iwrd] = self.dwrd[N_DWRD_SBF * N_SBF + iwrd];
                prevwrd = self.dwrd[iwrd];
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
                sbfwrd = self.sbf[isbf][iwrd];
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
                self.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd] =
                    Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd];
            }
        }
    }

    ///  \brief Compute the Checksum for one given word of a subframe
    ///  \param[in] source The input data
    ///  \param[in] nib Does this word contain non-information-bearing bits?
    ///  \returns Computed Checksum
    #[allow(non_snake_case)]
    #[inline]
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
}
