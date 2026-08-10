use std::collections::{BTreeMap, BTreeSet};

use gps::Error;

use crate::{
    quality::{
        MIN_VALID_WORDS, MutationFailurePath, PipelineParams,
        acquisition_fixture, assert_navigation_invalid_classification,
        assert_pvt_invalid_classification, baseline_case, baseline_params,
        recover_navigation_window, run_quality_pipeline,
    },
    support::mini_receiver::{
        Observation, assisted_acquisition, decode_navigation, solve_pvt,
    },
};

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
    let correct_metric =
        assisted_acquisition(&first_block, sample_frequency_hz, &[
            good_assists[0].clone(),
        ])?
        .pop()
        .ok_or_else(|| Error::msg("missing correct acquisition metric"))?;

    let mut wrong_assist = good_assists[0].clone();
    wrong_assist.prn = (1..=32)
        .find(|prn| !good_assists.iter().any(|assist| assist.prn == *prn))
        .unwrap_or((wrong_assist.prn % 32) + 1);
    wrong_assist.predicted_carrier_hz += 6_000.0;
    wrong_assist.predicted_code_phase_chips =
        (wrong_assist.predicted_code_phase_chips + 400.0) % 1023.0;
    let wrong_metric =
        assisted_acquisition(&first_block, sample_frequency_hz, &[
            wrong_assist,
        ])?
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
    let wrong_frequency_metric =
        assisted_acquisition(&first_block, sample_frequency_hz, &[
            wrong_frequency_assist,
        ])?
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
