use std::f64::consts::PI;

use constants::{WGS84_ECCENTRICITY, WGS84_RADIUS};

use crate::{Azel, Ecef, Error, Location, NavigationTarget, Neu};

const JAPAN_LLH_DEGREES: [f64; 3] = [
    35.274_015_989_114_844,
    137.014_864_091_813_68,
    100.000_843_243_673_44,
];
const JAPAN_ECEF_METERS: [f64; 3] =
    [-3_813_477.954, 3_554_276.552, 3_662_785.237];

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

fn assert_location_close(
    actual: &Location, expected: &Location, angle_tolerance: f64,
    height_tolerance: f64,
) {
    assert_close(
        actual.latitude_radians(),
        expected.latitude_radians(),
        angle_tolerance,
    );
    assert_close(
        actual.longitude_radians(),
        expected.longitude_radians(),
        angle_tolerance,
    );
    assert_close(
        actual.height_meters(),
        expected.height_meters(),
        height_tolerance,
    );
}

fn unit_vector(location: &Location) -> [f64; 3] {
    let (latitude_sine, latitude_cosine) =
        location.latitude_radians().sin_cos();
    let (longitude_sine, longitude_cosine) =
        location.longitude_radians().sin_cos();
    [
        latitude_cosine * longitude_cosine,
        latitude_cosine * longitude_sine,
        latitude_sine,
    ]
}

fn local_basis(location: &Location) -> ([f64; 3], [f64; 3]) {
    let (latitude_sine, latitude_cosine) =
        location.latitude_radians().sin_cos();
    let (longitude_sine, longitude_cosine) =
        location.longitude_radians().sin_cos();
    (
        [
            -latitude_sine * longitude_cosine,
            -latitude_sine * longitude_sine,
            latitude_cosine,
        ],
        [-longitude_sine, longitude_cosine, 0.0],
    )
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn bearing_oracle(source: &Location, destination: &Location) -> f64 {
    let destination = unit_vector(destination);
    let (north, east) = local_basis(source);
    dot(destination, east)
        .atan2(dot(destination, north))
        .to_degrees()
        .rem_euclid(360.0)
}

fn destination_oracle(
    source: &Location, bearing_degrees: f64, distance_meters: f64,
) -> (f64, f64) {
    let source_vector = unit_vector(source);
    let (north, east) = local_basis(source);
    let bearing = bearing_degrees.to_radians();
    let angular_distance = distance_meters / WGS84_RADIUS;
    let direction = [
        north[0] * bearing.cos() + east[0] * bearing.sin(),
        north[1] * bearing.cos() + east[1] * bearing.sin(),
        north[2] * bearing.cos() + east[2] * bearing.sin(),
    ];
    let destination = [
        source_vector[0] * angular_distance.cos()
            + direction[0] * angular_distance.sin(),
        source_vector[1] * angular_distance.cos()
            + direction[1] * angular_distance.sin(),
        source_vector[2] * angular_distance.cos()
            + direction[2] * angular_distance.sin(),
    ];
    (destination[2].asin(), destination[1].atan2(destination[0]))
}

fn navigation_target(
    source: Location, bearing_degrees: u16,
) -> NavigationTarget {
    let mut target = NavigationTarget::new();
    target.set_location(source);
    for _ in 0..bearing_degrees {
        target.inc_bearing();
    }
    target
}

#[test]
fn named_wgs84_japan_vector_round_trips() -> Result<(), Error> {
    let location = Location::try_from_degrees(
        JAPAN_LLH_DEGREES[0],
        JAPAN_LLH_DEGREES[1],
        JAPAN_LLH_DEGREES[2],
    )?;
    let expected = Ecef::from(&JAPAN_ECEF_METERS);
    let ecef = Ecef::from(&location);
    assert_close(ecef.x, expected.x, 0.002);
    assert_close(ecef.y, expected.y, 0.002);
    assert_close(ecef.z, expected.z, 0.002);

    let round_trip = Location::try_from(&expected)?;
    assert_location_close(&round_trip, &location, 5.0e-10, 0.004);
    Ok(())
}

#[test]
fn equator_poles_antimeridian_and_heights_are_defined() -> Result<(), Error> {
    let eccentricity_squared = WGS84_ECCENTRICITY.powi(2);
    let semi_minor_axis = WGS84_RADIUS * (1.0 - eccentricity_squared).sqrt();

    let equator = Location::try_from_degrees(0.0, 0.0, 0.0)?;
    let equator_ecef = Ecef::from(&equator);
    assert_close(equator_ecef.x, WGS84_RADIUS, 1.0e-9);
    assert_close(equator_ecef.y, 0.0, 1.0e-9);
    assert_close(equator_ecef.z, 0.0, 1.0e-9);

    for (z, latitude_degrees, height_meters) in [
        (semi_minor_axis + 100.0, 90.0, 100.0),
        (-(semi_minor_axis - 50.0), -90.0, -50.0),
    ] {
        let pole = Location::try_from(&Ecef::new(0.0, 0.0, z))?;
        assert_close(pole.latitude_degrees(), latitude_degrees, 1.0e-12);
        assert_close(pole.longitude_radians(), 0.0, 0.0);
        assert_close(pole.height_meters(), height_meters, 1.0e-8);
    }

    for height_meters in [-500.0, 0.0, 20_200_000.0] {
        let antimeridian =
            Location::try_from_degrees(12.5, 180.0, height_meters)?;
        let round_trip = Location::try_from(&Ecef::from(&antimeridian))?;
        assert_close(round_trip.latitude_degrees(), 12.5, 2.0e-9);
        assert_close(
            round_trip.longitude_radians().abs(),
            std::f64::consts::PI,
            1.0e-12,
        );
        assert_close(round_trip.height_meters(), height_meters, 1.0e-4);
    }
    Ok(())
}

#[test]
fn raw_geodetic_and_azel_constructors_enforce_units_and_domains()
-> Result<(), Error> {
    assert!(Location::try_from_degrees(90.0, -180.0, -100.0).is_ok());
    assert!(Location::try_from_degrees(-90.0, 180.0, 100.0).is_ok());
    for result in [
        Location::try_from_degrees(90.000_001, 0.0, 0.0),
        Location::try_from_degrees(0.0, 180.000_001, 0.0),
        Location::try_from_degrees(f64::NAN, 0.0, 0.0),
        Location::try_from_degrees(0.0, f64::INFINITY, 0.0),
        Location::try_from_degrees(0.0, 0.0, f64::NEG_INFINITY),
    ] {
        assert!(matches!(result, Err(Error::InvalidCoordinates { .. })));
    }

    let azel = Azel::try_from_degrees(270.0, -15.0)?;
    assert_close(azel.azimuth_degrees(), 270.0, 1.0e-12);
    assert_close(azel.elevation_degrees(), -15.0, 1.0e-12);
    assert_close(
        azel.azimuth_radians(),
        3.0 * std::f64::consts::FRAC_PI_2,
        2.0e-12,
    );

    for result in [
        Azel::try_from_degrees(-0.1, 0.0),
        Azel::try_from_degrees(360.0, 0.0),
        Azel::try_from_degrees(0.0, 90.000_001),
        Azel::try_from_radians(f64::NAN, 0.0),
    ] {
        assert!(matches!(result, Err(Error::InvalidAzel { .. })));
    }
    Ok(())
}

#[test]
fn inverse_ecef_rejects_non_finite_origin_and_unrepresentable_norms() {
    for ecef in [
        Ecef::new(f64::NAN, 0.0, 0.0),
        Ecef::new(0.0, f64::INFINITY, 0.0),
        Ecef::new(0.0, 0.0, f64::NEG_INFINITY),
        Ecef::new(f64::MAX, f64::MAX, 0.0),
    ] {
        assert!(matches!(
            Location::try_from(&ecef),
            Err(Error::InvalidEcef { .. })
        ));
    }
    assert!(matches!(
        Location::try_from(&Ecef::default()),
        Err(Error::EcefOrigin)
    ));
    assert!(matches!(
        Location::try_from(&Ecef::new(1.0e-300, -1.0e-300, 1.0e-300)),
        Err(Error::EcefOrigin)
    ));
}

#[test]
fn inverse_ecef_handles_huge_finite_values_without_overflow()
-> Result<(), Error> {
    for ecef in [
        Ecef::new(1.0e100, -1.0e100, 1.0e100),
        Ecef::new(1.0e300, -1.0e300, 1.0e300),
        Ecef::new(f64::MAX, 0.0, 0.0),
        Ecef::new(0.0, 0.0, -f64::MAX),
        Ecef::new(f64::MAX / 2.0, f64::MAX / 2.0, f64::MAX / 2.0),
    ] {
        let location = Location::try_from(&ecef)?;
        assert!(location.latitude_radians().is_finite());
        assert!(location.longitude_radians().is_finite());
        assert!(location.height_meters().is_finite());
        assert!((-90.0..=90.0).contains(&location.latitude_degrees()));
        assert!((-180.0..=180.0).contains(&location.longitude_degrees()));
    }
    Ok(())
}

#[test]
fn inverse_ecef_reports_deterministic_convergence_exhaustion() {
    let ecef = Ecef::new(
        8_879.790_271_552_554,
        9_761.938_749_430_401,
        -0.324_430_199_418_745_96,
    );
    assert!(matches!(
        Location::try_from(&ecef),
        Err(Error::EcefConversionDidNotConverge { iterations: 16, .. })
    ));
}

#[test]
fn deterministic_randomized_round_trips_remain_stable() -> Result<(), Error> {
    let mut state = 0x7a5b_39d2_c4e1_8f06_u64;
    let mut next_unit = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 11) as f64) / ((1_u64 << 53) as f64)
    };

    for _ in 0..512 {
        let latitude_degrees = -89.9 + 179.8 * next_unit();
        let longitude_degrees = -180.0 + 360.0 * next_unit();
        let height_meters = -1_000.0 + 40_001_000.0 * next_unit();
        let expected = Location::try_from_degrees(
            latitude_degrees,
            longitude_degrees,
            height_meters,
        )?;
        let actual = Location::try_from(&Ecef::from(&expected))?;
        assert_location_close(&actual, &expected, 1.0e-10, 2.0e-3);
    }
    Ok(())
}

#[test]
fn neu_to_azel_uses_explicit_radians_for_defined_directions()
-> Result<(), Error> {
    for (neu, expected_azimuth_degrees) in [
        (
            Neu {
                north: 1.0,
                east: 0.0,
                up: 0.0,
            },
            0.0,
        ),
        (
            Neu {
                north: 0.0,
                east: 1.0,
                up: 0.0,
            },
            90.0,
        ),
        (
            Neu {
                north: -1.0,
                east: 0.0,
                up: 0.0,
            },
            180.0,
        ),
        (
            Neu {
                north: 0.0,
                east: -1.0,
                up: 0.0,
            },
            270.0,
        ),
    ] {
        let azel = Azel::try_from(&neu)?;
        assert_close(azel.azimuth_degrees(), expected_azimuth_degrees, 1.0e-10);
        assert_close(azel.elevation_degrees(), 0.0, 0.0);
    }

    let elevated_east = Azel::try_from(&Neu {
        north: 0.0,
        east: 1.0,
        up: 1.0,
    })?;
    assert_close(elevated_east.azimuth_degrees(), 90.0, 5.0e-11);
    assert_close(elevated_east.elevation_degrees(), 45.0, 5.0e-11);

    let almost_north = Azel::try_from(&Neu {
        north: 1.0,
        east: -f64::EPSILON,
        up: 0.0,
    })?;
    assert!(
        (0.0..2.0 * std::f64::consts::PI)
            .contains(&almost_north.azimuth_radians())
    );

    let subnormal_north = Azel::try_from(&Neu {
        north: 1.0e-300,
        east: 0.0,
        up: 0.0,
    })?;
    assert_close(subnormal_north.azimuth_degrees(), 0.0, 0.0);
    Ok(())
}

#[test]
fn neu_to_azel_rejects_undefined_and_non_finite_directions() {
    for neu in [
        Neu::default(),
        Neu {
            north: 0.0,
            east: 0.0,
            up: 1.0,
        },
        Neu {
            north: 0.0,
            east: 0.0,
            up: -1.0,
        },
    ] {
        assert!(matches!(
            Azel::try_from(&neu),
            Err(Error::UndefinedNeuDirection { .. })
        ));
    }
    for neu in [
        Neu {
            north: f64::NAN,
            east: 1.0,
            up: 0.0,
        },
        Neu {
            north: 1.0,
            east: f64::INFINITY,
            up: 0.0,
        },
        Neu {
            north: 1.0,
            east: 0.0,
            up: f64::NEG_INFINITY,
        },
    ] {
        assert!(matches!(
            Azel::try_from(&neu),
            Err(Error::InvalidNeu { .. })
        ));
    }
}

#[test]
fn local_tangent_matrix_round_trips_neu() -> Result<(), Error> {
    let location = Location::try_from_degrees(
        JAPAN_LLH_DEGREES[0],
        JAPAN_LLH_DEGREES[1],
        JAPAN_LLH_DEGREES[2],
    )?;
    let matrix = location.ltcmat();
    let ecef = Ecef::from(&JAPAN_ECEF_METERS);
    let neu = Neu::from_ecef(&ecef, matrix);
    assert!(neu.north.is_finite());
    assert!(neu.east.is_finite());
    assert!(neu.up.is_finite());
    Ok(())
}

#[test]
fn navigation_bearings_match_tangent_vector_oracle() -> Result<(), Error> {
    for (
        source_latitude,
        source_longitude,
        target_latitude,
        target_longitude,
        expected_cardinal,
    ) in [
        (0.0, 0.0, 10.0, 0.0, Some(0.0)),
        (0.0, 0.0, 0.0, 10.0, Some(90.0)),
        (0.0, 0.0, -10.0, 0.0, Some(180.0)),
        (0.0, 0.0, 0.0, -10.0, Some(270.0)),
        (0.0, 0.0, 10.0, 10.0, None),
        (80.0, -60.0, 82.0, 40.0, None),
        (10.0, 179.0, 12.0, -179.0, None),
    ] {
        let source =
            Location::try_from_degrees(source_latitude, source_longitude, 0.0)?;
        let destination =
            Location::try_from_degrees(target_latitude, target_longitude, 0.0)?;
        let mut target = NavigationTarget::new();
        target.set_location(source);
        let actual = target.bearing(&destination);
        assert_close(actual, bearing_oracle(&source, &destination), 1.0e-10);
        if let Some(expected_cardinal) = expected_cardinal {
            assert_close(actual, expected_cardinal, 1.0e-10);
        }
        assert!((0.0..360.0).contains(&actual));
    }
    let source = Location::try_from_degrees(45.0, 90.0, 0.0)?;
    assert_close(navigation_target(source, 0).bearing(&source), 0.0, 0.0);
    Ok(())
}

#[test]
fn navigation_destinations_match_unit_vector_oracle() -> Result<(), Error> {
    for (latitude, longitude, bearing, distance) in [
        (10.0, 179.0, 90, 400_000.0),
        (-10.0, -179.0, 270, 400_000.0),
        (89.5, 30.0, 15, 200_000.0),
        (-89.5, -30.0, 345, 200_000.0),
        (35.0, 40.0, 225, -750_000.0),
        (23.0, -170.0, 123, WGS84_RADIUS * (4.0 * PI + 0.7)),
    ] {
        let source = Location::try_from_degrees(latitude, longitude, 123.4)?;
        let (expected_latitude, expected_longitude) =
            destination_oracle(&source, f64::from(bearing), distance);
        let actual = navigation_target(source, bearing).go(distance)?;
        assert_close(actual.latitude_radians(), expected_latitude, 2.0e-12);
        assert_close(actual.longitude_radians(), expected_longitude, 2.0e-12);
        assert_close(actual.height_meters(), source.height_meters(), 0.0);
        assert!((-PI..PI).contains(&actual.longitude_radians()));
    }
    Ok(())
}

#[test]
fn navigation_canonicalizes_antimeridian_and_mutates_only_on_success()
-> Result<(), Error> {
    let source = Location::try_from_degrees(0.0, 0.0, -50.0)?;
    let mut target = navigation_target(source, 90);
    let destination = target.go(PI * WGS84_RADIUS)?;
    assert_close(destination.latitude_radians(), 0.0, 1.0e-12);
    assert_close(destination.longitude_radians(), -PI, 0.0);
    assert_close(destination.height_meters(), source.height_meters(), 0.0);
    assert_location_close(&target.go(0.0)?, &destination, 0.0, 0.0);

    let mut target = navigation_target(source, 45);
    for distance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            target.go(distance),
            Err(Error::InvalidCoordinates { .. })
        ));
    }
    assert_location_close(&target.go(0.0)?, &source, 0.0, 0.0);
    Ok(())
}
