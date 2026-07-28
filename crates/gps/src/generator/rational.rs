use crate::Error;

/// Largest denominator retained while recovering a configured decimal ratio.
const MAX_RATIONAL_DENOMINATOR: u128 = 1_000_000_000;

/// Positive reduced rational used for exact sample-boundary calculations.
#[derive(Clone, Copy, Debug)]
pub(super) struct Rational {
    /// Reduced numerator.
    pub(super) numerator: u128,
    /// Reduced denominator.
    pub(super) denominator: u128,
}

impl Rational {
    /// Recovers a bounded rational representation of a positive finite value.
    pub(super) fn from_positive_f64(
        value: f64, name: &str,
    ) -> Result<Self, Error> {
        if !value.is_finite() || value <= 0.0 {
            return Err(Error::msg(format!(
                "{name} must be finite and greater than zero"
            )));
        }

        let mut remainder = value;
        let mut previous_numerator = 0u128;
        let mut numerator = 1u128;
        let mut previous_denominator = 1u128;
        let mut denominator = 0u128;

        loop {
            let whole = remainder.floor() as u128;
            let Some(next_numerator) = whole
                .checked_mul(numerator)
                .and_then(|term| term.checked_add(previous_numerator))
            else {
                break;
            };
            let Some(next_denominator) = whole
                .checked_mul(denominator)
                .and_then(|term| term.checked_add(previous_denominator))
            else {
                break;
            };
            if next_denominator > MAX_RATIONAL_DENOMINATOR {
                break;
            }

            previous_numerator = numerator;
            numerator = next_numerator;
            previous_denominator = denominator;
            denominator = next_denominator;

            let approximation = numerator as f64 / denominator as f64;
            let tolerance = f64::EPSILON * value.abs().max(1.0) * 8.0;
            if (approximation - value).abs() <= tolerance {
                break;
            }

            let fractional = remainder - whole as f64;
            if fractional <= f64::EPSILON {
                break;
            }
            remainder = fractional.recip();
        }

        if denominator == 0 {
            return Err(Error::unsupported_workload(format!(
                "could not represent {name} as a rational value"
            )));
        }

        let divisor = greatest_common_divisor(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Multiplies and rounds to the nearest whole unit.
    pub(super) fn rounded_product(
        self, multiplier: u128,
    ) -> Result<u64, Error> {
        let scaled =
            self.numerator.checked_mul(multiplier).ok_or_else(|| {
                Error::unsupported_workload(
                    "sample timeline multiplication overflow",
                )
            })?;
        let rounded =
            scaled.checked_add(self.denominator / 2).ok_or_else(|| {
                Error::unsupported_workload("sample timeline rounding overflow")
            })? / self.denominator;
        u64::try_from(rounded).map_err(|_| {
            Error::unsupported_workload("sample count exceeds supported range")
        })
    }

    /// Multiplies and rounds upward to a whole unit.
    pub(super) fn ceiling_product(
        self, multiplier: u128,
    ) -> Result<u64, Error> {
        let scaled =
            self.numerator.checked_mul(multiplier).ok_or_else(|| {
                Error::unsupported_workload(
                    "sample timeline multiplication overflow",
                )
            })?;
        let ceiling = divide_ceiling(scaled, self.denominator)?;
        u64::try_from(ceiling).map_err(|_| {
            Error::unsupported_workload("sample count exceeds supported range")
        })
    }
}

/// Returns the greatest common divisor for ratio reduction.
fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

/// Divides positive integers while rounding upward.
pub(super) fn divide_ceiling(
    numerator: u128, denominator: u128,
) -> Result<u128, Error> {
    numerator
        .checked_add(denominator.saturating_sub(1))
        .map(|value| value / denominator)
        .ok_or_else(|| {
            Error::unsupported_workload("sample timeline ceiling overflow")
        })
}
