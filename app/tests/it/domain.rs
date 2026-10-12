//! Laws of the domain's money: an amount's text parses back to it, a spelling outside the grammar
//! is malformed, and credits then a debit of their sum print zero at any scales.

use engine::domain::{Amount, AmountError, Balance};
use proptest::arbitrary::any;
use proptest::collection::vec;
use proptest::prop_oneof;
use proptest::sample::select;
use proptest::strategy::Strategy;

use crate::support;

/// The smallest whole part an amount cannot have: 10^15.
const TOO_LARGE: u64 = 1_000_000_000_000_000;

/// Whole parts at the edges: zero, one, the largest an amount may have, and the smallest it may
/// not.
const EDGE_WHOLES: [u64; 4] = [0, 1, TOO_LARGE - 1, TOO_LARGE];

#[test]
fn prop_amount_text_parses_back_to_its_value() {
    let wholes = prop_oneof![
        1 => select(&EDGE_WHOLES),
        3 => (any::<u64>(), 0..u64::BITS).prop_map(|(bits, shift)| bits >> shift),
    ];
    let law = support::for_all(
        "an amount's four-decimal text, padded or not, parses and prints back; only zero and 10^15 \
         up are refused",
        (wholes, 0..=9_999_u64),
        |&(whole, fraction)| {
            let text = format!("{whole}.{fraction:04}");
            let parsed = text.parse::<Amount>();
            let padded = format!("0{text}0").parse::<Amount>() == parsed;
            padded
                && match parsed {
                    Ok(amount) => {
                        whole < TOO_LARGE
                            && (whole, fraction) != (0, 0)
                            && amount.to_string() == text
                    }
                    Err(AmountError::Zero) => (whole, fraction) == (0, 0),
                    Err(AmountError::TooLarge) => whole >= TOO_LARGE,
                    Err(AmountError::Malformed | AmountError::TooPrecise) => false,
                }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// Spellings outside the grammar: empty, signed, padded, an exponent, a point without digits on
/// one side, two points, a separator, and digits that are not ASCII.
const MALFORMED: [&str; 12] =
    ["", "+1", "-1", " 1", "1 ", "1e2", ".5", "5.", "1.2.3", "1,5", "\u{663}", "1.\u{ff11}"];

#[test]
fn a_spelling_outside_the_grammar_is_malformed() {
    for text in MALFORMED {
        assert_eq!(text.parse::<Amount>(), Err(AmountError::Malformed), "{text:?}");
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
