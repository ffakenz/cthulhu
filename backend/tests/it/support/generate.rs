//! Generators: raw input as a source would deliver it, one draw in four at an edge.

use std::num::NonZeroU64;

use super::{EDGE_ODDS, Gen};

/// Amount spellings at the edges of the grammar: empty, zero, a bare or trailing point, a sign, an
/// exponent, a separator, padding and a non-ASCII digit; then amounts that parse or just miss:
/// the smallest, trailing and leading zeros, five decimals, the largest and one past it, and past
/// `u64`.
pub(crate) const AMOUNT_EDGES: [&str; 27] = [
    "",
    "0",
    "0.0000",
    ".",
    "1.",
    ".5",
    "+1",
    "-1",
    "1e3",
    "1_000",
    " 1",
    "1 ",
    "\u{663}",
    "0.0001",
    "1",
    "1.5",
    "1.50000",
    "1.23456",
    "0001.5000",
    "1.00001",
    "999999999999999.9999",
    "999999999999999.99990",
    "1000000000000000",
    "0999999999999999",
    "00000000000000000000000000000000000000001",
    "1.000000000000000000000000000000",
    "18446744073709551616",
];

/// One draw in two.
const EVEN_ODDS: NonZeroU64 = NonZeroU64::MIN.saturating_add(1);

/// The ASCII digits.
const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// What an amount's text may hold by mistake: digits, the characters number parsers treat
/// specially, and an Arabic-Indic three.
const AMOUNT_CHARACTERS: [char; 18] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '-', '+', 'e', ' ', '_', ',', '\u{663}',
];

/// Raw amount text: one draw in four an edge; otherwise, as often each, a number-shaped spelling
/// (up to seventeen whole digits, leading zeros included, then an optional point and up to six
/// fraction digits) or up to eight characters of `AMOUNT_CHARACTERS`.
pub(crate) fn amount_spelling(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&AMOUNT_EDGES).copied().unwrap_or_default().to_owned();
    }
    if random.one_in(EVEN_ODDS) {
        let whole: String = random.up_to(17, &DIGITS).into_iter().collect();
        if random.one_in(EVEN_ODDS) {
            let fraction: String = random.up_to(6, &DIGITS).into_iter().collect();
            return format!("{whole}.{fraction}");
        }
        return whole;
    }
    random.up_to(8, &AMOUNT_CHARACTERS).into_iter().collect()
}

/// Shifts a draw may take: every bit width of a `u64`.
const BITS: NonZeroU64 = NonZeroU64::MIN.saturating_add(63);

/// Fractions in units of 0.0001: `0..10_000`.
const FRACTIONS: NonZeroU64 = NonZeroU64::MIN.saturating_add(9_999);

/// An amount's value as a whole part and a fraction.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AmountValue {
    /// The whole part.
    pub(crate) whole: u64,
    /// The fraction, in units of 0.0001: below `10_000`.
    pub(crate) fraction: u64,
}

/// Values at the edges: zero, the smallest amount, one, the largest fraction, the largest amount,
/// 10^15, and the largest `u64`.
const AMOUNT_VALUE_EDGES: [AmountValue; 7] = [
    AmountValue { whole: 0, fraction: 0 },
    AmountValue { whole: 0, fraction: 1 },
    AmountValue { whole: 1, fraction: 0 },
    AmountValue { whole: 0, fraction: 9_999 },
    AmountValue { whole: 999_999_999_999_999, fraction: 9_999 },
    AmountValue { whole: 1_000_000_000_000_000, fraction: 0 },
    AmountValue { whole: u64::MAX, fraction: 9_999 },
];

/// An amount's value: one draw in four an edge, otherwise a whole part of random magnitude.
pub(crate) fn amount_value(random: &mut Gen) -> AmountValue {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&AMOUNT_VALUE_EDGES).copied().unwrap_or_default();
    }
    let whole = random.next_u64() >> random.below(BITS);
    AmountValue { whole, fraction: random.below(FRACTIONS) }
}

/// Characters for totality: ASCII digits, punctuation and letters, and multi-byte characters (an
/// Arabic-Indic three, a full-width one, an accented letter and a byte-order mark).
const ANY_CHARACTERS: [char; 18] = [
    '0', '1', '5', '9', '.', ',', '-', '+', '_', ' ', 'e', 'E', 'x', 'a', '\u{663}', '\u{ff11}',
    '\u{e9}', '\u{feff}',
];

/// Any text up to sixteen characters: one draw in four an edge (empty, sixty-four zeros then a
/// one, a multi-byte character after a point).
pub(crate) fn any_text(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        let edges = [String::new(), format!("{}1", "0".repeat(64)), "1.\u{ff11}".to_owned()];
        return random.pick(&edges).cloned().unwrap_or_default();
    }
    random.up_to(16, &ANY_CHARACTERS).into_iter().collect()
}

/// Which way a move takes a balance.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Direction {
    /// The amount is added.
    Credit,
    /// The amount is subtracted.
    Debit,
}

impl Direction {
    /// The other way.
    fn opposite(self) -> Self {
        match self {
            Self::Credit => Self::Debit,
            Self::Debit => Self::Credit,
        }
    }
}

/// Lengths of a sequence of moves: `0..=12`.
const MOVES: NonZeroU64 = NonZeroU64::MIN.saturating_add(12);

/// Amounts a move takes at its edges: the smallest and the largest.
const MOVE_EDGES: [&str; 2] = ["0.0001", "999999999999999.9999"];

/// How many amounts lie above zero and below 1000, in units of 0.0001.
const MOVE_UNITS: NonZeroU64 = NonZeroU64::MIN.saturating_add(9_999_998);

/// The units of 0.0001 in 10^15, the smallest value too large to be an amount.
const TOO_LARGE_UNITS: u64 = 10_000_000_000_000_000_000;

/// Up to twelve moves of a balance, each a direction and the text of a valid amount: one draw in
/// four an edge amount, otherwise `{whole}.{fraction:04}` below 1000 and above zero. One draw in
/// four instead repeats the previous amount the other way, so a balance lands on zero from either
/// side; and, while the balance's magnitude is an amount, one draw in four of the rest moves the
/// whole balance back to zero, spelled without trailing zeros, so a balance whose last decimal is
/// zero also lands on zero through an amount spelled with fewer decimals.
pub(crate) fn balance_moves(random: &mut Gen) -> Vec<(Direction, String)> {
    let count = random.below(MOVES);
    let mut moves: Vec<(Direction, String)> = Vec::new();
    for _ in 0..count {
        let repeat = moves
            .last()
            .filter(|_| random.one_in(EDGE_ODDS))
            .map(|(direction, text)| (direction.opposite(), text.clone()));
        let next = repeat
            .or_else(|| {
                moves
                    .iter()
                    .try_fold(0, moved)
                    .and_then(zeroing)
                    .filter(|_| random.one_in(EDGE_ODDS))
            })
            .unwrap_or_else(|| (direction(random), move_amount(random)));
        moves.push(next);
    }
    moves
}

/// `balance`, in units of 0.0001, after the move `(direction, text)`, its amount read through the
/// grammar model; `None` if the model refuses the amount.
fn moved(balance: i128, (direction, text): &(Direction, String)) -> Option<i128> {
    let units = i128::from(super::model::amount_units(text).ok()?.get());
    Some(match direction {
        Direction::Credit => balance.saturating_add(units),
        Direction::Debit => balance.saturating_sub(units),
    })
}

/// The move that takes `balance`, in units of 0.0001, back to zero, its amount spelled without
/// trailing zeros; `None` when the balance is zero or its magnitude is no amount.
fn zeroing(balance: i128) -> Option<(Direction, String)> {
    let magnitude = u64::try_from(balance.unsigned_abs())
        .ok()
        .filter(|units| (1..TOO_LARGE_UNITS).contains(units))?;
    let direction = if balance > 0 { Direction::Debit } else { Direction::Credit };
    Some((direction, unpadded(magnitude)))
}

/// `units` of 0.0001 spelled without trailing zeros: `5` for 50000, `5.12` for 51200.
fn unpadded(units: u64) -> String {
    let (whole, fraction) = (units / 10_000, units % 10_000);
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:04}").trim_end_matches('0').to_owned()
    }
}

/// A credit or a debit, as often each.
fn direction(random: &mut Gen) -> Direction {
    if random.one_in(EVEN_ODDS) { Direction::Credit } else { Direction::Debit }
}

/// The text of a valid amount: one draw in four an edge, otherwise above zero and below 1000.
fn move_amount(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&MOVE_EDGES).copied().unwrap_or_default().to_owned();
    }
    let units = random.below(MOVE_UNITS).saturating_add(1);
    format!("{}.{:04}", units / 10_000, units % 10_000)
}
