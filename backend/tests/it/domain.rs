//! Laws of the domain's money: amounts parse exactly as the grammar admits them, and balances
//! print their exact value.

use backend::domain::{Amount, AmountError, Balance};

use crate::support;
use crate::support::generate::{self, AmountValue, Direction};
use crate::support::model::{self, Units};

/// The smallest whole part an amount cannot have: 10^15.
const TOO_LARGE: u64 = 1_000_000_000_000_000;

#[test]
fn prop_amount_text_parses_back_to_its_value() {
    let law = support::for_all(
        "an amount's four-decimal text parses back to it; zero and 10^15 up are refused",
        500,
        generate::amount_value,
        |&AmountValue { whole, fraction }| {
            let text = format!("{whole}.{fraction:04}");
            let refusal = if whole >= TOO_LARGE {
                Some(AmountError::TooLarge)
            } else if whole == 0 && fraction == 0 {
                Some(AmountError::Zero)
            } else {
                None
            };
            match text.parse::<Amount>() {
                Ok(amount) => {
                    refusal.is_none()
                        && amount.to_string() == text
                        && amount.to_string().parse() == Ok(amount)
                }
                Err(error) => refusal == Some(error),
            }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_amount_spelling_parses_exactly_when_the_grammar_admits_it() {
    let law = support::for_all(
        "a spelling parses exactly when the grammar admits it, and prints as its four-decimal value",
        500,
        generate::amount_spelling,
        |text| {
            text.parse::<Amount>().map(|amount| amount.to_string())
                == model::amount_units(text).map(|units| Units::from(units).to_string())
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_amount_parsing_never_panics() {
    let law = support::for_all(
        "parsing any text answers, the same each time, and never panics",
        500,
        generate::any_text,
        |text| text.parse::<Amount>() == text.parse::<Amount>(),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_balance_never_prints_negative_zero() {
    let law = support::for_all(
        "a balance prints its exact value with four decimals, and zero as 0.0000, never -0.0000",
        500,
        generate::balance_moves,
        |moves| {
            moves
                .iter()
                .try_fold((Balance::ZERO, Units::ZERO), |(balance, units), (direction, text)| {
                    let amount = Balance::from(text.parse::<Amount>().ok()?);
                    let moved = Units::from(model::amount_units(text).ok()?);
                    let (next_balance, next_units) = match direction {
                        Direction::Credit => (balance.checked_add(amount)?, units.plus(moved)),
                        Direction::Debit => (balance.checked_sub(amount)?, units.minus(moved)),
                    };
                    (next_balance.to_string() == next_units.to_string())
                        .then_some((next_balance, next_units))
                })
                .is_some()
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
