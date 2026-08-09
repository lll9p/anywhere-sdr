use geometry::{Ecef, Neu};

/// Vector displacement and velocity at one interval endpoint.
#[derive(Debug, Clone, Copy)]
pub(super) struct LinearMotionStep {
    /// Interval displacement in the start-local NEU frame.
    pub(super) displacement_neu: Neu,
    /// Velocity at the interval endpoint.
    pub(super) endpoint_velocity_neu: Neu,
}

/// Horizontal distance and speed at one interval endpoint.
#[derive(Debug, Clone, Copy)]
pub(super) struct ScalarMotionStep {
    /// Horizontal distance traveled during the interval.
    pub(super) distance: f64,
    /// Horizontal speed at the interval endpoint.
    pub(super) endpoint_speed: f64,
}

/// Limits a signed delta to the configured magnitude.
pub(super) fn apply_limited_delta(delta: f64, max_delta: f64) -> f64 {
    delta.clamp(-max_delta, max_delta)
}

/// Computes horizontal speed magnitude from a NEU velocity vector.
pub(super) fn horizontal_speed_mps(velocity_neu: Neu) -> f64 {
    (velocity_neu.north * velocity_neu.north
        + velocity_neu.east * velocity_neu.east)
        .sqrt()
}

/// Computes total speed magnitude from a NEU velocity vector.
pub(super) fn total_speed_mps(velocity_neu: Neu) -> f64 {
    velocity_neu
        .north
        .hypot(velocity_neu.east)
        .hypot(velocity_neu.up)
}

/// Integrates constant vector acceleration over one interval.
pub(super) fn integrate_constant_acceleration(
    velocity_neu: Neu, acceleration_neu: Neu, dt: f64,
) -> LinearMotionStep {
    let half_dt_squared = 0.5 * dt * dt;
    LinearMotionStep {
        displacement_neu: Neu {
            north: velocity_neu.north * dt
                + acceleration_neu.north * half_dt_squared,
            east: velocity_neu.east * dt
                + acceleration_neu.east * half_dt_squared,
            up: velocity_neu.up * dt + acceleration_neu.up * half_dt_squared,
        },
        endpoint_velocity_neu: Neu {
            north: velocity_neu.north + acceleration_neu.north * dt,
            east: velocity_neu.east + acceleration_neu.east * dt,
            up: velocity_neu.up + acceleration_neu.up * dt,
        },
    }
}

/// Integrates a horizontal target-speed controller over one interval.
pub(super) fn integrate_target_speed(
    start_speed: f64, target_speed: f64, accel_limit_mps2: f64, dt: f64,
) -> ScalarMotionStep {
    if accel_limit_mps2 == 0.0
        && start_speed.is_finite()
        && start_speed >= 0.0
        && target_speed.is_finite()
        && target_speed >= 0.0
        && dt.is_finite()
        && dt >= 0.0
    {
        let distance = start_speed * dt;
        if distance.is_finite() {
            return ScalarMotionStep {
                distance,
                endpoint_speed: start_speed,
            };
        }
    }

    analytic_target_speed_step(start_speed, target_speed, accel_limit_mps2, dt)
        .unwrap_or_else(|| {
            derived_target_speed_fallback(
                start_speed,
                target_speed,
                accel_limit_mps2,
                dt,
            )
        })
}

/// Returns analytic target-speed motion when its full domain is supported.
fn analytic_target_speed_step(
    start_speed: f64, target_speed: f64, accel_limit_mps2: f64, dt: f64,
) -> Option<ScalarMotionStep> {
    if !start_speed.is_finite()
        || start_speed < 0.0
        || !target_speed.is_finite()
        || target_speed < 0.0
        || !accel_limit_mps2.is_finite()
        || accel_limit_mps2 <= 0.0
        || !dt.is_finite()
        || dt <= 0.0
    {
        return None;
    }

    let delta = target_speed - start_speed;
    let limit_step = accel_limit_mps2.abs() * dt;
    let speed_difference = delta.abs();
    let hit_time = speed_difference / accel_limit_mps2.abs();
    if !delta.is_finite()
        || !limit_step.is_finite()
        || !speed_difference.is_finite()
        || !hit_time.is_finite()
    {
        return None;
    }

    let signed_acceleration = delta.signum() * accel_limit_mps2.abs();
    if !signed_acceleration.is_finite() {
        return None;
    }

    if hit_time >= dt {
        let initial_distance = start_speed * dt;
        let acceleration_delta = signed_acceleration * dt;
        let acceleration_distance = 0.5 * signed_acceleration * dt;
        let acceleration_distance = acceleration_distance * dt;
        let endpoint_speed = start_speed + acceleration_delta;
        let distance = initial_distance + acceleration_distance;
        if !initial_distance.is_finite()
            || !acceleration_delta.is_finite()
            || !acceleration_distance.is_finite()
            || !endpoint_speed.is_finite()
            || !distance.is_finite()
        {
            return None;
        }
        let endpoint_speed = if delta >= 0.0 {
            endpoint_speed.min(target_speed)
        } else {
            endpoint_speed.max(target_speed)
        };
        Some(ScalarMotionStep {
            distance,
            endpoint_speed,
        })
    } else {
        let initial_distance = start_speed * hit_time;
        let acceleration_delta = signed_acceleration * hit_time;
        let hit_speed = start_speed + acceleration_delta;
        let acceleration_distance = 0.5 * signed_acceleration * hit_time;
        let acceleration_distance = acceleration_distance * hit_time;
        let accelerating_distance = initial_distance + acceleration_distance;
        let cruise_time = dt - hit_time;
        let cruise_distance = target_speed * cruise_time;
        let distance = accelerating_distance + cruise_distance;
        if !initial_distance.is_finite()
            || !acceleration_delta.is_finite()
            || !hit_speed.is_finite()
            || !acceleration_distance.is_finite()
            || !accelerating_distance.is_finite()
            || !cruise_time.is_finite()
            || !cruise_distance.is_finite()
            || !distance.is_finite()
        {
            return None;
        }
        Some(ScalarMotionStep {
            distance,
            endpoint_speed: target_speed,
        })
    }
}

/// Contains non-finite arithmetic derived from accepted finite state.
fn derived_target_speed_fallback(
    start_speed: f64, target_speed: f64, accel_limit_mps2: f64, dt: f64,
) -> ScalarMotionStep {
    let delta = target_speed - start_speed;
    let max_delta = (accel_limit_mps2.abs() * dt).max(0.0);
    let applied = apply_limited_delta(delta, max_delta);
    let endpoint_speed = (start_speed + applied).max(0.0);
    ScalarMotionStep {
        distance: endpoint_speed * dt,
        endpoint_speed,
    }
}

/// Normalizes a heading into the range `[0, 360)` degrees.
pub(super) fn normalize_heading_deg(heading_deg: f64) -> f64 {
    let degrees = heading_deg % 360.0;
    if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    }
}

/// Computes the signed smallest-angle heading delta in degrees.
pub(super) fn shortest_heading_delta_deg(
    current_degrees: f64, target_degrees: f64,
) -> f64 {
    let current = normalize_heading_deg(current_degrees);
    let target = normalize_heading_deg(target_degrees);
    let mut delta = target - current;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    delta
}

/// Converts a NEU displacement into ECEF with the tangent matrix transpose.
pub(super) fn ecef_from_neu(neu: Neu, ltcmat: [[f64; 3]; 3]) -> Ecef {
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
mod tests {
    use super::*;

    const EPS: f64 = 1.0e-12;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPS,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_neu(actual: Neu, expected: Neu) {
        assert_close(actual.north, expected.north);
        assert_close(actual.east, expected.east);
        assert_close(actual.up, expected.up);
    }

    #[test]
    fn constant_acceleration_integrates_three_axes() {
        let step = integrate_constant_acceleration(
            Neu {
                north: 1.0,
                east: -2.0,
                up: 3.0,
            },
            Neu {
                north: 4.0,
                east: 5.0,
                up: -6.0,
            },
            0.5,
        );
        assert_neu(step.displacement_neu, Neu {
            north: 1.0,
            east: -0.375,
            up: 0.75,
        });
        assert_neu(step.endpoint_velocity_neu, Neu {
            north: 3.0,
            east: 0.5,
            up: 0.0,
        });

        let constant_velocity = integrate_constant_acceleration(
            Neu {
                north: 1.0,
                east: -2.0,
                up: 3.0,
            },
            Neu::default(),
            0.5,
        );
        assert_neu(constant_velocity.displacement_neu, Neu {
            north: 0.5,
            east: -1.0,
            up: 1.5,
        });
        assert_neu(constant_velocity.endpoint_velocity_neu, Neu {
            north: 1.0,
            east: -2.0,
            up: 3.0,
        });

        let stationary = integrate_constant_acceleration(
            Neu::default(),
            Neu::default(),
            1.0,
        );
        assert_neu(stationary.displacement_neu, Neu::default());
        assert_neu(stationary.endpoint_velocity_neu, Neu::default());
    }

    #[test]
    fn constant_acceleration_composes_across_fixed_frame_partitions() {
        for count in [1, 10, 20, 100] {
            let dt = 1.0 / f64::from(count);
            let mut velocity = Neu::default();
            let mut displacement = Neu::default();
            for _ in 0..count {
                let step = integrate_constant_acceleration(
                    velocity,
                    Neu {
                        north: 1.0,
                        east: 0.0,
                        up: 0.0,
                    },
                    dt,
                );
                displacement.north += step.displacement_neu.north;
                velocity = step.endpoint_velocity_neu;
            }
            assert_close(displacement.north, 0.5);
            assert_close(velocity.north, 1.0);
        }
    }

    #[test]
    fn target_speed_splits_at_hit_time() {
        let cases = [
            (0.0, 1.0, 2.0, 0.25, 0.0625, 0.5),
            (0.0, 1.0, 2.0, 0.5, 0.25, 1.0),
            (0.0, 1.0, 2.0, 1.0, 0.75, 1.0),
            (2.0, 1.0, 2.0, 1.0, 1.25, 1.0),
            (1.0, 0.0, 2.0, 1.0, 0.25, 0.0),
        ];
        for (start, target, limit, dt, distance, endpoint) in cases {
            let step = integrate_target_speed(start, target, limit, dt);
            assert_close(step.distance, distance);
            assert_close(step.endpoint_speed, endpoint);
        }
    }

    #[test]
    fn target_speed_composes_and_is_continuous_at_hit() {
        let mut speed = 0.0;
        let mut distance = 0.0;
        for dt in [0.25, 0.25, 0.5] {
            let step = integrate_target_speed(speed, 1.0, 2.0, dt);
            distance += step.distance;
            speed = step.endpoint_speed;
        }
        assert_close(distance, 0.75);
        assert_close(speed, 1.0);

        let before = integrate_target_speed(0.0, 1.0, 2.0, 0.5 - 1.0e-9);
        let exact = integrate_target_speed(0.0, 1.0, 2.0, 0.5);
        let after = integrate_target_speed(0.0, 1.0, 2.0, 0.5 + 1.0e-9);
        assert!(before.endpoint_speed < 1.0);
        assert_close(exact.endpoint_speed, 1.0);
        assert_close(after.endpoint_speed, 1.0);
        assert!(before.distance < exact.distance);
        assert!(exact.distance < after.distance);
        assert_close(exact.distance - before.distance, 1.0e-9);
        assert_close(after.distance - exact.distance, 1.0e-9);
    }

    #[test]
    fn zero_target_speed_limit_freezes_motion() {
        let step = integrate_target_speed(3.0, 10.0, 0.0, 2.0);
        assert_close(step.distance, 6.0);
        assert_close(step.endpoint_speed, 3.0);
    }

    #[test]
    fn derived_target_speed_overflow_uses_containment_fallback() {
        assert!(analytic_target_speed_step(1.0, 2.0, f64::MAX, 2.0).is_none());
        let overflow = integrate_target_speed(1.0, 2.0, f64::MAX, 2.0);
        assert_close(overflow.distance, 4.0);
        assert_close(overflow.endpoint_speed, 2.0);

        let non_finite_start = integrate_target_speed(f64::NAN, 2.0, 2.0, 1.0);
        assert_close(non_finite_start.distance, 0.0);
        assert_close(non_finite_start.endpoint_speed, 0.0);
    }

    #[test]
    fn total_speed_uses_three_dimensional_overflow_resistant_norm() {
        assert_close(
            total_speed_mps(Neu {
                north: 3.0,
                east: 4.0,
                up: 0.0,
            }),
            5.0,
        );
        assert_close(
            total_speed_mps(Neu {
                north: 0.0,
                east: 0.0,
                up: 12.0,
            }),
            12.0,
        );
        assert_close(
            total_speed_mps(Neu {
                north: 3.0,
                east: 4.0,
                up: 12.0,
            }),
            13.0,
        );
        let large = total_speed_mps(Neu {
            north: f64::MAX / 4.0,
            east: f64::MAX / 4.0,
            up: f64::MAX / 4.0,
        });
        assert!(large.is_finite());
    }
}
