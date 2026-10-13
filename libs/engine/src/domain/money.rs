//! Money: amounts parsed once from text, and balances changed only by checked arithmetic.

use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

/// A positive amount of money, exact at four decimal places and below 10^15. Its only source is
/// text of the form `digits[.digits]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(Decimal);

/// An amount refusal: why text is not an amount. The variants are checked in order, and the first
/// that applies names the refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum AmountError {
    /// Not digits with an optional fraction: empty, signed, an exponent, a separator, padding, a
    /// non-ASCII digit, or a point without digits on both sides.
    #[error("not digits with an optional fraction")]
    Malformed,
    /// More than four decimal places once trailing zeros are dropped.
    #[error("more than four decimal places")]
    TooPrecise,
    /// 10^15 or more.
    #[error("10^15 or more")]
    TooLarge,
    /// Zero.
    #[error("zero")]
    Zero,
}

/// Decimal places an amount may carry, and that every amount and balance prints.
const PLACES: usize = 4;
/// Whole-part digits an amount may have: amounts stay below 10^15.
const WHOLE_DIGITS: usize = 15;

impl FromStr for Amount {
    type Err = AmountError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = text.split_once('.').unwrap_or((text, "0"));
        if !is_digits(whole) || !is_digits(fraction) {
            return Err(AmountError::Malformed);
        }
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        if fraction.len() > PLACES {
            return Err(AmountError::TooPrecise);
        }
        if whole.len() > WHOLE_DIGITS {
            return Err(AmountError::TooLarge);
        }
        let mantissa = whole
            .chars()
            .chain(fraction.chars())
            .try_fold(0_u64, |value, digit| {
                value.checked_mul(10)?.checked_add(u64::from(digit.to_digit(10)?))
            })
            .ok_or(AmountError::TooLarge)?;
        if mantissa == 0 {
            return Err(AmountError::Zero);
        }
        let scale = u32::try_from(fraction.len()).map_err(|_| AmountError::TooPrecise)?;
        Decimal::try_from_i128_with_scale(i128::from(mantissa), scale)
            .map(Self)
            .map_err(|_| AmountError::TooLarge)
    }
}

/// Whether `text` is one or more ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Exactly four decimal places.
impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.*}", PLACES, self.0)
    }
}

/// A signed amount of money: an account's available, held or total funds. Exact at four decimal
/// places, since its arithmetic refuses a result past the largest balance, either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Balance(Decimal);

/// The largest balance, and the size of the most negative one: (2^96 − 1) × 10^−4, the most a
/// `Decimal` holds at four decimal places.
const LARGEST: Decimal = Decimal::from_parts(u32::MAX, u32::MAX, u32::MAX, false, 4);

impl Balance {
    /// No money.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// The sum, or `None` when it lies past the largest balance, either way.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::bounded)
    }

    /// The difference, or `None` when it lies past the largest balance, either way.
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).and_then(Self::bounded)
    }

    /// `value` as a balance, or `None` when it lies past the largest balance, either way.
    fn bounded(value: Decimal) -> Option<Self> {
        (value.abs() <= LARGEST).then_some(Self(value))
    }
}

impl From<Amount> for Balance {
    fn from(amount: Amount) -> Self {
        Self(amount.0)
    }
}

/// Exactly four decimal places, and zero as `0.0000`.
impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.*}", PLACES, self.0)
    }
}
