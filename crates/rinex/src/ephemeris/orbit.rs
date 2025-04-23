/// BROADCAST ORBIT - 1
#[derive(Debug, Clone, Default)]
pub struct Orbit1 {
    /// IODE Issue of Data, Ephemeris
    pub iode: f64,
    /// Crs                 (meters)
    pub crs: f64,
    /// Delta n             (radians/sec)
    pub delta_n: f64,
    /// M0                  (radians)
    pub m0: f64,
}

/// BROADCAST ORBIT - 2
#[derive(Debug, Clone, Default)]
pub struct Orbit2 {
    /// Cuc                 (radians)
    pub cuc: f64,
    /// Eccentricity
    pub ecc: f64,
    /// Cus                 (radians)
    pub cus: f64,
    /// sqrt(A)             (sqrt(m))
    pub sqrta: f64,
}

/// BROADCAST ORBIT - 3
#[derive(Debug, Clone, Default)]
pub struct Orbit3 {
    /// Time of Ephemeris (sec of GPS week)
    pub toe: f64,
    /// Cic                 (radians)
    pub cic: f64,
    /// Omega               (radians)
    pub omega: f64,
    /// Cis                 (radians)
    pub cis: f64,
}

/// BROADCAST ORBIT - 4
#[derive(Debug, Clone, Default)]
pub struct Orbit4 {
    /// i0                  (radians)
    pub i0: f64,
    /// Crc                 (meters)
    pub crc: f64,
    /// Omega               (radians)
    pub omega: f64,
    /// Omega dot           (radians/sec)
    pub omega_dot: f64,
}

/// BROADCAST ORBIT - 5
#[derive(Debug, Clone, Default)]
pub struct Orbit5 {
    /// IDOT                (radians/sec)
    pub idot: f64,
    /// Codes on L2 channel
    pub code_l2: f64,
    /// GPS Week # (to go with TOE)
    pub week: f64,
    /// L2 Pseudorange flag
    pub l2_pseudorange: f64,
}

/// BROADCAST ORBIT - 6
#[derive(Debug, Clone, Default)]
pub struct Orbit6 {
    /// SV accuracy         (meters)
    pub sv_accuracy: f64,
    /// SV health           (MSB only)
    pub sv_health: f64,
    /// TGD                 (seconds)
    pub tgd: f64,
    /// IODC Issue of Data, Clock
    pub iodc: f64,
}

/// BROADCAST ORBIT - 7
#[derive(Debug, Clone, Default)]
pub struct Orbit7 {
    /// Transmission time of message (sec of GPS week, derived e.g. from
    /// Z-count in Hand Over Word (HOW)
    pub tom: f64,
    pub spare1: f64,
    pub spare2: f64,
    pub spare3: f64,
}

impl From<[f64; 4]> for Orbit1 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            iode: data[0],
            crs: data[1],
            delta_n: data[2],
            m0: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit2 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            cuc: data[0],
            ecc: data[1],
            cus: data[2],
            sqrta: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit3 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            toe: data[0],
            cic: data[1],
            omega: data[2],
            cis: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit4 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            i0: data[0],
            crc: data[1],
            omega: data[2],
            omega_dot: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit5 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            idot: data[0],
            code_l2: data[1],
            week: data[2],
            l2_pseudorange: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit6 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            sv_accuracy: data[0],
            sv_health: data[1],
            tgd: data[2],
            iodc: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit7 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            tom: data[0],
            spare1: data[1],
            spare2: data[2],
            spare3: data[3],
        }
    }
}
