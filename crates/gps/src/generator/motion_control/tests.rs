use super::*;

const EPS: f64 = 1e-7;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPS,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn neu_to_ecef_round_trip_matches_neu() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let ltcmat = Location::from(&origin).ltcmat();

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
}

#[test]
fn heading_0_deg_moves_north() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    });

    let next = integrator.step(1.0, &control);
    let delta_ecef = next - &origin;
    let ltcmat = Location::from(&origin).ltcmat();
    let delta_neu = Neu::from_ecef(&delta_ecef, ltcmat);
    assert_close(delta_neu.north, 5.0);
    assert_close(delta_neu.east, 0.0);
    assert_close(delta_neu.up, 0.0);
}

#[test]
fn heading_90_deg_moves_east() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    });

    let next = integrator.step(1.0, &control);
    let delta_ecef = next - &origin;
    let ltcmat = Location::from(&origin).ltcmat();
    let delta_neu = Neu::from_ecef(&delta_ecef, ltcmat);
    assert_close(delta_neu.north, 0.0);
    assert_close(delta_neu.east, 5.0);
    assert_close(delta_neu.up, 0.0);
}

#[test]
fn stop_then_start_resumes_previous_speed() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    });
    let _ = integrator.step(1.0, &control);
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 5.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 5.0);

    control.submit(MotionCommand::Stop);
    let _ = integrator.step(1.0, &control);
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 0.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 0.0);
    assert_close(snapshot.velocity_neu.up, 0.0);

    control.submit(MotionCommand::Start { speed_mps: None });
    let _ = integrator.step(1.0, &control);
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 5.0);
    assert_close(snapshot.velocity_neu.north, 0.0);
    assert_close(snapshot.velocity_neu.east, 5.0);
}

#[test]
fn start_without_prior_motion_defaults_to_1_mps() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    control.submit(MotionCommand::Start { speed_mps: None });
    let _ = integrator.step(1.0, &control);
    let snapshot = control.snapshot();
    assert_close(snapshot.speed_mps, 1.0);
}

#[test]
fn target_speed_obeys_accel_limit_per_step() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    let dt = 0.1;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 10.0,
        accel_limit_mps2: 2.0,
    });

    let mut last = control.snapshot().speed_mps;
    for step in 1..=5 {
        let _ = integrator.step(dt, &control);
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
}

#[test]
fn target_heading_obeys_turn_rate_limit_per_step() {
    let origin = Ecef::from(&Location {
        latitude: 0.615_647_719_411_178_2,
        longitude: 2.391_360_502_574_699,
        height: 100.0,
    });
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);

    let dt = 0.1;
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 5.0,
        climb_mps: 0.0,
    });
    let _ = integrator.step(dt, &control);

    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 30.0,
    });

    let mut last = control.snapshot().heading_deg;
    for step in 1..=5 {
        let _ = integrator.step(dt, &control);
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
}
