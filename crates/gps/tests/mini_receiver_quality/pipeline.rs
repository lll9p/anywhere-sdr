use std::collections::BTreeMap;

use gps::{BroadcastEphemeris, Error};

use super::scenarios::{PipelineParams, PipelineResult, ScenarioCase};
use crate::support::mini_receiver::{
    AcquisitionMetric, FixedScenario, Observation, RecoveredNavigation,
    RecoveredSubframe, TrackedSatellite, TrackingAssist, assisted_acquisition,
    build_ephemeris_from_decoded, build_tracking_assists, compare_ephemeris,
    decode_ephemeris, decode_navigation, observation_from_tracking,
    quantize_rinex_record, select_rinex_record, track_satellites,
};

pub(crate) fn build_reference_scenario(
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
pub(crate) fn acquisition_fixture(
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

pub(crate) fn recover_navigation_window(
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

pub(crate) fn recover_subframe(
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

pub(crate) fn run_quality_pipeline(
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
