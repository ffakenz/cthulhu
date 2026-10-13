//! Laws of the domain's money: any spelling of an amount reads as its value, text an amount refuses
//! says why, and moves then their sum back print zero, from above or below, at any scales. Every
//! balance that doublings, credits and debits reach prints with four decimals and adds up the same
//! in any grouping, and a balance holds 2^32 of any amount. The largest balance prints, and one unit
//! past it, either way, is refused.

use std::iter;

use engine::domain::{Amount, AmountError, Balance};
use proptest::collection::vec;
use proptest::prop_oneof;
use proptest::sample::select;
use proptest::strategy::{Just, Strategy};

use crate::support::{self, input};

#[test]
fn prop_any_spelling_of_an_amount_reads_as_its_value() {
    let law = support::for_all(
        "any spelling of an amount parses, and its value prints as a balance with four decimals",
        (input::any_money(), 0..=2_usize, 0..=2_usize),
        |&(money, leading, trailing)| {
            money
                .spelled(leading, trailing)
                .parse::<Amount>()
                .map(|amount| Balance::from(amount).to_string())
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

/// One step a balance takes: doubled, or `units` of 0.0001 moved one way.
#[derive(Debug, Clone, Copy)]
enum Step {
    Double,
    Move { units: u64, way: Move },
}

/// Any step, three in four a doubling, so that balances reach the largest a balance can be.
fn any_step() -> impl Strategy<Value = Step> {
    let any_move = (any_amount(), select(&[Move::Credit, Move::Debit]))
        .prop_map(|(units, way)| Step::Move { units, way });
    prop_oneof![3 => Just(Step::Double), 1 => any_move]
}

/// The balances `steps` reach from `start` units of 0.0001, the start first, up to the first step
/// refused.
fn reached(start: u64, steps: &[Step]) -> Vec<Balance> {
    let Some(first) = balance(start) else {
        return Vec::new();
    };
    let next = steps.iter().scan(first, |now, &step| {
        *now = match step {
            Step::Double => now.checked_add(*now),
            Step::Move { units, way } => moved(*now, units, way),
        }?;
        Some(*now)
    });
    iter::once(first).chain(next).collect()
}

/// Whether `text` writes a balance with four decimals: an optional minus, digits, a point and four
/// digits.
fn four_decimals(text: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    text.strip_prefix('-')
        .unwrap_or(text)
        .split_once('.')
        .is_some_and(|(whole, fraction)| digits(whole) && digits(fraction) && fraction.len() == 4)
}

#[test]
fn prop_every_balance_prints_with_four_decimals() {
    let law = support::for_all(
        "every balance that doublings, credits and debits reach prints with four decimals",
        (any_amount(), vec(any_step(), 0..=64)),
        |(units, steps)| {
            reached(*units, steps).iter().all(|balance| four_decimals(&balance.to_string()))
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// Any part of an amount, as units of 0.0001: at most half the largest, so that two parts sum to an
/// amount.
fn any_part() -> impl Strategy<Value = u64> {
    any_amount().prop_map(|units| (units / 2).max(1))
}

#[test]
fn prop_moves_add_up_the_same_in_any_grouping() {
    let law = support::for_all(
        "a balance moved by two amounts in turn equals it moved by their sum, wherever both moves \
         are accepted",
        (
            any_amount(),
            vec(any_step(), 0..=64),
            any_part(),
            any_part(),
            select(&[Move::Credit, Move::Debit]),
        ),
        |(units, steps, first, second, way)| {
            reached(*units, steps).last().is_none_or(|&now| {
                let in_turn =
                    moved(now, *first, *way).and_then(|after| moved(after, *second, *way));
                let at_once = moved(now, first + second, *way);
                in_turn.zip(at_once).is_none_or(|(left, right)| left == right)
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// The largest balance as it prints: (2^96 − 1) units of 0.0001.
const LARGEST: &str = "7922816251426433759354395.0335";

/// The largest balance, built through the public API: (2^62 − 1) units doubled 34 times, plus
/// 2^34 − 1 units.
fn largest() -> Option<Balance> {
    let start = balance(4_611_686_018_427_387_903)?;
    let doubled = (0..34).try_fold(start, |sum, _| sum.checked_add(sum))?;
    doubled.checked_add(balance(17_179_869_183)?)
}

#[test]
fn the_largest_balance_prints_and_one_unit_past_it_is_refused() {
    let unit = balance(1).expect("0.0001 is an amount");
    let largest = largest().expect("the largest balance is a balance");
    let most_negative = Balance::ZERO.checked_sub(largest).expect("its negation is a balance");
    assert_eq!(largest.to_string(), LARGEST);
    assert_eq!(most_negative.to_string(), format!("-{LARGEST}"));
    assert_eq!(largest.checked_add(unit), None);
    assert_eq!(most_negative.checked_sub(unit), None);
}
