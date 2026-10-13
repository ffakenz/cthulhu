//! Laws of the domain's money: any spelling of an amount reads as its value, text an amount refuses
//! says why, moves then their sum back print zero, from above or below, at any scales, and a
//! balance holds 2^32 of any amount.

use engine::domain::{Amount, AmountError, Balance};
use proptest::collection::vec;
use proptest::prop_oneof;
use proptest::sample::select;
use proptest::strategy::Strategy;

use crate::support::{self, input};

#[test]
fn prop_any_spelling_of_an_amount_reads_as_its_value() {
    let law = support::for_all(
        "any spelling of an amount parses, and prints its value with four decimals",
        (input::any_money(), 0..=2_usize, 0..=2_usize),
        |&(money, leading, trailing)| {
            money.spelled(leading, trailing).parse::<Amount>().map(|amount| amount.to_string())
                == Ok(money.text())
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// Text outside the grammar that no CSV field carries as it is: empty, padded, or with a comma.
const UNFIELDED: [&str; 4] = ["", " 1", "1 ", "1,5"];

#[test]
fn prop_text_an_amount_refuses_says_why() {
    let any_refused = prop_oneof![
        input::any_refused_amount(),
        select(&UNFIELDED).prop_map(|text| (text.to_owned(), AmountError::Malformed)),
    ];
    let law = support::for_all(
        "text outside the grammar or the limits is refused, with the reason",
        any_refused,
        |(text, why)| text.parse::<Amount>() == Err(*why),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// `units` of 0.0001 as a balance, spelled `{whole}.{fraction:04}` and parsed as an amount.
fn balance(units: u64) -> Option<Balance> {
    format!("{}.{:04}", units / 10_000, units % 10_000).parse::<Amount>().ok().map(Balance::from)
}

/// Which way money moves: a credit adds to a balance, a debit takes from it.
#[derive(Debug, Clone, Copy)]
enum Move {
    Credit,
    Debit,
}

impl Move {
    /// The move the other way.
    fn back(self) -> Self {
        match self {
            Self::Credit => Self::Debit,
            Self::Debit => Self::Credit,
        }
    }
}

/// `before` with `units` moved `way`; `None` when it overflows or the amount does not parse.
fn moved(before: Balance, units: u64, way: Move) -> Option<Balance> {
    let amount = balance(units)?;
    match way {
        Move::Credit => before.checked_add(amount),
        Move::Debit => before.checked_sub(amount),
    }
}

/// The balance after moving each of `units` `way`, then their sum back; `None` when the sum
/// overflows or an amount does not parse.
fn there_and_back(units: &[u64], way: Move) -> Option<Balance> {
    let total = units.iter().try_fold(0_u64, |sum, &unit| sum.checked_add(unit))?;
    let away = units.iter().try_fold(Balance::ZERO, |sum, &unit| moved(sum, unit, way))?;
    moved(away, total, way.back())
}

#[test]
fn prop_moves_then_their_sum_back_print_zero() {
    let any_units =
        (1..=9_999_u64, 0..=4_u32).prop_map(|(digits, zeros)| digits * 10_u64.pow(zeros));
    let law = support::for_all(
        "moves one way, then their sum the other way, leave a balance that prints 0.0000, from \
         above or below, at any scales",
        (vec(any_units, 2..=4), select(&[Move::Credit, Move::Debit])),
        |(units, way)| there_and_back(units, *way).is_some_and(|zero| zero.to_string() == "0.0000"),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// The units of 0.0001 every amount stays below: 10^19, for a whole part below 10^15.
const CAP: u64 = 10_000_000_000_000_000_000;

/// Any amount, as units of 0.0001, one draw in four at an edge: the smallest, one, or the largest.
fn any_amount() -> impl Strategy<Value = u64> {
    prop_oneof![1 => select(&[1, 10_000, CAP - 1]), 3 => 1..CAP]
}

#[test]
fn prop_any_amount_doubled_32_times_is_a_balance() {
    let law = support::for_all(
        "any amount doubled 32 times is a balance, as 2^32 deposits of it sum to one",
        any_amount(),
        |&units| {
            balance(units)
                .and_then(|start| (0..32).try_fold(start, |sum, _| sum.checked_add(sum)))
                .is_some()
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
