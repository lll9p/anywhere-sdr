use std::collections::{BTreeMap, BTreeSet};

use constants::CA_SEQ_LEN_FLOAT;
use geometry::Ecef;
use gps::{BroadcastEphemeris, Error, IonoUtc};

mod support;

use support::mini_receiver::{
    AcquisitionMetric, FixedScenario, Observation, RecoveredNavigation,
    RecoveredSubframe, TrackedSatellite, TrackingAssist, assisted_acquisition,
    build_ephemeris_from_decoded, build_tracking_assists, compare_ephemeris,
    decode_ephemeris, decode_navigation, observation_from_tracking,
    quantize_rinex_record, select_rinex_record, solve_pvt, track_satellites,
};

const TOKYO_LLH: [f64; 3] = [35.681_298, 139.766_247, 10.0];
const FIXED_SAMPLE_FREQUENCY_HZ: usize = 2_600_000;
const BASELINE_START_TIME: &str = "2022-01-01T00:00:00Z";

const MIN_ACQUIRED_SATELLITES: usize = 4;
const MIN_VALID_WORDS: usize = 15;

const MAX_ACQUISITION_CARRIER_ERROR_HZ: f64 = 250.0;
const MAX_ACQUISITION_CODE_PHASE_ERROR_CHIPS: f64 = 32.0;
const MAX_PVT_POSITION_ERROR_M: f64 = 100.0;
const MATRIX_MAX_PVT_POSITION_ERROR_M: f64 = 200.0;
const MAX_PVT_DUAL_PATH_GAP_M: f64 = 50.0;
const MAX_PVT_CLOCK_BIAS_GAP_M: f64 = 100.0;
const MAX_PVT_RESIDUAL_RMS_M: f64 = 50.0;
const MIN_OBSERVATIONS_PER_SATELLITE: usize = 3;

#[derive(Clone, Copy, Debug)]
struct ScenarioCase {
    name: &'static str,
    sf1_start_time_text: &'static str,
    sf2_start_time_text: &'static str,
    sf3_start_time_text: &'static str,
    location_llh: [f64; 3],
    sample_frequency_hz: usize,
    ionosphere_enabled: bool,
    fixed_gain: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
struct PipelineParams {
    visible_satellites: usize,
    tracked_satellites: usize,
    subframe_window_seconds: f64,
    target_fully_recovered: usize,
    min_valid_words: usize,
}

struct PipelineResult {
    used_acquisitions: Vec<AcquisitionMetric>,
    navigations: Vec<RecoveredNavigation>,
    observations: Vec<Observation>,
    decoded_ephemerides: BTreeMap<usize, BroadcastEphemeris>,
    rinex_ephemerides: BTreeMap<usize, BroadcastEphemeris>,
    truth_position: Ecef,
    ionoutc: IonoUtc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MutationFailurePath {
    NavigationInvalid,
    EphemerisMismatch,
    PvtInvalid,
}

fn baseline_case() -> ScenarioCase {
    ScenarioCase {
        name: "tokyo_baseline",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: TOKYO_LLH,
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn tokyo_t_plus_5s_case() -> ScenarioCase {
    ScenarioCase {
        name: "tokyo_t+5s",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: "2022-01-01T00:00:05Z",
        sf3_start_time_text: "2022-01-01T00:00:11Z",
        location_llh: TOKYO_LLH,
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn equator_0e_case() -> ScenarioCase {
    ScenarioCase {
        name: "equator_0E",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: [0.0, 0.0, 10.0],
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn high_lat_60n10e_case() -> ScenarioCase {
    ScenarioCase {
        name: "high_lat_60N10E",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: [60.0, 10.0, 10.0],
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn tokyo_iono_on_case() -> ScenarioCase {
    ScenarioCase {
        name: "tokyo_iono_on",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: TOKYO_LLH,
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: true,
        fixed_gain: Some(96),
    }
}

fn paris_48n2e_case() -> ScenarioCase {
    ScenarioCase {
        name: "paris_48N2E",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: [48.8566, 2.3522, 35.0],
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn sydney_33s151e_case() -> ScenarioCase {
    ScenarioCase {
        name: "sydney_33S151E",
        sf1_start_time_text: "2022-01-01T00:00:24Z",
        sf2_start_time_text: BASELINE_START_TIME,
        sf3_start_time_text: "2022-01-01T00:00:06Z",
        location_llh: [-33.8688, 151.2093, 20.0],
        sample_frequency_hz: FIXED_SAMPLE_FREQUENCY_HZ,
        ionosphere_enabled: false,
        fixed_gain: None,
    }
}

fn baseline_params() -> PipelineParams {
    PipelineParams {
        visible_satellites: 10,
        tracked_satellites: 8,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 5,
        min_valid_words: MIN_VALID_WORDS,
    }
}

fn assert_quality_case(
    case: ScenarioCase, params: PipelineParams, max_pvt_position_error_m: f64,
) -> Result<(), Error> {
    let pipeline = run_quality_pipeline(case, params)?;
    assert_pipeline_quality(case, params, &pipeline)?;
    assert_dual_path_pvt(case, &pipeline, max_pvt_position_error_m)?;
    Ok(())
}

fn matrix_params() -> PipelineParams {
    PipelineParams {
        visible_satellites: 16,
        tracked_satellites: 12,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 4,
        min_valid_words: MIN_VALID_WORDS,
    }
}

fn build_reference_scenario(
    case: ScenarioCase,
) -> Result<FixedScenario, Error> {
    FixedScenario::new_custom(
        case.sf2_start_time_text,
        0.2,
        case.location_llh,
        case.sample_frequency_hz,
        case.ionosphere_enabled,
        case.fixed_gain,
    )
}

fn wrapped_code_phase_error(left: f64, right: f64) -> f64 {
    let absolute_difference = (left - right).abs();
    absolute_difference.min(CA_SEQ_LEN_FLOAT - absolute_difference)
}

fn flip_tracked_navigation_bit(
    tracked_satellite: &mut TrackedSatellite, subframe: &RecoveredSubframe,
    word_index: usize, bit_index: usize,
) -> Result<(), Error> {
    if word_index >= 10 {
        return Err(Error::msg(format!(
            "invalid navigation word index for mutation: {word_index}"
        )));
    }
    if bit_index >= 30 {
        return Err(Error::msg(format!(
            "invalid navigation bit index for mutation: {bit_index}"
        )));
    }

    let subframe_start_epoch_index = tracked_satellite
        .prompt_epochs
        .iter()
        .position(|epoch| epoch.start_time == subframe.start_time)
        .ok_or_else(|| {
            Error::msg(format!(
                "PRN {} missing prompt epoch start for subframe {}",
                tracked_satellite.prn, subframe.subframe_id,
            ))
        })?;
    let bit_start_epoch_index =
        subframe_start_epoch_index + (word_index * 30 + bit_index) * 20;
    let bit_end_epoch_index = bit_start_epoch_index + 20;
    if bit_end_epoch_index > tracked_satellite.prompt_epochs.len() {
        return Err(Error::msg(format!(
            "PRN {} mutation bit range exceeds tracked prompt history",
            tracked_satellite.prn,
        )));
    }

    for prompt_epoch in &mut tracked_satellite.prompt_epochs
        [bit_start_epoch_index..bit_end_epoch_index]
    {
        prompt_epoch.value_re = -prompt_epoch.value_re;
        prompt_epoch.value_im = -prompt_epoch.value_im;
    }

    Ok(())
}

fn assert_navigation_invalid_classification(
    case: ScenarioCase, good_navigation: &RecoveredNavigation,
    mutated_navigation: Result<RecoveredNavigation, Error>,
) -> MutationFailurePath {
    match mutated_navigation {
        Err(error) => {
            assert!(
                error
                    .to_string()
                    .contains("failed to recover navigation words"),
                "{}: unexpected decode error: {error}",
                case.name,
            );
        }
        Ok(corrupted_navigation) => {
            assert!(
                corrupted_navigation.subframes.len()
                    < good_navigation.subframes.len()
                    || corrupted_navigation.diagnostics.valid_word_count
                        < good_navigation.diagnostics.valid_word_count / 2,
                "{}: navigation mutation did not degrade enough: \
                 corrupted={:?}, good={:?}",
                case.name,
                corrupted_navigation.diagnostics,
                good_navigation.diagnostics,
            );
        }
    }

    MutationFailurePath::NavigationInvalid
}

fn assert_ephemeris_mismatch_classification(
    case: ScenarioCase, differences: &[&'static str],
    expected_fields: &[&'static str],
) -> MutationFailurePath {
    for expected_field in expected_fields {
        assert!(
            differences.contains(expected_field),
            "{}: oracle failed to report expected field mismatch `{}`: {:?}",
            case.name,
            expected_field,
            differences,
        );
    }

    MutationFailurePath::EphemerisMismatch
}

fn assert_pvt_invalid_classification(
    case: ScenarioCase, baseline_error_m: f64, mutated_error_m: f64,
    mutated_residual_rms_m: f64,
) -> MutationFailurePath {
    assert!(
        mutated_error_m > baseline_error_m + 100.0,
        "{}: pseudorange mutation did not materially degrade position error: \
         baseline={} mutated={}",
        case.name,
        baseline_error_m,
        mutated_error_m,
    );
    assert!(
        mutated_error_m > MAX_PVT_POSITION_ERROR_M
            || mutated_residual_rms_m > MAX_PVT_RESIDUAL_RMS_M,
        "{}: pseudorange mutation did not break PVT quality enough: error={} \
         rms={}",
        case.name,
        mutated_error_m,
        mutated_residual_rms_m,
    );

    MutationFailurePath::PvtInvalid
}

#[test]
fn mini_receiver_quality_baseline_tokyo() -> Result<(), Error> {
    assert_quality_case(
        baseline_case(),
        baseline_params(),
        MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
fn mini_receiver_quality_tokyo_t_plus_5s() -> Result<(), Error> {
    assert_quality_case(
        tokyo_t_plus_5s_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
fn mini_receiver_quality_equator_0e() -> Result<(), Error> {
    assert_quality_case(
        equator_0e_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
fn mini_receiver_quality_high_lat_60n10e() -> Result<(), Error> {
    assert_quality_case(
        high_lat_60n10e_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
fn mini_receiver_quality_tokyo_iono_on() -> Result<(), Error> {
    assert_quality_case(
        tokyo_iono_on_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
#[ignore = "slow nightly scenario coverage"]
fn mini_receiver_quality_nightly_paris_48n2e() -> Result<(), Error> {
    assert_quality_case(
        paris_48n2e_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
#[ignore = "slow nightly scenario coverage"]
fn mini_receiver_quality_nightly_sydney_33s151e() -> Result<(), Error> {
    assert_quality_case(
        sydney_33s151e_case(),
        matrix_params(),
        MATRIX_MAX_PVT_POSITION_ERROR_M,
    )
}

#[test]
fn ephemeris_oracle_rejects_mutated_fields() -> Result<(), Error> {
    let case = baseline_case();
    let pipeline = run_quality_pipeline(case, baseline_params())?;
    let navigation = pipeline
        .navigations
        .iter()
        .find(|navigation| {
            pipeline.rinex_ephemerides.contains_key(&navigation.prn)
        })
        .ok_or_else(|| {
            Error::msg("missing recovered navigation for oracle test")
        })?;
    let reference_scenario = build_reference_scenario(case)?;
    let reference_week = reference_scenario.start_time().week;

    let decoded = decode_ephemeris(
        navigation.prn,
        &navigation.subframes,
        reference_week,
    )?;
    let rinex_record = select_rinex_record(
        reference_scenario.navigation_path(),
        navigation.prn,
        reference_scenario.start_time(),
    )?;
    let expected = quantize_rinex_record(&rinex_record, reference_week)?;
    let diagnostics = compare_ephemeris(&decoded, &expected);
    assert!(
        diagnostics.differences.is_empty(),
        "{}: baseline oracle unexpectedly found differences: {:?}",
        case.name,
        diagnostics.differences,
    );

    let mut mutated = decoded.clone();
    mutated.toe = mutated.toe.saturating_add(1);
    mutated.ecc = mutated.ecc.saturating_add(1);
    let diagnostics = compare_ephemeris(&mutated, &expected);
    let classification = assert_ephemeris_mismatch_classification(
        case,
        &diagnostics.differences,
        &["toe", "ecc"],
    );
    assert_eq!(classification, MutationFailurePath::EphemerisMismatch);

    Ok(())
}

#[test]
fn subframe_bit_flip_is_caught_by_ephemeris_oracle() -> Result<(), Error> {
    let case = baseline_case();
    let pipeline = run_quality_pipeline(case, baseline_params())?;
    let navigation = pipeline
        .navigations
        .iter()
        .find(|navigation| {
            pipeline.rinex_ephemerides.contains_key(&navigation.prn)
        })
        .cloned()
        .ok_or_else(|| {
            Error::msg(
                "missing recovered navigation for subframe mutation test",
            )
        })?;
    let reference_scenario = build_reference_scenario(case)?;
    let reference_week = reference_scenario.start_time().week;
    let rinex_record = select_rinex_record(
        reference_scenario.navigation_path(),
        navigation.prn,
        reference_scenario.start_time(),
    )?;
    let expected = quantize_rinex_record(&rinex_record, reference_week)?;

    let mut mutated_navigation = navigation.clone();
    let subframe_2 = mutated_navigation
        .subframes
        .iter_mut()
        .find(|subframe| subframe.subframe_id == 2)
        .ok_or_else(|| Error::msg("missing subframe 2 for mutation test"))?;
    subframe_2.data_words[9] ^= 1 << 8;

    let mutated = decode_ephemeris(
        mutated_navigation.prn,
        &mutated_navigation.subframes,
        reference_week,
    )?;
    let diagnostics = compare_ephemeris(&mutated, &expected);
    let classification = assert_ephemeris_mismatch_classification(
        case,
        &diagnostics.differences,
        &["toe"],
    );
    assert_eq!(classification, MutationFailurePath::EphemerisMismatch);

    Ok(())
}

#[test]
fn navigation_parity_bit_flip_is_classified_as_navigation_invalid()
-> Result<(), Error> {
    let case = baseline_case();
    let params = baseline_params();
    let (_, good_assists, _) =
        acquisition_fixture(case, params.visible_satellites)?;
    let (mut tracked_satellite, good_navigation) = recover_navigation_window(
        case,
        good_assists[0].prn,
        case.sf1_start_time_text,
        params.subframe_window_seconds,
    )?;

    for subframe in &good_navigation.subframes {
        flip_tracked_navigation_bit(&mut tracked_satellite, subframe, 1, 29)?;
    }

    let classification = assert_navigation_invalid_classification(
        case,
        &good_navigation,
        decode_navigation(
            tracked_satellite.prn,
            &tracked_satellite.prompt_epochs,
        ),
    );
    assert_eq!(classification, MutationFailurePath::NavigationInvalid);

    Ok(())
}

#[test]
fn navigation_payload_bit_flips_in_subframes_2_and_3_are_classified_as_ephemeris_mismatch()
-> Result<(), Error> {
    let case = baseline_case();
    let params = baseline_params();
    let pipeline = run_quality_pipeline(case, params)?;
    let good_navigation = pipeline
        .navigations
        .iter()
        .find(|navigation| {
            pipeline.rinex_ephemerides.contains_key(&navigation.prn)
        })
        .cloned()
        .ok_or_else(|| {
            Error::msg("missing recovered navigation for payload mutation")
        })?;
    let (_sf1_tracked, sf1_navigation) = recover_navigation_window(
        case,
        good_navigation.prn,
        case.sf1_start_time_text,
        params.subframe_window_seconds,
    )?;
    let subframe_1 = recover_subframe(&sf1_navigation, 1)?;
    let (subframe_2, mutated_subframe_2) =
        recover_original_and_mutated_subframe(
            case,
            good_navigation.prn,
            case.sf2_start_time_text,
            params.subframe_window_seconds,
            2,
            9,
            15,
        )?;
    let (subframe_3, mutated_subframe_3) =
        recover_original_and_mutated_subframe(
            case,
            good_navigation.prn,
            case.sf3_start_time_text,
            params.subframe_window_seconds,
            3,
            6,
            15,
        )?;
    let reference_scenario = build_reference_scenario(case)?;
    let reference_week = reference_scenario.start_time().week;
    let rinex_record = select_rinex_record(
        reference_scenario.navigation_path(),
        good_navigation.prn,
        reference_scenario.start_time(),
    )?;
    let expected = quantize_rinex_record(&rinex_record, reference_week)?;

    let subframe_2_mutated = decode_ephemeris(
        good_navigation.prn,
        &[subframe_1.clone(), mutated_subframe_2, subframe_3.clone()],
        reference_week,
    )?;
    let subframe_2_diagnostics =
        compare_ephemeris(&subframe_2_mutated, &expected);
    let subframe_2_classification = assert_ephemeris_mismatch_classification(
        case,
        &subframe_2_diagnostics.differences,
        &["toe"],
    );
    assert_eq!(
        subframe_2_classification,
        MutationFailurePath::EphemerisMismatch
    );

    let subframe_3_mutated = decode_ephemeris(
        good_navigation.prn,
        &[subframe_1, subframe_2, mutated_subframe_3],
        reference_week,
    )?;
    let subframe_3_diagnostics =
        compare_ephemeris(&subframe_3_mutated, &expected);
    let subframe_3_classification = assert_ephemeris_mismatch_classification(
        case,
        &subframe_3_diagnostics.differences,
        &["crc"],
    );
    assert_eq!(
        subframe_3_classification,
        MutationFailurePath::EphemerisMismatch
    );

    Ok(())
}

#[test]
fn pseudorange_bias_mutation_breaks_pvt_quality() -> Result<(), Error> {
    let case = baseline_case();
    let pipeline = run_quality_pipeline(case, baseline_params())?;
    let baseline_solution = solve_pvt(
        &pipeline.observations,
        &pipeline.rinex_ephemerides,
        &pipeline.ionoutc,
        pipeline.truth_position,
    )?;
    let unique_prns: Vec<usize> = pipeline
        .observations
        .iter()
        .map(|observation| observation.prn)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(
        unique_prns.len() >= 4,
        "{}: need at least four satellites for pseudorange mutation test",
        case.name,
    );

    let mut bias_by_prn = BTreeMap::new();
    let bias_pattern_m = [2_000.0, -1_500.0, 1_000.0, -750.0, 500.0];
    for (index, prn) in unique_prns.iter().enumerate() {
        bias_by_prn.insert(*prn, bias_pattern_m[index % bias_pattern_m.len()]);
    }

    let mut mutated_observations = pipeline.observations.clone();
    for observation in &mut mutated_observations {
        observation.pseudorange_m +=
            bias_by_prn.get(&observation.prn).copied().unwrap_or(0.0);
    }

    let mutated_solution = solve_pvt(
        &mutated_observations,
        &pipeline.rinex_ephemerides,
        &pipeline.ionoutc,
        pipeline.truth_position,
    )?;
    let baseline_error_m =
        baseline_solution.position_error_m(&pipeline.truth_position);
    let mutated_error_m =
        mutated_solution.position_error_m(&pipeline.truth_position);
    let classification = assert_pvt_invalid_classification(
        case,
        baseline_error_m,
        mutated_error_m,
        mutated_solution.residual_rms_m,
    );
    assert_eq!(classification, MutationFailurePath::PvtInvalid);

    Ok(())
}

#[test]
fn acquisition_rejects_wrong_prn_and_navigation_rejects_prompt_corruption()
-> Result<(), Error> {
    let case = baseline_case();
    let params = PipelineParams {
        visible_satellites: 10,
        tracked_satellites: 8,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 4,
        min_valid_words: MIN_VALID_WORDS,
    };

    let (first_block, good_assists, sample_frequency_hz) =
        acquisition_fixture(case, params.visible_satellites)?;
    let correct_metric = assisted_acquisition(
        &first_block,
        sample_frequency_hz,
        &[good_assists[0].clone()],
    )?
    .pop()
    .ok_or_else(|| Error::msg("missing correct acquisition metric"))?;

    let mut wrong_assist = good_assists[0].clone();
    wrong_assist.prn = (1..=32)
        .find(|prn| !good_assists.iter().any(|assist| assist.prn == *prn))
        .unwrap_or((wrong_assist.prn % 32) + 1);
    wrong_assist.predicted_carrier_hz += 6_000.0;
    wrong_assist.predicted_code_phase_chips =
        (wrong_assist.predicted_code_phase_chips + 400.0) % 1023.0;
    let wrong_metric = assisted_acquisition(
        &first_block,
        sample_frequency_hz,
        &[wrong_assist],
    )?
    .pop()
    .ok_or_else(|| Error::msg("missing wrong-PRN acquisition metric"))?;
    assert!(
        wrong_metric.peak_ratio < correct_metric.peak_ratio / 10.0,
        "{}: wrong PRN acquisition was not sufficiently degraded: wrong={}, \
         correct={}",
        case.name,
        wrong_metric.peak_ratio,
        correct_metric.peak_ratio
    );

    let mut wrong_frequency_assist = good_assists[0].clone();
    wrong_frequency_assist.predicted_carrier_hz += 2_000.0;
    let wrong_frequency_metric = assisted_acquisition(
        &first_block,
        sample_frequency_hz,
        &[wrong_frequency_assist],
    )?
    .pop()
    .ok_or_else(|| {
        Error::msg("missing large-frequency-offset acquisition metric")
    })?;
    assert!(
        wrong_frequency_metric.peak_ratio < correct_metric.peak_ratio / 4.0,
        "{}: large acquisition frequency error was not rejected strongly \
         enough: wrong={}, correct={}",
        case.name,
        wrong_frequency_metric.peak_ratio,
        correct_metric.peak_ratio,
    );

    let (tracked_satellite, good_navigation) = recover_navigation_window(
        case,
        good_assists[0].prn,
        case.sf1_start_time_text,
        params.subframe_window_seconds,
    )?;

    let mut corrupted_prompt_epochs = tracked_satellite.prompt_epochs.clone();
    for (prompt_index, prompt_epoch) in
        corrupted_prompt_epochs.iter_mut().enumerate()
    {
        if prompt_index % 20 < 10 {
            prompt_epoch.value_re = -prompt_epoch.value_re;
            prompt_epoch.value_im = -prompt_epoch.value_im;
        }
    }

    let classification = assert_navigation_invalid_classification(
        case,
        &good_navigation,
        decode_navigation(tracked_satellite.prn, &corrupted_prompt_epochs),
    );
    assert_eq!(classification, MutationFailurePath::NavigationInvalid);

    Ok(())
}

#[test]
fn pvt_requires_four_satellites() -> Result<(), Error> {
    let case = baseline_case();
    let params = PipelineParams {
        visible_satellites: 10,
        tracked_satellites: 8,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 4,
        min_valid_words: MIN_VALID_WORDS,
    };
    let pipeline = run_quality_pipeline(case, params)?;

    let mut unique = BTreeMap::new();
    for observation in &pipeline.observations {
        unique
            .entry(observation.prn)
            .or_insert_with(|| observation.clone());
    }
    let three: Vec<Observation> = unique.values().take(3).cloned().collect();
    let result = solve_pvt(
        &three,
        &pipeline.rinex_ephemerides,
        &pipeline.ionoutc,
        pipeline.truth_position,
    );
    assert!(matches!(
        result,
        Err(ref error) if error.to_string().contains("at least four satellites")
    ));
    Ok(())
}

fn acquisition_fixture(
    case: ScenarioCase, visible_satellites: usize,
) -> Result<(Vec<i16>, Vec<TrackingAssist>, f64), Error> {
    let mut scenario = FixedScenario::new_custom(
        case.sf2_start_time_text,
        0.2,
        case.location_llh,
        case.sample_frequency_hz,
        case.ionosphere_enabled,
        case.fixed_gain,
    )?;
    let all_visible = scenario.visible_satellites();
    let count = visible_satellites.min(all_visible.len());
    let assists = build_tracking_assists(&scenario, &all_visible[..count])?;

    let mut first_block = Vec::new();
    scenario.run_streaming::<_, Error>(|block_index, _, iq| {
        if block_index == 0 {
            first_block.extend_from_slice(iq);
        }
        Ok(())
    })?;
    Ok((first_block, assists, scenario.sample_frequency_hz()))
}

fn recover_navigation_window(
    case: ScenarioCase, prn: usize, start_time_text: &str,
    duration_seconds: f64,
) -> Result<(TrackedSatellite, RecoveredNavigation), Error> {
    let mut acquisition_scenario = FixedScenario::new_custom(
        start_time_text,
        0.2,
        case.location_llh,
        case.sample_frequency_hz,
        case.ionosphere_enabled,
        case.fixed_gain,
    )?;
    let visible_satellite = acquisition_scenario
        .visible_satellites()
        .into_iter()
        .find(|satellite| satellite.prn == prn)
        .ok_or_else(|| {
            Error::msg(format!(
                "{}: PRN {prn} is not visible at {start_time_text}",
                case.name
            ))
        })?;
    let assists = build_tracking_assists(
        &acquisition_scenario,
        std::slice::from_ref(&visible_satellite),
    )?;

    let mut first_block = Vec::new();
    acquisition_scenario.run_streaming::<_, Error>(|block_index, _, iq| {
        if block_index == 0 {
            first_block.extend_from_slice(iq);
        }
        Ok(())
    })?;

    let acquisition = assisted_acquisition(
        &first_block,
        acquisition_scenario.sample_frequency_hz(),
        &assists,
    )?
    .into_iter()
    .next()
    .ok_or_else(|| {
        Error::msg(format!(
            "{}: missing acquisition result for PRN {prn}",
            case.name
        ))
    })?;

    let mut tracking_scenario = FixedScenario::new_custom(
        start_time_text,
        duration_seconds,
        case.location_llh,
        case.sample_frequency_hz,
        case.ionosphere_enabled,
        case.fixed_gain,
    )?;
    let tracked_satellite =
        track_satellites(&mut tracking_scenario, &[acquisition])?
            .into_iter()
            .next()
            .ok_or_else(|| {
                Error::msg(format!(
                    "{}: missing tracked satellite for PRN {prn}",
                    case.name
                ))
            })?;
    let navigation = decode_navigation(prn, &tracked_satellite.prompt_epochs)?;
    Ok((tracked_satellite, navigation))
}

fn recover_original_and_mutated_subframe(
    case: ScenarioCase, prn: usize, start_time_text: &str,
    duration_seconds: f64, subframe_id: u8, word_index: usize,
    bit_index: usize,
) -> Result<(RecoveredSubframe, RecoveredSubframe), Error> {
    let (mut tracked_satellite, navigation) = recover_navigation_window(
        case,
        prn,
        start_time_text,
        duration_seconds,
    )?;
    let original_subframe = recover_subframe(&navigation, subframe_id)?;
    flip_tracked_navigation_bit(
        &mut tracked_satellite,
        &original_subframe,
        word_index,
        bit_index,
    )?;
    let mutated_navigation = decode_navigation(
        tracked_satellite.prn,
        &tracked_satellite.prompt_epochs,
    )
    .map_err(|error| {
        Error::msg(format!(
            "{}: subframe {subframe_id} payload mutation unexpectedly \
             invalidated navigation: {error}",
            case.name,
        ))
    })?;
    let mutated_subframe = recover_subframe(&mutated_navigation, subframe_id)?;
    Ok((original_subframe, mutated_subframe))
}

fn recover_subframe(
    navigation: &RecoveredNavigation, expected_subframe_id: u8,
) -> Result<RecoveredSubframe, Error> {
    navigation
        .subframes
        .iter()
        .find(|subframe| subframe.subframe_id == expected_subframe_id)
        .cloned()
        .ok_or_else(|| {
            Error::msg(format!(
                "PRN {} missing subframe {expected_subframe_id}",
                navigation.prn
            ))
        })
}

fn sort_acquisitions_by_quality(acquisitions: &mut [AcquisitionMetric]) {
    acquisitions.sort_by(|left, right| {
        right
            .peak_ratio
            .total_cmp(&left.peak_ratio)
            .then_with(|| left.prn.cmp(&right.prn))
    });
}

fn recover_navigation_triplet(
    case: ScenarioCase, prn: usize, duration_seconds: f64,
) -> Option<[(TrackedSatellite, RecoveredNavigation); 3]> {
    Some([
        recover_navigation_window(
            case,
            prn,
            case.sf1_start_time_text,
            duration_seconds,
        )
        .ok()?,
        recover_navigation_window(
            case,
            prn,
            case.sf2_start_time_text,
            duration_seconds,
        )
        .ok()?,
        recover_navigation_window(
            case,
            prn,
            case.sf3_start_time_text,
            duration_seconds,
        )
        .ok()?,
    ])
}

fn build_combined_navigation(
    prn: usize, navigations: [&RecoveredNavigation; 3], min_valid_words: usize,
) -> Option<RecoveredNavigation> {
    let subframe_1 = recover_subframe(navigations[0], 1).ok()?;
    let subframe_2 = recover_subframe(navigations[1], 2).ok()?;
    let subframe_3 = recover_subframe(navigations[2], 3).ok()?;

    let mut diagnostics = navigations[0].diagnostics.clone();
    diagnostics.valid_word_count = navigations
        .iter()
        .map(|navigation| navigation.diagnostics.valid_word_count)
        .sum();
    if diagnostics.valid_word_count < min_valid_words {
        return None;
    }

    Some(RecoveredNavigation {
        prn,
        diagnostics,
        subframes: vec![subframe_1, subframe_2, subframe_3],
    })
}

fn build_verified_ephemeris_pair(
    prn: usize, navigation: &RecoveredNavigation,
    reference_scenario: &FixedScenario, reference_week: i32,
) -> Result<Option<(BroadcastEphemeris, BroadcastEphemeris)>, Error> {
    let Ok(decoded) =
        decode_ephemeris(prn, &navigation.subframes, reference_week)
    else {
        return Ok(None);
    };
    let rinex_record = select_rinex_record(
        reference_scenario.navigation_path(),
        prn,
        reference_scenario.start_time(),
    )?;
    let expected = quantize_rinex_record(&rinex_record, reference_week)?;
    if !compare_ephemeris(&decoded, &expected)
        .differences
        .is_empty()
    {
        return Ok(None);
    }

    Ok(Some((
        build_ephemeris_from_decoded(&decoded, reference_week),
        build_ephemeris_from_decoded(&expected, reference_week),
    )))
}

fn collect_navigation_observations(
    prn: usize, tracked_satellite: &TrackedSatellite,
    navigation: &RecoveredNavigation,
    observations_by_key: &mut BTreeMap<(usize, u32), Observation>,
    reference_week: i32,
) {
    for subframe in &navigation.subframes {
        let observation = observation_from_tracking(
            tracked_satellite,
            subframe,
            reference_week,
        );
        if !(10_000_000.0..=30_000_000.0).contains(&observation.pseudorange_m) {
            continue;
        }
        observations_by_key
            .entry((prn, subframe.tow_count))
            .or_insert(observation);
    }
}

fn run_quality_pipeline(
    case: ScenarioCase, params: PipelineParams,
) -> Result<PipelineResult, Error> {
    let (first_block, assists, sample_frequency_hz) =
        acquisition_fixture(case, params.visible_satellites)?;
    let mut acquisitions =
        assisted_acquisition(&first_block, sample_frequency_hz, &assists)?;
    sort_acquisitions_by_quality(&mut acquisitions);

    let used_acquisitions: Vec<AcquisitionMetric> = acquisitions
        .iter()
        .take(params.tracked_satellites)
        .cloned()
        .collect();
    let selected_prns: Vec<usize> =
        used_acquisitions.iter().map(|metric| metric.prn).collect();

    let reference_scenario = FixedScenario::new_custom(
        case.sf2_start_time_text,
        0.2,
        case.location_llh,
        case.sample_frequency_hz,
        case.ionosphere_enabled,
        case.fixed_gain,
    )?;
    let truth_position = reference_scenario.receiver_position();
    let ionoutc = reference_scenario.ionoutc().clone();
    let reference_week = reference_scenario.start_time().week;

    let mut decoded_ephemerides = BTreeMap::new();
    let mut rinex_ephemerides = BTreeMap::new();
    let mut observations_by_key: BTreeMap<(usize, u32), Observation> =
        BTreeMap::new();
    let mut navigations = Vec::new();

    for prn in selected_prns {
        let Some(
            [
                (sf1_tracked, sf1_navigation),
                (sf2_tracked, sf2_navigation),
                (sf3_tracked, sf3_navigation),
            ],
        ) = recover_navigation_triplet(
            case,
            prn,
            params.subframe_window_seconds,
        )
        else {
            continue;
        };
        let Some(navigation) = build_combined_navigation(
            prn,
            [&sf1_navigation, &sf2_navigation, &sf3_navigation],
            params.min_valid_words,
        ) else {
            continue;
        };
        navigations.push(navigation.clone());

        let Some((decoded_ephemeris, rinex_ephemeris)) =
            build_verified_ephemeris_pair(
                prn,
                &navigation,
                &reference_scenario,
                reference_week,
            )?
        else {
            continue;
        };
        decoded_ephemerides.insert(prn, decoded_ephemeris);
        rinex_ephemerides.insert(prn, rinex_ephemeris);

        for (tracked_satellite, navigation) in [
            (&sf1_tracked, &sf1_navigation),
            (&sf2_tracked, &sf2_navigation),
            (&sf3_tracked, &sf3_navigation),
        ] {
            collect_navigation_observations(
                prn,
                tracked_satellite,
                navigation,
                &mut observations_by_key,
                reference_week,
            );
        }

        if decoded_ephemerides.len() >= params.target_fully_recovered {
            break;
        }
    }

    let mut observations: Vec<Observation> =
        observations_by_key.into_values().collect();
    observations.sort_by(|left, right| {
        left.prn.cmp(&right.prn).then_with(|| {
            left.transmit_time.sec.total_cmp(&right.transmit_time.sec)
        })
    });

    Ok(PipelineResult {
        used_acquisitions,
        navigations,
        observations,
        decoded_ephemerides,
        rinex_ephemerides,
        truth_position,
        ionoutc,
    })
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
