#![allow(unused)]
pub const USER_MOTION_SIZE: usize = 3000;
#[allow(clippy::approx_constant)]
pub const PI: f64 = 3.141_592_653_589_8;

// \brief Maximum number of satellites in RINEX file */
pub const MAX_SAT: usize = 32;

// \brief Maximum number of channels we simulate */
pub const MAX_CHAN: usize = 16;

// \brief Maximum duration for static mode*/
pub const STATIC_MAX_DURATION: usize = 86400; // second

// \brief Number of subframes */
pub const N_SBF: usize = 5; // 5 subframes per frame

// \brief Number of words per subframe */
pub const N_DWRD_SBF: usize = 10; // 10 word per subframe

// \brief Number of words */
pub const N_DWRD: usize = (N_SBF + 1) * N_DWRD_SBF; // Subframe word buffer size

// \brief C/A code sequence length */
pub const CA_SEQ_LEN: usize = 1023;
pub const CA_SEQ_LEN_FLOAT: f64 = CA_SEQ_LEN as f64;

pub const SECONDS_IN_WEEK: f64 = 604_800.0;
pub const SECONDS_IN_HALF_WEEK: f64 = 302_400.0;
pub const SECONDS_IN_DAY: f64 = 86400.0;
pub const SECONDS_IN_HOUR: f64 = 3600.0;
pub const SECONDS_IN_MINUTE: f64 = 60.0;

pub const POW2_M5: f64 = 0.03125;
pub const POW2_M19: f64 = 1.907_348_632_812_5e-6;
pub const POW2_M29: f64 = 1.862_645_149_230_957e-9;
pub const POW2_M31: f64 = 4.656_612_873_077_393e-10;
pub const POW2_M33: f64 = 1.164_153_218_269_348e-10;
pub const POW2_M43: f64 = 1.136_868_377_216_16e-13;
pub const POW2_M55: f64 = 2.775_557_561_562_891e-17;

pub const POW2_M50: f64 = 8.881_784_197_001_252e-16;
pub const POW2_M30: f64 = 9.313_225_746_154_785e-10;
pub const POW2_M27: f64 = 7.450_580_596_923_828e-9;
pub const POW2_M24: f64 = 5.960_464_477_539_063e-8;

// Conventional values employed in GPS ephemeris model (ICD-GPS-200)
pub const GM_EARTH: f64 = 3.986_005e14;
pub const OMEGA_EARTH: f64 = 7.292_115_146_7e-5;

pub const WGS84_RADIUS: f64 = 6_378_137.0;
pub const WGS84_ECCENTRICITY: f64 = 0.081_819_190_842_6;

pub const R2D: f64 = 57.295_779_513_1;

pub const SPEED_OF_LIGHT: f64 = 2.997_924_58e8;
pub const SPEED_OF_LIGHT_INV: f64 = SPEED_OF_LIGHT.recip();
pub const LAMBDA_L1: f64 = 0.190_293_672_798_365;
pub const LAMBDA_L1_INV: f64 = LAMBDA_L1.recip();

// \brief GPS L1 Carrier frequency */
#[allow(dead_code)]
pub const CARR_FREQ: f64 = 1575.42e6;
// \brief C/A code frequency */
pub const CODE_FREQ: f64 = 1.023e6;
pub const CARR_TO_CODE: f64 = 1.0 / 1540.0;

// Sampling data format
pub const SC01: i32 = 1;
pub const SC08: i32 = 8;
#[allow(dead_code)]
pub const SC16: i32 = 16;

pub const EPHEM_ARRAY_SIZE: usize = 15; // for daily GPS broadcast ephemers file (brdc)

pub const SAMPLE_RATE: f64 = 0.1;
