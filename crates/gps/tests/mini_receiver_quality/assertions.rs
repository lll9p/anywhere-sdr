use std::collections::{BTreeMap, BTreeSet};

use constants::CA_SEQ_LEN_FLOAT;
use gps::Error;

use super::{
    pipeline::{recover_subframe, run_quality_pipeline},
    scenarios::{
        MAX_ACQUISITION_CARRIER_ERROR_HZ,
        MAX_ACQUISITION_CODE_PHASE_ERROR_CHIPS, MAX_PVT_CLOCK_BIAS_GAP_M,
        MAX_PVT_DUAL_PATH_GAP_M, MAX_PVT_RESIDUAL_RMS_M,
        MIN_ACQUIRED_SATELLITES, MIN_OBSERVATIONS_PER_SATELLITE,
        PipelineParams, PipelineResult, ScenarioCase,
    },
};
use crate::support::mini_receiver::{Observation, solve_pvt};

pub(crate) fn assert_quality_case(
    case: ScenarioCase, params: PipelineParams, max_pvt_position_error_m: f64,
) -> Result<(), Error> {
    let pipeline = run_quality_pipeline(case, params)?;
    assert_pipeline_quality(case, params, &pipeline)?;
    assert_dual_path_pvt(case, &pipeline, max_pvt_position_error_m)?;
    Ok(())
}
fn wrapped_code_phase_error(left: f64, right: f64) -> f64 {
    let absolute_difference = (left - right).abs();
    absolute_difference.min(CA_SEQ_LEN_FLOAT - absolute_difference)
}
fn assert_acquisition_quality(case: ScenarioCase, pipeline: &PipelineResult) {
    assert!(
        pipeline.used_acquisitions.len() >= MIN_ACQUIRED_SATELLITES,
        "{}: need at least four acquired satellites, got {}",
        case.name,
        pipeline.used_acquisitions.len(),
    );
    for acquisition in &pipeline.used_acquisitions {
        assert!(
            acquisition.peak_ratio > 1.5,
            "{}: PRN {} acquisition peak ratio too low: {}",
            case.name,
            acquisition.prn,
            acquisition.peak_ratio,
        );
        let carrier_error_hz = (acquisition.acquired_carrier_hz
            - acquisition.predicted_carrier_hz)
            .abs();
        assert!(
            carrier_error_hz <= MAX_ACQUISITION_CARRIER_ERROR_HZ,
            "{}: PRN {} acquisition carrier pull-in too large: {} Hz",
            case.name,
            acquisition.prn,
            carrier_error_hz,
        );
        let code_phase_error_chips = wrapped_code_phase_error(
            acquisition.acquired_code_phase_chips,
            acquisition.predicted_code_phase_chips,
        );
        assert!(
            code_phase_error_chips <= MAX_ACQUISITION_CODE_PHASE_ERROR_CHIPS,
            "{}: PRN {} acquisition code phase pull-in too large: {} chips",
            case.name,
            acquisition.prn,
            code_phase_error_chips,
        );
    }
}

fn assert_navigation_quality(
    case: ScenarioCase, params: PipelineParams, pipeline: &PipelineResult,
) -> Result<(), Error> {
    assert!(
        pipeline.decoded_ephemerides.len() >= MIN_ACQUIRED_SATELLITES
            && pipeline.rinex_ephemerides.len() >= MIN_ACQUIRED_SATELLITES,
        "{}: insufficient fully recovered satellites: decoded={}, rinex={} \
         (target {})",
        case.name,
        pipeline.decoded_ephemerides.len(),
        pipeline.rinex_ephemerides.len(),
        params.target_fully_recovered,
    );
    assert!(
        pipeline.navigations.len() >= MIN_ACQUIRED_SATELLITES,
        "{}: insufficient decoded navigations: {}",
        case.name,
        pipeline.navigations.len(),
    );

    for navigation in &pipeline.navigations {
        assert!(
            navigation.diagnostics.valid_word_count >= params.min_valid_words,
            "{}: PRN {} valid words too low: {}",
            case.name,
            navigation.prn,
            navigation.diagnostics.valid_word_count,
        );
        for expected_subframe_id in 1u8..=3 {
            assert!(
                navigation.subframes.iter().any(|subframe| subframe
                    .subframe_id
                    == expected_subframe_id),
                "{}: PRN {} missing subframe {}",
                case.name,
                navigation.prn,
                expected_subframe_id,
            );
        }

        let subframe_1 = recover_subframe(navigation, 1).map_err(|_| {
            Error::msg(format!(
                "{}: PRN {} missing subframe 1 after validation",
                case.name, navigation.prn
            ))
        })?;
        let subframe_2 = recover_subframe(navigation, 2).map_err(|_| {
            Error::msg(format!(
                "{}: PRN {} missing subframe 2 after validation",
                case.name, navigation.prn
            ))
        })?;
        let subframe_3 = recover_subframe(navigation, 3).map_err(|_| {
            Error::msg(format!(
                "{}: PRN {} missing subframe 3 after validation",
                case.name, navigation.prn
            ))
        })?;

        assert!(
            subframe_1.tow_count % 5 == 1
                && subframe_2.tow_count % 5 == 2
                && subframe_3.tow_count % 5 == 3,
            "{}: PRN {} recovered subframe TOW modulo sequence is \
             inconsistent: [{}, {}, {}]",
            case.name,
            navigation.prn,
            subframe_1.tow_count,
            subframe_2.tow_count,
            subframe_3.tow_count,
        );
    }

    Ok(())
}

fn assert_observation_quality(case: ScenarioCase, pipeline: &PipelineResult) {
    let unique_prns: BTreeSet<usize> =
        pipeline.observations.iter().map(|obs| obs.prn).collect();
    assert!(
        unique_prns.len() >= MIN_ACQUIRED_SATELLITES,
        "{}: need at least four satellites for PVT observations, got {}",
        case.name,
        unique_prns.len(),
    );
    assert!(
        pipeline.observations.len()
            >= MIN_ACQUIRED_SATELLITES * MIN_OBSERVATIONS_PER_SATELLITE,
        "{}: insufficient observations for PVT: {}",
        case.name,
        pipeline.observations.len(),
    );

    let mut observations_per_prn = BTreeMap::new();
    for observation in &pipeline.observations {
        assert!(
            (10_000_000.0..=30_000_000.0).contains(&observation.pseudorange_m),
            "{}: PRN {} pseudorange out of range: {} m",
            case.name,
            observation.prn,
            observation.pseudorange_m,
        );
        assert!(
            observation
                .receive_time
                .diff_secs(&observation.transmit_time)
                > 0.0,
            "{}: PRN {} observation has non-positive time-of-flight",
            case.name,
            observation.prn,
        );
        *observations_per_prn
            .entry(observation.prn)
            .or_insert(0usize) += 1;
    }

    for prn in pipeline.decoded_ephemerides.keys() {
        let observation_count =
            observations_per_prn.get(prn).copied().unwrap_or(0);
        assert!(
            observation_count >= MIN_OBSERVATIONS_PER_SATELLITE,
            "{}: PRN {} has too few observations for stable PVT: {}",
            case.name,
            prn,
            observation_count,
        );
    }
}

fn assert_pipeline_quality(
    case: ScenarioCase, params: PipelineParams, pipeline: &PipelineResult,
) -> Result<(), Error> {
    assert_acquisition_quality(case, pipeline);
    assert_navigation_quality(case, params, pipeline)?;
    assert_observation_quality(case, pipeline);

    Ok(())
}

fn assert_dual_path_pvt(
    case: ScenarioCase, pipeline: &PipelineResult, max_position_error_m: f64,
) -> Result<(), Error> {
    let unique_prns: BTreeSet<usize> =
        pipeline.observations.iter().map(|obs| obs.prn).collect();
    assert!(
        unique_prns.len() >= MIN_ACQUIRED_SATELLITES,
        "{}: need at least four satellites for PVT, got {}",
        case.name,
        unique_prns.len(),
    );

    let observations: Vec<Observation> = pipeline.observations.clone();

    let decoded_solution = solve_pvt(
        &observations,
        &pipeline.decoded_ephemerides,
        &pipeline.ionoutc,
        pipeline.truth_position,
    )?;
    let rinex_solution = solve_pvt(
        &observations,
        &pipeline.rinex_ephemerides,
        &pipeline.ionoutc,
        pipeline.truth_position,
    )?;

    if std::env::var("MINI_RECEIVER_PVT_DEBUG").is_ok() {
        let decoded_error_m =
            decoded_solution.position_error_m(&pipeline.truth_position);
        let rinex_error_m =
            rinex_solution.position_error_m(&pipeline.truth_position);
        eprintln!(
            "{}: prns={:?} obs={} decoded_err_m={:.3} rinex_err_m={:.3} \
             decoded_rms_m={:.3} rinex_rms_m={:.3}",
            case.name,
            unique_prns,
            observations.len(),
            decoded_error_m,
            rinex_error_m,
            decoded_solution.residual_rms_m,
            rinex_solution.residual_rms_m,
        );
    }

    assert!(
        decoded_solution.used_satellites >= 4
            && rinex_solution.used_satellites >= 4,
        "{}: PVT used insufficient satellites: decoded={}, rinex={}",
        case.name,
        decoded_solution.used_satellites,
        rinex_solution.used_satellites,
    );

    let decoded_error_m =
        decoded_solution.position_error_m(&pipeline.truth_position);
    let rinex_error_m =
        rinex_solution.position_error_m(&pipeline.truth_position);
    assert!(
        decoded_error_m < max_position_error_m,
        "{}: decoded-ephemeris PVT position error too large: {} m",
        case.name,
        decoded_error_m,
    );
    assert!(
        rinex_error_m < max_position_error_m,
        "{}: RINEX-ephemeris PVT position error too large: {} m",
        case.name,
        rinex_error_m,
    );
    assert!(
        decoded_solution.residual_rms_m < MAX_PVT_RESIDUAL_RMS_M,
        "{}: decoded-ephemeris PVT residual RMS too large: {} m",
        case.name,
        decoded_solution.residual_rms_m,
    );
    assert!(
        rinex_solution.residual_rms_m < MAX_PVT_RESIDUAL_RMS_M,
        "{}: RINEX-ephemeris PVT residual RMS too large: {} m",
        case.name,
        rinex_solution.residual_rms_m,
    );

    let dual_path_gap_m = {
        let dx =
            decoded_solution.position_ecef.x - rinex_solution.position_ecef.x;
        let dy =
            decoded_solution.position_ecef.y - rinex_solution.position_ecef.y;
        let dz =
            decoded_solution.position_ecef.z - rinex_solution.position_ecef.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    assert!(
        dual_path_gap_m < MAX_PVT_DUAL_PATH_GAP_M,
        "{}: dual-path PVT position gap too large: {} m",
        case.name,
        dual_path_gap_m,
    );
    assert!(
        (decoded_solution.clock_bias_m - rinex_solution.clock_bias_m).abs()
            < MAX_PVT_CLOCK_BIAS_GAP_M,
        "{}: dual-path PVT clock bias gap too large: {} m",
        case.name,
        (decoded_solution.clock_bias_m - rinex_solution.clock_bias_m).abs(),
    );

    Ok(())
}
