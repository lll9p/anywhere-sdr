use super::*;

const EPS: f64 = 1e-7;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPS,
        "expected {expected}, got {actual}"
    );
}

fn origin() -> Result<Ecef, Error> {
    let location = Location::try_from_radians(
        0.615_647_719_411_178_2,
        2.391_360_502_574_699,
        100.0,
    )?;
    Ok(Ecef::from(&location))
}

fn local_displacement(start: Ecef, end: Ecef) -> Result<Neu, Error> {
    let delta_ecef = end - &start;
    let ltcmat = Location::try_from(&start)?.ltcmat();
    Ok(Neu::from_ecef(&delta_ecef, ltcmat))
}

#[test]
fn neu_to_ecef_round_trip_matches_neu() -> Result<(), Error> {
    let origin = origin()?;
    let ltcmat = Location::try_from(&origin)?.ltcmat();

    let neu = Neu {
        north: 10.0,
        east: -5.0,
        up: 2.0,
    };
    let ecef = ecef_from_neu(neu, ltcmat);
    let neu_back = Neu::from_ecef(&ecef, ltcmat);
    assert_close(neu_back.north, neu.north);
    assert_close(neu_back.east, neu.east);
    assert_close(neu_back.up, neu.up);
    Ok(())
}

#[test]
fn heading_0_deg_moves_north() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let delta_ecef = next - &origin;
    let ltcmat = Location::try_from(&origin)?.ltcmat();
    let delta_neu = Neu::from_ecef(&delta_ecef, ltcmat);
    assert_close(delta_neu.north, 5.0);
    assert_close(delta_neu.east, 0.0);
    assert_close(delta_neu.up, 0.0);
    Ok(())
}

#[test]
fn heading_90_deg_moves_east() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let delta_ecef = next - &origin;
    let ltcmat = Location::try_from(&origin)?.ltcmat();
    let delta_neu = Neu::from_ecef(&delta_ecef, ltcmat);
    assert_close(delta_neu.north, 0.0);
    assert_close(delta_neu.east, 5.0);
    assert_close(delta_neu.up, 0.0);
    Ok(())
}

#[test]
fn direct_acceleration_uses_interval_start_velocity() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetAccelerationNeu(Neu {
        north: 1.0,
        east: 0.0,
        up: 0.0,
    }))?;

    let next = integrator.step(1.0, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 0.5);
    assert_close(displacement.east, 0.0);
    assert_close(displacement.up, 0.0);
    let snapshot = control.snapshot();
    assert_close(snapshot.velocity_neu.north, 1.0);
    assert_close(snapshot.speed_mps, 1.0);
    assert_close(snapshot.heading_deg, 0.0);
    Ok(())
}

#[test]
fn combined_acceleration_updates_displacement_velocity_and_heading()
-> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetVelocityNeu(Neu {
        north: 1.0,
        east: 2.0,
        up: 3.0,
    }))?;
    control.submit(MotionCommand::SetAccelerationNeu(Neu {
        north: 4.0,
        east: -2.0,
        up: 1.0,
    }))?;

    let next = integrator.step(0.5, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 1.0);
    assert_close(displacement.east, 0.75);
    assert_close(displacement.up, 1.625);
    let snapshot = control.snapshot();
    assert_close(snapshot.velocity_neu.north, 3.0);
    assert_close(snapshot.velocity_neu.east, 1.0);
    assert_close(snapshot.velocity_neu.up, 3.5);
    assert_close(snapshot.heading_deg, 1.0_f64.atan2(3.0).to_degrees());
    assert_close(snapshot.speed_mps, 22.25_f64.sqrt());
    Ok(())
}

#[test]
fn target_speed_splits_acceleration_and_cruise() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 1.0,
        accel_limit_mps2: 2.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 0.75);
    assert_close(displacement.east, 0.0);
    let snapshot = control.snapshot();
    assert_close(snapshot.velocity_neu.north, 1.0);
    assert_close(snapshot.speed_mps, 1.0);
    Ok(())
}

#[test]
fn target_speed_preserves_vertical_motion() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 0.0,
        climb_mps: 2.0,
    })?;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 1.0,
        accel_limit_mps2: 2.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 0.75);
    assert_close(displacement.up, 2.0);
    let snapshot = control.snapshot();
    assert_close(snapshot.velocity_neu.north, 1.0);
    assert_close(snapshot.velocity_neu.up, 2.0);
    assert_close(snapshot.speed_mps, 5.0_f64.sqrt());
    Ok(())
}

#[test]
fn target_heading_selects_endpoint_direction_before_speed_integration()
-> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 0.0,
        climb_mps: 0.0,
    })?;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 1.0,
        accel_limit_mps2: 2.0,
    })?;
    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 90.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 0.0);
    assert_close(displacement.east, 0.75);
    assert_close(control.snapshot().heading_deg, 90.0);
    Ok(())
}

#[test]
fn heading_only_motion_uses_endpoint_discrete_direction() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    })?;
    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 90.0,
    })?;

    let next = integrator.step(1.0, &control)?;
    let displacement = local_displacement(origin, next)?;
    assert_close(displacement.north, 0.0);
    assert_close(displacement.east, 5.0);
    assert_close(control.snapshot().heading_deg, 90.0);
    Ok(())
}

#[test]
fn snapshots_report_total_speed() -> Result<(), Error> {
    let cases = [
        (
            Neu {
                north: 3.0,
                east: 4.0,
                up: 0.0,
            },
            5.0,
        ),
        (
            Neu {
                north: 0.0,
                east: 0.0,
                up: 12.0,
            },
            12.0,
        ),
        (
            Neu {
                north: 3.0,
                east: 4.0,
                up: 12.0,
            },
            13.0,
        ),
    ];
    for (velocity, expected_speed) in cases {
        let origin = origin()?;
        let control = RuntimeMotionControl::new(origin);
        let mut integrator = MotionIntegrator::new(origin);
        control.submit(MotionCommand::SetVelocityNeu(velocity))?;
        integrator.step(0.01, &control)?;
        assert_close(control.snapshot().speed_mps, expected_speed);
    }
    Ok(())
}

#[test]
fn geometry_failure_keeps_committed_controller_state_without_displacement()
-> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetPositionEcef(Ecef::default()))?;
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 0.0,
        climb_mps: 1.0,
    })?;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 2.0,
        accel_limit_mps2: 1.0,
    })?;
    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 90.0,
    })?;

    let Err(error) = integrator.step(1.0, &control) else {
        return Err(Error::msg("invalid ECEF unexpectedly succeeded"));
    };
    assert!(matches!(error, Error::Geometry(_)));
    assert_close(integrator.position_ecef.x, 0.0);
    assert_close(integrator.position_ecef.y, 0.0);
    assert_close(integrator.position_ecef.z, 0.0);
    assert_close(integrator.velocity_neu.north, 0.0);
    assert_close(integrator.velocity_neu.east, 1.0);
    assert_close(integrator.velocity_neu.up, 1.0);
    assert_close(integrator.heading_deg, 90.0);
    assert!(integrator.target_speed.is_some());
    assert!(integrator.target_heading.is_some());

    let pending = control.try_take_pending();
    assert!(pending.into_iter().next().is_none());
    let snapshot = control.snapshot();
    assert_close(snapshot.position_ecef.x, origin.x);
    assert_close(snapshot.position_ecef.y, origin.y);
    assert_close(snapshot.position_ecef.z, origin.z);
    assert_close(snapshot.speed_mps, 0.0);
    Ok(())
}

#[test]
fn stop_then_start_resumes_previous_speed() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    })?;
    integrator.step(1.0, &control)?;
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 5.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 5.0);

    control.submit(MotionCommand::Stop)?;
    integrator.step(1.0, &control)?;
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 0.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 0.0);
    assert_close(snapshot.velocity_neu.up, 0.0);

    control.submit(MotionCommand::Start { speed_mps: None })?;
    integrator.step(1.0, &control)?;
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 5.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 5.0);
    Ok(())
}

#[test]
fn start_without_prior_motion_defaults_to_1_mps() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    control.submit(MotionCommand::Start { speed_mps: None })?;
    integrator.step(1.0, &control)?;
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 1.0);
    Ok(())
}

#[test]
fn target_speed_obeys_accel_limit_per_step() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    let dt = 0.1;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 10.0,
        accel_limit_mps2: 2.0,
    })?;

    let mut last = control.snapshot().speed_mps;
    for step in 1..=5 {
        integrator.step(dt, &control)?;
        let current = control.snapshot().speed_mps;
        let max_delta = 2.0 * dt + 1e-9;
        assert!(
            (current - last) <= max_delta,
            "step {step}: delta {} exceeded {}",
            current - last,
            max_delta
        );
        last = current;
    }

    assert_close(control.snapshot().speed_mps, 1.0);
    Ok(())
}

#[test]
fn target_heading_obeys_turn_rate_limit_per_step() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    let dt = 0.1;
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    })?;
    integrator.step(dt, &control)?;

    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 30.0,
    })?;

    let mut last = control.snapshot().heading_deg;
    for step in 1..=5 {
        integrator.step(dt, &control)?;
        let current = control.snapshot().heading_deg;
        let max_delta = 30.0 * dt + 1e-9;
        assert!(
            (current - last) <= max_delta,
            "step {step}: delta {} exceeded {}",
            current - last,
            max_delta
        );
        last = current;
    }

    assert_close(control.snapshot().heading_deg, 15.0);
    Ok(())
}
