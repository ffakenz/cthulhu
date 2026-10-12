//! The runtime through the CSV source, over fresh in-memory stores, held to equations between its
//! runs: clients run independently, and suffixes the rules make equivalent leave the same
//! accounts after any rows. A source error stops the run, and the input as the overview spells it
//! reads as its plainest spelling.

use std::io::{self, Read};
use std::iter;

use app::csv_io::CsvSource;
use engine::run::{Row, RowError, RunError, SourceError, run};
use proptest::collection::vec;
use proptest::sample::select;
use proptest::strategy::Strategy;
use store::memory::{MemoryLedger, MemorySeenIds};

use crate::support;

/// The types a line may name.
const KINDS: [&str; 5] = ["deposit", "withdrawal", "dispute", "resolve", "chargeback"];

/// Amounts at mixed scales: the smallest, a whole one, one with a single decimal, two that differ
/// only in their fourth decimal, and the largest an amount may be.
const AMOUNTS: [&str; 6] = ["0.0001", "1", "2.5", "711.6569", "711.657", "999999999999999.9999"];

/// A line of CSV input: its type, client, tx id and amount.
#[derive(Debug, Clone, Copy)]
struct Line {
    kind: &'static str,
    client: u16,
    tx: u32,
    amount: &'static str,
}

/// Up to sixteen lines of any type, by clients 1 to 3, on tx ids below 8, for amounts from
/// `AMOUNTS`.
fn lines() -> impl Strategy<Value = Vec<Line>> {
    let line = (select(&KINDS), 1..=3_u16, 0..8_u32, select(&AMOUNTS))
        .prop_map(|(kind, client, tx, amount)| Line { kind, client, tx, amount });
    vec(line, 0..=16)
}

/// `lines` as CSV text, under the header `type,client,tx,amount`.
fn text(lines: &[Line]) -> String {
    lines.iter().fold(String::from("type,client,tx,amount\n"), |csv, line| {
        csv + &format!("{},{},{},{}\n", line.kind, line.client, line.tx, line.amount)
    })
}

#[test]
fn prop_clients_run_independently() {
    let law = support::for_all(
        "lines of different clients give the same summary in any order, given distinct tx ids",
        lines(),
        |drawn| {
            let distinct: Vec<Line> = drawn
                .iter()
                .zip(0_u32..)
                .map(|(&line, position)| match line.kind {
                    "deposit" | "withdrawal" => Line { tx: position, ..line },
                    _ => line,
                })
                .collect();
            let mut by_client = distinct.clone();
            by_client.sort_by_key(|line| line.client);
            support::summary(text(&distinct).as_bytes())
                == support::summary(text(&by_client).as_bytes())
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
        // A malformed row opens no account and spends no tx id.
        (format!("deposit,{F},{X},0\n{deposit}"), deposit),
    ]
}

#[test]
fn prop_equivalent_commands_leave_the_same_accounts() {
    let law = support::for_all(
        "each pair of equivalent suffixes leaves the same accounts after any lines",
        (lines(), 1..=3_u16, select(&AMOUNTS), select(&AMOUNTS)),
        |(drawn, client, first, second)| {
            let prefix = text(drawn);
            let accounts = |suffix: &str| {
                support::summary(format!("{prefix}{suffix}").as_bytes())
                    .map(|summary| summary.accounts)
            };
            for (left, right) in pairs(*client, first, second) {
                assert_eq!(accounts(&left), accounts(&right), "{left:?} and {right:?} differ");
            }
            true
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

/// Lines in the plainest spelling: a deposit, a withdrawal, a dispute, a malformed line and a
/// resolve, all by client 1.
const PLAIN: &[u8] = b"\
type,client,tx,amount
deposit,1,1,2.5
withdrawal,1,2,1
dispute,1,1,
Deposit,1,3,1
resolve,1,1,
";

/// The same lines as the overview allows them: a byte-order mark; the columns in another order,
/// padded, with one no line needs; CRLF line ends and a blank line; a padded field, a quoted one
/// and a client spelled `0x1`; a dispute that stops before its amount; a line that is not UTF-8;
/// and a resolve that carries an amount.
const RESPELLED: &[u8] = b"\xEF\xBB\xBFtx , type,note, client ,amount\r\n\
    \r\n\
    1,deposit , x,0x1,\"2.5\"\r\n\
    \x202, withdrawal,,1,1\r\n\
    1,dispute,,1\r\n\
    3,\xFF,,1,1\r\n\
    1,resolve,,1,9\r\n";

#[test]
fn the_input_as_the_overview_spells_it_reads_as_its_plainest_spelling() {
    let plain = support::summary(PLAIN).expect("the plainest spelling runs");
    assert_eq!(support::summary(RESPELLED), Ok(plain));
}
