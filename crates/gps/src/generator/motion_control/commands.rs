//! Runtime motion command validation and bounded pending storage.

use geometry::{Ecef, Neu};

use crate::Error;

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
    /// If `speed_mps` is None, the integrator resumes its remembered nonzero
    /// horizontal speed or defaults to 1 m/s.
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

/// Storage identity for each public command variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum MotionCommandKind {
    /// Absolute position command.
    Position,
    /// Direct velocity command.
    Velocity,
    /// Direct acceleration command.
    Acceleration,
    /// Direct heading and speed command.
    HeadingSpeed,
    /// Stop command.
    Stop,
    /// Start command.
    Start,
    /// Target speed command.
    TargetSpeed,
    /// Target heading command.
    TargetHeading,
    /// Number of command kinds; never used as a storage identity.
    Count,
}

impl MotionCommandKind {
    /// Fixed pending capacity derived from the exhaustive variant set.
    const COUNT: usize = Self::Count as usize;
}

impl MotionCommand {
    /// Returns the command's fixed-storage identity.
    fn kind(self) -> MotionCommandKind {
        match self {
            Self::SetPositionEcef(_) => MotionCommandKind::Position,
            Self::SetVelocityNeu(_) => MotionCommandKind::Velocity,
            Self::SetAccelerationNeu(_) => MotionCommandKind::Acceleration,
            Self::SetHeadingSpeed { .. } => MotionCommandKind::HeadingSpeed,
            Self::Stop => MotionCommandKind::Stop,
            Self::Start { .. } => MotionCommandKind::Start,
            Self::SetTargetSpeed { .. } => MotionCommandKind::TargetSpeed,
            Self::SetTargetHeading { .. } => MotionCommandKind::TargetHeading,
        }
    }

    /// Validates every numeric field before pending state is locked.
    pub(super) fn validate(self) -> Result<(), Error> {
        match self {
            Self::SetPositionEcef(position) => {
                validate_finite("SetPositionEcef", "x", position.x)?;
                validate_finite("SetPositionEcef", "y", position.y)?;
                validate_finite("SetPositionEcef", "z", position.z)
            }
            Self::SetVelocityNeu(velocity) => {
                validate_finite("SetVelocityNeu", "north", velocity.north)?;
                validate_finite("SetVelocityNeu", "east", velocity.east)?;
                validate_finite("SetVelocityNeu", "up", velocity.up)
            }
            Self::SetAccelerationNeu(acceleration) => {
                validate_finite(
                    "SetAccelerationNeu",
                    "north",
                    acceleration.north,
                )?;
                validate_finite(
                    "SetAccelerationNeu",
                    "east",
                    acceleration.east,
                )?;
                validate_finite("SetAccelerationNeu", "up", acceleration.up)
            }
            Self::SetHeadingSpeed {
                heading_deg,
                speed_mps,
                climb_mps,
            } => {
                validate_finite("SetHeadingSpeed", "heading_deg", heading_deg)?;
                validate_nonnegative(
                    "SetHeadingSpeed",
                    "speed_mps",
                    speed_mps,
                )?;
                validate_finite("SetHeadingSpeed", "climb_mps", climb_mps)
            }
            Self::Stop => Ok(()),
            Self::Start { speed_mps } => {
                if let Some(speed_mps) = speed_mps {
                    validate_nonnegative("Start", "speed_mps", speed_mps)?;
                }
                Ok(())
            }
            Self::SetTargetSpeed {
                speed_mps,
                accel_limit_mps2,
            } => {
                validate_nonnegative("SetTargetSpeed", "speed_mps", speed_mps)?;
                validate_nonnegative(
                    "SetTargetSpeed",
                    "accel_limit_mps2",
                    accel_limit_mps2,
                )
            }
            Self::SetTargetHeading {
                heading_deg,
                turn_rate_limit_dps,
            } => {
                validate_finite(
                    "SetTargetHeading",
                    "heading_deg",
                    heading_deg,
                )?;
                validate_nonnegative(
                    "SetTargetHeading",
                    "turn_rate_limit_dps",
                    turn_rate_limit_dps,
                )
            }
        }
    }
}

/// Rejects a non-finite command field with stable labels.
fn validate_finite(
    command: &'static str, field: &'static str, value: f64,
) -> Result<(), Error> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(Error::NonFiniteMotionCommandValue {
            command,
            field,
            value,
        })
    }
}

/// Rejects a negative finite scalar after finite-domain validation.
fn validate_nonnegative(
    command: &'static str, field: &'static str, value: f64,
) -> Result<(), Error> {
    validate_finite(command, field, value)?;
    if value < 0.0 {
        Err(Error::NegativeMotionCommandValue {
            command,
            field,
            value,
        })
    } else {
        Ok(())
    }
}

/// Command and exact labels used by numeric validation tests.
#[cfg(test)]
pub(super) type ValidationCase = (MotionCommand, &'static str, &'static str);

/// Builds one case for every numeric command field.
#[cfg(test)]
#[allow(clippy::too_many_lines)]
pub(super) fn numeric_validation_cases(value: f64) -> [ValidationCase; 17] {
    [
        (
            MotionCommand::SetPositionEcef(Ecef::new(value, 0.0, 0.0)),
            "SetPositionEcef",
            "x",
        ),
        (
            MotionCommand::SetPositionEcef(Ecef::new(0.0, value, 0.0)),
            "SetPositionEcef",
            "y",
        ),
        (
            MotionCommand::SetPositionEcef(Ecef::new(0.0, 0.0, value)),
            "SetPositionEcef",
            "z",
        ),
        (
            MotionCommand::SetVelocityNeu(Neu {
                north: value,
                ..Neu::default()
            }),
            "SetVelocityNeu",
            "north",
        ),
        (
            MotionCommand::SetVelocityNeu(Neu {
                east: value,
                ..Neu::default()
            }),
            "SetVelocityNeu",
            "east",
        ),
        (
            MotionCommand::SetVelocityNeu(Neu {
                up: value,
                ..Neu::default()
            }),
            "SetVelocityNeu",
            "up",
        ),
        (
            MotionCommand::SetAccelerationNeu(Neu {
                north: value,
                ..Neu::default()
            }),
            "SetAccelerationNeu",
            "north",
        ),
        (
            MotionCommand::SetAccelerationNeu(Neu {
                east: value,
                ..Neu::default()
            }),
            "SetAccelerationNeu",
            "east",
        ),
        (
            MotionCommand::SetAccelerationNeu(Neu {
                up: value,
                ..Neu::default()
            }),
            "SetAccelerationNeu",
            "up",
        ),
        (
            MotionCommand::SetHeadingSpeed {
                heading_deg: value,
                speed_mps: 0.0,
                climb_mps: 0.0,
            },
            "SetHeadingSpeed",
            "heading_deg",
        ),
        (
            MotionCommand::SetHeadingSpeed {
                heading_deg: 0.0,
                speed_mps: value,
                climb_mps: 0.0,
            },
            "SetHeadingSpeed",
            "speed_mps",
        ),
        (
            MotionCommand::SetHeadingSpeed {
                heading_deg: 0.0,
                speed_mps: 0.0,
                climb_mps: value,
            },
            "SetHeadingSpeed",
            "climb_mps",
        ),
        (
            MotionCommand::Start {
                speed_mps: Some(value),
            },
            "Start",
            "speed_mps",
        ),
        (
            MotionCommand::SetTargetSpeed {
                speed_mps: value,
                accel_limit_mps2: 0.0,
            },
            "SetTargetSpeed",
            "speed_mps",
        ),
        (
            MotionCommand::SetTargetSpeed {
                speed_mps: 0.0,
                accel_limit_mps2: value,
            },
            "SetTargetSpeed",
            "accel_limit_mps2",
        ),
        (
            MotionCommand::SetTargetHeading {
                heading_deg: value,
                turn_rate_limit_dps: 0.0,
            },
            "SetTargetHeading",
            "heading_deg",
        ),
        (
            MotionCommand::SetTargetHeading {
                heading_deg: 0.0,
                turn_rate_limit_dps: value,
            },
            "SetTargetHeading",
            "turn_rate_limit_dps",
        ),
    ]
}

/// Builds cases for every nonnegative scalar command field.
#[cfg(test)]
pub(super) fn scalar_validation_cases(value: f64) -> [ValidationCase; 5] {
    [
        (
            MotionCommand::SetHeadingSpeed {
                heading_deg: 0.0,
                speed_mps: value,
                climb_mps: 0.0,
            },
            "SetHeadingSpeed",
            "speed_mps",
        ),
        (
            MotionCommand::Start {
                speed_mps: Some(value),
            },
            "Start",
            "speed_mps",
        ),
        (
            MotionCommand::SetTargetSpeed {
                speed_mps: value,
                accel_limit_mps2: 0.0,
            },
            "SetTargetSpeed",
            "speed_mps",
        ),
        (
            MotionCommand::SetTargetSpeed {
                speed_mps: 0.0,
                accel_limit_mps2: value,
            },
            "SetTargetSpeed",
            "accel_limit_mps2",
        ),
        (
            MotionCommand::SetTargetHeading {
                heading_deg: 0.0,
                turn_rate_limit_dps: value,
            },
            "SetTargetHeading",
            "turn_rate_limit_dps",
        ),
    ]
}

/// Fixed-capacity pending commands in oldest-to-newest replay order.
#[derive(Debug, Clone, Copy)]
pub(super) struct PendingMotionCommands {
    /// Active commands occupy the prefix and remain unique by variant.
    commands: [Option<MotionCommand>; MotionCommandKind::COUNT],
}

impl Default for PendingMotionCommands {
    fn default() -> Self {
        Self {
            commands: [None; MotionCommandKind::COUNT],
        }
    }
}

impl PendingMotionCommands {
    /// Replaces the same variant and appends it after retained variants.
    pub(super) fn push(&mut self, command: MotionCommand) {
        let command_kind = command.kind();
        let mut ordered = self
            .commands
            .into_iter()
            .flatten()
            .filter(|existing| existing.kind() != command_kind)
            .chain(std::iter::once(command));
        self.commands = std::array::from_fn(|_| ordered.next());
    }

    /// Iterates retained commands in replay order without consuming storage.
    pub(super) fn iter(&self) -> impl Iterator<Item = MotionCommand> + '_ {
        self.commands.iter().flatten().copied()
    }
}

impl IntoIterator for PendingMotionCommands {
    type IntoIter = std::iter::Flatten<
        std::array::IntoIter<
            Option<MotionCommand>,
            { MotionCommandKind::COUNT },
        >,
    >;
    type Item = MotionCommand;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.into_iter().flatten()
    }
}
