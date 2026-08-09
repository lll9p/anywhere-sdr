use std::{collections::BTreeSet, path::PathBuf};

use constants::{LAMBDA_L1, MAX_CHAN};
use geometry::Ecef;

use super::{CARRIER_PHASE_MASK, Channel, carrier_phase_from_range};
use crate::{
    Error, SignalGenerator, SignalGeneratorBuilder, TimeRange, compute_range,
};

const INDEPENDENT_PHASE_COUNT: u32 = 1_u32 << 25;
const INDEPENDENT_PHASE_SCALE: f64 = INDEPENDENT_PHASE_COUNT as f64;
const SAMPLE_FREQUENCY: usize = 1_000_000;
const UPDATE_SECONDS: f64 = 0.1;

fn independent_phase_from_range(range_meters: f64) -> u32 {
    let cycles = (-range_meters / LAMBDA_L1).rem_euclid(1.0);
    ((INDEPENDENT_PHASE_SCALE * cycles) as u32) % INDEPENDENT_PHASE_COUNT
}

fn integrated_phase(initial: u32, step: i32, sample_count: usize) -> u32 {
    (i128::from(initial) + i128::from(step) * sample_count as i128)
        .rem_euclid(i128::from(INDEPENDENT_PHASE_COUNT)) as u32
}

fn circular_distance(left: u32, right: u32) -> u32 {
    let direct = left.abs_diff(right);
    direct.min(INDEPENDENT_PHASE_COUNT - direct)
}

fn initialized_generator() -> Result<SignalGenerator, Error> {
    let navigation = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .frequency(Some(SAMPLE_FREQUENCY))?
        .sample_rate(Some(UPDATE_SECONDS))
        .duration(Some(UPDATE_SECONDS))
        .data_format(Some(8))?
        .verbose(Some(false))
        .build()?;
    generator.initialize()?;
    Ok(generator)
}

fn allocated_prns(generator: &SignalGenerator) -> BTreeSet<usize> {
    generator
        .channels
        .iter()
        .filter_map(|channel| (channel.prn != 0).then_some(channel.prn))
        .collect()
}

fn later_allocatable_prns(
    generator: &SignalGenerator, receiver: &Ecef, later_time: &crate::GpsTime,
) -> Result<BTreeSet<usize>, Error> {
    let mut prns = BTreeSet::new();
    for (satellite_index, ephemeris) in generator.ephemerides
        [generator.valid_ephemerides_index]
        .iter()
        .enumerate()
    {
        if ephemeris
            .check_visibility(later_time, receiver, -90.0)?
            .is_some_and(|(_, visible)| visible)
        {
            prns.insert(satellite_index + 1);
            if prns.len() == MAX_CHAN {
                break;
            }
        }
    }
    Ok(prns)
}

fn channel_index(
    generator: &SignalGenerator, prn: usize,
) -> Result<usize, Error> {
    generator
        .channels
        .iter()
        .position(|channel| channel.prn == prn)
        .ok_or_else(|| {
            Error::msg(format!("PRN {prn} has no allocated channel"))
        })
}

#[test]
fn carrier_phase_from_range_matches_exact_vectors() {
    let vectors = [
        (0.0, 0),
        (LAMBDA_L1 / 4.0, 25_165_824),
        (-LAMBDA_L1 / 4.0, 8_388_608),
        (1.25 * LAMBDA_L1, 25_165_824),
        (5.125 * LAMBDA_L1, 29_360_128),
        (20_200_000.0, 17_947_305),
    ];

    for (range_meters, expected) in vectors {
        let actual = carrier_phase_from_range(range_meters);
        assert_eq!(actual, expected, "range {range_meters:.15} m");
        assert!(actual <= CARRIER_PHASE_MASK);
    }
}

#[test]
fn carrier_phase_modeled_range_has_the_expected_iq_quadrature_sign() {
    let iq_for_range = |range_meters| {
        Channel {
            carrier_phase: carrier_phase_from_range(range_meters),
            current_data_bit: 1,
            current_code_chip: 1,
            ..Channel::default()
        }
        .generate_iq_contribution(1)
    };

    let (positive_range_i, positive_range_q) = iq_for_range(LAMBDA_L1 / 4.0);
    let (negative_range_i, negative_range_q) = iq_for_range(-LAMBDA_L1 / 4.0);
    assert!(positive_range_q < 0);
    assert!(negative_range_q > 0);
    assert!(positive_range_q.abs() > positive_range_i.abs());
    assert!(negative_range_q.abs() > negative_range_i.abs());
}

#[test]
fn carrier_phase_signed_steps_remain_modulo_25_bits() {
    let cases = [
        (CARRIER_PHASE_MASK, 2, 1),
        (10, -2, 8),
        (1, -2, CARRIER_PHASE_MASK),
    ];
    for (initial, step, expected) in cases {
        let mut channel = Channel {
            carrier_phase: initial,
            carrier_phase_step: step,
            ..Channel::default()
        };
        channel.update_navigation_bits();
        assert_eq!(channel.carrier_phase, expected);
        assert_eq!(channel.carrier_phase, integrated_phase(initial, step, 1));
    }

    let mut increasing_range = Channel {
        rho0: TimeRange {
            range: 20_200_000.0,
            ..TimeRange::default()
        },
        ..Channel::default()
    };
    let later = TimeRange {
        range: 20_200_001.0,
        ..TimeRange::default()
    };
    increasing_range.update_state(&later, UPDATE_SECONDS, 1.0e-6);
    assert!(increasing_range.carrier_frequency < 0.0);
    assert!(increasing_range.carrier_phase_step < 0);

    let mut decreasing_range = Channel {
        rho0: TimeRange {
            range: 20_200_001.0,
            ..TimeRange::default()
        },
        ..Channel::default()
    };
    decreasing_range.update_state(
        &TimeRange {
            range: 20_200_000.0,
            ..TimeRange::default()
        },
        UPDATE_SECONDS,
        1.0e-6,
    );
    assert!(decreasing_range.carrier_frequency > 0.0);
    assert!(decreasing_range.carrier_phase_step > 0);
}

fn select_common_prn(
    retained: &SignalGenerator, reallocated: &SignalGenerator, receiver: &Ecef,
    later_time: &crate::GpsTime,
) -> Result<usize, Error> {
    let initial_prns = allocated_prns(retained);
    assert_eq!(initial_prns, allocated_prns(reallocated));
    let later_prns = later_allocatable_prns(retained, receiver, later_time)?;
    initial_prns
        .intersection(&later_prns)
        .copied()
        .find(|prn| {
            retained
                .channels
                .iter()
                .find(|channel| channel.prn == *prn)
                .is_some_and(|channel| {
                    independent_phase_from_range(channel.rho0.range) != 0
                })
        })
        .ok_or_else(|| {
            Error::msg("no nonzero-phase PRN is allocatable at both epochs")
        })
}

fn assert_retained_allocation(
    retained: &mut SignalGenerator, receiver: Ecef, target_prn: usize,
) -> Result<(usize, u32), Error> {
    let retained_index = channel_index(retained, target_prn)?;
    let initial_phase = independent_phase_from_range(
        retained.channels[retained_index].rho0.range,
    );
    assert_ne!(initial_phase, 0);
    assert_eq!(
        retained.channels[retained_index].carrier_phase,
        initial_phase
    );

    let initial_mapping = retained.allocated_satellite[target_prn - 1];
    let sentinel = 0x0055_aa55 & CARRIER_PHASE_MASK;
    retained.channels[retained_index].carrier_phase = sentinel;
    retained.allocate_channel(receiver)?;
    assert_eq!(
        retained.allocated_satellite[target_prn - 1],
        initial_mapping
    );
    assert_eq!(channel_index(retained, target_prn)?, retained_index);
    assert_eq!(retained.channels[retained_index].carrier_phase, sentinel);
    retained.channels[retained_index].carrier_phase = initial_phase;
    Ok((retained_index, initial_phase))
}

fn advance_retained_channel(
    retained: &mut SignalGenerator, receiver: &Ecef,
    later_time: &crate::GpsTime, target_prn: usize, retained_index: usize,
    initial_phase: u32,
) -> Result<usize, Error> {
    let later_range = compute_range(
        &retained.ephemerides[retained.valid_ephemerides_index][target_prn - 1],
        &retained.ionoutc,
        later_time,
        receiver,
    )?;
    let initial_range = retained.channels[retained_index].rho0.range;
    retained.channels[retained_index].update_state(
        &later_range,
        UPDATE_SECONDS,
        1.0 / retained.sample_frequency,
    );
    let expected_frequency =
        -(later_range.range - initial_range) / (UPDATE_SECONDS * LAMBDA_L1);
    let expected_step = (INDEPENDENT_PHASE_SCALE * expected_frequency
        / retained.sample_frequency)
        .round() as i32;
    let carrier_step = retained.channels[retained_index].carrier_phase_step;
    assert_eq!(carrier_step, expected_step);

    let sample_count =
        (retained.sample_frequency * UPDATE_SECONDS).round() as usize;
    let integrated =
        integrated_phase(initial_phase, carrier_step, sample_count);
    for _ in 0..sample_count {
        retained.channels[retained_index].update_navigation_bits();
    }
    assert_eq!(retained.channels[retained_index].carrier_phase, integrated);
    Ok(sample_count)
}

fn reallocate_channel_at_later_epoch(
    reallocated: &mut SignalGenerator, receiver: Ecef,
    later_time: &crate::GpsTime, target_prn: usize,
) -> Result<usize, Error> {
    reallocated.elevation_mask_degrees = 90.0;
    reallocated.allocate_channel(receiver)?;
    assert_eq!(reallocated.allocated_satellite[target_prn - 1], -1);
    assert!(
        reallocated
            .channels
            .iter()
            .all(|channel| channel.prn != target_prn)
    );
    for (slot, channel) in reallocated.channels.iter_mut().enumerate() {
        channel.carrier_phase =
            ((slot as u32 + 1) * 1_000_003) & CARRIER_PHASE_MASK;
    }
    reallocated.receiver_gps_time = later_time.clone();
    reallocated.elevation_mask_degrees = -90.0;
    reallocated.allocate_channel(receiver)?;

    let reallocated_index = channel_index(reallocated, target_prn)?;
    let mapped_index =
        usize::try_from(reallocated.allocated_satellite[target_prn - 1])
            .map_err(|_| {
                Error::msg(format!("PRN {target_prn} has invalid mapping"))
            })?;
    assert_eq!(mapped_index, reallocated_index);
    let reallocated_range = compute_range(
        &reallocated.ephemerides[reallocated.valid_ephemerides_index]
            [target_prn - 1],
        &reallocated.ionoutc,
        later_time,
        &receiver,
    )?;
    assert!(
        (reallocated.channels[reallocated_index].rho0.range
            - reallocated_range.range)
            .abs()
            < 1.0e-9
    );
    let expected_phase = independent_phase_from_range(reallocated_range.range);
    assert_eq!(
        reallocated.channels[reallocated_index].carrier_phase,
        expected_phase
    );
    Ok(reallocated_index)
}

#[test]
fn carrier_phase_allocation_and_reallocation_share_modeled_range()
-> Result<(), Error> {
    let mut retained = initialized_generator()?;
    let mut reallocated = initialized_generator()?;
    let receiver = retained
        .positions
        .first()
        .copied()
        .ok_or_else(Error::wrong_positions)?;
    let later_time = retained.receiver_gps_time.add_secs(UPDATE_SECONDS);
    let target_prn =
        select_common_prn(&retained, &reallocated, &receiver, &later_time)?;
    let (retained_index, initial_phase) =
        assert_retained_allocation(&mut retained, receiver, target_prn)?;
    let sample_count = advance_retained_channel(
        &mut retained,
        &receiver,
        &later_time,
        target_prn,
        retained_index,
        initial_phase,
    )?;
    let reallocated_index = reallocate_channel_at_later_epoch(
        &mut reallocated,
        receiver,
        &later_time,
        target_prn,
    )?;

    let endpoint_bound = sample_count.div_ceil(2) as u32 + 2;
    assert!(endpoint_bound < INDEPENDENT_PHASE_COUNT / 2);
    let endpoint_distance = circular_distance(
        retained.channels[retained_index].carrier_phase,
        reallocated.channels[reallocated_index].carrier_phase,
    );
    assert!(
        endpoint_distance <= endpoint_bound,
        "PRN {target_prn} endpoint distance {endpoint_distance} exceeds \
         {endpoint_bound}"
    );
    Ok(())
}
