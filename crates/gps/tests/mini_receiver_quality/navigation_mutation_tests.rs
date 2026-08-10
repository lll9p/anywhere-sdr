use gps::Error;

use crate::{
    quality::{
        MutationFailurePath, acquisition_fixture,
        assert_ephemeris_mismatch_classification,
        assert_navigation_invalid_classification, baseline_case,
        baseline_params, build_reference_scenario, flip_tracked_navigation_bit,
        recover_navigation_window, recover_original_and_mutated_subframe,
        recover_subframe, run_quality_pipeline,
    },
    support::mini_receiver::{
        compare_ephemeris, decode_ephemeris, decode_navigation,
        quantize_rinex_record, select_rinex_record,
    },
};

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
