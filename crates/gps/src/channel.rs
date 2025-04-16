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
