mod orbit;
pub use self::orbit::*;
use crate::error::Error;

#[derive(Debug, Clone, Default)]
pub struct SvClock {
    pub bias: f64,
    pub drift: f64,
    pub drift_rate: f64,
}
impl SvClock {
    pub fn new(bias: f64, drift: f64, drift_rate: f64) -> Self {
        Self {
            bias,
            drift,
            drift_rate,
        }
    }
}

// Structure representing ephemeris of a single satellite
#[derive(Debug, Clone, Default)]
pub struct Ephemeris {
    /// Satellite PRN number
    pub prn: usize,
    /// Epoch: Toc - Time of Clock
    pub time_of_clock: jiff::Timestamp,
    pub sv_clock: SvClock,
    pub orbit1: Orbit1,
    pub orbit2: Orbit2,
    pub orbit3: Orbit3,
    pub orbit4: Orbit4,
    pub orbit5: Orbit5,
    pub orbit6: Orbit6,
    pub orbit7: Orbit7,
}

#[derive(Debug, Default)]
pub struct EphemerisBuilder {
    prn: Option<usize>,
    time_of_clock: Option<jiff::Timestamp>,
    sv_clock: Option<SvClock>,
    orbit1: Option<Orbit1>,
    orbit2: Option<Orbit2>,
    orbit3: Option<Orbit3>,
    orbit4: Option<Orbit4>,
    orbit5: Option<Orbit5>,
    orbit6: Option<Orbit6>,
    orbit7: Option<Orbit7>,
}
impl EphemerisBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_prn(&mut self, prn: usize) {
        self.prn.replace(prn);
    }

    pub fn set_time_of_clock(&mut self, time_of_clock: jiff::Timestamp) {
        self.time_of_clock.replace(time_of_clock);
    }

    pub fn set_sv_clock(&mut self, sv_clock: SvClock) {
        self.sv_clock.replace(sv_clock);
    }

    pub fn set_orbit1(&mut self, orbit1: Orbit1) {
        self.orbit1.replace(orbit1);
    }

    pub fn set_orbit2(&mut self, orbit2: Orbit2) {
        self.orbit2.replace(orbit2);
    }

    pub fn set_orbit3(&mut self, orbit3: Orbit3) {
        self.orbit3.replace(orbit3);
    }

    pub fn set_orbit4(&mut self, orbit4: Orbit4) {
        self.orbit4.replace(orbit4);
    }

    pub fn set_orbit5(&mut self, orbit5: Orbit5) {
        self.orbit5.replace(orbit5);
    }

    pub fn set_orbit6(&mut self, orbit6: Orbit6) {
        self.orbit6.replace(orbit6);
    }

    pub fn set_orbit7(&mut self, orbit7: Orbit7) {
        self.orbit7.replace(orbit7);
    }

    pub fn build(&mut self) -> Result<Ephemeris, Error> {
        fn take<T>(v: &mut Option<T>, msg: &str) -> Result<T, Error> {
            v.take().ok_or_else(|| Error::EphemerisBuilder(msg.into()))
        }
        let ephemeris = Ephemeris {
            prn: take(&mut self.prn, "prn is none")?,
            time_of_clock: take(
                &mut self.time_of_clock,
                "time_of_clock is none",
            )?,
            sv_clock: take(&mut self.sv_clock, "sv_clock is none")?,
            orbit1: take(&mut self.orbit1, "orbit1 is none")?,
            orbit2: take(&mut self.orbit2, "orbit2 is none")?,
            orbit3: take(&mut self.orbit3, "orbit3 is none")?,
            orbit4: take(&mut self.orbit4, "orbit4 is none")?,
            orbit5: take(&mut self.orbit5, "orbit5 is none")?,
            orbit6: take(&mut self.orbit6, "orbit6 is none")?,
            orbit7: take(&mut self.orbit7, "orbit7 is none")?,
        };
        Ok(ephemeris)
    }
}
