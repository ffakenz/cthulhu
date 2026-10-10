//! The model the laws compare against: the amount grammar, read apart from the production parser,
//! and the model's money.

use std::fmt;
use std::num::NonZeroU64;

use backend::domain::AmountError;

/// What the amount grammar makes of `text`: its value in units of 0.0001, never zero, or why it
/// is refused.
pub(crate) fn amount_units(text: &str) -> Result<NonZeroU64, AmountError> {
    let mut whole = String::new();
    let mut fraction = String::new();
    let mut point = false;
    for character in text.chars() {
        match character {
            '0'..='9' if point => fraction.push(character),
            '0'..='9' => whole.push(character),
            '.' if !point => point = true,
            _ => return Err(AmountError::Malformed),
        }
    }
    if whole.is_empty() || (point && fraction.is_empty()) {
        return Err(AmountError::Malformed);
    }
    let significant_whole = whole.trim_start_matches('0');
    let significant_fraction = fraction.trim_end_matches('0');
    if significant_fraction.len() > 4 {
        return Err(AmountError::TooPrecise);
    }
    if significant_whole.len() > 15 {
        return Err(AmountError::TooLarge);
    }
    let units = format!("{significant_whole}{significant_fraction:0<4}")
        .parse::<u64>()
        .map_err(|_| AmountError::TooLarge)?;
    NonZeroU64::new(units).ok_or(AmountError::Zero)
}

/// An amount of money in units of 0.0001, signed: the model's money.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Units(i128);

impl Units {
    pub(crate) const ZERO: Self = Self(0);

    /// The sum, saturating at the bounds of `i128`, far past any amount the cap admits.
    pub(crate) fn plus(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// The difference, saturating likewise.
    pub(crate) fn minus(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl From<NonZeroU64> for Units {
    fn from(units: NonZeroU64) -> Self {
        Self(i128::from(units.get()))
    }
}

/// As a balance prints: a sign when negative, then exactly four decimals.
impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:04}", magnitude / 10_000, magnitude % 10_000)
    }
}
