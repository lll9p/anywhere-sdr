use std::{error::Error as StdError, path::PathBuf};

use constants::{OMEGA_EARTH, SPEED_OF_LIGHT};
use geometry::{Ecef, Location};
use gps::{
    BroadcastEphemeris, Error, GpsTime, compute_range, read_navigation_data,
};

const APPROXIMATE_BASELINES_MPS: [f64; 3] =
    [-265.199_329_003, -495.031_170_911, -558.755_591_125];
const GEOMETRIC_BASELINES_MPS: [f64; 3] =
    [-265.154_619_515, -494.990_161_285, -558.719_704_784];

fn norm(vector: [f64; 3]) -> f64 {
    vector
        .into_iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt()
}

fn receiver_to_satellite(
    satellite_position: [f64; 3], receiver: &Ecef,
) -> [f64; 3] {
    [
        satellite_position[0] - receiver.x,
        satellite_position[1] - receiver.y,
        satellite_position[2] - receiver.z,
    ]
}

fn independent_satellite_los_rate_approx_mps(
    ephemeris: &BroadcastEphemeris, time: &GpsTime, receiver: &Ecef,
) -> f64 {
    let (position, velocity, _) = ephemeris.compute_satellite_state(time);
    let travel_time =
        norm(receiver_to_satellite(position, receiver)) / SPEED_OF_LIGHT;
    let transmit_position = [
        position[0] - velocity[0] * travel_time,
        position[1] - velocity[1] * travel_time,
        position[2] - velocity[2] * travel_time,
    ];
    let corrected_position = [
        transmit_position[0] + transmit_position[1] * OMEGA_EARTH * travel_time,
        transmit_position[1] - transmit_position[0] * OMEGA_EARTH * travel_time,
        transmit_position[2],
    ];
    let corrected_los = receiver_to_satellite(corrected_position, receiver);
    velocity
        .into_iter()
        .zip(corrected_los)
        .map(|(velocity_component, los_component)| {
            velocity_component * los_component
        })
        .sum::<f64>()
        / norm(corrected_los)
}

fn independent_geometric_distance(
    ephemeris: &BroadcastEphemeris, time: &GpsTime, receiver: &Ecef,
) -> f64 {
    let (receive_position, ..) = ephemeris.compute_satellite_state(time);
    let mut travel_time =
        norm(receiver_to_satellite(receive_position, receiver))
            / SPEED_OF_LIGHT;
    let mut distance = 0.0;
    for _ in 0..12 {
        let transmit_time = time.add_secs(-travel_time);
        let (transmit_position, ..) =
            ephemeris.compute_satellite_state(&transmit_time);
        let angle = OMEGA_EARTH * travel_time;
        let (sine, cosine) = angle.sin_cos();
        let rotated_position = [
            cosine * transmit_position[0] + sine * transmit_position[1],
            -sine * transmit_position[0] + cosine * transmit_position[1],
            transmit_position[2],
        ];
        distance = norm(receiver_to_satellite(rotated_position, receiver));
        let next_travel_time = distance / SPEED_OF_LIGHT;
        if (next_travel_time - travel_time).abs() < 1.0e-14 {
            break;
        }
        travel_time = next_travel_time;
    }
    distance
}

fn independent_geometric_rate_mps(
    ephemeris: &BroadcastEphemeris, time: &GpsTime, receiver: &Ecef,
) -> f64 {
    let half_interval_seconds = 0.05;
    let before = independent_geometric_distance(
        ephemeris,
        &time.add_secs(-half_interval_seconds),
        receiver,
    );
    let after = independent_geometric_distance(
        ephemeris,
        &time.add_secs(half_interval_seconds),
        receiver,
    );
    (after - before) / (2.0 * half_interval_seconds)
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let difference = (actual - expected).abs();
    assert!(
        difference <= tolerance,
        "{label}: actual {actual:.12}, expected {expected:.12}, difference \
         {difference:.12}"
    );
}

#[test]
fn satellite_los_approximation_is_not_a_geometric_range_derivative()
-> Result<(), Box<dyn StdError>> {
    let navigation = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let (_, ionoutc, ephemerides) = read_navigation_data(&navigation)?;
    let ephemeris = ephemerides
        .iter()
        .flat_map(|set| set.iter())
        .find(|ephemeris| ephemeris.vflg)
        .ok_or(Error::NoEphemeris)?;
    let receiver_location =
        Location::try_from_degrees(35.681_298, 139.766_247, 10.0)?;
    let receiver = Ecef::from(&receiver_location);

    for (index, offset_seconds) in
        [-3600.0, 0.0, 3600.0].into_iter().enumerate()
    {
        let time = ephemeris.toe.add_secs(offset_seconds);
        let range = compute_range(ephemeris, &ionoutc, &time, &receiver)?;
        let independent_approximation =
            independent_satellite_los_rate_approx_mps(
                ephemeris, &time, &receiver,
            );
        let geometric_rate =
            independent_geometric_rate_mps(ephemeris, &time, &receiver);

        assert_close(
            range.satellite_los_rate_approx_mps,
            independent_approximation,
            1.0e-9,
            "exported satellite LOS approximation",
        );
        assert_close(
            range.satellite_los_rate_approx_mps,
            APPROXIMATE_BASELINES_MPS[index],
            1.0e-9,
            "archived satellite LOS approximation",
        );
        assert_close(
            geometric_rate,
            GEOMETRIC_BASELINES_MPS[index],
            1.0e-6,
            "independent geometric range derivative",
        );
        assert!(
            (range.satellite_los_rate_approx_mps - geometric_rate).abs() > 0.01,
            "epoch offset {offset_seconds} did not distinguish the \
             approximation"
        );
    }
    Ok(())
}
