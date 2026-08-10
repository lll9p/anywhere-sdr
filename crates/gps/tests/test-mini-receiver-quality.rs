use gps::Error;

#[path = "mini_receiver_quality/navigation_mutation_tests.rs"]
mod navigation_mutation_tests;
#[path = "mini_receiver_quality/common.rs"]
mod quality;
#[path = "mini_receiver_quality/signal_mutation_tests.rs"]
mod signal_mutation_tests;
mod support;

use quality::{
    MATRIX_MAX_PVT_POSITION_ERROR_M, MAX_PVT_POSITION_ERROR_M,
    assert_quality_case, baseline_case, baseline_params, equator_0e_case,
    high_lat_60n10e_case, matrix_params, paris_48n2e_case, sydney_33s151e_case,
    tokyo_iono_on_case, tokyo_t_plus_5s_case,
};

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
