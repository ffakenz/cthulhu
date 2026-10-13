//! The runtime through the CSV source, over fresh in-memory stores. On valid lines, clients run
//! independently, suffixes the rules make equivalent leave the same accounts after any lines, and
//! any spelling reads as the canonical one. A malformed line changes nothing but its count, and a
//! header lacking a column or a source error stops the run.

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::iter;

use app::csv_io::CsvSource;
use engine::run::{Row, RowError, RunError, SourceError, Summary, run};
use proptest::collection::vec;
use proptest::strategy::Strategy;
use store::memory::{MemoryLedger, MemorySeenIds};

use crate::support::{self, input};

/// Up to sixteen valid lines, by clients 1 to 3 on tx ids below 8, so that their ids collide.
fn lines() -> impl Strategy<Value = Vec<input::Line>> {
    vec(input::any_line(1..=3_u16, 0..8_u32), 0..=16)
}

#[test]
fn prop_clients_run_independently() {
    let law = support::for_all(
        "lines of different clients give the same summary in any order, given distinct tx ids",
        lines(),
        |drawn| {
            let distinct: Vec<input::Line> = drawn
                .iter()
                .zip(0_u32..)
                .map(|(&line, position)| {
                    if input::moves_money(line.kind) {
                        input::Line { tx: position, ..line }
                    } else {
                        line
                    }
                })
                .collect();
            let mut by_client = distinct.clone();
            by_client.sort_by_key(|line| line.client);
            support::summary(input::canonical(&distinct).as_bytes())
                == support::summary(input::canonical(&by_client).as_bytes())
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// Pairs of suffixes that leave the same accounts after any lines, each under the rule it pins:
/// `c` is the drawn client, `o` another and `F` one no line names; `a` and `b` are the drawn
/// amounts; `X`, `Y` and `Z` are tx ids no line names.
fn pairs(c: u16, a: &str, b: &str) -> [(String, String); 15] {
    const F: u16 = 9;
    const X: u32 = 100;
    const Y: u32 = 101;
    const Z: u32 = 102;
    let o = c % 3 + 1;
    let deposit = format!("deposit,{c},{X},{a}\n");
    let held = format!("{deposit}dispute,{c},{X}\n");
    let charged = format!("{held}chargeback,{c},{X}\n");
    let other = format!("deposit,{c},{Y},{b}\n");
    let withdrawal = format!("withdrawal,{c},{X},{a}\n");
    let fresh = format!("deposit,{F},{X},{a}\n");
    let fresh_held = format!("{fresh}dispute,{F},{X}\n");
    [
        // A resolve releases what its dispute held.
        (format!("{held}resolve,{c},{X}\n"), deposit.clone()),
        // A resolve or a chargeback of a deposit no dispute holds changes nothing.
        (format!("{deposit}resolve,{c},{X}\nchargeback,{c},{X}\n"), deposit.clone()),
        // A second dispute of a held deposit changes nothing.
        (format!("{held}dispute,{c},{X}\n"), held.clone()),
        // A resolved deposit can be disputed again.
        (format!("{held}resolve,{c},{X}\ndispute,{c},{X}\n"), held.clone()),
        // A chargeback takes back exactly what its deposit added, whatever the amount, and locks.
        (charged.clone(), format!("{other}dispute,{c},{Y}\nchargeback,{c},{Y}\n")),
        // A locked account refuses deposits.
        (format!("{charged}{other}"), charged.clone()),
        // A locked account refuses withdrawals.
        (format!("{charged}withdrawal,{c},{Y},{b}\n"), charged.clone()),
        // A charged-back deposit leaves history: a resolve finds nothing to release.
        (format!("{charged}resolve,{c},{X}\n"), charged.clone()),
        // A charged-back deposit leaves history: a dispute finds nothing to hold.
        (format!("{charged}dispute,{c},{X}\n"), charged.clone()),
        // Dispute commands still apply on a locked account.
        (
            format!("{other}{charged}dispute,{c},{Y}\n"),
            format!("{other}dispute,{c},{Y}\n{charged}"),
        ),
        // A tx id is spent by its first deposit or withdrawal, refused or not, for every client.
        (
            format!("{withdrawal}withdrawal,{o},{X},{b}\ndeposit,{c},{X},{b}\n"),
            format!("{withdrawal}dispute,{o},{Z}\n"),
        ),
        // A dispute reaches only its own client's deposits.
        (format!("{deposit}dispute,{o},{X}\n"), format!("{deposit}dispute,{o},{Z}\n")),
        // A withdrawal is not in history: a dispute finds nothing to hold.
        (format!("{withdrawal}dispute,{c},{X}\n"), format!("{withdrawal}dispute,{c},{Z}\n")),
        // A withdrawal of all the available funds is accepted and leaves zero.
        (format!("{fresh}withdrawal,{F},{Y},{a}\n"), format!("dispute,{F},{Z}\n")),
        // Held funds cannot be withdrawn.
        (format!("{fresh_held}withdrawal,{F},{Y},{a}\n"), fresh_held),
    ]
}

#[test]
fn prop_equivalent_commands_leave_the_same_accounts() {
    let law = support::for_all(
        "each pair of equivalent suffixes leaves the same accounts after any lines",
        (lines(), 1..=3_u16, input::any_money(), input::any_money()),
        |(drawn, client, first, second)| {
            let prefix = input::canonical(drawn);
            let accounts = |suffix: &str| {
                support::summary(format!("{prefix}{suffix}").as_bytes())
                    .map(|summary| summary.accounts)
            };
            for (left, right) in pairs(*client, &first.text(), &second.text()) {
                assert_eq!(accounts(&left), accounts(&right), "{left:?} and {right:?} differ");
            }
            true
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_any_spelling_reads_as_the_canonical_one() {
    let any_line = input::any_line(input::any_client(), input::any_tx());
    let law = support::for_all(
        "any spelling of valid lines reads as their canonical spelling",
        (vec((any_line, input::any_spelling()), 0..=8), input::any_layout()),
        |(drawn, layout)| {
            let spelled: Vec<(input::Line, input::Spelling)> = drawn
                .iter()
                .map(|(line, spelling)| (layout.fit(*line), spelling.clone()))
                .collect();
            let lines: Vec<input::Line> = spelled.iter().map(|&(line, _)| line).collect();
            support::summary(input::spelled(&spelled, layout).as_bytes())
                == support::summary(input::canonical(&lines).as_bytes())
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_a_header_lacking_type_client_or_tx_stops_the_run() {
    let any_line = input::any_line(input::any_client(), input::any_tx());
    let law = support::for_all(
        "a header lacking type, client or tx stops the run, whatever lines follow it",
        (vec((any_line, input::any_spelling()), 0..=4), input::any_incomplete_layout()),
        |(spelled, layout)| {
            support::summary(input::spelled(spelled, layout).as_bytes())
                == Err(RunError::Source(SourceError::IncompleteHeader))
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// Each kind of malformed line, with how many lines of it `summary` counts.
fn malformed(summary: &Summary) -> BTreeMap<RowError, u64> {
    summary.report.malformed().map(|(kind, count)| (kind, count.get())).collect()
}

#[test]
fn prop_a_malformed_line_changes_nothing_but_its_count() {
    let law = support::for_all(
        "a malformed line among valid ones changes no account and no rejection, and counts once as \
         its kind",
        (lines(), input::any_malformed(1..=3_u16, 0..8_u32), lines()),
        |(before, (line, flaw), after)| {
            let valid = [before.as_slice(), after.as_slice()].concat();
            let flawed = [
                input::canonical(before).into_bytes(),
                input::malformed(*line, flaw),
                input::rows(after).into_bytes(),
            ]
            .concat();
            let (Ok(with), Ok(without)) =
                (support::summary(&flawed), support::summary(input::canonical(&valid).as_bytes()))
            else {
                return false;
            };
            let mut counted = malformed(&without);
            *counted.entry(flaw.kind()).or_insert(0) += 1;
            with.accounts == without.accounts
                && with.report.rejected().eq(without.report.rejected())
                && malformed(&with) == counted
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// A reader whose every read fails.
#[derive(Debug)]
struct Failing;

impl Read for Failing {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the input is gone"))
    }
}

#[test]
fn a_source_error_stops_the_run_before_the_next_row_is_read() {
    let mut pulled = 0_u8;
    let source = CsvSource::new(b"type,client,tx,amount\n".as_slice().chain(Failing))
        .take(2)
        .chain(iter::repeat_n(Ok(Row::Malformed(RowError::Unreadable)), 3))
        .inspect(|_| pulled += 1);
    let outcome = run(source, &mut MemoryLedger::default(), &mut MemorySeenIds::default());
    assert_eq!(outcome, Err(RunError::Source(SourceError::Unreadable)));
    assert_eq!(pulled, 1);
}
