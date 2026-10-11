//! Laws of the runtime: rows through intake, the decider and the stores, against the overview's
//! laws and the naive model.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::num::NonZeroU64;

use backend::domain::{Account, Balance, ClientId, ClientReason, Command, DisputeReason, Reason};
use backend::memory::{MemoryLedger, MemorySeenIds};
use backend::run::{Row, RowError, RunError, SourceError, Summary, run};
use proptest::arbitrary::any;
use proptest::sample::{Index, select};
use proptest::strategy::Strategy;

use crate::support::generate::{self, InputRow, Unique, chargeback, deposit, dispute, withdrawal};
use crate::support::model::{Printed, PrintedAccount};
use crate::support::{self, model};

type Source = Vec<Result<Row, SourceError>>;

/// What a source would yield for `rows`, every one readable.
fn source(rows: &[InputRow]) -> Source {
    rows.iter().map(|input| Ok(generate::row(input))).collect()
}

/// `run` over fresh stores.
fn summarize(
    source: impl IntoIterator<Item = Result<Row, SourceError>>,
) -> Result<Summary, RunError<Infallible, Infallible>> {
    run(source, &mut MemoryLedger::default(), &mut MemorySeenIds::default())
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

/// Rows with unique tx ids, and the same rows with the clients' turns reordered.
#[derive(Debug, Clone)]
struct Reordering {
    original: Unique,
    interleaved: Unique,
}

/// Generated rows made unique, and an interleaving of them.
fn reordering() -> impl Strategy<Value = Reordering> {
    (generate::rows(), generate::turns()).prop_map(|(rows, turns)| {
        let original = generate::unique(&rows);
        let interleaved = generate::interleave(&original, &turns);
        Reordering { original, interleaved }
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
        generate::rows(),
        |rows| {
            let mut used = BTreeSet::new();
            let repeats = rows
                .iter()
                .filter(|input| match generate::row(input) {
                    Row::Parsed(Command::Client(command)) => !used.insert(command.address().tx()),
                    Row::Parsed(Command::Dispute(_)) | Row::Malformed(_) => false,
                })
                .count();
            summarize(source(rows)).is_ok_and(|summary| {
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
        generate::rows(),
        |rows| {
            let summaries: Option<Vec<Summary>> = (0..=rows.len())
                .map(|end| rows.get(..end).and_then(|prefix| summarize(source(prefix)).ok()))
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
        generate::rows(),
        |rows| {
            let parsed: Vec<InputRow> =
                rows.iter().filter(|input| parses(input)).cloned().collect();
            let expected = malformed_counts(rows);
            match (summarize(source(rows)), summarize(source(&parsed))) {
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
            summarize(rest.by_ref()) == Err(RunError::Source(*first)) && rest.len() == unread
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
        generate::rows(),
        |rows| {
            let malformed_rows = malformed_counts(rows)
                .values()
                .fold(0_u64, |sum, count| sum.saturating_add(count.get()));
            summarize(source(rows)).is_ok_and(|summary| {
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
    let law = support::for_all("the same rows give the same summary", generate::rows(), |rows| {
        summarize(source(rows)) == summarize(source(rows))
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
        |Reordering { original, interleaved }| match (
            summarize(source(original.rows())),
            summarize(source(interleaved.rows())),
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
        support::for_all("run reports what the naive model expects", generate::rows(), |rows| {
            summarize(source(rows)).is_ok_and(|summary| printed(&summary) == model::printed(rows))
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
    let summary =
        summarize(source(&overview_rows())).expect("a source of readable rows does not fail");
    assert_eq!(printed(&summary), overview_printed());
}
