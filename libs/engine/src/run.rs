//! The sequential runtime: each row, in arrival order, through intake, load, open, decide, evolve
//! and commit, over the store ports.

mod port;
mod row;
mod summary;

pub use port::{LedgerStore, SeenIds};
pub use row::{Row, RowError, RunError, SourceError};
pub use summary::{Accounts, Report, Summary};

use crate::core::{Overflow, decide_client, decide_dispute, evolve, open};
use crate::domain::{Address, Command, Event, Footprint, Reason};

/// Runs `rows` in arrival order over `ledger` and `seen`: the accounts and the report, or the
/// first source or store failure.
pub fn run<L: LedgerStore, S: SeenIds>(
    rows: impl IntoIterator<Item = Result<Row, SourceError>>,
    ledger: &mut L,
    seen: &mut S,
) -> Result<Summary, RunError<L::Error, S::Error>> {
    let report = rows.into_iter().try_fold(Report::EMPTY, |report, row| {
        match row.map_err(RunError::Source)? {
            Row::Malformed(error) => Ok(report.with_malformed(error)),
            Row::Parsed(command) => Ok(match apply(&command, ledger, seen)? {
                Ok(_) => report,
                Err(reason) => report.with_rejection(reason),
            }),
        }
    })?;
    let accounts = ledger.snapshot().map_err(RunError::Ledger)?;
    Ok(Summary { accounts, report })
}

/// One command through intake, then decided and applied at its address; the inner result is its
/// event or its rejection.
fn apply<L: LedgerStore, S: SeenIds>(
    command: &Command,
    ledger: &mut L,
    seen: &mut S,
) -> Result<Result<Event, Reason>, RunError<L::Error, S::Error>> {
    match command {
        Command::Client(client_command) => {
            let address = client_command.address();
            let sight = seen.sight(address.tx()).map_err(RunError::Seen)?;
            apply_at(ledger, address, |footprint| {
                decide_client(footprint.account(), client_command, sight)
                    .map(Event::Client)
                    .map_err(Reason::Client)
            })
            .map_err(RunError::Ledger)
        }
        Command::Dispute(dispute_command) => {
            apply_at(ledger, dispute_command.address(), |footprint| {
                decide_dispute(footprint.deposit(), dispute_command)
                    .map(Event::Dispute)
                    .map_err(Reason::Dispute)
            })
            .map_err(RunError::Ledger)
        }
    }
}

/// Where a step's footprint came from: the store, or `open` for a client with no account.
#[derive(Debug, Clone, Copy)]
enum Start {
    Loaded(Footprint),
    Opened,
}

/// Loads the footprint at `address`, opens the account when there is none, decides, evolves, and
/// commits what changed at the same address: the evolved footprint, or after a rejection only an
/// account this step opened.
fn apply_at<L: LedgerStore>(
    ledger: &mut L,
    address: Address,
    decide: impl FnOnce(&Footprint) -> Result<Event, Reason>,
) -> Result<Result<Event, Reason>, L::Error> {
    let start = ledger.load(address)?.map_or(Start::Opened, Start::Loaded);
    let before = match start {
        Start::Loaded(footprint) => footprint,
        Start::Opened => open(),
    };
    let decided = decide(&before).and_then(|event| {
        evolve(Some(before), &event).map(|next| (event, next)).map_err(|Overflow| Reason::Overflow)
    });
    let write = match (&decided, start) {
        (Ok((_, next)), _) => Some(*next),
        (Err(_), Start::Opened) => Some(before),
        (Err(_), Start::Loaded(_)) => None,
    };
    if let Some(footprint) = write {
        ledger.commit(address, footprint)?;
    }
    Ok(decided.map(|(event, _)| event))
}
