use crate::{
    constants::*,
    datetime::{GpsTime, TimeRange},
};

///  Structure representing a Channel
#[allow(non_snake_case)]
#[derive(Copy, Clone)]
pub struct Channel {
    /// PRN Number(Pseudorandom Noise)
    pub prn: i32,
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
impl Channel {
    ///  \brief Compute the code phase for a given channel (satellite)
    ///  \param chan Channel on which we operate (is updated)
    ///  \param[in] rho1 Current range, after \a dt has expired
    ///  \param[in dt delta-t (time difference) in seconds
    #[inline]
    pub fn compute_code_phase(&mut self, rho1: TimeRange, dt: f64) {
        // Pseudorange rate.
        let rhorate = (rho1.range - self.rho0.range) / dt;
        // Carrier and code frequency.
        self.f_carr = -rhorate / LAMBDA_L1;
        self.f_code = CODE_FREQ + self.f_carr * CARR_TO_CODE;
        // Initial code phase and data bit counters.
        let ms = (self.rho0.g.diff_secs(&self.g0) + 6.0
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
        self.rho0 = rho1;
    }
}
