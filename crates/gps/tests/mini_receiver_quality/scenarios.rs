use std::collections::BTreeMap;

use geometry::Ecef;
use gps::{BroadcastEphemeris, IonoUtc};

use crate::support::mini_receiver::{
    AcquisitionMetric, Observation, RecoveredNavigation,
};

const TOKYO_LLH: [f64; 3] = [35.681_298, 139.766_247, 10.0];
const FIXED_SAMPLE_FREQUENCY_HZ: usize = 2_600_000;
const BASELINE_START_TIME: &str = "2022-01-01T00:00:00Z";

pub(crate) const MIN_ACQUIRED_SATELLITES: usize = 4;
pub(crate) const MIN_VALID_WORDS: usize = 15;

pub(crate) const MAX_ACQUISITION_CARRIER_ERROR_HZ: f64 = 250.0;
pub(crate) const MAX_ACQUISITION_CODE_PHASE_ERROR_CHIPS: f64 = 32.0;
pub(crate) const MAX_PVT_POSITION_ERROR_M: f64 = 100.0;
pub(crate) const MATRIX_MAX_PVT_POSITION_ERROR_M: f64 = 200.0;
pub(crate) const MAX_PVT_DUAL_PATH_GAP_M: f64 = 50.0;
pub(crate) const MAX_PVT_CLOCK_BIAS_GAP_M: f64 = 100.0;
pub(crate) const MAX_PVT_RESIDUAL_RMS_M: f64 = 50.0;
pub(crate) const MIN_OBSERVATIONS_PER_SATELLITE: usize = 3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScenarioCase {
    pub(crate) name: &'static str,
    pub(crate) sf1_start_time_text: &'static str,
    pub(crate) sf2_start_time_text: &'static str,
    pub(crate) sf3_start_time_text: &'static str,
    pub(crate) location_llh: [f64; 3],
    pub(crate) sample_frequency_hz: usize,
    pub(crate) ionosphere_enabled: bool,
    pub(crate) fixed_gain: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PipelineParams {
    pub(crate) visible_satellites: usize,
    pub(crate) tracked_satellites: usize,
    pub(crate) subframe_window_seconds: f64,
    pub(crate) target_fully_recovered: usize,
    pub(crate) min_valid_words: usize,
}

pub(crate) struct PipelineResult {
    pub(crate) used_acquisitions: Vec<AcquisitionMetric>,
    pub(crate) navigations: Vec<RecoveredNavigation>,
    pub(crate) observations: Vec<Observation>,
    pub(crate) decoded_ephemerides: BTreeMap<usize, BroadcastEphemeris>,
    pub(crate) rinex_ephemerides: BTreeMap<usize, BroadcastEphemeris>,
    pub(crate) truth_position: Ecef,
    pub(crate) ionoutc: IonoUtc,
}

pub(crate) fn baseline_case() -> ScenarioCase {
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

pub(crate) fn tokyo_t_plus_5s_case() -> ScenarioCase {
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

pub(crate) fn equator_0e_case() -> ScenarioCase {
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

pub(crate) fn high_lat_60n10e_case() -> ScenarioCase {
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

pub(crate) fn tokyo_iono_on_case() -> ScenarioCase {
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

pub(crate) fn paris_48n2e_case() -> ScenarioCase {
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

pub(crate) fn sydney_33s151e_case() -> ScenarioCase {
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

pub(crate) fn baseline_params() -> PipelineParams {
    PipelineParams {
        visible_satellites: 10,
        tracked_satellites: 8,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 5,
        min_valid_words: MIN_VALID_WORDS,
    }
}
pub(crate) fn matrix_params() -> PipelineParams {
    PipelineParams {
        visible_satellites: 16,
        tracked_satellites: 12,
        subframe_window_seconds: 13.0,
        target_fully_recovered: 4,
        min_valid_words: MIN_VALID_WORDS,
    }
}
