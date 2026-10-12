//! Laws of the runtime: rows spelled as CSV text and read by the CSV source, then through intake,
//! the decider and the stores, against the overview's laws and the naive model; and the runtime's
//! contract with any source.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::num::NonZeroU64;

use backend::csv_io::CsvSource;
use backend::domain::{Account, Balance, ClientId, ClientReason, Command, DisputeReason, Reason};
use backend::run::{Row, RowError, RunError, SourceError, Summary};
use proptest::arbitrary::any;
use proptest::sample::{Index, select};
use proptest::strategy::Strategy;

use crate::support::generate::{self, InputRow, Unique, chargeback, deposit, dispute, withdrawal};
use crate::support::model::{Printed, PrintedAccount};
use crate::support::spelling::{self, Input};
use crate::support::{self, model};

type Source = Vec<Result<Row, SourceError>>;

/// What a source would yield for `rows`, every one readable.
fn source(rows: &[InputRow]) -> Source {
    rows.iter().map(|input| Ok(generate::row(input))).collect()
}

/// `input` through the CSV source and `run`, over fresh stores.
fn summarized(input: &Input) -> Result<Summary, RunError<Infallible, Infallible>> {
    support::summarize(CsvSource::new(input.text().as_slice()))
}

/// The summary as the output would print it.
fn printed(summary: &Summary) -> Printed {
    Printed {
        accounts: summary
            .accounts
            .by_client()
            .map(|(client, account)| PrintedAccount {
                client,
                available: account.available().to_string(),
                held: account.held().to_string(),
                total: account.total().map(|total| total.to_string()),
                locked: account.is_locked(),
            })
            .collect(),
        rejected: summary.report.rejected().collect(),
        malformed: summary.report.malformed().collect(),
    }
}

/// How many rows `summary` reports rejected.
fn rejections(summary: &Summary) -> u64 {
    summary.report.rejected().fold(0, |sum, (_, count)| sum.saturating_add(count.get()))
}

/// Whether `after` keeps every account `before` holds as it was, and holds any other only as
/// opened: zero available and held, unlocked.
fn unchanged_but_opened(before: &Summary, after: &Summary) -> bool {
    let earlier: BTreeMap<ClientId, Account> = before.accounts.by_client().collect();
    let later: BTreeMap<ClientId, Account> = after.accounts.by_client().collect();
    earlier.keys().all(|client| later.contains_key(client))
        && later.iter().all(|(client, account)| match earlier.get(client) {
            Some(previous) => previous == account,
            None => {
                account.available() == Balance::ZERO
                    && account.held() == Balance::ZERO
                    && !account.is_locked()
            }
        })
}

/// The source errors, each named once.
const SOURCE_ERRORS: [SourceError; 2] = [SourceError::Unreadable, SourceError::IncompleteHeader];

/// Rows, and the source error a source meets before the row at `position`.
#[derive(Debug, Clone)]
struct BrokenSource {
    rows: Vec<InputRow>,
    position: usize,
    first: SourceError,
}

/// Rows with a source error at a drawn position.
fn broken_source() -> impl Strategy<Value = BrokenSource> {
    (generate::rows(), any::<Index>(), select(&SOURCE_ERRORS)).prop_map(
        |(rows, position, first)| {
            let position = position.index(rows.len().saturating_add(1));
            BrokenSource { rows, position, first }
        },
    )
}

/// A drawn input, its rows made unique, and those rows with the clients' turns reordered; the law
/// spells both under the input's layout.
#[derive(Debug, Clone)]
struct Reordering {
    input: Input,
    original: Unique,
    interleaved: Unique,
}

/// Generated rows made unique, and an interleaving of them, under a drawn layout.
fn reordering() -> impl Strategy<Value = Reordering> {
    (spelling::input(), generate::turns()).prop_map(|(input, turns)| {
        let original = generate::unique(input.rows());
        let interleaved = generate::interleave(&original, &turns);
        Reordering { input, original, interleaved }
    })
}

/// Whether `input` reads as a command.
fn parses(input: &InputRow) -> bool {
    match generate::row(input) {
        Row::Parsed(_) => true,
        Row::Malformed(_) => false,
    }
}

/// The malformed rows of `rows`, counted by kind.
fn malformed_counts(rows: &[InputRow]) -> BTreeMap<RowError, NonZeroU64> {
    rows.iter().fold(BTreeMap::new(), |mut counts, input| {
        match generate::row(input) {
            Row::Malformed(error) => model::count(&mut counts, error),
            Row::Parsed(_) => {}
        }
        counts
    })
}

#[test]
fn prop_tx_ids_are_consumed_on_first_sight_across_clients() {
    let law = support::for_all(
        "a tx id is consumed by the first deposit or withdrawal naming it, whatever its outcome, for every client",
        spelling::input(),
        |input| {
            let mut used = BTreeSet::new();
            let repeats = input
                .rows()
                .iter()
                .filter(|row| match generate::row(row) {
                    Row::Parsed(Command::Client(command)) => !used.insert(command.address().tx()),
                    Row::Parsed(Command::Dispute(_)) | Row::Malformed(_) => false,
                })
                .count();
            summarized(input).is_ok_and(|summary| {
                let duplicates = summary
                    .report
                    .rejected()
                    .find(|&(reason, _)| reason == Reason::Client(ClientReason::DuplicateTx))
                    .map_or(0, |(_, count)| count.get());
                u64::try_from(repeats).is_ok_and(|repeats| repeats == duplicates)
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_rejection_changes_no_balance() {
    let law = support::for_all(
        "a rejection changes no balance; a client with no account counts as zero",
        spelling::input(),
        |input| {
            let summaries: Option<Vec<Summary>> = (0..=input.rows().len())
                .map(|end| input.prefix(end).and_then(|prefix| summarized(&prefix).ok()))
                .collect();
            summaries.is_some_and(|summaries| {
                summaries.windows(2).all(|pair| match pair {
                    [before, after] => {
                        rejections(after) == rejections(before)
                            || unchanged_but_opened(before, after)
                    }
                    _ => false,
                })
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_malformed_rows_are_skipped_and_counted() {
    let law = support::for_all(
        "a malformed row changes no account and no rejection, and is counted by kind",
        spelling::input(),
        |input| {
            let parsed: Vec<InputRow> =
                input.rows().iter().filter(|row| parses(row)).cloned().collect();
            let expected = malformed_counts(input.rows());
            match (summarized(input), summarized(&input.with_rows(parsed))) {
                (Ok(all), Ok(without)) => {
                    all.accounts == without.accounts
                        && all.report.rejected().eq(without.report.rejected())
                        && all.report.malformed().eq(expected.iter().map(|(&e, &n)| (e, n)))
                        && without.report.malformed().next().is_none()
                }
                _ => false,
            }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_source_error_stops_the_run_with_no_summary() {
    let law = support::for_all(
        "an unreadable source stops the run: the first source error is the result, and nothing after it is read",
        broken_source(),
        |BrokenSource { rows, position, first }| {
            let last = match first {
                SourceError::Unreadable => SourceError::IncompleteHeader,
                SourceError::IncompleteHeader => SourceError::Unreadable,
            };
            let mut items = source(rows);
            items.insert((*position).min(items.len()), Err(*first));
            items.push(Err(last));
            let unread = items.len().saturating_sub(position.saturating_add(1));
            let mut rest = items.into_iter();
            support::summarize(rest.by_ref()) == Err(RunError::Source(*first))
                && rest.len() == unread
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_report_stays_bounded() {
    let law = support::for_all(
        "the report holds one count per kind, however many rows",
        spelling::input(),
        |input| {
            let malformed_rows = malformed_counts(input.rows())
                .values()
                .fold(0_u64, |sum, count| sum.saturating_add(count.get()));
            summarized(input).is_ok_and(|summary| {
                let reasons: Vec<Reason> = summary.report.rejected().map(|(key, _)| key).collect();
                let kinds: Vec<RowError> = summary.report.malformed().map(|(key, _)| key).collect();
                let counted = summary
                    .report
                    .malformed()
                    .fold(0_u64, |sum, (_, count)| sum.saturating_add(count.get()));
                reasons.windows(2).all(|pair| matches!(pair, [a, b] if a < b))
                    && kinds.windows(2).all(|pair| matches!(pair, [a, b] if a < b))
                    && counted == malformed_rows
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_same_rows_give_the_same_summary() {
    let law = support::for_all("the same rows give the same summary", spelling::input(), |input| {
        summarized(input) == summarized(input)
    });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_reordering_clients_changes_no_account() {
    let law = support::for_all(
        "reordering rows of different clients changes no account, given unique tx ids",
        reordering(),
        |Reordering { input, original, interleaved }| match (
            summarized(&input.with_rows(original.rows().to_vec())),
            summarized(&input.with_rows(interleaved.rows().to_vec())),
        ) {
            (Ok(original), Ok(reordered)) => original.accounts == reordered.accounts,
            _ => false,
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_engine_agrees_with_the_naive_model() {
    let law =
        support::for_all("run reports what the naive model expects", spelling::input(), |input| {
            summarized(input).is_ok_and(|summary| printed(&summary) == model::printed(input.rows()))
        });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// The overview's worked example: nine rows over four clients.
fn overview_rows() -> [InputRow; 9] {
    [
        deposit(1, 10, "25.5"),
        withdrawal(1, 11, "5.25"),
        deposit(2, 20, "10"),
        withdrawal(2, 21, "10"),
        dispute(2, 20),
        chargeback(2, 20),
        deposit(2, 22, "3"),
        withdrawal(3, 30, "1"),
        dispute(4, 40),
    ]
}

/// What the overview prints for it.
fn overview_printed() -> Printed {
    Printed {
        accounts: vec![
            PrintedAccount {
                client: ClientId::from(1),
                available: "20.2500".to_owned(),
                held: "0.0000".to_owned(),
                total: Some("20.2500".to_owned()),
                locked: false,
            },
            PrintedAccount {
                client: ClientId::from(2),
                available: "-10.0000".to_owned(),
                held: "0.0000".to_owned(),
                total: Some("-10.0000".to_owned()),
                locked: true,
            },
            PrintedAccount {
                client: ClientId::from(3),
                available: "0.0000".to_owned(),
                held: "0.0000".to_owned(),
                total: Some("0.0000".to_owned()),
                locked: false,
            },
            PrintedAccount {
                client: ClientId::from(4),
                available: "0.0000".to_owned(),
                held: "0.0000".to_owned(),
                total: Some("0.0000".to_owned()),
                locked: false,
            },
        ],
        rejected: vec![
            (Reason::Client(ClientReason::AccountLocked), NonZeroU64::MIN),
            (Reason::Client(ClientReason::InsufficientFunds), NonZeroU64::MIN),
            (Reason::Dispute(DisputeReason::UnknownDeposit), NonZeroU64::MIN),
        ],
        malformed: vec![],
    }
}

#[test]
fn the_overview_example_ends_with_its_documented_accounts() {
    let summary = support::summarize(source(&overview_rows()))
        .expect("a source of readable rows does not fail");
    assert_eq!(printed(&summary), overview_printed());
}
