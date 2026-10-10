# stage-1-run — the sequential reference runtime over the store ports

Plan of record: `tasks/todo.md`

## Context

The decider exists but nothing feeds it rows or keeps its state. This ticket adds the `run`
module: the store ports, the row and error types, the summary projections, and the sequential
runtime that takes each row through intake, load, open, decide, evolve and commit. Its laws run
over minimal store doubles in the test crate, which the memory ticket replaces. Among them is the
overview's *a rejection changes no balance*: the core's types settle it for the decider, and here
it holds the runtime's commit after a rejection, the half that can fail. It lands as one commit,
`feat(run): …`.

Records that govern it: *Effects behind ports*, *Sources, sinks and projections*, *Stores defined
by their laws*, *State partitioned by client*, *Memory bounded by state, not input*, *Three failure
tiers*, *Correctness defined by laws and a model*, *Runtimes proven against a sequential
reference*, *Entities identified by their keys*; design records *Rows parsed into commands at the
boundary*, *Tx ids consumed on first sight*, *Accounts opened by the first command*; the pipeline
in `docs/architecture.md`. The `domain-modeling` and `curry-howard` skills apply to every type it
adds, test types included.

## Phase 0 — evidence (read-only)

- [ ] `git log --oneline -1` — shows the `feat(core): …` commit.
- [ ] `make ci` — green, with the domain and core laws passing.

## Phase 1 — declarations and a stub runtime

Files: `backend/src/lib.rs`, `backend/src/run.rs`, `backend/src/run/port.rs`,
`backend/src/run/row.rs`, `backend/src/run/summary.rs`.

- [ ] `pub mod run;` in `lib.rs`. `run.rs` declares `mod port; mod row; mod summary;` and
      re-exports every public item, so the API is `backend::run::{run, LedgerStore, SeenIds, Row,
      RowError, SourceError, RunError, Summary, Accounts, Report}`.
- [ ] The types and signatures exactly as `tasks/todo.md` § Interfaces gives them, with `Display`
      and `error::Error` for `RowError`, `SourceError` and `RunError`.
- [ ] `Report` keeps `BTreeMap<Reason, NonZeroU64>` and `BTreeMap<RowError, NonZeroU64>`. Its
      `pub(super)` `EMPTY`, `with_rejection` and `with_malformed` count through one private
      `fn count<K: Ord>(counts: &mut BTreeMap<K, NonZeroU64>, key: K)`:
      `let _ = counts.entry(key).and_modify(|n| *n = n.saturating_add(1)).or_insert(NonZeroU64::MIN);`.
- [ ] Stub: `run` drains nothing and returns `Ok(Summary { accounts: Accounts::from_iter([]),
      report: Report::EMPTY })`.

## Phase 2 — doubles, generators, the naive model, the laws (red)

Files: `backend/tests/it/main.rs`, `backend/tests/it/support.rs`,
`backend/tests/it/support/stores.rs`, `backend/tests/it/support/generate.rs`,
`backend/tests/it/support/model.rs`, `backend/tests/it/run.rs`.

- [ ] `support/stores.rs`: `Ledger` (`BTreeMap<ClientId, Account>` and `BTreeMap<Address,
      Deposit>`) and `Seen` (`BTreeSet<TxId>`), both `#[derive(Debug, Default)]`,
      `Error = Infallible`. `load(address)` is `None` without an account, else
      `Footprint::new(account, deposit)` with the deposit at `address`; `commit(address,
      footprint)` inserts the account, then inserts the footprint's deposit at `address` or
      removes that key when it holds none; `snapshot` collects the accounts in key order.
      `pub(crate) mod stores;` in `support.rs`.
- [ ] `support/generate.rs`: add `InputRow::Unreadable` and `InputRow::TooManyFields`, the
      `MalformedRow` edge in `Edge` and `EDGES`, and `row(&InputRow) -> Row`; rewrite `command` as
      `match row(input) { Row::Parsed(command) => Some(command), Row::Malformed(_) => None }`, so
      one function reads an input row. Add `Unique`, `unique` and `interleave` as
      `tasks/todo.md` § Generators states.
- [ ] `support/model.rs`: the naive engine, `PrintedAccount`, `Printed` and
      `printed(rows: &[InputRow]) -> Printed`, below.
- [ ] `tests/it/run.rs` with `mod run;` in `main.rs`; the helpers below and the eight laws.
- [ ] Red: `cargo test --test it run::` against the stub — the model, first-sight, malformed,
      source-error and bounded-report laws fail with counterexamples (the stub reports nothing and
      never stops, so even the malformed counts fall short); the same-rows, reordering and
      rejection laws pass vacuously over empty summaries, so their red is their break in Phase 4.
      Record the failure lines.

The naive model reads the input rows, not the parsed ones, and states the overview's tables over
plain numbers: amounts in `Units` through `amount_units`, kept in named structs of its own, apart
from the production types it is checked against.

```rust
/// An account as the model keeps it.
#[derive(Debug, Clone, Copy, Default)]
struct Account { available: Units, held: Units, locked: bool }

/// A deposit in the model's history.
#[derive(Debug, Clone, Copy)]
struct Deposit { amount: Units, disputed: bool }

/// The naive engine: accounts by client, history by client and tx id, the used tx ids, and the
/// counts.
#[derive(Debug, Default)]
struct Engine {
    accounts: BTreeMap<u16, Account>,
    deposits: BTreeMap<(u16, u32), Deposit>,
    seen: BTreeSet<u32>,
    rejected: BTreeMap<Reason, NonZeroU64>,
    malformed: BTreeMap<RowError, NonZeroU64>,
}

/// An account as the output prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrintedAccount {
    pub(crate) client: ClientId,
    pub(crate) available: String,
    pub(crate) held: String,
    pub(crate) total: Option<String>,
    pub(crate) locked: bool,
}

/// What a run reports, printed: the accounts by client id, then the counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Printed {
    pub(crate) accounts: Vec<PrintedAccount>,
    pub(crate) rejected: Vec<(Reason, NonZeroU64)>,
    pub(crate) malformed: Vec<(RowError, NonZeroU64)>,
}
```

- `Unreadable` and `TooManyFields` count as those `RowError`s; a deposit or withdrawal with no
  amount counts `MissingAmount`; one whose amount `amount_units` refuses counts
  `InvalidAmount(error)`. A malformed row opens nothing and consumes no id.
- Every other row opens its client's account (zeros, unlocked) if absent, then:
  - deposit / withdrawal: the id is recorded first (`seen.insert`); a repeat is `DuplicateTx`;
    then a locked account is `AccountLocked`; then a withdrawal above `available` is
    `InsufficientFunds`; else `available` moves and a deposit is recorded, not disputed;
  - dispute / resolve / chargeback: no deposit under the client and tx id is `UnknownDeposit`;
    dispute of a disputed one is `NotSettled`; resolve or chargeback of an undisputed one is
    `NotDisputed`; else the balances move as the lifecycle table says, and a chargeback removes
    the deposit and locks the account.
- Each reason counts under its family, `Reason::Client(…)` or `Reason::Dispute(…)`. `printed`
  prints each balance through `Units`'s `Display`, and the total as `Some(available + held)`.

Helpers in `tests/it/run.rs`:

```rust
type Source = Vec<Result<Row, SourceError>>;

/// What a source would yield for `rows`, every one readable.
fn source(rows: &[InputRow]) -> Source {
    rows.iter().map(|input| Ok(generate::row(input))).collect()
}

/// `run` over fresh stores.
fn summarize(source: Source) -> Result<Summary, RunError<Infallible, Infallible>> {
    run(source, &mut stores::Ledger::default(), &mut stores::Seen::default())
}

/// The summary as the output would print it.
fn printed(summary: &Summary) -> model::Printed {
    model::Printed {
        accounts: summary
            .accounts
            .by_client()
            .map(|(client, account)| model::PrintedAccount {
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
```

A total that overflows prints as `None` and so disagrees with the model's `Some`, which fails the
law instead of hiding the overflow.

## Phase 3 — the runtime (green)

File: `backend/src/run.rs`.

```rust
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
        Command::Dispute(dispute_command) => apply_at(ledger, dispute_command.address(), |footprint| {
            decide_dispute(footprint.deposit(), dispute_command)
                .map(Event::Dispute)
                .map_err(Reason::Dispute)
        })
        .map_err(RunError::Ledger),
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
```

- [ ] Verify: `cargo test --test it run::` — 8 passed.

## Phase 4 — the laws, and each one broken once

File: `backend/tests/it/run.rs`. Each runs `support::for_all(label, 500, …)` over
`generate::rows` unless the row says otherwise.

| Test | Label (the law) | Holds when | Deliberate break |
|---|---|---|---|
| `prop_tx_ids_are_consumed_on_first_sight_across_clients` | a tx id is consumed by the first deposit or withdrawal naming it, whatever its outcome, for every client | the `Reason::Client(DuplicateTx)` count equals the parsed deposit and withdrawal rows whose tx an earlier one named | withdrawals skip intake and are decided as `Sight::First` |
| `prop_rejection_changes_no_balance` | a rejection changes no balance; a client with no account counts as zero | over the summaries of every prefix of the rows, each row that adds a rejection leaves `unchanged_but_opened(before, after)`; a summary that fails to build fails the law | after a rejection at a loaded footprint, `apply_at` commits `open()` there instead of nothing |
| `prop_malformed_rows_are_skipped_and_counted` | a malformed row changes no account and no rejection, and is counted by kind | `run(rows)` and `run(rows without malformed)` have equal accounts and rejections; the first counts exactly the malformed rows by kind, the second none | `Row::Malformed` is skipped uncounted |
| `prop_source_error_stops_the_run_with_no_summary` | an unreadable source stops the run: the first source error is the result | rows with a `SourceError` inserted at a generated position and another appended give `Err(RunError::Source(first))` | an `Err` row is skipped like a malformed row |
| `prop_report_stays_bounded` | the report holds one count per kind, however many rows | `rejected()` and `malformed()` keys are strictly increasing; the malformed counts sum to the malformed rows | rejections kept as a `Vec`, one entry per refused row |
| `prop_same_rows_give_the_same_summary` | the same rows give the same summary | two runs over fresh stores are equal | a `static` run counter added to the first rejection count |
| `prop_reordering_clients_changes_no_account` | reordering rows of different clients changes no account, given unique tx ids | over a generated `Unique` and `interleave(random, &unique)`, both runs give equal accounts | a one-entry footprint cache keyed by the tx id alone, not the whole address, serves a dispute from the previous command's footprint |
| `prop_engine_agrees_with_the_naive_model` | run reports what the naive model expects | `printed(&summary) == model::printed(&rows)` | `InsufficientFunds` when the amount is not below `available` (`>=`) |

- [ ] Stage this ticket's files first (`git add` with explicit paths), so a break in a new file
      shows in `git diff`. Then for each row: apply the break by editing, run
      `cargo test --test it <test>`, paste the failure line into Law evidence, undo the break by
      editing, and confirm `git diff --quiet` exits 0. Never `git checkout` or `git stash`
      mid-step, and never undo by copying a saved file back: the copy keeps its old modification
      time, and cargo reuses the broken build.
- [ ] Verify: `cargo test --test it` — 21 passed (domain 4, core 9, run 8); `make ci` — green.

## Phase 5 — sync and land

- [ ] `CLAUDE.md` and `README.md` layouts: add `run.rs` and `run/`.
- [ ] Code-reviewer agent over the staged diff; act on every blocker and should-fix.
- [ ] The two forbidden-name greps of `tasks/todo.md` § Global constraints: both silent.
- [ ] `make ci` — green; paste the test summary line here.
- [ ] Tick this ticket and its line in `tasks/todo.md`; stage explicit paths; commit with
      `git commit -F` as `tasks/todo.md` § Commits gives it.

## Law evidence

| Law | Break | Command | Failure line |
|---|---|---|---|
| `prop_tx_ids_are_consumed_on_first_sight_across_clients` | withdrawals skip intake and are decided as `Sight::First` | `cargo test --test it prop_tx_ids_are_consumed_on_first_sight_across_clients` | |
| `prop_rejection_changes_no_balance` | after a rejection at a loaded footprint, `apply_at` commits `open()` | `cargo test --test it prop_rejection_changes_no_balance` | |
| `prop_malformed_rows_are_skipped_and_counted` | `Row::Malformed` is skipped uncounted | `cargo test --test it prop_malformed_rows_are_skipped_and_counted` | |
| `prop_source_error_stops_the_run_with_no_summary` | an `Err` row is skipped like a malformed row | `cargo test --test it prop_source_error_stops_the_run_with_no_summary` | |
| `prop_report_stays_bounded` | rejections kept as a `Vec`, one entry per refused row | `cargo test --test it prop_report_stays_bounded` | |
| `prop_same_rows_give_the_same_summary` | a `static` run counter added to the first rejection count | `cargo test --test it prop_same_rows_give_the_same_summary` | |
| `prop_reordering_clients_changes_no_account` | a one-entry footprint cache keyed by the tx id alone | `cargo test --test it prop_reordering_clients_changes_no_account` | |
| `prop_engine_agrees_with_the_naive_model` | `InsufficientFunds` when the amount is not below `available` | `cargo test --test it prop_engine_agrees_with_the_naive_model` | |

## Blocked / handed off

- [ ] None expected.

## Review

*Filled in when the work closes.*
