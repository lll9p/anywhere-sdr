use super::*;

const EPS: f64 = 1.0e-7;

fn command_name(command: MotionCommand) -> &'static str {
    match command {
        MotionCommand::SetPositionEcef(_) => "Position",
        MotionCommand::SetVelocityNeu(_) => "Velocity",
        MotionCommand::SetAccelerationNeu(_) => "Acceleration",
        MotionCommand::SetHeadingSpeed { .. } => "HeadingSpeed",
        MotionCommand::Stop => "Stop",
        MotionCommand::Start { .. } => "Start",
        MotionCommand::SetTargetSpeed { .. } => "TargetSpeed",
        MotionCommand::SetTargetHeading { .. } => "TargetHeading",
    }
}

fn origin() -> Result<Ecef, Error> {
    let location = Location::try_from_radians(
        0.615_647_719_411_178_2,
        2.391_360_502_574_699,
        100.0,
    )?;
    Ok(Ecef::from(&location))
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPS,
        "expected {expected}, got {actual}"
    );
}

fn moving_integrator(position: Ecef, speed_mps: f64) -> MotionIntegrator {
    let mut integrator = MotionIntegrator::new(position);
    integrator.apply_command(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps,
        climb_mps: 0.0,
    });
    integrator
}

fn apply(
    integrator: &mut MotionIntegrator, commands: &[MotionCommand],
) -> Result<(), Error> {
    let control = RuntimeMotionControl::new(integrator.position_ecef);
    for command in commands {
        control.submit(*command)?;
    }
    let pending = control.try_take_pending();
    integrator.apply_pending(&pending);
    Ok(())
}

#[test]
fn pending_storage_is_bounded_ordered_and_overwrite_to_tail() {
    let mut pending = PendingMotionCommands::default();
    for command in [
        MotionCommand::SetPositionEcef(Ecef::default()),
        MotionCommand::SetVelocityNeu(Neu::default()),
        MotionCommand::SetAccelerationNeu(Neu::default()),
        MotionCommand::SetHeadingSpeed {
            heading_deg: 0.0,
            speed_mps: 0.0,
            climb_mps: 0.0,
        },
        MotionCommand::Stop,
        MotionCommand::Start { speed_mps: None },
        MotionCommand::SetTargetSpeed {
            speed_mps: 0.0,
            accel_limit_mps2: 0.0,
        },
        MotionCommand::SetTargetHeading {
            heading_deg: 0.0,
            turn_rate_limit_dps: 0.0,
        },
    ] {
        pending.push(command);
    }
    pending.push(MotionCommand::SetVelocityNeu(Neu {
        north: 7.0,
        east: 8.0,
        up: 9.0,
    }));

    let commands: Vec<_> = pending.into_iter().collect();
    assert_eq!(commands.len(), 8);
    assert_eq!(
        commands
            .iter()
            .map(|command| command_name(*command))
            .collect::<Vec<_>>(),
        vec![
            "Position",
            "Acceleration",
            "HeadingSpeed",
            "Stop",
            "Start",
            "TargetSpeed",
            "TargetHeading",
            "Velocity",
        ]
    );
    assert!(matches!(
        commands.last(),
        Some(MotionCommand::SetVelocityNeu(Neu {
            north: 7.0,
            east: 8.0,
            up: 9.0
        }))
    ));
}

#[test]
fn invalid_submit_preserves_pending_values_and_order() -> Result<(), Error> {
    let control = RuntimeMotionControl::new(Ecef::default());
    control.submit(MotionCommand::SetPositionEcef(Ecef::new(1.0, 2.0, 3.0)))?;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 4.0,
        accel_limit_mps2: 5.0,
    })?;
    control.submit(MotionCommand::Start {
        speed_mps: Some(6.0),
    })?;
    assert!(matches!(
        control.submit(MotionCommand::SetTargetSpeed {
            speed_mps: 9.0,
            accel_limit_mps2: f64::NAN,
        }),
        Err(Error::NonFiniteMotionCommandValue {
            command: "SetTargetSpeed",
            field: "accel_limit_mps2",
            value,
        }) if value.is_nan()
    ));

    let commands: Vec<_> = control.try_take_pending().into_iter().collect();
    assert!(matches!(commands.as_slice(), [
        MotionCommand::SetPositionEcef(Ecef {
            x: 1.0,
            y: 2.0,
            z: 3.0
        }),
        MotionCommand::SetTargetSpeed {
            speed_mps: 4.0,
            accel_limit_mps2: 5.0
        },
        MotionCommand::Start {
            speed_mps: Some(6.0)
        },
    ]));
    Ok(())
}

#[test]
fn every_numeric_field_rejects_non_finite_values_with_exact_labels() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (command, expected_command, expected_field) in
            commands::numeric_validation_cases(invalid)
        {
            assert!(matches!(
                command.validate(),
                Err(Error::NonFiniteMotionCommandValue {
                    command,
                    field,
                    value,
                }) if command == expected_command
                    && field == expected_field
                    && value.to_bits() == invalid.to_bits()
            ));
        }
    }
}

#[test]
fn scalar_domains_reject_negative_finite_and_accept_zero() -> Result<(), Error>
{
    for (command, expected_command, expected_field) in
        commands::scalar_validation_cases(-1.0)
    {
        assert!(matches!(
            command.validate(),
            Err(Error::NegativeMotionCommandValue {
                command,
                field,
                value: -1.0,
            }) if command == expected_command && field == expected_field
        ));
    }
    for value in [0.0, -0.0] {
        for (command, ..) in commands::scalar_validation_cases(value) {
            command.validate()?;
        }
    }
    Ok(())
}

#[test]
fn finite_signed_and_large_values_are_accepted_and_headings_normalize()
-> Result<(), Error> {
    for (command, ..) in commands::numeric_validation_cases(f64::MAX) {
        command.validate()?;
    }
    MotionCommand::SetPositionEcef(Ecef::default()).validate()?;
    MotionCommand::SetVelocityNeu(Neu {
        north: -f64::MAX,
        east: -2.0,
        up: -3.0,
    })
    .validate()?;
    MotionCommand::SetAccelerationNeu(Neu {
        north: -1.0,
        east: -2.0,
        up: -3.0,
    })
    .validate()?;
    MotionCommand::SetHeadingSpeed {
        heading_deg: -450.0,
        speed_mps: -0.0,
        climb_mps: -f64::MAX,
    }
    .validate()?;

    let origin = origin()?;
    let mut integrator = MotionIntegrator::new(origin);
    integrator.apply_command(MotionCommand::SetHeadingSpeed {
        heading_deg: 450.0,
        speed_mps: 1.0,
        climb_mps: 0.0,
    });
    assert_close(integrator.heading_deg, 90.0);
    integrator.apply_command(MotionCommand::SetTargetHeading {
        heading_deg: -90.0,
        turn_rate_limit_dps: 1.0,
    });
    let Some(target_heading) = integrator.target_heading else {
        return Err(Error::msg("target heading was not installed"));
    };
    assert_close(target_heading.heading_deg, 270.0);
    Ok(())
}

#[test]
fn start_stop_order_and_same_variant_overwrite_control_resume()
-> Result<(), Error> {
    let origin = origin()?;
    let mut integrator = moving_integrator(origin, 5.0);
    apply(&mut integrator, &[
        MotionCommand::Start {
            speed_mps: Some(2.0),
        },
        MotionCommand::Stop,
    ])?;
    assert_close(horizontal_speed_mps(integrator.velocity_neu), 0.0);
    assert_close(integrator.resume_speed_mps, 2.0);

    let mut integrator = moving_integrator(origin, 5.0);
    apply(&mut integrator, &[
        MotionCommand::Stop,
        MotionCommand::Start {
            speed_mps: Some(2.0),
        },
    ])?;
    assert_close(horizontal_speed_mps(integrator.velocity_neu), 2.0);
    assert_close(integrator.resume_speed_mps, 5.0);

    let mut integrator = moving_integrator(origin, 5.0);
    apply(&mut integrator, &[
        MotionCommand::Start {
            speed_mps: Some(2.0),
        },
        MotionCommand::Stop,
        MotionCommand::Start {
            speed_mps: Some(3.0),
        },
    ])?;
    assert_close(horizontal_speed_mps(integrator.velocity_neu), 3.0);
    assert_close(integrator.resume_speed_mps, 5.0);
    Ok(())
}

#[test]
fn direct_commands_replay_in_order_and_preserve_target_side_effects()
-> Result<(), Error> {
    let origin = origin()?;
    let velocity = MotionCommand::SetVelocityNeu(Neu {
        north: 3.0,
        east: 4.0,
        up: 5.0,
    });
    let heading = MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 2.0,
        climb_mps: 1.0,
    };
    let acceleration = Neu {
        north: 1.0,
        east: 2.0,
        up: 3.0,
    };

    let mut integrator = MotionIntegrator::new(origin);
    apply(&mut integrator, &[velocity, heading])?;
    assert_close(integrator.velocity_neu.east, 2.0);
    let mut integrator = MotionIntegrator::new(origin);
    apply(&mut integrator, &[heading, velocity])?;
    assert_close(integrator.velocity_neu.north, 3.0);
    assert_close(integrator.heading_deg, 4.0_f64.atan2(3.0).to_degrees());
    let mut integrator = MotionIntegrator::new(origin);
    apply(&mut integrator, &[
        MotionCommand::SetVelocityNeu(Neu {
            north: 1.0,
            ..Neu::default()
        }),
        heading,
        velocity,
    ])?;
    assert_close(integrator.velocity_neu.north, 3.0);

    let acceleration_command = MotionCommand::SetAccelerationNeu(acceleration);
    for direct in [velocity, heading] {
        for target in [
            MotionCommand::SetTargetSpeed {
                speed_mps: 8.0,
                accel_limit_mps2: 2.0,
            },
            MotionCommand::SetTargetHeading {
                heading_deg: 180.0,
                turn_rate_limit_dps: 30.0,
            },
        ] {
            let mut integrator = MotionIntegrator::new(origin);
            integrator.acceleration_neu = acceleration;
            apply(&mut integrator, &[target, direct])?;
            assert!(integrator.target_speed.is_none());
            assert!(integrator.target_heading.is_none());
            assert_close(integrator.acceleration_neu.north, 0.0);
        }
        let expected_up = match direct {
            MotionCommand::SetVelocityNeu(velocity) => velocity.up,
            MotionCommand::SetHeadingSpeed { climb_mps, .. } => climb_mps,
            _ => 0.0,
        };
        for commands in [[acceleration_command, direct], [
            direct,
            acceleration_command,
        ]] {
            let mut integrator = MotionIntegrator::new(origin);
            apply(&mut integrator, &commands)?;
            assert_close(integrator.acceleration_neu.north, 1.0);
            assert_close(integrator.velocity_neu.up, expected_up);
        }
    }
    Ok(())
}

#[test]
fn acceleration_and_target_replay_matrix_preserves_exact_write_sets()
-> Result<(), Error> {
    let origin = origin()?;
    let acceleration = MotionCommand::SetAccelerationNeu(Neu {
        north: 1.0,
        east: 2.0,
        up: 3.0,
    });
    let speed = MotionCommand::SetTargetSpeed {
        speed_mps: 8.0,
        accel_limit_mps2: 2.0,
    };
    let heading = MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 30.0,
    };

    let cases = [
        (&[acceleration, speed][..], true, false, 0.0),
        (&[speed, acceleration][..], false, false, 1.0),
        (&[acceleration, heading][..], false, true, 0.0),
        (&[heading, acceleration][..], false, false, 1.0),
        (&[speed, heading][..], true, true, 0.0),
        (&[heading, speed][..], true, true, 0.0),
        (&[speed, acceleration, heading][..], false, true, 0.0),
        (&[heading, acceleration, speed][..], true, false, 0.0),
        (&[speed, heading, acceleration][..], false, false, 1.0),
        (&[acceleration, speed, heading][..], true, true, 0.0),
    ];
    for (commands, has_speed, has_heading, acceleration_north) in cases {
        let mut integrator = MotionIntegrator::new(origin);
        apply(&mut integrator, commands)?;
        assert_eq!(integrator.target_speed.is_some(), has_speed);
        assert_eq!(integrator.target_heading.is_some(), has_heading);
        assert_close(integrator.acceleration_neu.north, acceleration_north);
    }
    Ok(())
}

#[test]
fn coexisting_targets_keep_heading_first_controller_order() -> Result<(), Error>
{
    let origin = origin()?;
    let speed = MotionCommand::SetTargetSpeed {
        speed_mps: 1.0,
        accel_limit_mps2: 2.0,
    };
    let heading = MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 90.0,
    };
    for targets in [[speed, heading], [heading, speed]] {
        let control = RuntimeMotionControl::new(origin);
        let mut integrator = MotionIntegrator::new(origin);
        control.submit(MotionCommand::SetHeadingSpeed {
            heading_deg: 0.0,
            speed_mps: 0.0,
            climb_mps: 0.0,
        })?;
        for target in targets {
            control.submit(target)?;
        }
        let endpoint = integrator.step(1.0, &control)?;
        let delta = endpoint - &origin;
        let displacement =
            Neu::from_ecef(&delta, Location::try_from(&origin)?.ltcmat());
        assert_close(displacement.north, 0.0);
        assert_close(displacement.east, 0.75);
    }
    Ok(())
}

#[test]
fn position_is_independent_and_latest_position_moves_to_tail()
-> Result<(), Error> {
    let origin = origin()?;
    let latest = Ecef::new(origin.x + 1.0, origin.y + 2.0, origin.z + 3.0);
    let mut integrator = MotionIntegrator::new(origin);
    apply(&mut integrator, &[
        MotionCommand::SetPositionEcef(Ecef::default()),
        MotionCommand::SetVelocityNeu(Neu {
            north: 4.0,
            ..Neu::default()
        }),
        MotionCommand::SetPositionEcef(latest),
    ])?;
    assert_close(integrator.position_ecef.x, latest.x);
    assert_close(integrator.velocity_neu.north, 4.0);
    Ok(())
}

#[test]
fn zero_limits_freeze_across_steps_and_keep_targets() -> Result<(), Error> {
    let origin = origin()?;
    let control = RuntimeMotionControl::new(origin);
    let mut integrator = MotionIntegrator::new(origin);
    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 0.0,
        speed_mps: 3.0,
        climb_mps: 0.0,
    })?;
    control.submit(MotionCommand::SetAccelerationNeu(Neu {
        north: 1.0,
        ..Neu::default()
    }))?;
    control.submit(MotionCommand::SetTargetSpeed {
        speed_mps: 10.0,
        accel_limit_mps2: 0.0,
    })?;
    control.submit(MotionCommand::SetTargetHeading {
        heading_deg: 90.0,
        turn_rate_limit_dps: 0.0,
    })?;

    let first = integrator.step(2.0, &control)?;
    let second = integrator.step(2.0, &control)?;
    assert_close(horizontal_speed_mps(integrator.velocity_neu), 3.0);
    assert_close(integrator.heading_deg, 0.0);
    assert!(integrator.target_speed.is_some());
    assert!(integrator.target_heading.is_some());
    assert_close(integrator.acceleration_neu.north, 0.0);

    let first_delta = first - &origin;
    let second_delta = second - &first;
    let first_neu =
        Neu::from_ecef(&first_delta, Location::try_from(&origin)?.ltcmat());
    let second_neu =
        Neu::from_ecef(&second_delta, Location::try_from(&first)?.ltcmat());
    assert_close(first_neu.north, 6.0);
    assert_close(second_neu.north, 6.0);
    assert_close(first_neu.east, 0.0);
    assert_close(second_neu.east, 0.0);
    Ok(())
}
