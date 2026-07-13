use geometry::{Ecef, Location};
use gps::{MotionCommand, MotionSnapshot, RuntimeMotionControl};

use crate::{Error, tui_config::ManualMotionConfig};

pub(super) const MANUAL_HEADING_STEP_DEG: f64 = 5.0;
pub(super) const MANUAL_SPEED_STEP_MPS: f64 = 1.0;

#[derive(Debug, Clone)]
pub(super) struct ManualControlSession {
    pub(super) control: RuntimeMotionControl,
    pub(super) target_heading_deg: f64,
    pub(super) target_speed_mps: f64,
    pub(super) cruise_speed_mps: f64,
    pub(super) accel_limit_mps2: f64,
    pub(super) turn_rate_limit_dps: f64,
    pub(super) last_snapshot: Option<MotionSnapshot>,
}

impl ManualControlSession {
    pub(super) fn from_config(
        config: &ManualMotionConfig,
    ) -> Result<Self, Error> {
        let Some(initial_llh) = config.initial_llh else {
            return Err(Error::msg(
                "manual mode requires initial LLH position".to_string(),
            ));
        };

        let control =
            RuntimeMotionControl::new(initial_position_ecef(initial_llh)?);
        let target_heading_deg =
            normalize_heading_deg(config.initial_heading_deg);
        control.submit(MotionCommand::SetHeadingSpeed {
            heading_deg: target_heading_deg,
            speed_mps: config.cruise_speed_mps,
            climb_mps: 0.0,
        });

        Ok(Self {
            last_snapshot: Some(control.snapshot()),
            control,
            target_heading_deg,
            target_speed_mps: config.cruise_speed_mps,
            cruise_speed_mps: config.cruise_speed_mps,
            accel_limit_mps2: config.accel_limit_mps2,
            turn_rate_limit_dps: config.turn_rate_limit_dps,
        })
    }

    pub(super) fn refresh_snapshot(&mut self) {
        if let Some(snapshot) = self.control.try_snapshot() {
            self.last_snapshot = Some(snapshot);
        }
    }

    pub(super) fn adjust_heading(&mut self, delta_deg: f64) {
        self.target_heading_deg =
            normalize_heading_deg(self.target_heading_deg + delta_deg);
        self.control.submit(MotionCommand::SetTargetHeading {
            heading_deg: self.target_heading_deg,
            turn_rate_limit_dps: self.turn_rate_limit_dps,
        });
    }

    pub(super) fn adjust_speed(&mut self, delta_mps: f64) {
        self.target_speed_mps = (self.target_speed_mps + delta_mps).max(0.0);
        if self.target_speed_mps > 0.0 {
            self.cruise_speed_mps = self.target_speed_mps;
        }
        self.control.submit(MotionCommand::SetTargetSpeed {
            speed_mps: self.target_speed_mps,
            accel_limit_mps2: self.accel_limit_mps2,
        });
    }

    pub(super) fn stop(&mut self) {
        self.target_speed_mps = 0.0;
        self.control.submit(MotionCommand::SetTargetSpeed {
            speed_mps: 0.0,
            accel_limit_mps2: self.accel_limit_mps2,
        });
    }

    pub(super) fn resume_cruise(&mut self) {
        self.target_speed_mps = self.cruise_speed_mps;
        self.control.submit(MotionCommand::SetTargetSpeed {
            speed_mps: self.target_speed_mps,
            accel_limit_mps2: self.accel_limit_mps2,
        });
    }

    pub(super) fn actual_position_llh(
        &self,
    ) -> Result<Option<[f64; 3]>, geometry::Error> {
        let Some(snapshot) = self.last_snapshot.as_ref() else {
            return Ok(None);
        };
        Ok(Some(display_llh(snapshot.position_ecef)?))
    }
}

pub(super) fn format_llh(
    [latitude_deg, longitude_deg, height_m]: [f64; 3],
) -> String {
    format!("{latitude_deg:.6}, {longitude_deg:.6}, {height_m:.1}")
}

fn normalize_heading_deg(heading_deg: f64) -> f64 {
    let normalized = heading_deg % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

fn initial_position_ecef(
    [latitude_deg, longitude_deg, height_m]: [f64; 3],
) -> Result<Ecef, geometry::Error> {
    let location =
        Location::try_from_degrees(latitude_deg, longitude_deg, height_m)?;
    Ok(Ecef::from(&location))
}

fn display_llh(position_ecef: Ecef) -> Result<[f64; 3], geometry::Error> {
    let location = Location::try_from(&position_ecef)?;
    Ok([
        location.latitude_degrees(),
        location.longitude_degrees(),
        location.height_meters(),
    ])
}
