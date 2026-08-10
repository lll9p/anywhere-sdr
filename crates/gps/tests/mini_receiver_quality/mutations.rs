use gps::Error;

use super::{
    pipeline::{recover_navigation_window, recover_subframe},
    scenarios::{
        MAX_PVT_POSITION_ERROR_M, MAX_PVT_RESIDUAL_RMS_M, ScenarioCase,
    },
};
use crate::support::mini_receiver::{
    RecoveredNavigation, RecoveredSubframe, TrackedSatellite, decode_navigation,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MutationFailurePath {
    NavigationInvalid,
    EphemerisMismatch,
    PvtInvalid,
}
pub(crate) fn flip_tracked_navigation_bit(
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

pub(crate) fn assert_navigation_invalid_classification(
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

pub(crate) fn assert_ephemeris_mismatch_classification(
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

pub(crate) fn assert_pvt_invalid_classification(
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
pub(crate) fn recover_original_and_mutated_subframe(
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
