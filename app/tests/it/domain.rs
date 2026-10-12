//! Laws of the domain's money: any spelling of an amount reads as its value, text an amount refuses
//! says why, and credits then a debit of their sum print zero at any scales.

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

/// The balance after crediting each of `units`, then debiting their sum; `None` when the sum
/// overflows or an amount does not parse.
fn credited_then_debited(units: &[u64]) -> Option<Balance> {
    let total = units.iter().try_fold(0_u64, |sum, &unit| sum.checked_add(unit))?;
    let credited =
        units.iter().try_fold(Balance::ZERO, |sum, &unit| sum.checked_add(balance(unit)?))?;
    credited.checked_sub(balance(total)?)
}

#[test]
fn prop_credits_then_a_debit_of_their_sum_print_zero() {
    let any_units =
        (1..=9_999_u64, 0..=4_u32).prop_map(|(digits, zeros)| digits * 10_u64.pow(zeros));
    let law = support::for_all(
        "credits then one debit of their sum leave a balance that prints 0.0000, at any scales",
        vec(any_units, 2..=4),
        |units| credited_then_debited(units).is_some_and(|zero| zero.to_string() == "0.0000"),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
