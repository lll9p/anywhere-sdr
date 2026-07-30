use crate::{
    constants::{MAX2837, MHZ},
    error::Error,
};

/// Lowest center frequency supported by the ordinary tuning API.
const RF_FREQUENCY_MIN_HZ: u64 = 1_000_000;
/// Highest center frequency supported by the ordinary tuning API.
const RF_FREQUENCY_MAX_HZ: u64 = 6_000_000_000;
/// Lowest supported effective sample rate.
const SAMPLE_RATE_MIN_HZ: u64 = 2_000_000;
/// Highest supported effective sample rate.
const SAMPLE_RATE_MAX_HZ: u64 = 20_000_000;
/// Highest sample-rate divider supported by the firmware protocol.
const SAMPLE_RATE_DIVIDER_MAX: u32 = 31;
/// Search limit that preserves the existing fractional approximation.
const AUTOMATIC_MULTIPLIER_LIMIT: u32 = 32;
/// Number of fraction bits in an IEEE-754 binary64 value.
const F64_FRACTION_BITS: u32 = 52;
/// Bias applied to an IEEE-754 binary64 exponent.
const F64_EXPONENT_BIAS: i32 = 1023;

/// An RF frequency validated and serialized for `SetFreq`.
pub(super) struct PreparedRfFrequency {
    /// Little-endian MHz and residual-Hz protocol fields.
    pub(super) payload: [u8; 8],
}

/// A validated sample-rate payload and its derived filter setting.
pub(super) struct PreparedSampleRate {
    /// Little-endian numerator and divider protocol fields.
    pub(super) payload: [u8; 8],
    /// An official MAX2837 setting derived from original host values.
    pub(super) baseband_filter_hz: u32,
}

/// Validates the ordinary RF domain before serializing protocol fields.
pub(super) fn prepare_rf_frequency(
    frequency_hz: u64,
) -> Result<PreparedRfFrequency, Error> {
    if !(RF_FREQUENCY_MIN_HZ..=RF_FREQUENCY_MAX_HZ).contains(&frequency_hz) {
        return Err(Error::Argument);
    }

    let frequency_mhz =
        u32::try_from(frequency_hz / MHZ).map_err(|_| Error::Argument)?;
    let residual_hz =
        u32::try_from(frequency_hz % MHZ).map_err(|_| Error::Argument)?;

    Ok(PreparedRfFrequency {
        payload: encode_u32_pair(frequency_mhz, residual_hz),
    })
}

/// Validates the exact rational rate before deriving and serializing values.
pub(super) fn prepare_sample_rate_manual(
    frequency_hz: u32, divider: u32,
) -> Result<PreparedSampleRate, Error> {
    if !(1..=SAMPLE_RATE_DIVIDER_MAX).contains(&divider) {
        return Err(Error::Argument);
    }

    let frequency_hz_wide = u64::from(frequency_hz);
    let divider_wide = u64::from(divider);
    let minimum_numerator = SAMPLE_RATE_MIN_HZ
        .checked_mul(divider_wide)
        .ok_or(Error::Argument)?;
    let maximum_numerator = SAMPLE_RATE_MAX_HZ
        .checked_mul(divider_wide)
        .ok_or(Error::Argument)?;
    if !(minimum_numerator..=maximum_numerator).contains(&frequency_hz_wide) {
        return Err(Error::Argument);
    }

    let requested_filter_hz =
        requested_baseband_filter_hz(frequency_hz, divider)?;
    let baseband_filter_hz = select_baseband_filter_hz(requested_filter_hz)?;

    Ok(PreparedSampleRate {
        payload: encode_u32_pair(frequency_hz, divider),
        baseband_filter_hz,
    })
}

/// Preserves valid automatic approximation behavior with checked operations.
pub(super) fn prepare_sample_rate_auto(
    frequency_hz: f64,
) -> Result<PreparedSampleRate, Error> {
    if !frequency_hz.is_finite()
        || !(SAMPLE_RATE_MIN_HZ as f64..=SAMPLE_RATE_MAX_HZ as f64)
            .contains(&frequency_hz)
    {
        return Err(Error::Argument);
    }

    // Preserve existing valid-input numerator/divider behavior.
    let fractional_frequency = 1.0 + frequency_hz.fract();
    let frequency_bits = frequency_hz.to_bits();
    let exponent_bits =
        u16::try_from((frequency_bits >> F64_FRACTION_BITS) & 0x7ff)
            .map_err(|_| Error::Argument)?;
    let exponent = i32::from(exponent_bits)
        .checked_sub(F64_EXPONENT_BIAS)
        .ok_or(Error::Argument)?;
    let mask_shift = exponent
        .checked_add(4)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(Error::Argument)?;

    let fraction_mask = 1u64
        .checked_shl(F64_FRACTION_BITS)
        .and_then(|value| value.checked_sub(1))
        .ok_or(Error::Argument)?;
    let lower_mask = 1u64
        .checked_shl(mask_shift)
        .and_then(|value| value.checked_sub(1))
        .ok_or(Error::Argument)?;
    let mask = fraction_mask & !lower_mask;
    let fraction_bits = fractional_frequency.to_bits() & fraction_mask;

    let mut accumulator = 0u64;
    let mut multiplier = 1u32;
    for candidate in 1..=AUTOMATIC_MULTIPLIER_LIMIT {
        multiplier = candidate;
        accumulator = accumulator
            .checked_add(fraction_bits)
            .ok_or(Error::Argument)?;
        if accumulator & mask == 0 || !accumulator & mask == 0 {
            break;
        }
    }
    if multiplier == AUTOMATIC_MULTIPLIER_LIMIT {
        multiplier = 1;
    }

    let scaled_frequency = frequency_hz * f64::from(multiplier);
    if !scaled_frequency.is_finite() {
        return Err(Error::Argument);
    }
    let rounded_frequency = scaled_frequency.round();
    let maximum_u32 = f64::from(u32::MAX);
    if !rounded_frequency.is_finite()
        || rounded_frequency < 0.0
        || rounded_frequency > maximum_u32
        || rounded_frequency.fract() != 0.0
    {
        return Err(Error::Argument);
    }

    // The checks above prove this conversion is finite, integral, and in range.
    let frequency_hz = rounded_frequency as u32;
    prepare_sample_rate_manual(frequency_hz, multiplier)
}

/// Computes `floor(3 * frequency_hz / (4 * divider))` on host values.
fn requested_baseband_filter_hz(
    frequency_hz: u32, divider: u32,
) -> Result<u32, Error> {
    let numerator = u64::from(frequency_hz)
        .checked_mul(3)
        .ok_or(Error::Argument)?;
    let denominator = u64::from(divider)
        .checked_mul(4)
        .filter(|value| *value != 0)
        .ok_or(Error::Argument)?;
    u32::try_from(numerator / denominator).map_err(|_| Error::Argument)
}

/// Selects an official setting without exceeding the request when possible.
fn select_baseband_filter_hz(requested_hz: u32) -> Result<u32, Error> {
    let mut settings =
        MAX2837.iter().copied().take_while(|setting| *setting != 0);
    let mut selected = settings.next().ok_or(Error::Argument)?;
    for setting in settings {
        if setting > requested_hz {
            break;
        }
        selected = setting;
    }
    Ok(selected)
}

/// Serializes host-order fields exactly once at the wire boundary.
fn encode_u32_pair(first: u32, second: u32) -> [u8; 8] {
    let mut payload = [0; 8];
    let (first_bytes, second_bytes) = payload.split_at_mut(4);
    first_bytes.copy_from_slice(&first.to_le_bytes());
    second_bytes.copy_from_slice(&second.to_le_bytes());
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_argument<T>(result: Result<T, Error>) {
        assert!(matches!(result, Err(Error::Argument)));
    }

    #[test]
    fn parameter_rf_rejects_unsupported_and_truncating_values() {
        let former_truncation = (u64::from(u32::MAX) + 1) * MHZ;
        for frequency_hz in [
            0,
            999_999,
            6_000_000_001,
            7_250_000_000,
            former_truncation,
            u64::MAX,
        ] {
            assert_argument(prepare_rf_frequency(frequency_hz));
        }
    }

    #[test]
    fn parameter_rf_serializes_literal_little_endian_fields() {
        let cases = [
            (1_000_000, [0x01, 0, 0, 0, 0, 0, 0, 0]),
            (1_575_420_000, [0x27, 0x06, 0, 0, 0xa0, 0x68, 0x06, 0]),
            (6_000_000_000, [0x70, 0x17, 0, 0, 0, 0, 0, 0]),
        ];

        for (frequency_hz, expected) in cases {
            match prepare_rf_frequency(frequency_hz) {
                Ok(prepared) => assert_eq!(prepared.payload, expected),
                Err(error) => panic!("valid RF frequency failed: {error}"),
            }
        }
    }

    #[test]
    fn parameter_manual_rejects_invalid_dividers_and_rational_bounds() {
        let cases = [
            (0, 1),
            (2_000_000, 0),
            (64_000_000, 32),
            (1_999_999, 1),
            (20_000_001, 1),
            (61_999_999, 31),
            (620_000_001, 31),
            (u32::MAX, 1),
            (u32::MAX, 31),
        ];

        for (frequency_hz, divider) in cases {
            assert_argument(prepare_sample_rate_manual(frequency_hz, divider));
        }
    }

    #[test]
    fn parameter_manual_accepts_exact_bounds_and_literal_payloads() {
        let cases = [
            (2_000_000, 1, [0x80, 0x84, 0x1e, 0, 1, 0, 0, 0]),
            (20_000_000, 1, [0, 0x2d, 0x31, 0x01, 1, 0, 0, 0]),
            (62_000_000, 31, [0x80, 0x0b, 0xb2, 0x03, 0x1f, 0, 0, 0]),
            (620_000_000, 31, [0, 0x73, 0xf4, 0x24, 0x1f, 0, 0, 0]),
            (5_200_001, 2, [0x81, 0x58, 0x4f, 0, 2, 0, 0, 0]),
        ];

        for (frequency_hz, divider, expected) in cases {
            match prepare_sample_rate_manual(frequency_hz, divider) {
                Ok(prepared) => assert_eq!(prepared.payload, expected),
                Err(error) => {
                    panic!("valid manual sample rate failed: {error}")
                }
            }
        }
    }

    #[test]
    fn parameter_auto_rejects_non_finite_and_out_of_range_values() {
        let below_minimum = f64::from_bits(2_000_000.0f64.to_bits() - 1);
        let above_maximum = f64::from_bits(20_000_000.0f64.to_bits() + 1);
        for frequency_hz in [
            f64::NEG_INFINITY,
            -2_600_000.0,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::NAN,
            f64::INFINITY,
            f64::MAX,
            below_minimum,
            above_maximum,
        ] {
            assert_argument(prepare_sample_rate_auto(frequency_hz));
        }
    }

    #[test]
    fn parameter_auto_preserves_integer_and_fractional_approximations() {
        let cases = [
            (2_000_000.0, [0x80, 0x84, 0x1e, 0, 1, 0, 0, 0]),
            (2_600_000.0, [0x40, 0xac, 0x27, 0, 1, 0, 0, 0]),
            (2_600_000.5, [0x81, 0x58, 0x4f, 0, 2, 0, 0, 0]),
            (20_000_000.0, [0, 0x2d, 0x31, 0x01, 1, 0, 0, 0]),
        ];

        for (frequency_hz, expected) in cases {
            match prepare_sample_rate_auto(frequency_hz) {
                Ok(prepared) => assert_eq!(prepared.payload, expected),
                Err(error) => {
                    panic!("valid automatic sample rate failed: {error}")
                }
            }
        }
    }

    #[test]
    fn parameter_filter_uses_exact_host_arithmetic_and_discrete_settings() {
        let cases = [
            (2_000_000, 1, 1_500_000, 1_750_000),
            (2_600_000, 1, 1_950_000, 1_750_000),
            (10_000_000, 1, 7_500_000, 7_000_000),
            (20_000_000, 1, 15_000_000, 15_000_000),
            (20_000_000, 2, 7_500_000, 7_000_000),
            (7_333_334, 1, 5_500_000, 5_500_000),
            (7_333_333, 1, 5_499_999, 5_000_000),
            (8_000_000, 1, 6_000_000, 6_000_000),
            (7_999_999, 1, 5_999_999, 5_500_000),
            (9_333_334, 1, 7_000_000, 7_000_000),
            (9_333_333, 1, 6_999_999, 6_000_000),
            (13_333_333, 1, 9_999_999, 9_000_000),
            (19_999_999, 1, 14_999_999, 14_000_000),
        ];

        for (frequency_hz, divider, expected_request, expected_setting) in cases
        {
            let requested = requested_baseband_filter_hz(frequency_hz, divider);
            match requested {
                Ok(requested) => {
                    assert_eq!(requested, expected_request);
                    match select_baseband_filter_hz(requested) {
                        Ok(selected) => {
                            assert_eq!(selected, expected_setting);
                            assert_ne!(selected, 0);
                            assert!(MAX2837.contains(&selected));
                        }
                        Err(error) => {
                            panic!("filter selection failed: {error}")
                        }
                    }
                }
                Err(error) => panic!("filter arithmetic failed: {error}"),
            }
        }
    }

    #[test]
    fn parameter_manual_keeps_host_arithmetic_separate_from_wire_bytes() {
        match prepare_sample_rate_manual(10_000_000, 1) {
            Ok(prepared) => {
                assert_eq!(prepared.payload, [0x80, 0x96, 0x98, 0, 1, 0, 0, 0]);
                assert_eq!(prepared.baseband_filter_hz, 7_000_000);
            }
            Err(error) => panic!("valid manual sample rate failed: {error}"),
        }
    }
}
