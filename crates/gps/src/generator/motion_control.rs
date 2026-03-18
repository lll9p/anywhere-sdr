use std::sync::{Arc, Mutex, TryLockError};

use geometry::{Ecef, Location, Neu};

/// Runtime motion control commands.
///
/// Commands are applied at the next simulation step boundary.
#[derive(Debug, Clone, Copy)]
pub enum MotionCommand {
    /// Set absolute receiver position (ECEF meters).
    SetPositionEcef(Ecef),
    /// Set receiver velocity in local NEU (m/s).
    SetVelocityNeu(Neu),
    /// Set receiver acceleration in local NEU (m/s^2).
    SetAccelerationNeu(Neu),
    /// Set heading (deg) and speed (m/s); optional climb rate (m/s).
    ///
    /// Heading uses 0=North, 90=East, clockwise.
    SetHeadingSpeed {
        heading_deg: f64,
        speed_mps: f64,
        climb_mps: f64,
    },
    /// Stop motion (speed becomes 0).
    Stop,
    /// Start motion from a stopped state.
    ///
    /// If `speed_mps` is None, the integrator may resume a previously stored
    /// speed (implementation-defined but must be predictable).
    Start { speed_mps: Option<f64> },
    /// Set a target speed with an acceleration limit (m/s^2).
    SetTargetSpeed {
        speed_mps: f64,
        accel_limit_mps2: f64,
    },
    /// Set a target heading with a turn-rate limit (deg/s).
    SetTargetHeading {
        heading_deg: f64,
        turn_rate_limit_dps: f64,
    },
}

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

/// Pending motion updates aggregated from [`MotionCommand`] submissions.
///
/// The generator consumes these updates at step boundaries. When multiple
/// commands update the same field within one step interval, the latest value
/// wins.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PendingMotionUpdates {
    /// Absolute receiver position override (ECEF meters).
    pub set_position_ecef: Option<Ecef>,
    /// Receiver velocity override in local NEU (m/s).
    pub set_velocity_neu: Option<Neu>,
    /// Receiver acceleration override in local NEU (m/s^2).
    pub set_acceleration_neu: Option<Neu>,
    /// Heading/speed/climb override.
    ///
    /// Stored as `(heading_deg, speed_mps, climb_mps)`.
    pub set_heading_speed: Option<(f64, f64, f64)>,
    /// Whether a stop command was requested.
    pub stop: bool,
    /// Whether a start command was requested.
    pub start: bool,
    /// Optional speed to use for the start command (m/s).
    pub start_speed_mps: Option<f64>,
    /// Target speed and acceleration limit.
    ///
    /// Stored as `(speed_mps, accel_limit_mps2)`.
    pub set_target_speed: Option<(f64, f64)>,
    /// Target heading and turn-rate limit.
    ///
    /// Stored as `(heading_deg, turn_rate_limit_dps)`.
    pub set_target_heading: Option<(f64, f64)>,
}

impl PendingMotionUpdates {
    /// Apply a single [`MotionCommand`] into this update set.
    ///
    /// This is a best-effort, "latest-wins" merge used at step boundaries.
    fn apply_command(&mut self, command: MotionCommand) {
        match command {
            MotionCommand::SetPositionEcef(ecef) => {
                self.set_position_ecef = Some(ecef);
            }
            MotionCommand::SetVelocityNeu(neu) => {
                self.set_velocity_neu = Some(neu);
            }
            MotionCommand::SetAccelerationNeu(neu) => {
                self.set_acceleration_neu = Some(neu);
            }
            MotionCommand::SetHeadingSpeed {
                heading_deg,
                speed_mps,
                climb_mps,
            } => {
                self.set_heading_speed =
                    Some((heading_deg, speed_mps, climb_mps));
            }
            MotionCommand::Stop => {
                self.stop = true;
            }
            MotionCommand::Start { speed_mps } => {
                self.start = true;
                self.start_speed_mps = speed_mps;
            }
            MotionCommand::SetTargetSpeed {
                speed_mps,
                accel_limit_mps2,
            } => {
                self.set_target_speed = Some((speed_mps, accel_limit_mps2));
            }
            MotionCommand::SetTargetHeading {
                heading_deg,
                turn_rate_limit_dps,
            } => {
                self.set_target_heading =
                    Some((heading_deg, turn_rate_limit_dps));
            }
        }
    }
}

#[derive(Debug)]
/// Shared runtime motion control state.
struct MotionShared {
    /// Pending updates waiting to be consumed by the generator.
    pending: Mutex<PendingMotionUpdates>,
    /// Last published motion snapshot.
    snapshot: Mutex<MotionSnapshot>,
}

/// Thread-safe runtime motion control handle.
///
/// - Callers submit commands via [`RuntimeMotionControl::submit`].
/// - The generator hot path uses non-blocking reads to observe pending updates
///   and to publish snapshots.
#[derive(Clone, Debug)]
pub struct RuntimeMotionControl {
    /// Shared pending and snapshot state.
    shared: Arc<MotionShared>,
}

impl RuntimeMotionControl {
    pub fn new(initial_position_ecef: Ecef) -> Self {
        Self {
            shared: Arc::new(MotionShared {
                pending: Mutex::new(PendingMotionUpdates::default()),
                snapshot: Mutex::new(MotionSnapshot {
                    position_ecef: initial_position_ecef,
                    ..MotionSnapshot::default()
                }),
            }),
        }
    }

    pub fn submit(&self, command: MotionCommand) {
        let mut guard = match self.shared.pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("motion pending lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        guard.apply_command(command);
    }

    pub fn snapshot(&self) -> MotionSnapshot {
        match self.shared.snapshot.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                tracing::warn!("motion snapshot lock poisoned; recovering");
                *poisoned.into_inner()
            }
        }
    }

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

    /// Non-blocking take of all pending updates.
    ///
    /// If the generator cannot acquire the pending lock immediately, this
    /// returns an empty update set.
    pub(crate) fn try_take_pending(&self) -> PendingMotionUpdates {
        let mut guard = match self.shared.pending.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => {
                return PendingMotionUpdates::default();
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("motion pending lock poisoned; recovering");
                poisoned.into_inner()
            }
        };
        std::mem::take(&mut *guard)
    }

    /// Non-blocking publish of a new snapshot.
    ///
    /// If the snapshot lock is contended, the publish is skipped.
    pub(crate) fn try_publish_snapshot(&self, snapshot: MotionSnapshot) {
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

/// Target speed controller parameters.
#[derive(Debug, Clone, Copy)]
struct TargetSpeed {
    /// Target horizontal speed (m/s).
    speed_mps: f64,
    /// Maximum horizontal acceleration magnitude (m/s^2).
    accel_limit_mps2: f64,
}

/// Target heading controller parameters.
#[derive(Debug, Clone, Copy)]
struct TargetHeading {
    /// Target heading in degrees (0=North, 90=East, clockwise).
    heading_deg: f64,
    /// Maximum turn rate magnitude (deg/s).
    turn_rate_limit_dps: f64,
}

/// Motion integrator for runtime receiver movement.
///
/// The integrator maintains velocity/acceleration in local NEU coordinates and
/// produces an updated ECEF position each step.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MotionIntegrator {
    /// Current receiver position in ECEF meters.
    position_ecef: Ecef,
    /// Current receiver velocity in local NEU (m/s).
    velocity_neu: Neu,
    /// Current receiver acceleration in local NEU (m/s^2).
    acceleration_neu: Neu,
    /// Current receiver heading in degrees.
    heading_deg: f64,
    /// Speed to resume when receiving a start command without an explicit
    /// speed.
    resume_speed_mps: f64,
    /// Optional target speed controller.
    target_speed: Option<TargetSpeed>,
    /// Optional target heading controller.
    target_heading: Option<TargetHeading>,
}

impl MotionIntegrator {
    /// Create a new integrator seeded at the provided initial position.
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

    /// Advance the integrator by one time step.
    ///
    /// This applies any pending commands, updates targets/acceleration, updates
    /// position, and publishes a snapshot back to the controller.
    pub(crate) fn step(
        &mut self, dt: f64, control: &RuntimeMotionControl,
    ) -> Ecef {
        let pending = control.try_take_pending();
        self.apply_pending(pending);

        self.apply_targets(dt);
        self.apply_acceleration(dt);
        self.integrate_position(dt);

        let snapshot = self.snapshot();
        control.try_publish_snapshot(snapshot);
        self.position_ecef
    }

    /// Apply pending updates collected since the last step.
    fn apply_pending(&mut self, pending: PendingMotionUpdates) {
        if let Some(position_ecef) = pending.set_position_ecef {
            self.position_ecef = position_ecef;
        }

        if let Some(velocity_neu) = pending.set_velocity_neu {
            self.set_velocity_neu(velocity_neu);
            self.clear_targets();
        }

        if let Some((heading_deg, speed_mps, climb_mps)) =
            pending.set_heading_speed
        {
            self.set_heading_speed(heading_deg, speed_mps, climb_mps);
            self.clear_targets();
        }

        if let Some(acceleration_neu) = pending.set_acceleration_neu {
            self.acceleration_neu = acceleration_neu;
            self.clear_targets();
        }

        if let Some((speed_mps, accel_limit_mps2)) = pending.set_target_speed {
            self.target_speed = Some(TargetSpeed {
                speed_mps,
                accel_limit_mps2,
            });
            self.acceleration_neu = Neu::default();
        }

        if let Some((heading_deg, turn_rate_limit_dps)) =
            pending.set_target_heading
        {
            self.target_heading = Some(TargetHeading {
                heading_deg: normalize_heading_deg(heading_deg),
                turn_rate_limit_dps,
            });
            self.acceleration_neu = Neu::default();
        }

        if pending.stop {
            let current_speed = horizontal_speed_mps(self.velocity_neu);
            if current_speed > 0.0 {
                self.resume_speed_mps = current_speed;
            }
            self.velocity_neu = Neu::default();
            self.acceleration_neu = Neu::default();
            self.clear_targets();
        }

        if pending.start {
            let speed_mps = match pending.start_speed_mps {
                Some(speed_mps) => speed_mps,
                None if self.resume_speed_mps > 0.0 => self.resume_speed_mps,
                None => 1.0,
            };
            self.set_heading_speed(self.heading_deg, speed_mps, 0.0);
            self.acceleration_neu = Neu::default();
            self.clear_targets();
        }
    }

    /// Apply target heading/speed controllers, respecting per-step limits.
    fn apply_targets(&mut self, dt: f64) {
        let horizontal_speed = horizontal_speed_mps(self.velocity_neu);

        let apply_limited_delta = |delta: f64, max_delta: f64| -> f64 {
            if max_delta == 0.0 {
                delta
            } else {
                delta.clamp(-max_delta, max_delta)
            }
        };

        if let Some(target_heading) = self.target_heading {
            let current_heading = normalize_heading_deg(self.heading_deg);
            let delta = shortest_heading_delta_deg(
                current_heading,
                target_heading.heading_deg,
            );
            let max_delta =
                (target_heading.turn_rate_limit_dps.abs() * dt).max(0.0);
            let applied = apply_limited_delta(delta, max_delta);
            self.heading_deg = normalize_heading_deg(current_heading + applied);
        }

        if let Some(target_speed) = self.target_speed {
            let delta = target_speed.speed_mps - horizontal_speed;
            let max_delta = (target_speed.accel_limit_mps2.abs() * dt).max(0.0);
            let applied = apply_limited_delta(delta, max_delta);
            let new_speed = (horizontal_speed + applied).max(0.0);
            let climb_mps = self.velocity_neu.up;
            self.set_heading_speed(self.heading_deg, new_speed, climb_mps);
        } else if self.target_heading.is_some() {
            // Heading-only target: rotate the horizontal velocity while keeping
            // speed.
            let climb_mps = self.velocity_neu.up;
            self.set_heading_speed(
                self.heading_deg,
                horizontal_speed,
                climb_mps,
            );
        }
    }

    /// Apply acceleration to velocity when no target controllers are active.
    fn apply_acceleration(&mut self, dt: f64) {
        if self.target_speed.is_some() || self.target_heading.is_some() {
            return;
        }

        self.velocity_neu.north += self.acceleration_neu.north * dt;
        self.velocity_neu.east += self.acceleration_neu.east * dt;
        self.velocity_neu.up += self.acceleration_neu.up * dt;

        self.update_heading_from_velocity();
    }

    /// Integrate position using the current velocity.
    fn integrate_position(&mut self, dt: f64) {
        let displacement_neu = Neu {
            north: self.velocity_neu.north * dt,
            east: self.velocity_neu.east * dt,
            up: self.velocity_neu.up * dt,
        };

        let reference_location = Location::from(&self.position_ecef);
        let ltcmat = reference_location.ltcmat();
        let displacement_ecef = ecef_from_neu(displacement_neu, ltcmat);

        self.position_ecef.x += displacement_ecef.x;
        self.position_ecef.y += displacement_ecef.y;
        self.position_ecef.z += displacement_ecef.z;
    }

    /// Build a snapshot representing the current motion state.
    fn snapshot(&self) -> MotionSnapshot {
        let speed_mps = horizontal_speed_mps(self.velocity_neu);
        MotionSnapshot {
            position_ecef: self.position_ecef,
            velocity_neu: self.velocity_neu,
            heading_deg: normalize_heading_deg(self.heading_deg),
            speed_mps,
        }
    }

    /// Set NEU velocity and update heading to match horizontal motion.
    fn set_velocity_neu(&mut self, velocity_neu: Neu) {
        self.velocity_neu = velocity_neu;
        self.update_heading_from_velocity();
    }

    /// Set heading (deg), horizontal speed (m/s), and climb rate (m/s).
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

    /// Update heading based on the current NEU velocity.
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

    /// Clear all active target controllers.
    fn clear_targets(&mut self) {
        self.target_speed = None;
        self.target_heading = None;
    }
}

/// Compute horizontal speed magnitude from a NEU velocity vector.
fn horizontal_speed_mps(velocity_neu: Neu) -> f64 {
    (velocity_neu.north * velocity_neu.north
        + velocity_neu.east * velocity_neu.east)
        .sqrt()
}

/// Normalize a heading in degrees into the range `[0, 360)`.
fn normalize_heading_deg(heading_deg: f64) -> f64 {
    let deg = heading_deg % 360.0;
    if deg < 0.0 { deg + 360.0 } else { deg }
}

/// Compute the signed smallest-angle delta from current to target heading.
///
/// The returned value is in degrees and lies in `[-180, 180]`.
fn shortest_heading_delta_deg(current_deg: f64, target_deg: f64) -> f64 {
    let current = normalize_heading_deg(current_deg);
    let target = normalize_heading_deg(target_deg);
    let mut delta = target - current;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    delta
}

/// Convert a NEU displacement to an ECEF displacement using a local tangent
/// rotation matrix.
///
/// The provided `ltcmat` matrix is used elsewhere as an ECEF->NEU rotation
/// (rows correspond to North/East/Up). For NEU->ECEF, this applies the
/// transpose.
fn ecef_from_neu(neu: Neu, ltcmat: [[f64; 3]; 3]) -> Ecef {
    Ecef {
        x: ltcmat[0][0] * neu.north
            + ltcmat[1][0] * neu.east
            + ltcmat[2][0] * neu.up,
        y: ltcmat[0][1] * neu.north
            + ltcmat[1][1] * neu.east
            + ltcmat[2][1] * neu.up,
        z: ltcmat[0][2] * neu.north
            + ltcmat[1][2] * neu.east
            + ltcmat[2][2] * neu.up,
    }
}

#[cfg(test)]
mod tests;
