use std::collections::{BTreeMap, BTreeSet};

use constants::SPEED_OF_LIGHT;
use geometry::Ecef;
use gps::{BroadcastEphemeris, Error, GpsTime, IonoUtc, compute_range};

use super::{PvtSolution, RecoveredSubframe, TrackedSatellite};

#[derive(Clone, Debug)]
pub struct Observation {
    pub prn: usize,
    pub receive_time: GpsTime,
    pub transmit_time: GpsTime,
    pub pseudorange_m: f64,
}

pub fn observation_from_tracking(
    tracked_satellite: &TrackedSatellite, subframe: &RecoveredSubframe,
    reference_week: i32,
) -> Observation {
    let receive_time = subframe.start_time.clone();
    let transmit_seconds =
        f64::from(subframe.tow_count.saturating_sub(1)) * 6.0;
    let transmit_time = GpsTime {
        week: expand_tow_week(reference_week, transmit_seconds),
        sec: transmit_seconds,
    };
    let pseudorange_m =
        receive_time.diff_secs(&transmit_time).abs() * SPEED_OF_LIGHT;

    Observation {
        prn: tracked_satellite.prn,
        receive_time,
        transmit_time,
        pseudorange_m,
    }
}

pub fn solve_pvt(
    observations: &[Observation],
    ephemerides_by_prn: &BTreeMap<usize, BroadcastEphemeris>,
    ionoutc: &IonoUtc, initial_guess: Ecef,
) -> Result<PvtSolution, Error> {
    const HUBER_THRESHOLD_M: f64 = 150.0;

    let unique_prns: BTreeSet<usize> = observations
        .iter()
        .map(|observation| observation.prn)
        .collect();
    if unique_prns.len() < 4 {
        return Err(Error::msg("PVT requires at least four satellites"));
    }

    let mut state = [initial_guess.x, initial_guess.y, initial_guess.z, 0.0f64];
    for _ in 0..10 {
        let mut normal = [[0.0f64; 4]; 4];
        let mut rhs = [0.0f64; 4];

        for observation in observations {
            let ephemeris =
                ephemerides_by_prn.get(&observation.prn).ok_or_else(|| {
                    Error::msg(format!(
                        "missing ephemeris for PRN {}",
                        observation.prn
                    ))
                })?;
            let receiver_position = Ecef::new(state[0], state[1], state[2]);
            let predicted = compute_range(
                ephemeris,
                ionoutc,
                &observation.receive_time,
                &receiver_position,
            )?
            .range
                + state[3];
            let residual = observation.pseudorange_m - predicted;
            let jacobian = numerical_jacobian(
                ephemeris,
                ionoutc,
                &observation.receive_time,
                &receiver_position,
            )?;
            let row = [jacobian[0], jacobian[1], jacobian[2], 1.0];

            let scale = {
                let abs_residual = residual.abs();
                if abs_residual <= HUBER_THRESHOLD_M {
                    1.0
                } else {
                    (HUBER_THRESHOLD_M / abs_residual).sqrt()
                }
            };
            let residual = residual * scale;
            let row = row.map(|value| value * scale);

            for row_index in 0..4 {
                rhs[row_index] += row[row_index] * residual;
                for column_index in 0..4 {
                    normal[row_index][column_index] +=
                        row[row_index] * row[column_index];
                }
            }
        }

        for (diagonal_index, row) in normal.iter_mut().enumerate() {
            row[diagonal_index] += 1.0e-6;
        }

        let delta = solve_linear_system(normal, rhs)?;
        let position_delta =
            (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2])
                .sqrt();
        state[0] += delta[0];
        state[1] += delta[1];
        state[2] += delta[2];
        state[3] += delta[3];

        if position_delta < 1.0e-3 && delta[3].abs() < 1.0e-3 {
            break;
        }
    }

    let receiver_position = Ecef::new(state[0], state[1], state[2]);
    let mut squared_error_sum = 0.0;
    for observation in observations {
        let ephemeris =
            ephemerides_by_prn.get(&observation.prn).ok_or_else(|| {
                Error::msg(format!(
                    "missing ephemeris for PRN {}",
                    observation.prn
                ))
            })?;
        let predicted = compute_range(
            ephemeris,
            ionoutc,
            &observation.receive_time,
            &receiver_position,
        )?
        .range
            + state[3];
        let residual = observation.pseudorange_m - predicted;
        squared_error_sum += residual * residual;
    }

    Ok(PvtSolution {
        position_ecef: receiver_position,
        clock_bias_m: state[3],
        residual_rms_m: (squared_error_sum / observations.len() as f64).sqrt(),
        used_satellites: unique_prns.len(),
    })
}

fn expand_tow_week(reference_week: i32, transmit_seconds: f64) -> i32 {
    let mut week = reference_week;
    if transmit_seconds > 604_800.0 {
        week += (transmit_seconds / 604_800.0).floor() as i32;
    }
    week
}

fn numerical_jacobian(
    ephemeris: &BroadcastEphemeris, ionoutc: &IonoUtc, receive_time: &GpsTime,
    receiver_position: &Ecef,
) -> Result<[f64; 3], Error> {
    let baseline =
        compute_range(ephemeris, ionoutc, receive_time, receiver_position)?
            .range;
    let delta = 1.0;

    let x_plus = Ecef::new(
        receiver_position.x + delta,
        receiver_position.y,
        receiver_position.z,
    );
    let y_plus = Ecef::new(
        receiver_position.x,
        receiver_position.y + delta,
        receiver_position.z,
    );
    let z_plus = Ecef::new(
        receiver_position.x,
        receiver_position.y,
        receiver_position.z + delta,
    );

    let dx = compute_range(ephemeris, ionoutc, receive_time, &x_plus)?.range
        - baseline;
    let dy = compute_range(ephemeris, ionoutc, receive_time, &y_plus)?.range
        - baseline;
    let dz = compute_range(ephemeris, ionoutc, receive_time, &z_plus)?.range
        - baseline;
    Ok([dx / delta, dy / delta, dz / delta])
}

fn solve_linear_system(
    mut matrix: [[f64; 4]; 4], mut rhs: [f64; 4],
) -> Result<[f64; 4], Error> {
    for pivot_index in 0..4 {
        let mut best_row = pivot_index;
        for row_index in pivot_index + 1..4 {
            if matrix[row_index][pivot_index].abs()
                > matrix[best_row][pivot_index].abs()
            {
                best_row = row_index;
            }
        }
        if matrix[best_row][pivot_index].abs() < 1.0e-12 {
            return Err(Error::msg("singular PVT normal matrix"));
        }
        if best_row != pivot_index {
            matrix.swap(best_row, pivot_index);
            rhs.swap(best_row, pivot_index);
        }

        let pivot = matrix[pivot_index][pivot_index];
        for value in matrix[pivot_index].iter_mut().skip(pivot_index) {
            *value /= pivot;
        }
        rhs[pivot_index] /= pivot;

        let pivot_row = matrix[pivot_index];
        for row_index in 0..4 {
            if row_index == pivot_index {
                continue;
            }
            let factor = matrix[row_index][pivot_index];
            for (column_index, value) in
                matrix[row_index].iter_mut().enumerate().skip(pivot_index)
            {
                *value -= factor * pivot_row[column_index];
            }
            rhs[row_index] -= factor * rhs[pivot_index];
        }
    }

    Ok(rhs)
}
