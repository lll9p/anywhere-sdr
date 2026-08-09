use std::sync::{Arc, Mutex, TryLockError};

use geometry::{Ecef, Location, Neu};

use super::motion_math::{
    apply_limited_delta, ecef_from_neu, horizontal_speed_mps,
    integrate_constant_acceleration, integrate_target_speed,
    normalize_heading_deg, shortest_heading_delta_deg, total_speed_mps,
};
use crate::Error;

/// Runtime command definitions, validation, and pending storage.
mod commands;

pub use commands::MotionCommand;
use commands::PendingMotionCommands;

/// Observable motion state snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub struct MotionSnapshot {
    /// Receiver position (ECEF meters).
    pub position_ecef: Ecef,
    /// Receiver velocity in local NEU (m/s).
    pub velocity_neu: Neu,
    /// Receiver heading in degrees (0=North, 90=East, clockwise).
    pub heading_deg: f64,
    /// Receiver speed magnitude in m/s.
    pub speed_mps: f64,
}

/// Shared pending commands and last published snapshot.
#[derive(Debug)]
struct MotionShared {
    /// Commands waiting for a generator step boundary.
    pending: Mutex<PendingMotionCommands>,
    /// Last successfully published endpoint state.
    snapshot: Mutex<MotionSnapshot>,
}

/// Thread-safe runtime motion control handle.
///
/// - Callers submit commands via [`RuntimeMotionControl::submit`].
/// - The generator hot path uses non-blocking reads to observe pending commands
///   and to publish snapshots.
#[derive(Clone, Debug)]
pub struct RuntimeMotionControl {
    /// Shared state used by submitters and the generator.
    shared: Arc<MotionShared>,
}

impl RuntimeMotionControl {
    /// Creates a runtime control seeded at an ECEF position.
    pub fn new(initial_position_ecef: Ecef) -> Self {
        Self {
            shared: Arc::new(MotionShared {
                pending: Mutex::new(PendingMotionCommands::default()),
                snapshot: Mutex::new(MotionSnapshot {
                    position_ecef: initial_position_ecef,
                    ..MotionSnapshot::default()
                }),
            }),
        }
    }

    /// Validates and retains a command for the next available step boundary.
    ///
    /// At most one command per variant is retained. Replacing a variant moves
    /// it after all other retained variants in replay order.
    pub fn submit(&self, command: MotionCommand) -> Result<(), Error> {
        command.validate()?;
        let mut guard = match self.shared.pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("motion pending lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        guard.push(command);
        Ok(())
    }

    /// Returns the last published motion snapshot.
    pub fn snapshot(&self) -> MotionSnapshot {
        match self.shared.snapshot.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                tracing::warn!("motion snapshot lock poisoned; recovering");
                *poisoned.into_inner()
            }
        }
    }

    /// Attempts to return the last published snapshot without blocking.
    pub fn try_snapshot(&self) -> Option<MotionSnapshot> {
        let guard = match self.shared.snapshot.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return None,
            Err(TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("motion snapshot lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        Some(*guard)
    }

    /// Takes pending commands without blocking the generator hot path.
    fn try_take_pending(&self) -> PendingMotionCommands {
        let mut guard = match self.shared.pending.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => {
                return PendingMotionCommands::default();
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("motion pending lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        std::mem::take(&mut *guard)
    }

    /// Publishes an endpoint snapshot without blocking the generator.
    fn try_publish_snapshot(&self, snapshot: MotionSnapshot) {
        let mut guard = match self.shared.snapshot.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return,
            Err(TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("motion snapshot lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        *guard = snapshot;
    }
}

/// Active horizontal speed target and acceleration limit.
#[derive(Debug, Clone, Copy)]
struct TargetSpeed {
    /// Target horizontal speed in meters per second.
    speed_mps: f64,
    /// Maximum horizontal acceleration in meters per second squared.
    accel_limit_mps2: f64,
}

/// Active heading target and turn-rate limit.
#[derive(Debug, Clone, Copy)]
struct TargetHeading {
    /// Normalized target heading in degrees.
    heading_deg: f64,
    /// Maximum turn rate in degrees per second.
    turn_rate_limit_dps: f64,
}

/// Runtime receiver motion state and interval integrator.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MotionIntegrator {
    /// Current receiver position in ECEF meters.
    position_ecef: Ecef,
    /// Current local NEU velocity in meters per second.
    velocity_neu: Neu,
    /// Current direct local NEU acceleration.
    acceleration_neu: Neu,
    /// Current horizontal heading in degrees.
    heading_deg: f64,
    /// Remembered nonzero horizontal speed for a later start command.
    resume_speed_mps: f64,
    /// Optional horizontal speed controller.
    target_speed: Option<TargetSpeed>,
    /// Optional heading controller.
    target_heading: Option<TargetHeading>,
}

impl MotionIntegrator {
    /// Creates stationary motion state at the initial ECEF position.
    pub(crate) fn new(initial_position_ecef: Ecef) -> Self {
        Self {
            position_ecef: initial_position_ecef,
            velocity_neu: Neu::default(),
            acceleration_neu: Neu::default(),
            heading_deg: 0.0,
            resume_speed_mps: 0.0,
            target_speed: None,
            target_heading: None,
        }
    }

    /// Advances one interval after replaying all retained commands.
    pub(crate) fn step(
        &mut self, dt: f64, control: &RuntimeMotionControl,
    ) -> Result<Ecef, Error> {
        let pending = control.try_take_pending();
        self.apply_pending(&pending);

        let displacement_neu = self.integrate_interval(dt);
        self.apply_displacement(displacement_neu)?;

        control.try_publish_snapshot(self.snapshot());
        Ok(self.position_ecef)
    }

    /// Replays retained variants oldest-to-newest.
    fn apply_pending(&mut self, pending: &PendingMotionCommands) {
        for command in pending.iter() {
            self.apply_command(command);
        }
    }

    /// Applies one command's complete state transition.
    fn apply_command(&mut self, command: MotionCommand) {
        match command {
            MotionCommand::SetPositionEcef(position_ecef) => {
                self.position_ecef = position_ecef;
            }
            MotionCommand::SetVelocityNeu(velocity_neu) => {
                self.set_velocity_neu(velocity_neu);
                self.clear_targets();
            }
            MotionCommand::SetAccelerationNeu(acceleration_neu) => {
                self.acceleration_neu = acceleration_neu;
                self.clear_targets();
            }
            MotionCommand::SetHeadingSpeed {
                heading_deg,
                speed_mps,
                climb_mps,
            } => {
                self.set_heading_speed(heading_deg, speed_mps, climb_mps);
                self.clear_targets();
            }
            MotionCommand::Stop => {
                let current_speed = horizontal_speed_mps(self.velocity_neu);
                if current_speed > 0.0 {
                    self.resume_speed_mps = current_speed;
                }
                self.velocity_neu = Neu::default();
                self.acceleration_neu = Neu::default();
                self.clear_targets();
            }
            MotionCommand::Start { speed_mps } => {
                let speed_mps = match speed_mps {
                    Some(speed_mps) => speed_mps,
                    None if self.resume_speed_mps > 0.0 => {
                        self.resume_speed_mps
                    }
                    None => 1.0,
                };
                self.set_heading_speed(self.heading_deg, speed_mps, 0.0);
                self.acceleration_neu = Neu::default();
                self.clear_targets();
            }
            MotionCommand::SetTargetSpeed {
                speed_mps,
                accel_limit_mps2,
            } => {
                self.target_speed = Some(TargetSpeed {
                    speed_mps,
                    accel_limit_mps2,
                });
                self.acceleration_neu = Neu::default();
            }
            MotionCommand::SetTargetHeading {
                heading_deg,
                turn_rate_limit_dps,
            } => {
                self.target_heading = Some(TargetHeading {
                    heading_deg: normalize_heading_deg(heading_deg),
                    turn_rate_limit_dps,
                });
                self.acceleration_neu = Neu::default();
            }
        }
    }

    /// Integrates controller or direct acceleration state for one interval.
    fn integrate_interval(&mut self, dt: f64) -> Neu {
        let speed_mps = horizontal_speed_mps(self.velocity_neu);

        if let Some(target_heading) = self.target_heading {
            let current_heading = normalize_heading_deg(self.heading_deg);
            let delta = shortest_heading_delta_deg(
                current_heading,
                target_heading.heading_deg,
            );
            let max_delta = (target_heading.turn_rate_limit_dps * dt).max(0.0);
            let applied = apply_limited_delta(delta, max_delta);
            self.heading_deg = normalize_heading_deg(current_heading + applied);
        }

        if let Some(target_speed) = self.target_speed {
            let motion = integrate_target_speed(
                speed_mps,
                target_speed.speed_mps,
                target_speed.accel_limit_mps2,
                dt,
            );
            let heading_rad = self.heading_deg.to_radians();
            let climb_mps = self.velocity_neu.up;
            self.set_heading_speed(
                self.heading_deg,
                motion.endpoint_speed,
                climb_mps,
            );
            return Neu {
                north: heading_rad.cos() * motion.distance,
                east: heading_rad.sin() * motion.distance,
                up: climb_mps * dt,
            };
        }

        if self.target_heading.is_some() {
            let climb_mps = self.velocity_neu.up;
            self.set_heading_speed(self.heading_deg, speed_mps, climb_mps);
            return Neu {
                north: self.velocity_neu.north * dt,
                east: self.velocity_neu.east * dt,
                up: self.velocity_neu.up * dt,
            };
        }

        let motion = integrate_constant_acceleration(
            self.velocity_neu,
            self.acceleration_neu,
            dt,
        );
        self.velocity_neu = motion.endpoint_velocity_neu;
        self.update_heading_from_velocity();
        motion.displacement_neu
    }

    /// Converts start-local NEU displacement and commits the ECEF endpoint.
    fn apply_displacement(&mut self, displacement: Neu) -> Result<(), Error> {
        let reference_location = Location::try_from(&self.position_ecef)?;
        let ltcmat = reference_location.ltcmat();
        let displacement_ecef = ecef_from_neu(displacement, ltcmat);

        self.position_ecef.x += displacement_ecef.x;
        self.position_ecef.y += displacement_ecef.y;
        self.position_ecef.z += displacement_ecef.z;
        Ok(())
    }

    /// Builds the current public endpoint snapshot.
    fn snapshot(&self) -> MotionSnapshot {
        MotionSnapshot {
            position_ecef: self.position_ecef,
            velocity_neu: self.velocity_neu,
            heading_deg: normalize_heading_deg(self.heading_deg),
            speed_mps: total_speed_mps(self.velocity_neu),
        }
    }

    /// Replaces direct velocity and derives horizontal heading.
    fn set_velocity_neu(&mut self, velocity_neu: Neu) {
        self.velocity_neu = velocity_neu;
        self.update_heading_from_velocity();
    }

    /// Replaces heading, horizontal speed, and climb rate.
    fn set_heading_speed(
        &mut self, heading_deg: f64, speed_mps: f64, climb_mps: f64,
    ) {
        let heading_deg = normalize_heading_deg(heading_deg);
        let heading_rad = heading_deg.to_radians();
        self.velocity_neu = Neu {
            north: heading_rad.cos() * speed_mps,
            east: heading_rad.sin() * speed_mps,
            up: climb_mps,
        };
        self.heading_deg = heading_deg;
    }

    /// Updates heading only when horizontal velocity is nonzero.
    fn update_heading_from_velocity(&mut self) {
        let horizontal_speed = horizontal_speed_mps(self.velocity_neu);
        if horizontal_speed > 0.0 {
            self.heading_deg = normalize_heading_deg(
                self.velocity_neu
                    .east
                    .atan2(self.velocity_neu.north)
                    .to_degrees(),
            );
        }
    }

    /// Clears both target controllers.
    fn clear_targets(&mut self) {
        self.target_speed = None;
        self.target_heading = None;
    }
}

#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod tests;
