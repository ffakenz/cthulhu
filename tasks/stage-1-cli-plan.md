# Stage 1 command line — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: the execution method the person chooses at
> hand-off, superpowers:executing-plans (inline, recommended in Decisions for sign-off) or
> superpowers:subagent-driven-development, to implement this plan task by task on
> `feat/stage-1-cli`. Steps use checkbox (`- [ ]`) syntax; `- [x]` means the command beside it ran
> and its output supported the claim.

**Goal:** `make run INPUT=<file>` turns an input file into the accounts table: the CSV adapters
(`csv_io`: `CsvRow`, `CsvSource`, `render_csv`), the composition root in `main.rs`, and fixtures
through the binary; every runtime law is fed CSV text through the production source.

**Architecture:** `CsvSource<R: Read>` is the source port's adapter. It reads the header lazily
and checks that it names `type`, `client` and `tx`. It then reads each record into one reused
`StringRecord`, counts a row longer than the header as malformed, deserializes the rest into a
borrowed `CsvRow` against the checked header, and builds the `Command` with `TryFrom`.
`render_csv` writes `Accounts` onto any `Write` through the `csv` writer and flushes it, and
`text_io::render_report` writes the `Report` as lines of text. `main.rs` reads the one argument,
opens the file and runs it over the in-memory stores. Only after the run succeeds does it hand
the table to stdout and the report to stderr; every failure maps to its own exit status.

**Tech stack:** Rust 1.88.0, edition 2024. `csv` 1.4 and `serde` 1 with `derive` enter
`[dependencies]`. `proptest` 1.12 runs the laws, as before.

**Spec:**
- The records *Sources, sinks and projections*, *Effects behind ports*, *Three failure tiers*,
  *Correctness defined by laws and a model* and *csv, serde, rust_decimal and proptest as
  dependencies*.
- The design records *Exact decimal amounts with four places*, *Rows parsed into commands at the
  boundary* and *The command-line contract*.
- `docs/overview.md`, `docs/architecture.md`, `docs/roadmap.md` § 1, and `CLAUDE.md` with the
  skills it lists.

Where code and records disagree, the records win. A gap goes to the person (Questions on gaps),
and a reversal needs a superseding record.

## Context

Stage 1's library core landed on `main` as the squash commit `6e94920`: the domain, the core, the
sequential runtime and the in-memory stores, held by 25 laws on proptest. Its tree equals the tip
of `feat/stage-1-core`, `779a604`.

Three things are still missing:
- `backend/src/main.rs` is `fn main() {}`;
- `csv` and `serde` are admitted but unused;
- every runtime law feeds `run` rows that the model builds (`generate::row`), never CSV text.

This plan finishes roadmap § 1 up to a working command line. The benchmark generator, the
measurements and the speed-ups follow in a later session, once `make run` works end to end.

## Decisions for sign-off

Approving this plan signs off each recommendation below, and each one in Questions on gaps,
unless the person rules otherwise.

1. **Commits: red and green apart, as `CLAUDE.md` § Implementation asks.** Each of the three
   pieces lands as a red commit, then a green commit:
   - the red commit holds the new laws or fixtures and a wrong but lint-clean stub, and fails
     only on its new tests;
   - the green commit holds the implementation.

   - Why: stage 1 set the rule aside by the person's decision. `main` now takes a branch as one
     squash commit while the remote branch keeps every commit, so splitting costs `main` nothing.
   - What it gives: a red commit is evidence anyone can check out.
   - Cost: three more commits, each through the per-commit gate.
   - Alternative: one commit per piece, as stage 1 landed.
2. **Keep the closed plans as files.** Restore the stage 1 plan (`9819088:tasks/todo.md`) as
   `tasks/stage-1-library-plan.md` and the proptest plan (`cc9ef1f:tasks/todo.md`) as
   `tasks/stage-1-proptest-plan.md`, verbatim. Then repoint the links that named them:
   - every `tasks/todo.md` in the four stage 1 tickets, each naming their plan of record;
   - the plan links in prompt histories 0002 to 0004. Quoted prompts stay as they are.

   `tasks/README.md` and `CLAUDE.md` § Knowledge base gain the convention: before replacing
   `todo.md`, keep the closed plan as `tasks/<name>-plan.md` and repoint the links to it.
   - Why: `tasks/` is the evidence of how the project was built, and today three prompt
     histories open whatever plan is current.
   - Alternative: git history only, plus one sentence in `tasks/README.md`.
3. **Exit statuses, one per step that fails:**

   | Status | Meaning |
   |---|---|
   | `0` | the table is written |
   | `2` | not exactly one argument |
   | `3` | the input cannot be opened |
   | `4` | the run stopped: the input cannot be read, or its header lacks `type`, `client` or `tx` |
   | `5` | the table or the report cannot be written |

   Every failure prints one line on stderr. They are documented in `docs/overview.md` § Output and
   `README.md` § Usage. No new record is needed: *The command-line contract* already gives every
   failure its own mapping in `main.rs`, and these values are its detail.
   - Alternative: the `sysexits.h` values: 64 usage, 66 no input, 65 bad header, 74 read or write.
4. **The report's lines on stderr.** One line per count, in the report's order:
   `<count> rejected: <reason>`, then `<count> malformed: <kind>`. Nothing is printed when every
   row was accepted.
   - The phrases sit in the report's sink (Questions on gaps 6), each in one exhaustive `match`,
     so editing an error message never changes the output.
   - Reasons, each naming what the overview's Commands table says:

     | Reason | Line |
     |---|---|
     | `DuplicateTx` | the tx id was used before |
     | `AccountLocked` | the account is locked |
     | `InsufficientFunds` | the amount is above available |
     | `UnknownDeposit` | no such deposit for this client |
     | `NotSettled` | the deposit is already disputed |
     | `NotDisputed` | the deposit is not disputed |
     | `Overflow` | a balance would overflow |

   - Kinds:

     | Kind | Line |
     |---|---|
     | `Unreadable` | a field does not read |
     | `TooManyFields` | more fields than the header |
     | `MissingAmount` | no amount |
     | `InvalidAmount(_)` | `invalid amount: ` then its refusal: not digits with an optional fraction, more than four decimal places, 10^15 or more, zero |

   - Failure lines: `usage: cthulhu <input.csv>`, `error: cannot open <path>: <os error>`,
     `error: <SourceError>`, `error: <SinkError>`, `error: cannot write the report: <os error>`.
   - Alternative: the reasons' names (`AccountLocked`) as labels.
5. **`make run INPUT=<file>`.** The recipe is
   `@$(CARGO) run --locked --bin cthulhu -- $(INPUT)`:
   - `@` keeps make's echo off stdout, so `make run INPUT=x > output.csv` holds only the table;
   - no `INPUT` gives the binary no argument, so its own usage line answers. `$(INPUT)` is
     unquoted for that reason: a path with spaces needs `cargo run -- "<path>"`;
   - the dev profile is kept, as the roadmap measures it.

   `.claude/settings.json`'s `Bash(make run)` becomes `Bash(make run:*)`.
6. **Fixtures through the binary**, in a new `backend/tests/it/cli.rs`.
   - Each input is written beside its test into `env!("CARGO_TARGET_TMPDIR")/cli/`.
   - The worked example is read from `docs/overview.md` itself with `include_str!`. That covers
     its Input, Output and a new Report block, so the overview is the oracle and cannot drift.
   - The library-level worked example in `run.rs` is then covered and is deleted.
7. **A write failure through the binary, on Linux only.** A fixture sends stdout to `/dev/full`
   and expects status `5`.
   - It carries
     `#[cfg_attr(not(target_os = "linux"), ignore = "/dev/full exists only on Linux")]`, so
     macOS compiles and lints it and CI runs it.
   - Elsewhere, the sink's write-failure law covers the path.
   - Alternative: no binary fixture for writes.
8. **The four items stage 1 deferred: none taken, all carried forward** (Deliberately deferred).
   None of them is on the command line's path. The sink prints `is_locked()`, the bool reading
   that exists today, and the domain gains no `Display` but `ClientId`'s.
9. **The CSV spelling gets its own support module**, `backend/tests/it/support/spelling.rs`:
   layouts, `Input` and their strategies. `support/generate.rs` (899 lines) keeps the rows.
   `CLAUDE.md` § Tests names the new module.
10. **Execution: inline.** The tasks run in sequence, each builds on the previous one's
    interfaces, and every commit already passes the code-reviewer agent and ends in a whole-branch
    review.

## Questions on gaps in the records

The records do not settle these. Each one carries a recommendation.

1. **Header columns besides `type`, `client`, `tx` and `amount`.**
   - Recommendation: serde's default. Such a column is ignored, but a row that stops before it
     does not read, because csv asks for a value for every column the header names.
   - The source law pins it with a `note` column, one layout in eight, never left out of a row.
   - `docs/overview.md` § Input says so.
   - Consequence, the commonest real case: a header with a trailing comma
     (`type,client,tx,amount,`) names an empty column. Every row without that trailing comma
     then stops before it and does not read, so the run exits `0` with an empty table and every
     row counted on stderr.
   - Alternative A: project the four known columns out of each record before deserializing, so
     any other column, a trailing comma included, is ignored outright. About 15 lines in
     `csv_io`; a column named twice then reads its first occurrence.
   - Alternative B: refuse such a header as a source error, with a superseding design record.
2. **A header naming a column twice.** When the column is `type`, `client`, `tx` or `amount`,
   serde refuses every row under it as a duplicate field. Every row is then counted malformed
   (`Unreadable`) and the table is empty. A duplicated unknown column is ignored.
   - Recommendation: accept, with one line in § Input, and pin it with no law.
   - Alternative: an `IncompleteHeader`-like source error, with a record.
3. **A row of any length.** csv grows its record buffer for whatever one row holds, while
   *Memory bounded by state, not input* bounds what the engine keeps.
   - Recommendation: defer. A bound needs a reader below csv, such as stage 3's block reader, and
     a design record of its own.
   - Name it in an `Open:` line of roadmap § 1.
4. **An empty input (zero bytes).** It has no header, so the run stops with `IncompleteHeader`,
   status `4` and no table. That is the letter of *Rows parsed into commands at the boundary*.
   - Recommendation: accept; a fixture pins it.
5. **A total the sink cannot print.** `Account::total()` is `None` when `available + held`
   overflows the decimal type, and no record says what the sink does then.
   - Recommendation: `render_csv` stops with `SinkError::Overflow(ClientId)`, status `5`.
   - The amount cap keeps it out of reach, as *Exact decimal amounts with four places* already
     says of the overflow branch.
   - Rows before it stay written, as *The command-line contract* allows of a failure while
     writing.
6. **Where the report's format lives.** *Sources, sinks and projections* puts every output format
   in an adapter, and the composition root hands each view to a sink. The `rust-errors` skill
   takes the labels `main` reports from a `match`, never from `Display`, which is prose and may
   change.
   - Recommendation: a second sink adapter, `text_io::render_report(&Report, &mut impl Write) ->
     io::Result<()>`, with the phrases of Decisions for sign-off 4 in private exhaustive
     `match`es.
   - Consequences: the domain gains no `Display`. `docs/architecture.md` gains the module, a
     row in § Modules and the arrow `Report ─► render_report ─► stderr`.
   - It returns `io::Result` because no foreign crate is involved and its one failure is the
     write's.
   - Alternative: the same function in `main.rs`, as the CLI surface; or beside `render_csv` in
     `csv_io`, which names CSV.
7. **Input that is not UTF-8.** csv checks a whole record's UTF-8 before deserializing it.
   - A header that is not UTF-8 (a UTF-16 file, say) is `IncompleteHeader`: status `4`,
     "the header lacks type, client or tx".
   - A row holding a non-UTF-8 byte anywhere is `Unreadable`, even in a dispute command's
     amount, which is otherwise ignored, or in an unknown column.
   - Recommendation: accept both, pinned by the `NotUtf8` flaw and stated in § Input.
   - Alternative: read `ByteRecord`s and deserialize only the four columns, near the roadmap's
     `ByteRecord` follow-up.

## Global constraints

- Branch `feat/stage-1-cli`, created from `main` (`6e94920`). Never push, merge, rebase or touch a
  remote, and never force anything. Landing is the person's: one squash commit on `main`.
- Commit format: `action(scope): description`, a blank line, `> impact`, no trailer lines.
  - Write the message to a scratch file outside the repository and commit it with `git commit -F`.
  - Stage explicit paths, never `git add -A`.
  - Commits are signed. `origin` is `git@github-ffakenz:ffakenz/cthulhu.git`, so the `ffakenz`
    identity applies.
- Dependencies:
  - only `csv = "1.4"` and `serde = { version = "1", features = ["derive"] }`, added by hand to
    `backend/Cargo.toml` `[dependencies]`; no `cargo add`;
  - `Cargo.lock` gains `csv`, `csv-core` and `ryu` and changes no other pin;
  - `make deps` stays green;
  - `csv` and `serde` are named only in `backend/src/csv_io.rs`.
- `main.rs` is the only file that touches `std::env`, `std::fs`, stdout or stderr. It is also
  the only file under `backend/src` that carries `#[expect(clippy::disallowed_methods, …)]`.
- No test-only code in production. Every helper in `backend/tests/it` is used, or `dead_code`
  fails `make lint`.
- Comments say what the code is and does: never a record number, a plan, a ticket or a
  trade-off. A record is named by its title everywhere outside the decision logs, this plan
  included.
- Never name the organisation behind the external brief, nor the project the decision log's
  style comes from. Never quote, restate or paraphrase the brief in any tracked file or commit
  message.
  - The pattern file lives in the session's scratchpad, built from the three names the project
    memory lists for it, never in the repository.
  - Before each commit, `git grep -n -i -E --untracked -f <pattern file>` and
    `git diff --cached | grep -n -i -E -f <pattern file>` are both silent.
- TDD drives every piece. Every new or rewired law is shown failing once against a deliberate
  break (Law evidence):
  - stage the work first, so a break in a new file shows in `git diff`;
  - apply and undo each break by editing, never by `git checkout`, `git stash` or a copied file,
    so cargo never reuses a broken build;
  - after each undo, `git diff --quiet` exits 0;
  - run each break on its own law alone, with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`, so no
    regression file is written.
- A Phase 0 check that fails stops the work: ask the person before going on.
- `make ci` is the only evidence of green.

## Review focus

The input classes and failure modes most likely to bite a person running the binary, and the test
that pins each:

1. **A file from another editor.** It may start with a byte-order mark, end lines in `\r\n` or
   `\r`, hold blank lines, or lack a final newline. It must read as its plain twin.
   - Pinned by `prop_source_reads_every_spelling_as_the_model_reads_the_rows`, through the
     layout's mark, line ends, blank lines and unended last line.
2. **A dispute row with its amount column left out, left empty, or carrying stray text.** It
   must still read as its dispute command. A deposit or withdrawal without one is `MissingAmount`.
   - Pinned by the same law (a row's `Tail` and `stray`) and by the malformed fixture.
3. **Padded or quoted fields and header names, in any column order.** They must read as the bare
   ones, with any padding inside the quotes. csv opens a quote only at a field's first byte, so
   padding outside the quotes keeps them as text: `deposit, 1, 1, "1.0"` is an invalid amount.
   - Pinned by the same law (each line's `Field`s, the shuffled header).
4. **An output that takes only part of the table** (a closed pipe, a full disk). It must exit
   non-zero, never succeed silently.
   - Pinned by `prop_table_that_cannot_be_written_whole_is_an_error` and the Linux `/dev/full`
     fixture.
5. **An empty input, or a header lacking a column.** The run must stop with status `4` and no
   table.
   - Pinned by `prop_header_lacking_type_client_or_tx_ends_the_source` and the incomplete-header
     fixture.

## File map

| Path | Change | Task |
|---|---|---|
| `tasks/todo.md` | this plan, ticked by every task | 0 onward |
| `tasks/stage-1-library-plan.md`, `tasks/stage-1-proptest-plan.md` | the closed plans, verbatim | 0 |
| `tasks/stage-1-{domain,core,run,memory}.md` | their links to the plan of record | 0 |
| `docs/prompts/0002-*.md`, `0003-*.md`, `0004-*.md` | their links to the plan | 0 |
| `tasks/README.md`, `CLAUDE.md` § Knowledge base | keeping a replaced plan | 0 |
| `backend/src/lib.rs` | `pub mod csv_io;` | 1 |
| `backend/src/csv_io.rs` | `CsvSource` (stub, then real), `CsvRow`, `CsvType`; later `SinkError`, `render_csv` | 1, 3 |
| `backend/Cargo.toml`, `Cargo.lock` | `csv`, `serde` | 1 |
| `backend/src/domain/id.rs` | `Display for ClientId` | 3 |
| `backend/src/run/row.rs` | `RowError::Unreadable`'s doc | 1 |
| `backend/src/text_io.rs`, `backend/src/lib.rs` | new: `render_report`; `pub mod text_io;` | 4 |
| `backend/src/main.rs` | the composition root | 4 |
| `Makefile`, `.claude/settings.json` | `make run INPUT=<file>` | 4 |
| `backend/tests/it/main.rs` | `mod csv_io;` (1), `mod cli;` (4) | 1, 4 |
| `backend/tests/it/support.rs` | `pub(crate) mod spelling;` (1); `summarize`, moved from `run.rs` (2) | 1, 2 |
| `backend/tests/it/support/spelling.rs` | new: layouts, `Input`, strategies; `prefix`, `with_rows` in 2 | 1, 2 |
| `backend/tests/it/support/generate.rs` | `Flaw`; payloads on `InputRow::Unreadable` and `::TooManyFields`; `MAX_ROWS` crate-visible | 1 |
| `backend/tests/it/support/model.rs` | the payload patterns (1); `Engine::after` (2); `table` (3) | 1, 2, 3 |
| `backend/tests/it/csv_io.rs` | new: three source laws (1), two sink laws (3) | 1, 3 |
| `backend/tests/it/run.rs` | laws on `spelling::input()` (2); worked example deleted (4) | 2, 4 |
| `backend/tests/it/cli.rs` | new: the binary fixtures | 4 |
| `docs/overview.md` | the example's Report block (4 red); § Input and § Output (4 green) | 4 |
| `docs/architecture.md` | `SinkError` (3); `text_io` (4) | 3, 4 |
| `docs/roadmap.md` | a row-size bound among stage 1's follow-ups | 4 |
| `CLAUDE.md`, `README.md` | status, layout, usage, tests | 1, 4 |

## Interfaces

### `backend/src/csv_io.rs` (complete, after Task 3)

Task 1 lands the source:
- `REQUIRED`, `CsvRow`, `CsvType`, `TryFrom`, `CsvSource`, `State`, `header`, `row`, `Spoiled`
  and `spoiled`;
- the imports they use;
- the module doc's first clause.

Task 3 adds the rest:
- red: `SinkError` with its impls, the stub `render_csv`, and the imports `error`, `fmt`,
  `Write` and `Accounts`;
- green: `HEADER`, the `Writer` import, the real `render_csv`, and the module doc's clause on
  `render_csv`.

```rust
//! The CSV adapters: `CsvSource` reads rows from CSV text into commands, and `render_csv` writes
//! the accounts table as CSV.

use std::error;
use std::fmt;
use std::io::{Read, Write};
use std::mem;

use csv::{ErrorKind, ReaderBuilder, StringRecord, Trim, Writer};
use serde::Deserialize;

use crate::domain::{Address, Amount, ClientCommand, ClientId, Command, DisputeCommand, TxId};
use crate::run::{Accounts, Row, RowError, SourceError};

/// The columns every header names.
const REQUIRED: [&str; 3] = ["type", "client", "tx"];

/// The accounts table's header.
const HEADER: [&str; 5] = ["client", "available", "held", "total", "locked"];

/// A row as the CSV source reads it: its type, its client and tx id, and its amount's text,
/// `None` when the header names no amount, or the row leaves it empty or stops before it.
#[derive(Debug, Deserialize)]
struct CsvRow<'a> {
    #[serde(rename = "type")]
    kind: CsvType,
    client: u16,
    tx: u32,
    #[serde(borrow)]
    amount: Option<&'a str>,
}

/// The type column: one of the five commands, in lowercase.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CsvType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TryFrom<CsvRow<'_>> for Command {
    type Error = RowError;

    /// The command a row reads as: a deposit or a withdrawal needs an amount that parses; a
    /// dispute command ignores any amount.
    fn try_from(row: CsvRow<'_>) -> Result<Self, Self::Error> {
        let address = Address::new(ClientId::from(row.client), TxId::from(row.tx));
        let amount = || -> Result<Amount, RowError> {
            row.amount.ok_or(RowError::MissingAmount)?.parse().map_err(RowError::InvalidAmount)
        };
        Ok(match row.kind {
            CsvType::Deposit => Self::Client(ClientCommand::Deposit { address, amount: amount()? }),
            CsvType::Withdrawal => {
                Self::Client(ClientCommand::Withdrawal { address, amount: amount()? })
            }
            CsvType::Dispute => Self::Dispute(DisputeCommand::Dispute(address)),
            CsvType::Resolve => Self::Dispute(DisputeCommand::Resolve(address)),
            CsvType::Chargeback => Self::Dispute(DisputeCommand::Chargeback(address)),
        })
    }
}

/// The CSV source over an input: a header naming `type`, `client`, `tx` and optionally `amount`,
/// in any order, then a row per record. Fields are trimmed, and a row may stop before its last
/// column. It yields each row, in order, as the command it reads as or why it is malformed, and
/// ends after a read failure or a header lacking `type`, `client` or `tx`, that error last.
#[derive(Debug)]
pub struct CsvSource<R> {
    reader: csv::Reader<R>,
    record: StringRecord,
    state: State,
}

/// Where a source stands.
#[derive(Debug)]
enum State {
    /// Nothing is read yet.
    Header,
    /// Reading rows under this header.
    Rows(StringRecord),
    /// The input ran out, or a source error was yielded.
    Ended,
}

impl<R: Read> CsvSource<R> {
    /// The source over `input`; nothing is read until the first row is asked for.
    pub fn new(input: R) -> Self {
        let reader = ReaderBuilder::new().trim(Trim::All).flexible(true).from_reader(input);
        Self { reader, record: StringRecord::new(), state: State::Header }
    }
}

impl<R: Read> Iterator for CsvSource<R> {
    type Item = Result<Row, SourceError>;

    fn next(&mut self) -> Option<Self::Item> {
        let headers = match mem::replace(&mut self.state, State::Ended) {
            State::Ended => return None,
            State::Rows(headers) => headers,
            State::Header => match header(&mut self.reader) {
                Ok(headers) => headers,
                Err(error) => return Some(Err(error)),
            },
        };
        let row = match self.reader.read_record(&mut self.record) {
            Ok(false) => return None,
            Ok(true) => row(&self.record, &headers),
            Err(error) => match spoiled(&error) {
                Spoiled::Record => Row::Malformed(RowError::Unreadable),
                Spoiled::Input | Spoiled::Unknown => return Some(Err(SourceError::Unreadable)),
            },
        };
        self.state = State::Rows(headers);
        Some(Ok(row))
    }
}

/// The header, once it names `type`, `client` and `tx`; a header that is not UTF-8 names none, and
/// a failed read is `Unreadable`.
fn header<R: Read>(reader: &mut csv::Reader<R>) -> Result<StringRecord, SourceError> {
    match reader.headers() {
        Ok(headers) if REQUIRED.iter().all(|name| headers.iter().any(|column| column == *name)) => {
            Ok(headers.clone())
        }
        Ok(_) => Err(SourceError::IncompleteHeader),
        Err(error) => Err(match spoiled(&error) {
            Spoiled::Record => SourceError::IncompleteHeader,
            Spoiled::Input | Spoiled::Unknown => SourceError::Unreadable,
        }),
    }
}

/// What `record` reads as under `headers`: the command, or why the row is malformed.
fn row(record: &StringRecord, headers: &StringRecord) -> Row {
    if record.len() > headers.len() {
        return Row::Malformed(RowError::TooManyFields);
    }
    match record.deserialize::<CsvRow<'_>>(Some(headers)).map(Command::try_from) {
        Ok(Ok(command)) => Row::Parsed(command),
        Ok(Err(error)) => Row::Malformed(error),
        Err(_) => Row::Malformed(RowError::Unreadable),
    }
}

/// What a failed read spoils.
#[derive(Debug, Clone, Copy)]
enum Spoiled {
    /// The record being read.
    Record,
    /// The input.
    Input,
    /// A kind of failure `spoiled` does not name.
    Unknown,
}

/// Classifies a failed read: a record that is not UTF-8, that does not deserialize or whose
/// length is refused spoils that record; a failed read, seek or write spoils the input.
fn spoiled(error: &csv::Error) -> Spoiled {
    match error.kind() {
        ErrorKind::Utf8 { .. }
        | ErrorKind::UnequalLengths { .. }
        | ErrorKind::Deserialize { .. } => Spoiled::Record,
        ErrorKind::Io(_) | ErrorKind::Seek | ErrorKind::Serialize(_) => Spoiled::Input,
        _ => Spoiled::Unknown,
    }
}

/// Why the accounts table could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkError {
    /// Writing the output failed.
    Unwritable,
    /// The client's total, available plus held, overflows the decimal type.
    Overflow(ClientId),
}

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unwritable => f.write_str("the output cannot be written"),
            Self::Overflow(client) => write!(f, "the total of client {client} overflows"),
        }
    }
}

impl error::Error for SinkError {}

/// Writes `accounts` to `out` as CSV and flushes it: the header
/// `client,available,held,total,locked`, then a line per account by client id, each balance with
/// four decimals and `locked` as `true` or `false`. Lines written before a failure stay written.
pub fn render_csv(accounts: &Accounts, out: &mut impl Write) -> Result<(), SinkError> {
    let mut writer = Writer::from_writer(out);
    writer.write_record(HEADER).map_err(|_| SinkError::Unwritable)?;
    for (client, account) in accounts.by_client() {
        let total = account.total().ok_or(SinkError::Overflow(client))?;
        let line = [
            client.to_string(),
            account.available().to_string(),
            account.held().to_string(),
            total.to_string(),
            account.is_locked().to_string(),
        ];
        writer.write_record(line).map_err(|_| SinkError::Unwritable)?;
    }
    writer.flush().map_err(|_| SinkError::Unwritable)
}
```

`csv::Writer`'s `Drop` flushes and discards the error, which is why `render_csv` flushes itself
and its write-failure law is broken by deleting that flush.

### `backend/src/domain/id.rs` (Task 3, red)

With `use std::fmt;` at the top:

```rust
/// The client's number, in decimal digits.
impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
```

### `backend/src/text_io.rs` (Task 4, green)

`backend/src/lib.rs` gains `pub mod text_io;`.

```rust
//! The text adapter: `render_report` writes the report as lines of text.

use std::io::{self, Write};

use crate::domain::{AmountError, ClientReason, DisputeReason, Reason};
use crate::run::{Report, RowError};

/// Writes `report` to `out`, a line per count: each rejection reason, then each malformed row
/// kind, as `<count> rejected: <reason>` and `<count> malformed: <kind>`.
pub fn render_report(report: &Report, out: &mut impl Write) -> io::Result<()> {
    for (reason, count) in report.rejected() {
        writeln!(out, "{count} rejected: {}", rejection(reason))?;
    }
    for (kind, count) in report.malformed() {
        writeln!(out, "{count} malformed: {}", malformation(kind))?;
    }
    Ok(())
}

/// How the report names a rejection's reason.
fn rejection(reason: Reason) -> &'static str {
    match reason {
        Reason::Client(ClientReason::DuplicateTx) => "the tx id was used before",
        Reason::Client(ClientReason::AccountLocked) => "the account is locked",
        Reason::Client(ClientReason::InsufficientFunds) => "the amount is above available",
        Reason::Dispute(DisputeReason::UnknownDeposit) => "no such deposit for this client",
        Reason::Dispute(DisputeReason::NotSettled) => "the deposit is already disputed",
        Reason::Dispute(DisputeReason::NotDisputed) => "the deposit is not disputed",
        Reason::Overflow => "a balance would overflow",
    }
}

/// How the report names a malformed row's kind.
fn malformation(kind: RowError) -> &'static str {
    match kind {
        RowError::Unreadable => "a field does not read",
        RowError::TooManyFields => "more fields than the header",
        RowError::MissingAmount => "no amount",
        RowError::InvalidAmount(AmountError::Malformed) => {
            "invalid amount: not digits with an optional fraction"
        }
        RowError::InvalidAmount(AmountError::TooPrecise) => {
            "invalid amount: more than four decimal places"
        }
        RowError::InvalidAmount(AmountError::TooLarge) => "invalid amount: 10^15 or more",
        RowError::InvalidAmount(AmountError::Zero) => "invalid amount: zero",
    }
}
```

### `backend/src/main.rs` (Task 4)

```rust
//! The composition root: reads the one argument, opens the input it names, runs the input over
//! in-memory stores, writes the accounts table to stdout and the report to stderr, and maps every
//! failure to an exit status.

use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use backend::csv_io::{CsvSource, SinkError, render_csv};
use backend::memory::{MemoryLedger, MemorySeenIds};
use backend::run::{RunError, run};
use backend::text_io::render_report;

/// Runs the input the arguments name. A failure's line goes to stderr, and its exit status is
/// returned whether or not that line could be written.
fn main() -> ExitCode {
    #[expect(clippy::disallowed_methods, reason = "the composition root reads the arguments once")]
    let arguments = std::env::args_os();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match execute(arguments, &mut stdout, &mut stderr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => match writeln!(stderr, "{failure}") {
            Ok(()) | Err(_) => failure.code(),
        },
    }
}

/// Opens the input the arguments name and runs it: the table to `stdout` once the run succeeds,
/// then the report to `stderr`.
fn execute(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), Failure> {
    let path = input(arguments).ok_or(Failure::Usage)?;
    let file = File::open(&path).map_err(|error| Failure::Open { path, error })?;
    let summary =
        run(CsvSource::new(file), &mut MemoryLedger::default(), &mut MemorySeenIds::default())
            .map_err(Failure::Run)?;
    render_csv(&summary.accounts, stdout).map_err(Failure::Table)?;
    render_report(&summary.report, stderr).map_err(Failure::Report)
}

/// The input's path: the one argument after the program's name; `None` for any other count.
fn input(arguments: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut given = arguments.into_iter().skip(1);
    match (given.next(), given.next()) {
        (Some(path), None) => Some(PathBuf::from(path)),
        (None, _) | (Some(_), Some(_)) => None,
    }
}

/// Why the program failed, by the step that failed.
#[derive(Debug)]
enum Failure {
    /// Not exactly one argument.
    Usage,
    /// The input could not be opened.
    Open { path: PathBuf, error: io::Error },
    /// The run stopped.
    Run(RunError<Infallible, Infallible>),
    /// The accounts table could not be written.
    Table(SinkError),
    /// The report could not be written.
    Report(io::Error),
}

impl Failure {
    /// The exit status of the step that failed.
    fn code(&self) -> ExitCode {
        ExitCode::from(match self {
            Self::Usage => 2,
            Self::Open { .. } => 3,
            Self::Run(_) => 4,
            Self::Table(_) | Self::Report(_) => 5,
        })
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => f.write_str("usage: cthulhu <input.csv>"),
            Self::Open { path, error } => {
                write!(f, "error: cannot open {}: {error}", path.display())
            }
            Self::Run(error) => write!(f, "error: {error}"),
            Self::Table(error) => write!(f, "error: {error}"),
            Self::Report(error) => write!(f, "error: cannot write the report: {error}"),
        }
    }
}
```

`Makefile`:

```make
run: ## Run the cthulhu binary on INPUT, the table on stdout: make run INPUT=<file>
	@$(CARGO) run --locked --bin cthulhu -- $${INPUT:+"$$INPUT"}
```

## Test code

### `backend/tests/it/support/generate.rs` changes (Task 1)

- `const MAX_ROWS` becomes `pub(crate) const MAX_ROWS`.
- `InputRow::Unreadable` becomes `Unreadable(Flaw)`, documented as "A row that does not read,
  and how". `InputRow::TooManyFields` becomes
  `TooManyFields { first: &'static str, rest: &'static [&'static str] }`, documented as "A row
  with more fields than the header names: `first`, then `rest`, after a full row". The shape
  admits no empty list of extra fields.
- `Shape::Unreadable` becomes `Unreadable(Flaw)` and `Shape::TooManyFields` becomes
  `TooManyFields { first, rest }`. `shape()` draws them as
  `1 => flaw().prop_map(Shape::Unreadable)` and
  `1 => select(&EXTRA_FIELDS).prop_map(|(first, rest)| Shape::TooManyFields { first, rest })`,
  and `Draft::malformed_row` passes each payload on.
- `row` matches `InputRow::Unreadable(_)` and `InputRow::TooManyFields(_)`, and so does
  `model.rs`'s `Engine::take`.
- New, beside `Shape`:

```rust
/// How a row fails to read.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Flaw {
    /// Its type is none of the five commands, as spelled.
    Type(&'static str),
    /// Its client is not a `u16`, as spelled.
    Client(&'static str),
    /// Its tx is not a `u32`, as spelled.
    Tx(&'static str),
    /// It stops before a column every header names.
    Short,
    /// It holds a byte that is not UTF-8.
    NotUtf8,
    /// It holds nothing but spaces.
    Blank,
}

/// Types no reader accepts: a capital, all capitals, an unknown word, a plural and nothing.
const BAD_TYPES: [&str; 5] = ["Deposit", "DEPOSIT", "transfer", "deposits", ""];

/// Clients no reader accepts: past `u16`, negative, a fraction, a word, nothing, and a hex prefix
/// with a capital X.
const BAD_CLIENTS: [&str; 6] = ["65536", "-1", "1.0", "x", "", "0X1"];

/// Tx ids no reader accepts: past `u32`, negative, an exponent, a word, nothing, and hex past
/// `u32`.
const BAD_TXS: [&str; 6] = ["4294967296", "-1", "1e3", "x", "", "0x100000000"];

/// The fields after a full row with too many, the first and the rest: an empty one, a word, two
/// empty ones, an amount and a word.
const EXTRA_FIELDS: [(&str, &[&str]); 4] = [("", &[]), ("x", &[]), ("", &[""]), ("1.5", &["note"])];

/// A flaw, each kind as likely.
pub(crate) fn flaw() -> impl Strategy<Value = Flaw> {
    prop_oneof![
        select(&BAD_TYPES).prop_map(Flaw::Type),
        select(&BAD_CLIENTS).prop_map(Flaw::Client),
        select(&BAD_TXS).prop_map(Flaw::Tx),
        Just(Flaw::Short),
        Just(Flaw::NotUtf8),
        Just(Flaw::Blank),
    ]
}
```

### `backend/tests/it/support/spelling.rs` (Task 1; `prefix` and `with_rows` in Task 2)

```rust
//! Rows spelled as the CSV text a source reads: a header naming its columns in any order, then a
//! line per row; every field bare or quoted and padded or not, every line but an unended last one
//! ending in `\n`, `\r\n` or `\r`, blank lines before any line and after the last, and a
//! byte-order mark or none.

use proptest::arbitrary::any;
use proptest::array::uniform7;
use proptest::bool::weighted;
use proptest::collection::vec;
use proptest::option;
use proptest::prop_oneof;
use proptest::sample::{Index, select};
use proptest::strategy::{Just, Strategy};

use super::generate::{self, Flaw, InputRow, Kind, MAX_ROWS};

/// A column a header names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    /// `type`.
    Type,
    /// `client`.
    Client,
    /// `tx`.
    Tx,
    /// `amount`.
    Amount,
    /// `note`, whose text every reader ignores.
    Note,
}

impl Column {
    /// The column's name in the header.
    fn name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Client => "client",
            Self::Tx => "tx",
            Self::Amount => "amount",
            Self::Note => "note",
        }
    }
}

/// The columns a complete header names.
const REQUIRED: [Column; 3] = [Column::Type, Column::Client, Column::Tx];

/// The ways a header lacks `type`, `client` or `tx`: each lacks the columns listed.
const LACKING: [&[Column]; 7] = [
    &[Column::Type],
    &[Column::Client],
    &[Column::Tx],
    &[Column::Type, Column::Client],
    &[Column::Type, Column::Tx],
    &[Column::Client, Column::Tx],
    &[Column::Type, Column::Client, Column::Tx],
];

/// Whether a field is quoted. Text holding a comma, a quote or a line break is quoted either way.
#[derive(Debug, Clone, Copy)]
enum Quoting {
    Bare,
    Quoted,
}

/// How a field is written: its quoting, and the whitespace before and after its text, inside any
/// quotes; every reader trims it.
#[derive(Debug, Clone, Copy)]
struct Field {
    quoting: Quoting,
    before: &'static str,
    after: &'static str,
}

/// A bare field with no padding.
const BARE: Field = Field { quoting: Quoting::Bare, before: "", after: "" };

/// Whitespace a field may carry around its text.
const PADDING: [&str; 3] = [" ", "  ", "\t"];

/// How a line ends: each a terminator every reader accepts.
#[derive(Debug, Clone, Copy)]
enum End {
    Lf,
    CrLf,
    Cr,
}

/// Whether a blank line, which every reader skips, is written, and how it ends.
#[derive(Debug, Clone, Copy)]
enum Blank {
    Absent,
    Line(End),
}

/// How an id is spelled; every reader reads `7`, `+7`, `07` and `0x7` as 7.
#[derive(Debug, Clone, Copy)]
enum Spelling {
    Plain,
    Signed,
    Padded,
    Hex,
}

/// How a row's ids are spelled.
#[derive(Debug, Clone, Copy)]
struct Ids {
    client: Spelling,
    tx: Spelling,
}

/// Whether a row whose header ends with `amount` writes its empty amount or leaves it out.
#[derive(Debug, Clone, Copy)]
enum Tail {
    Written,
    Left,
}

/// The choices a line's spelling reads. Every line reads how each of its fields is written, how
/// it ends and whether a blank line comes first. A row also reads how its ids are spelled,
/// whether it leaves out an empty trailing amount, the stray amount a dispute command carries
/// (which every reader ignores) and its `note` column's text.
#[derive(Debug, Clone)]
struct Line {
    fields: [Field; 7],
    end: End,
    blank: Blank,
    ids: Ids,
    tail: Tail,
    stray: Option<&'static str>,
    note: &'static str,
}

/// Amounts a dispute command may carry: every reader ignores them.
const STRAY_AMOUNTS: [&str; 4] = ["1.5", "-1", "abc", "0"];

/// Texts of the `note` column: empty, a word, a comma, quotes and a line break.
const NOTES: [&str; 5] = ["", "memo", "a, b", "say \"hi\"", "two\nlines"];

/// Whether the text starts with a UTF-8 byte-order mark, which every reader skips.
#[derive(Debug, Clone, Copy)]
enum Mark {
    Unmarked,
    Marked,
}

/// Whether the last line ends with its terminator.
#[derive(Debug, Clone, Copy)]
enum Last {
    Ended,
    Unended,
}

/// How rows are spelled: a mark or none, the header's columns and its line, a line for each of
/// the first rows (the rest take the header's), whether the last line ends, and whether a blank
/// line follows it.
#[derive(Debug, Clone)]
struct Layout {
    mark: Mark,
    columns: Vec<Column>,
    header: Line,
    lines: Vec<Line>,
    last: Last,
    trailer: Blank,
}

impl Layout {
    /// The layout whose header lacks `lacking`.
    fn lacking(mut self, lacking: &[Column]) -> Self {
        self.columns.retain(|column| !lacking.contains(column));
        self
    }
}

/// Rows as the source must read them under a layout, and the layout that spells them.
#[derive(Debug, Clone)]
pub(crate) struct Input {
    rows: Vec<InputRow>,
    layout: Layout,
}

impl Input {
    /// `rows` under `layout`; under a header without `amount`, a row carrying an amount holds a
    /// field more than the header names.
    fn new(rows: Vec<InputRow>, layout: Layout) -> Self {
        let names_amount = layout.columns.contains(&Column::Amount);
        let rows = rows
            .into_iter()
            .map(|row| match row {
                InputRow::Fields { amount: Some(_), .. } if !names_amount => {
                    InputRow::TooManyFields { first: "", rest: &[] }
                }
                InputRow::Fields { .. }
                | InputRow::Unreadable(_)
                | InputRow::TooManyFields { .. } => row,
            })
            .collect();
        Self { rows, layout }
    }

    /// The rows, as the source must read them.
    pub(crate) fn rows(&self) -> &[InputRow] {
        &self.rows
    }

    /// The rows spelled as CSV text.
    pub(crate) fn text(&self) -> Vec<u8> {
        let Layout { mark, columns, header, lines, last, trailer } = &self.layout;
        let mut text = Vec::new();
        match mark {
            Mark::Unmarked => {}
            Mark::Marked => text.extend_from_slice("\u{feff}".as_bytes()),
        }
        let names = columns.iter().map(|column| column.name().as_bytes().to_vec()).collect();
        let mut spelled: Vec<(Vec<Vec<u8>>, &Line)> = vec![(names, header)];
        for (position, row) in self.rows.iter().enumerate() {
            let line = lines.get(position).unwrap_or(header);
            spelled.push((row_fields(row, columns, line, position), line));
        }
        let count = spelled.len();
        for (index, (fields, line)) in spelled.iter().enumerate() {
            let ending = if index.saturating_add(1) < count { Last::Ended } else { *last };
            write_line(&mut text, fields, line, ending);
        }
        match trailer {
            Blank::Absent => {}
            Blank::Line(end) => text.extend_from_slice(terminator(*end)),
        }
        text
    }

    /// The first `end` rows under the same layout; `None` when there are fewer than `end` rows.
    pub(crate) fn prefix(&self, end: usize) -> Option<Self> {
        self.rows.get(..end).map(|rows| self.with_rows(rows.to_vec()))
    }

    /// `rows` under this input's layout.
    pub(crate) fn with_rows(&self, rows: Vec<InputRow>) -> Self {
        Self::new(rows, self.layout.clone())
    }
}

/// The fields `row` is written with under `columns`, as text before quoting and padding.
fn row_fields(row: &InputRow, columns: &[Column], line: &Line, position: usize) -> Vec<Vec<u8>> {
    match row {
        InputRow::Fields { kind, client, tx, amount } => {
            let mut fields: Vec<Vec<u8>> = columns
                .iter()
                .map(|column| match column {
                    Column::Type => kind_name(*kind).as_bytes().to_vec(),
                    Column::Client => id(u32::from(*client), line.ids.client),
                    Column::Tx => id(*tx, line.ids.tx),
                    Column::Amount => {
                        amount_text(*kind, amount.as_deref(), line.stray).as_bytes().to_vec()
                    }
                    Column::Note => line.note.as_bytes().to_vec(),
                })
                .collect();
            let leaves_amount = match line.tail {
                Tail::Written => false,
                Tail::Left => {
                    columns.last() == Some(&Column::Amount) && fields.last().is_some_and(Vec::is_empty)
                }
            };
            if leaves_amount {
                let _ = fields.pop();
            }
            fields
        }
        InputRow::Unreadable(flaw) => unreadable(columns, *flaw, position),
        InputRow::TooManyFields { first, rest } => {
            let mut fields = plain(columns, position);
            fields.push(first.as_bytes().to_vec());
            fields.extend(rest.iter().map(|text| text.as_bytes().to_vec()));
            fields
        }
    }
}

/// A deposit of 1 by client 1 on tx `position`, under `columns`.
fn plain(columns: &[Column], position: usize) -> Vec<Vec<u8>> {
    columns
        .iter()
        .map(|column| match column {
            Column::Type => b"deposit".to_vec(),
            Column::Client | Column::Amount => b"1".to_vec(),
            Column::Tx => position.to_string().into_bytes(),
            Column::Note => Vec::new(),
        })
        .collect()
}

/// The fields of a row that does not read: a plain row under `columns`, with `flaw` applied.
fn unreadable(columns: &[Column], flaw: Flaw, position: usize) -> Vec<Vec<u8>> {
    let mut fields = plain(columns, position);
    match flaw {
        Flaw::Type(text) => replace(&mut fields, columns, Column::Type, text.as_bytes()),
        Flaw::Client(text) => replace(&mut fields, columns, Column::Client, text.as_bytes()),
        Flaw::Tx(text) => replace(&mut fields, columns, Column::Tx, text.as_bytes()),
        Flaw::Short => {
            let kept = columns.iter().rposition(|column| REQUIRED.contains(column)).unwrap_or(0);
            fields.truncate(kept);
        }
        Flaw::NotUtf8 => replace(&mut fields, columns, Column::Type, b"deposit\xff"),
        Flaw::Blank => fields = vec![b"   ".to_vec()],
    }
    fields
}

/// Writes `text` in the field of `column`, when `columns` has one.
fn replace(fields: &mut [Vec<u8>], columns: &[Column], column: Column, text: &[u8]) {
    let at = columns.iter().position(|named| *named == column);
    if let Some(field) = at.and_then(|at| fields.get_mut(at)) {
        *field = text.to_vec();
    }
}

/// The type column's text for `kind`.
fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Deposit => "deposit",
        Kind::Withdrawal => "withdrawal",
        Kind::Dispute => "dispute",
        Kind::Resolve => "resolve",
        Kind::Chargeback => "chargeback",
    }
}

/// `value` as `spelling` writes it.
fn id(value: u32, spelling: Spelling) -> Vec<u8> {
    match spelling {
        Spelling::Plain => value.to_string(),
        Spelling::Signed => format!("+{value}"),
        Spelling::Padded => format!("0{value}"),
        Spelling::Hex => format!("0x{value:x}"),
    }
    .into_bytes()
}

/// The amount column's text: the row's amount, else a dispute command's stray amount, else
/// nothing.
fn amount_text<'a>(kind: Kind, amount: Option<&'a str>, stray: Option<&'static str>) -> &'a str {
    match amount {
        Some(text) => text,
        None => match kind {
            Kind::Dispute | Kind::Resolve | Kind::Chargeback => stray.unwrap_or(""),
            Kind::Deposit | Kind::Withdrawal => "",
        },
    }
}

/// Writes `fields` as one line: a blank line first if `line` says so, each field as `line`
/// writes it, then its terminator unless `ending` leaves the line unended.
fn write_line(text: &mut Vec<u8>, fields: &[Vec<u8>], line: &Line, ending: Last) {
    match line.blank {
        Blank::Absent => {}
        Blank::Line(blank) => text.extend_from_slice(terminator(blank)),
    }
    for (index, content) in fields.iter().enumerate() {
        if index > 0 {
            text.push(b',');
        }
        write_field(text, content, line.fields.get(index).copied().unwrap_or(BARE));
    }
    match ending {
        Last::Ended => text.extend_from_slice(terminator(line.end)),
        Last::Unended => {}
    }
}

/// Writes `content` as `field` says, quoted when asked or when it holds a comma, a quote or a
/// line break, its padding inside any quotes and every quote doubled.
fn write_field(text: &mut Vec<u8>, content: &[u8], field: Field) {
    let quoted = match field.quoting {
        Quoting::Quoted => true,
        Quoting::Bare => content.iter().any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n')),
    };
    if quoted {
        text.push(b'"');
    }
    text.extend_from_slice(field.before.as_bytes());
    for &byte in content {
        if byte == b'"' {
            text.push(b'"');
        }
        text.push(byte);
    }
    text.extend_from_slice(field.after.as_bytes());
    if quoted {
        text.push(b'"');
    }
}

/// The bytes `end` writes.
fn terminator(end: End) -> &'static [u8] {
    match end {
        End::Lf => b"\n",
        End::CrLf => b"\r\n",
        End::Cr => b"\r",
    }
}

/// Generated rows under a layout whose header names `type`, `client` and `tx`.
pub(crate) fn input() -> impl Strategy<Value = Input> {
    (layout(), generate::rows()).prop_map(|(layout, rows)| Input::new(rows, layout))
}

/// Generated rows with up to two unreadable rows spliced in at drawn places, under a layout whose
/// header names `type`, `client` and `tx`: each flaw a row can have, often.
pub(crate) fn flawed() -> impl Strategy<Value = Input> {
    (layout(), generate::rows(), vec((any::<Index>(), generate::flaw()), 0..=2)).prop_map(
        |(layout, mut rows, flaws)| {
            for (place, flaw) in flaws {
                rows.insert(place.index(rows.len().saturating_add(1)), InputRow::Unreadable(flaw));
            }
            Input::new(rows, layout)
        },
    )
}

/// Generated rows under such a layout with no byte-order mark: a reader whose first read holds
/// only the mark reads it as an empty input.
pub(crate) fn unmarked() -> impl Strategy<Value = Input> {
    input().prop_map(|input| Input { layout: Layout { mark: Mark::Unmarked, ..input.layout }, ..input })
}

/// Generated rows under a header that lacks at least one of `type`, `client` and `tx`.
pub(crate) fn incomplete() -> impl Strategy<Value = Input> {
    (layout(), generate::rows(), select(&LACKING))
        .prop_map(|(layout, rows, lacking)| Input::new(rows, layout.lacking(lacking)))
}

/// A layout: the four columns shuffled; `amount` left out and `note` inserted at a drawn place,
/// one draw in eight each; a byte-order mark one draw in eight; up to a line per row a run may
/// hold; the last line unended one draw in four; a trailing blank line one draw in eight.
fn layout() -> impl Strategy<Value = Layout> {
    let order =
        Just(vec![Column::Type, Column::Client, Column::Tx, Column::Amount]).prop_shuffle();
    (
        weighted(0.125),
        order,
        weighted(0.125),
        option::weighted(0.125, any::<Index>()),
        line(),
        vec(line(), 0..=MAX_ROWS),
        weighted(0.25),
        prop_oneof![7 => Just(Blank::Absent), 1 => ending().prop_map(Blank::Line)],
    )
        .prop_map(
            |(marked, mut columns, without_amount, note, header, lines, unended, trailer)| {
                if without_amount {
                    columns.retain(|column| *column != Column::Amount);
                }
                if let Some(place) = note {
                    columns.insert(place.index(columns.len().saturating_add(1)), Column::Note);
                }
                Layout {
                    mark: if marked { Mark::Marked } else { Mark::Unmarked },
                    columns,
                    header,
                    lines,
                    last: if unended { Last::Unended } else { Last::Ended },
                    trailer,
                }
            },
        )
}

/// A line's choices: a blank line before it one draw in eight; ids spelled plainly five draws in
/// eight; an empty trailing amount left out one draw in two; a stray amount one draw in four.
fn line() -> impl Strategy<Value = Line> {
    (
        uniform7(field()),
        ending(),
        prop_oneof![7 => Just(Blank::Absent), 1 => ending().prop_map(Blank::Line)],
        (id_spelling(), id_spelling()).prop_map(|(client, tx)| Ids { client, tx }),
        prop_oneof![Just(Tail::Written), Just(Tail::Left)],
        option::weighted(0.25, select(&STRAY_AMOUNTS)),
        select(&NOTES),
    )
        .prop_map(|(fields, end, blank, ids, tail, stray, note)| Line {
            fields,
            end,
            blank,
            ids,
            tail,
            stray,
            note,
        })
}

/// A field: quoted one draw in four, and each side padded one draw in four.
fn field() -> impl Strategy<Value = Field> {
    let padding = || prop_oneof![3 => Just(""), 1 => select(&PADDING)];
    (prop_oneof![3 => Just(Quoting::Bare), 1 => Just(Quoting::Quoted)], padding(), padding())
        .prop_map(|(quoting, before, after)| Field { quoting, before, after })
}

/// A line end: `\n` one draw in two, `\r\n` and `\r` one in four each.
fn ending() -> impl Strategy<Value = End> {
    prop_oneof![2 => Just(End::Lf), 1 => Just(End::CrLf), 1 => Just(End::Cr)]
}

/// An id's spelling: plain five draws in eight, each other one in eight.
fn id_spelling() -> impl Strategy<Value = Spelling> {
    prop_oneof![
        5 => Just(Spelling::Plain),
        1 => Just(Spelling::Signed),
        1 => Just(Spelling::Padded),
        1 => Just(Spelling::Hex),
    ]
}
```

`Line` is a product of draws, one for each choice a line's spelling may read. The row's shape
picks which choices it reads, so every shrunk `Line` still spells every row. This is the
choices-then-interpretation shape of `generate::rows`. If a reviewer asks for sums, the answer is
that a line's shape is known only once it meets its row.

### `backend/tests/it/support/model.rs` (`Engine::after` in Task 2, `table` in Task 3)

In Task 2's refactor commit, `printed` comes to fold through `Engine::after(rows)`, the loop it
holds today. Task 3's red commit then adds only `table`:

```rust
impl Engine {
    /// The naive engine after taking `rows`, in order.
    fn after(rows: &[InputRow]) -> Self {
        let mut engine = Self::default();
        for input in rows {
            engine.take(input);
        }
        engine
    }
}

/// The accounts table the naive engine prints for `rows`: the header, then a line per account by
/// client id.
pub(crate) fn table(rows: &[InputRow]) -> String {
    let lines: Vec<String> = Engine::after(rows)
        .accounts
        .iter()
        .map(|(client, &Account { available, held, locked })| {
            let total = available.plus(held);
            format!("{client},{available},{held},{total},{locked}\n")
        })
        .collect();
    format!("client,available,held,total,locked\n{}", lines.concat())
}
```

### `backend/tests/it/csv_io.rs` (laws: three in Task 1, two in Task 3)

```rust
//! Laws of the CSV adapters: every spelling of the rows reads as the model reads them; a header
//! lacking a column, or a failed read, ends the source with its error; the table printed is the
//! naive model's, and one the output takes only in part is an error.

use std::io::{self, Read, Write};

use backend::core::open;
use backend::csv_io::{CsvSource, SinkError, render_csv};
use backend::domain::ClientId;
use backend::run::{Accounts, Row, SourceError};
use proptest::arbitrary::any;
use proptest::sample::Index;

use crate::support::spelling::{self, Input};
use crate::support::{self, generate, model};

/// A header whose first name holds a byte that is not UTF-8, then a row.
const NOT_UTF8_HEADER: &[u8] = b"type\xff,client,tx\ndeposit,1,1\n";

/// What the model makes of `input`'s rows, as a source yields them.
fn expected(input: &Input) -> Vec<Result<Row, SourceError>> {
    input.rows().iter().map(|row| Ok(generate::row(row))).collect()
}

/// What the CSV source reads from `input`'s text, at most two items past its rows.
fn read(input: &Input) -> Vec<Result<Row, SourceError>> {
    let bound = input.rows().len().saturating_add(2);
    CsvSource::new(input.text().as_slice()).take(bound).collect()
}

/// What the CSV source reads from `input`'s text when reading fails at the byte `cut` picks, at
/// most two items past its rows.
fn read_broken(input: &Input, cut: Index) -> Vec<Result<Row, SourceError>> {
    let text = input.text();
    let left = text.get(..cut.index(text.len().saturating_add(1))).unwrap_or(&text);
    let bound = input.rows().len().saturating_add(2);
    CsvSource::new(Breaking { left }).take(bound).collect()
}

/// A reader that yields `left`, then fails every read.
#[derive(Debug)]
struct Breaking<'a> {
    left: &'a [u8],
}

impl Read for Breaking<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.left.is_empty() {
            return Err(io::Error::other("the input broke"));
        }
        self.left.read(buffer)
    }
}

/// An output that takes at most `room` bytes: a write that does not fit is cut short, and every
/// write once it is full fails.
#[derive(Debug)]
struct Full {
    written: Vec<u8>,
    room: usize,
}

impl Write for Full {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let space = self.room.saturating_sub(self.written.len());
        if space == 0 && !bytes.is_empty() {
            return Err(io::Error::other("the output is full"));
        }
        let taken = bytes.get(..space.min(bytes.len())).unwrap_or_default();
        self.written.extend_from_slice(taken);
        Ok(taken.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn prop_source_reads_every_spelling_as_the_model_reads_the_rows() {
    let law = support::for_all(
        "the source reads each row, however it is spelled, as the model reads it, in order",
        spelling::flawed(),
        |input| read(input) == expected(input),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_header_lacking_type_client_or_tx_ends_the_source() {
    let law = support::for_all(
        "a header lacking type, client or tx ends the source with that error, before any row",
        spelling::incomplete(),
        |input| read(input) == [Err(SourceError::IncompleteHeader)],
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn a_header_that_is_not_utf8_lacks_its_columns() {
    let items: Vec<Result<Row, SourceError>> = CsvSource::new(NOT_UTF8_HEADER).collect();
    assert_eq!(items, [Err(SourceError::IncompleteHeader)]);
}

#[test]
fn prop_read_failure_ends_the_source() {
    let law = support::for_all(
        "a read failure ends the source with Unreadable, after rows the model reads as they were",
        (spelling::unmarked(), any::<Index>()),
        |(input, cut)| match read_broken(input, *cut).split_last() {
            Some((last, before)) => {
                *last == Err(SourceError::Unreadable) && expected(input).starts_with(before)
            }
            None => false,
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_table_is_the_naive_models() {
    let law = support::for_all(
        "the table printed for any input is the naive model's, header first, accounts by client id",
        spelling::input(),
        |input| {
            support::summarize(CsvSource::new(input.text().as_slice())).is_ok_and(|summary| {
                let mut table = Vec::new();
                render_csv(&summary.accounts, &mut table).is_ok()
                    && table == model::table(input.rows()).into_bytes()
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// `accounts`, and `extra` more opened accounts from client 65535 down: enough of them make a
/// table longer than the writer's buffer, so writes fail before the final flush.
fn widened(accounts: &Accounts, extra: u16) -> Accounts {
    let opened = (0..extra).map(|n| (ClientId::from(u16::MAX.saturating_sub(n)), *open().account()));
    accounts.by_client().chain(opened).collect()
}

#[test]
fn prop_table_that_cannot_be_written_whole_is_an_error() {
    let law = support::for_all(
        "a table, never empty, that the output takes only in part is an Unwritable error; one it takes whole succeeds",
        (spelling::input(), 0..=400_u16, any::<Index>()),
        |(input, extra, room)| {
            let Ok(summary) = support::summarize(CsvSource::new(input.text().as_slice())) else {
                return false;
            };
            let accounts = widened(&summary.accounts, *extra);
            let mut whole = Vec::new();
            if render_csv(&accounts, &mut whole).is_err() || whole.is_empty() {
                return false;
            }
            let room = room.index(whole.len().saturating_add(1));
            let mut out = Full { written: Vec::new(), room };
            match render_csv(&accounts, &mut out) {
                Ok(()) => room == whole.len() && out.written == whole,
                Err(error) => room < whole.len() && error == SinkError::Unwritable,
            }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
```

### `backend/tests/it/run.rs` (Task 2)

- `summarize` moves to `support.rs`, unchanged, as
  `pub(crate) fn summarize(source: impl IntoIterator<Item = Result<Row, SourceError>>) -> Result<Summary, RunError<Infallible, Infallible>>`.
- A new helper beside the laws:

  ```rust
  /// `input` through the CSV source and `run`, over fresh stores.
  fn summarized(input: &Input) -> Result<Summary, RunError<Infallible, Infallible>> {
      support::summarize(CsvSource::new(input.text().as_slice()))
  }
  ```

- Each law but `prop_source_error_stops_the_run_with_no_summary` takes `spelling::input()` instead
  of `generate::rows()`. It reads `input.rows()` where it read `rows`, and calls `summarized`
  where it called `summarize(source(…))`:
  - the rejection law folds over `input.prefix(end)`;
  - the malformed-row law runs `input.with_rows(parsed)`.
- The broken-source law calls `support::summarize(rest.by_ref())`. `source` and `BrokenSource`
  stay for it, since it states `run`'s contract with any source.
- The reordering law:

  ```rust
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
  ```

  Its body compares
  `summarized(&input.with_rows(original.rows().to_vec()))` with the same for `interleaved`.
- Imports:
  - drop `backend::memory::{MemoryLedger, MemorySeenIds}` and `run`, which leave with
    `summarize`;
  - add `backend::csv_io::CsvSource` and `crate::support::spelling::{self, Input}`;
  - `support.rs` takes the imports `summarize` needs.
- `git diff -w` shows only those changes.

### `backend/tests/it/cli.rs` (Task 4)

```rust
//! The command-line contract through the binary: one argument, the table on stdout only once the
//! run succeeds, the counts on stderr, and an exit status for every failure.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use backend::csv_io::SinkError;
use backend::run::SourceError;

/// The overview, whose worked example is the contract's own.
const OVERVIEW: &str = include_str!("../../../docs/overview.md");

/// Every rejection reason but overflow, and every malformed kind, a malformed row first.
const REJECTED_AND_MALFORMED: &str = "\
type, client, tx, amount
deposit, 3, 13, 0
deposit, 1, 1, 10.0
deposit, 1, 1, 5.0
withdrawal, 1, 2, 20.0
dispute, 1, 1,
dispute, 1, 1,
resolve, 1, 3,
resolve, 1, 1,
chargeback, 1, 1,
deposit, 2, 4, 1.5
dispute, 2, 4,
chargeback, 2, 4,
deposit, 2, 5, 1.0
Deposit, 3, 6, 1.0
deposit, 70000, 7, 1.0
deposit, 3, 8, 1.0, extra
withdrawal, 3, 9,
deposit, 3, 10, abc
deposit, 3, 11, 1.23456
deposit, 3, 12, 1000000000000000
";

/// The table for `REJECTED_AND_MALFORMED`: client 3, named only by malformed rows, has no account.
const REJECTED_AND_MALFORMED_TABLE: &str = "\
client,available,held,total,locked
1,10.0000,0.0000,10.0000,false
2,0.0000,0.0000,0.0000,true
";

/// The report for `REJECTED_AND_MALFORMED`.
const REJECTED_AND_MALFORMED_REPORT: &str = "\
1 rejected: the tx id was used before
1 rejected: the account is locked
1 rejected: the amount is above available
1 rejected: no such deposit for this client
1 rejected: the deposit is already disputed
1 rejected: the deposit is not disputed
2 malformed: a field does not read
1 malformed: more fields than the header
1 malformed: no amount
1 malformed: invalid amount: not digits with an optional fraction
1 malformed: invalid amount: more than four decimal places
1 malformed: invalid amount: 10^15 or more
1 malformed: invalid amount: zero
";

/// A header and no rows.
const HEADER_ONLY: &str = "type,client,tx,amount\n";

/// An empty input, which has no header.
const EMPTY: &str = "";

/// A header without `tx`, then a row.
const NO_TX: &str = "type, client, amount\ndeposit, 1, 1.0\n";

/// One deposit.
const ONE_DEPOSIT: &str = "type,client,tx,amount\ndeposit,1,1,1.0\n";

/// Runs the binary with `arguments`.
fn cthulhu(arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).output()
}

/// Runs the binary with `arguments`, its stdout written to `stdout`.
fn cthulhu_into(stdout: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stdout(stdout).output()
}

/// Runs the binary with `arguments`, its stderr written to `stderr`.
fn cthulhu_reporting_into(stderr: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stderr(stderr).output()
}

/// The writing end of a pipe whose reading end is already closed: every write to it fails.
fn closed_pipe() -> io::Result<io::PipeWriter> {
    let (reader, writer) = io::pipe()?;
    drop(reader);
    Ok(writer)
}

/// These tests' scratch directory, created if missing.
fn scratch() -> io::Result<PathBuf> {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

/// A file named `name` in the scratch directory, holding `text`.
fn input(name: &str, text: &str) -> io::Result<PathBuf> {
    let path = scratch()?.join(name);
    fs::write(&path, text)?;
    Ok(path)
}

/// The code block after the line `label` in the overview.
fn block(label: &str) -> Option<&'static str> {
    let (_, after) = OVERVIEW.split_once(&format!("{label}\n\n```text\n"))?;
    after.split_once("```").map(|(block, _)| block)
}

#[test]
fn the_overview_example_prints_its_table_and_report() {
    let (Some(text), Some(table), Some(report)) =
        (block("Input:"), block("Output:"), block("Report:"))
    else {
        panic!("docs/overview.md holds the worked example's input, output and report");
    };
    let path = input("overview.csv", text).expect("the scratch directory takes a file");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), table);
    assert_eq!(String::from_utf8_lossy(&output.stderr), report);
}

#[test]
fn every_rejection_and_malformed_kind_is_counted_on_stderr() {
    let path = input("rejected-and-malformed.csv", REJECTED_AND_MALFORMED)
        .expect("the scratch directory takes a file");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stderr), REJECTED_AND_MALFORMED_REPORT);
    assert_eq!(String::from_utf8_lossy(&output.stdout), REJECTED_AND_MALFORMED_TABLE);
}

#[test]
fn a_wrong_argument_count_is_a_usage_error() {
    let path = input("usage.csv", HEADER_ONLY).expect("the scratch directory takes a file");
    for arguments in [&[][..], &[path.as_path(), path.as_path()][..]] {
        let output = cthulhu(arguments).expect("the binary runs");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "usage: cthulhu <input.csv>\n");
    }
}

#[test]
fn a_missing_input_cannot_be_opened() {
    let path = scratch().expect("the scratch directory exists").join("missing.csv");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with(&format!("error: cannot open {}: ", path.display())), "{stderr}");
}

#[test]
fn an_unreadable_input_stops_the_run_with_no_table() {
    let directory = scratch().expect("the scratch directory exists");
    let output = cthulhu(&[&directory]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("error: {}\n", SourceError::Unreadable)
    );
}

#[test]
fn an_input_without_a_complete_header_stops_the_run_with_no_table() {
    for (name, text) in [("empty.csv", EMPTY), ("no-tx.csv", NO_TX)] {
        let path = input(name, text).expect("the scratch directory takes a file");
        let output = cthulhu(&[&path]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("error: {}\n", SourceError::IncompleteHeader)
        );
    }
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "/dev/full exists only on Linux")]
fn a_full_output_is_a_write_failure() {
    let path = input("full.csv", ONE_DEPOSIT).expect("the scratch directory takes a file");
    let full = fs::File::options().write(true).open("/dev/full").expect("/dev/full opens");
    let output = cthulhu_into(full, &[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("error: {}\n", SinkError::Unwritable)
    );
}

#[test]
fn a_report_that_cannot_be_written_is_a_write_failure() {
    let path = input("closed-stderr.csv", REJECTED_AND_MALFORMED)
        .expect("the scratch directory takes a file");
    let closed = closed_pipe().expect("a pipe opens");
    let output = cthulhu_reporting_into(closed, &[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(String::from_utf8_lossy(&output.stdout), REJECTED_AND_MALFORMED_TABLE);
}
```

The overview's worked example gains, after its Output block, a `Report:` block the first test
reads:

```text
1 rejected: the account is locked
1 rejected: the amount is above available
1 rejected: no such deposit for this client
```

## Phase 0 — evidence (read-only)

Run in a scratch crate in the session's scratchpad, built with this workspace's `[lints]` and
`clippy.toml`, never in this checkout. Each fact below was read from the `csv` 1.4.0, `csv-core`
0.1.13 and serde 1.0.229 sources while planning; this phase runs them.

Ran on 2026-10-11: a probe crate with this workspace's lints, `clippy.toml` and toolchain mirrors
`CsvRow`, `TryFrom`, the header check, the length check and `spoiled`. `cargo clippy
--all-targets -- -D warnings` is clean, and its 13 tests pass, one per group below. They include
the trailing-comma header whose rows fail (Questions on gaps 1) and the dispute row whose
non-UTF-8 amount is unreadable (Questions on gaps 7).

- [x] `git diff --stat 779a604 main` is empty, `git status --short` is clean, and
      `git config --show-origin user.email` resolves from `~/.gitconfig-ffakenz`.
- [x] Reading, with `ReaderBuilder::new().trim(Trim::All).flexible(true)`:
  - a leading byte-order mark is stripped;
  - a first read holding only the mark reads as an empty input;
  - blank lines are skipped, before the header and between rows;
  - a line of spaces is one empty field;
  - trimming applies to headers and inside quotes;
  - `\n`, `\r\n` and `\r` all end a line, and an unended last line is a record, a trailing
    comma keeping its empty last field.
- [x] Deserializing `CsvRow<'a>` with `#[serde(borrow)] amount: Option<&'a str>` against the
      header:
  - a row cut before a trailing `amount` gives `None`;
  - a cut required field fails;
  - a longer row deserializes, its extras ignored;
  - an empty field, quoted or not, is `None`;
  - an unknown column is ignored but fails when cut;
  - a duplicate column fails every row;
  - `+7`, `07` and `0x7` read as 7, while `-0`, `0X7` and an empty field fail.
- [x] A non-UTF-8 row is `ErrorKind::Utf8` and the next `read_record` reads the following row.
      A non-UTF-8 header fails `headers()`.
- [x] A failing `Read` gives `ErrorKind::Io`, and every later `read_record` gives `Ok(false)`.
- [x] `impl TryFrom<CsvRow<'_>> for Command`, with `CsvRow` private, compiles clippy-clean.
      `#[expect(clippy::wildcard_enum_match_arm, …)]` on `spoiled` is fulfilled.
- [x] `csv::Writer`:
  - its terminator is `\n`;
  - it quotes none of the table's fields;
  - `flush` returns the failure of an output that refuses bytes, while dropping an unflushed
    writer discards it.
- [x] `File::open` on a directory succeeds and its first read fails, here on macOS (and on
      Linux, by `read(2)` returning `EISDIR`).
- [x] `CARGO_TARGET_TMPDIR` is set for the `it` crate under cargo 1.88.

## Tasks

### Task 0 — the plan, and the closed plans kept

- [x] `git switch -c feat/stage-1-cli main`: on `6e94920`.
- [x] `git show 9819088:tasks/todo.md > tasks/stage-1-library-plan.md`; `git show
      cc9ef1f:tasks/todo.md > tasks/stage-1-proptest-plan.md`; `cmp` each with its source: both
      identical.
- [x] In `tasks/stage-1-{domain,core,run,memory}.md`, every `tasks/todo.md` names their plan of
      record: repoint each one, the `Plan of record:` line included, to
      `tasks/stage-1-library-plan.md`. That is 27 references (9, 6, 6, 6), and `git diff --stat`
      shows only those lines.
      Done: 27 lines changed (9, 6, 6, 6). 16 of them, like the four prompt-history links
      below, now run past 100 columns and stay unwrapped, so that only those lines change.
- [x] In `docs/prompts/0002-stage-1-planning.md` and `0003-stage-1-library-core.md`, repoint the
      link `[tasks/todo.md](../../tasks/todo.md)` to
      `[tasks/stage-1-library-plan.md](../../tasks/stage-1-library-plan.md)`. In
      `0004-proptest-follow-up.md`, repoint it to `tasks/stage-1-proptest-plan.md`. That is four
      links: 0002 line 9, 0003 line 16, 0004 lines 20 and 34. The quoted prompts at 0002 line 29
      and 0003 line 25 stay as they are. Done: 4 lines changed; only the
      two quoted prompts still say `tasks/todo.md`.
- [x] `tasks/README.md` (a new row after `todo.md`'s) and `CLAUDE.md` § Knowledge base: before
      the next piece of work replaces `todo.md`, the closed plan is kept verbatim as
      `tasks/<name>-plan.md`, and every reference that named it as `todo.md`, quoted prompts
      aside, names that file.
- [x] Write this plan to `tasks/todo.md`.
- [x] Per-commit gate; commit `docs(tasks): …` (Commits). `make ci` green, 26 passed. The
      code-reviewer found no blocker and one should-fix: two evidence lines here. Both are
      corrected, as are its four nits: the convention's wording, task markers inside code
      blocks, `output.csv` for the brief's file name, and this tick. The name greps and the
      record scan are silent.

### Task 1 — the CSV source

Red:

- [x] `generate.rs` and `model.rs` as Test code says. `support.rs`: `pub(crate) mod spelling;`.
      `support/spelling.rs` without `prefix` and `with_rows`. `main.rs`: `mod csv_io;`.
      `csv_io.rs`: the three source laws, with only the imports, helpers and stubs they use
      (`expected`, `read`, `Breaking`) and the module doc's clause on the source; Task 3 adds
      the rest.
- [x] `backend/src/lib.rs`: `pub mod csv_io;`. `backend/src/csv_io.rs`, the stub:

  ```rust
  //! The CSV adapters.

  use std::io::Read;
  use std::marker::PhantomData;

  use crate::run::{Row, SourceError};

  /// A source that drops its input and yields no row.
  #[derive(Debug)]
  pub struct CsvSource<R>(PhantomData<R>);

  impl<R: Read> CsvSource<R> {
      /// The source over `input`.
      pub fn new(input: R) -> Self {
          drop(input);
          Self(PhantomData)
      }
  }

  impl<R: Read> Iterator for CsvSource<R> {
      type Item = Result<Row, SourceError>;

      fn next(&mut self) -> Option<Self::Item> {
          None
      }
  }
  ```

- [x] `make deps fmt-check lint build` green. `cargo test --workspace --locked` fails on exactly
      the three source laws: the source law on any row, the header law on any input, the
      read-failure law on any input, since the stub yields no `Err`. The 26 earlier tests pass.
      Ran with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`: "26 passed; 3 failed". The source law
      shrank to one deposit row, the header law to a header without `type`, and the read-failure
      law to an empty input. `cargo fmt` reflowed three lines of `spelling.rs`.
- [x] Review fixes, staged before the red commit. The code-reviewer found no blocker and one
      should-fix: each flawed spelling was reached about once per run.
  - The should-fix: the source law draws `spelling::flawed()`, which splices up to two
    unreadable rows into the rows. The runtime laws keep `input()`.
  - Nits taken:
    - layouts are drawn before rows, so a shrunk row keeps its line;
    - three doc comments are made true;
    - `Blank::Before` becomes `Blank::Line`;
    - `TooManyFields { first, rest }` admits no empty list of extra fields;
    - the read-failure law reads through a `read_broken` helper.
  - The private row speller `flawed` became `unreadable`, freeing the name.
  - Re-run: "26 passed; 3 failed", the same three laws.
- [x] Per-commit gate (red); commit `test(csv_io): …`: `ca064cb`. The deps, fmt-check, lint
      and build checks are green; the tests fail as above; the name greps and the record scan
      are silent.

Green:

- [x] `backend/Cargo.toml` `[dependencies]`, by hand:
      `csv = "1.4"`, `serde = { version = "1", features = ["derive"] }`. Run
      `cargo check --workspace --tests` once without `--locked`. The lock's diff adds:
  - the packages `csv`, `csv-core` and `ryu`;
  - the `backend` entry's dependency lines;
  - `serde_derive` among `serde`'s dependencies, now that `derive` is on.

  No pin changes. The diff is exactly that: `csv` 1.4.0, `csv-core` 0.1.13, `ryu` 1.0.23,
  `backend`'s two lines and `serde_derive` under `serde`.
- [x] `backend/src/run/row.rs`: `RowError::Unreadable`'s doc names every way a row fails to read:
      a byte that is not UTF-8, an unknown type, a client or tx id that is missing or is not a
      `u16` or a `u32`, or a column the header names left out or named twice.
- [x] `csv_io.rs`: the source half of Interfaces, from the module doc to `spoiled`, with the
      imports it uses; `SinkError`, `render_csv` and their imports come with Task 3.
- [x] `cargo test --workspace --locked`: 29 passed. Run: "29 passed; 0 failed". After the
      review's example test, 30.
- [x] Law evidence: with the work staged, each source break of Law evidence fails its law; after
      each undo, `git diff --quiet`. All six breaks gave `rc=101` and were undone, and no
      regression file was written.
- [x] Review fixes, staged before the green commit. The code-reviewer found no blocker and three
      should-fixes, all taken:
  - `spoiled` names all six `csv::ErrorKind`s, and its `_` arm builds `Spoiled::Unknown`, which
    both callers read as the input's failure. No `#[expect]` is left, and a kind a later csv
    adds fails `make lint` until it is classified.
  - The non-UTF-8 header ruling is pinned by the example test
    `a_header_that_is_not_utf8_lacks_its_columns`. It failed against the break in Law evidence,
    and that break was undone.
  - `RowError::Unreadable`'s doc is true: an unread column other than `amount`, and a known
    column named twice.

  Nits taken: the `CsvRow` and `header` docs; Review focus 3 and § Input on quotes opened
  only at a field's first byte.
- [x] Docs:
  - `CLAUDE.md` § Repository layout and `README.md` § Layout: `csv_io.rs  the CSV adapters`;
  - `README.md` § Setup: `csv` and `serde` are in use;
  - `CLAUDE.md` § Tests: `support/spelling.rs` spells rows as CSV text.
- [x] Per-commit gate; commit `feat(csv_io): …`. `make ci` green, 30 passed. The review's three
      should-fixes are taken (above); the name greps and the record scan are silent.

### Task 2 — the runtime's laws through the source

- [x] `support.rs`: `summarize`, moved from `run.rs`, with the imports it needs: `Infallible`,
      `MemoryLedger`, `MemorySeenIds`, `Row`, `RunError`, `SourceError`, `Summary` and `run`.
      `spelling.rs`: `prefix` and `with_rows`. `model.rs`: `Engine::after`, through which
      `printed` folds. `run.rs` as Test code says.
- [x] `cargo test --workspace --locked`: 30 passed. `git diff -w backend/tests/it/run.rs` shows
      only inputs and helpers changing. Run: "30 passed; 0 failed". The diff changes only module
      docs, imports, `summarized`, `Reordering` and each law's input and calls; no label and no
      law's logic changes.
- [x] Law evidence: the seven switched laws, each against its stage 1 break. All seven failed
      their law, not the build, and were undone with `git diff --quiet`; no regression file was
      written.
- [x] Review fixes, staged before the commit. The code-reviewer found no blocker and one
      should-fix: two evidence rows did not run stage 1's breaks as worded.
  - The footprint cache outlived its run, so it failed the determinism law too.
  - The run counter added a rejection instead of raising the first count.
  - Re-run with the cache reset at each run and the counter added to the first rejection count:
    both laws fail with the counterexamples above.
  - Nits taken: the `Reordering` and `prefix` docs, and "over rows" in the commit message, since
    the broken-source law keeps its synthetic source.
- [x] Per-commit gate; commit `test(it): …`. `make ci` green, 30 passed; the review's should-fix
      is taken (above); the name greps and the record scan are silent.

### Task 3 — the CSV sink

Red:

- [x] `model.rs`: `table`. `csv_io.rs` (tests): `Full` and the two sink laws, with
      their imports and the module doc's clause on the sink. `id.rs`: `Display for ClientId`,
      which `SinkError`'s message needs. `csv_io.rs` (src): `SinkError` with its `Display` and
      `error::Error`, and the stub:

  ```rust
  /// Writes nothing to `_out`.
  pub fn render_csv(_accounts: &Accounts, _out: &mut impl Write) -> Result<(), SinkError> {
      Ok(())
  }
  ```

- [x] `cargo test --workspace --locked` fails on exactly the two sink laws: the empty table is
      not the model's, nor is it non-empty. 30 pass. Run with persistence off:
      "30 passed; 2 failed", both laws shrinking to an input with no rows.
  - `model::table` collects its lines and concatenates them, since clippy refuses `format!`
    pushed onto a `String`.
  - The module doc's clause on `render_csv` waits for the green, where it becomes true.
- [x] Per-commit gate (red); commit `test(csv_io): …`: `603a0a0`. The deps, fmt-check, lint and
      build checks are green; the tests fail as above; the name greps and the record scan are
      silent. The code-reviewer found no blocker and no should-fix, and six nits are taken:
  - the write-failure law guards on an empty table, not on the header, so a format break fails
    only the table law;
  - `ClientId`'s doc names no table;
  - `Full`'s doc matches its cut-short writes;
  - the oracle destructures the model's account;
  - the plan's red and green lists agree.

  Re-run: "30 passed; 2 failed", the same two laws.

Green:

- [x] `render_csv` as Interfaces gives it, with the module doc's clause on it.
- [x] `cargo test --workspace --locked`: 32 passed. Law evidence: the two sink breaks. Run:
      "32 passed; 0 failed". Both breaks failed their law and were undone with `git diff --quiet`;
      no regression file was written.
- [x] `docs/architecture.md`:
  - the `csv_io` row of § Modules holds `SinkError`;
  - § In detail shows `render_csv`'s failure, `SinkError`, reaching `ExitCode` beside
    `RunError`.
- [x] Review fixes, staged before the green commit. The code-reviewer found no blocker and one
      should-fix: generated tables never passed `csv::Writer`'s 8 KiB buffer, so no law reached
      the two `write_record` failures.
  - The write-failure law now renders up to 400 extra opened accounts (`widened`), and the
    break mapping those failures to `Overflow` fails it (Law evidence).
  - Nits taken: `TABLE` becomes `HEADER`, and the architecture picture branches `SinkError` off
    `render_csv` with a `┬`.
  - Left for the final review: "accounts table" against the records' "account table", and the
    `Drop` retry, which the Risks already record.
- [x] Per-commit gate; commit `feat(csv_io): …`. `make ci` green, 32 passed; the name greps and
      the record scan are silent.

### Task 4 — the command line

Red:

- [x] `backend/tests/it/cli.rs` as Test code says; `main.rs`: `mod cli;`. `docs/overview.md`: the
      `Report:` block after the worked example's Output block.
- [x] `backend/src/main.rs` stays `fn main() {}`. `cargo test --workspace --locked` fails on
      exactly six `cli` tests, since each expects a table, a status other than `0`, or a line on
      stderr. 32 pass; on macOS the `/dev/full` fixture is ignored, and on Linux it is a seventh
      failure. Run: "32 passed; 6 failed; 1 ignored".
  - The table fixtures fail on an empty stdout; the others on status `0` where `2`, `3` or `4`
    is due.
  - The overview's three blocks parse, and `cli.rs` matches the plan's copy after `cargo fmt`.
- [x] Review fixes, staged before the red commit. The code-reviewer found no blocker and two
      should-fixes, both taken:
  - the rejected-and-malformed fixture asserts stderr first, so its red shows the missing
    report;
  - the error lines are built from the public values (`SourceError`, `SinkError`), not copied
    prose.

  Nits taken:
  - a malformed row comes first, so the report's order is not the input's;
  - `cthulhu_into` serves the `/dev/full` fixture;
  - the incomplete inputs are named constants, and the main fixture is
    `REJECTED_AND_MALFORMED`;
  - plain `assert!`s print what they saw;
  - the test crate's module doc names `cli`.

  Re-run: "32 passed; 6 failed; 1 ignored", and the stderr fixture now fails on stderr.
- [x] Per-commit gate (red); commit `test(cli): …`. The deps, fmt-check, lint and build checks
      are green; the name greps and the record scan are silent.

Green:

- [x] `main.rs`, `text_io.rs` and `lib.rs`'s `pub mod text_io;` as Interfaces give them. The
      `run` target in `Makefile`. `.claude/settings.json`: `Bash(make run:*)`.
- [x] `run.rs`: delete `the_overview_example_ends_with_its_documented_accounts`, `overview_rows`
      and `overview_printed`, with the imports only they used: `DisputeReason`, `chargeback`,
      `deposit`, `dispute` and `withdrawal`.
- [x] `cargo test --workspace --locked`: 37 passed and 1 ignored on macOS (38 passed on Linux).
      Run: "37 passed; 0 failed; 1 ignored". With the review's report fixture, 38 and 1 ignored
      (39 on Linux).
- [x] `make run INPUT=<the overview's input, saved in the scratchpad> > <scratchpad>/out.csv`:
      `out.csv` holds exactly the overview's Output block, and stderr ends with its Report block
      after cargo's own lines. Both compare byte for byte. `make run` without `INPUT` prints
      `usage: cthulhu <input.csv>`, and make names the binary's status in `Error 2`.
- [x] Docs, each in step with the code:
  - `docs/overview.md`:
    - § Input: a byte-order mark, blank lines, quoted fields and `\r\n` or `\r` line ends read as
      usual; a quote opens a quoted field only at the field's first byte;
    - also § Input, as Questions on gaps 1, 2 and 7 settle: other columns are ignored, but not
      when a row stops before them; a known column named twice makes every row malformed; a
      header that is not UTF-8 lacks the columns, and a row holding a byte that is not UTF-8 is
      malformed;
    - § Output: the report's lines and the exit statuses (Decisions for sign-off 3 and 4).
  - `docs/architecture.md`, for `text_io`:
    - § Modules: the box and the table gain the row
      `` `text_io` | `render_report` | `domain`, `run` ``;
    - § In detail and § Wiring: `Report ─► render_report ─► stderr`.
  - `README.md`:
    - § Status, and § Layout with `text_io.rs`;
    - § Usage: `make run INPUT=input.csv > output.csv`,
      `cargo run -- input.csv > output.csv`, the streams and the exit statuses.
  - `CLAUDE.md`:
    - § Status: the library and the command line are built; the benchmarks are next;
    - § Repository layout: `text_io.rs`;
    - § Build, test, run: `make run INPUT=<file>`;
    - § Tests: `cli.rs` drives the binary.
  - `docs/roadmap.md` § 1: an `Open:` line for a bound on one row's size, which needs a reader
    below csv and its own record (Questions on gaps 3).
- [x] Review fixes, staged before the green commit. The code-reviewer found no blocker and two
      should-fixes, both taken:
  - The report's write failure had no test. The fixture
    `a_report_that_cannot_be_written_is_a_write_failure` hands the binary a stderr pipe whose
    reader is closed (`io::pipe`, so it runs on every platform). It failed once against a
    report mapped to status `4` (Law evidence), and that break was undone.
  - `README.md` and `CLAUDE.md` no longer say the text adapter is held to laws: the binary's
    fixtures hold it.

  Nits taken, each a doc that was untrue or incomplete:
  - the architecture legend names stderr;
  - the report's `io::Error` path is drawn;
  - the pipeline picture, `docs/patterns.md` and the roadmap name the report's sink;
  - README's exit statuses are the binary's;
  - the `make run` evidence above names make's `Error` line.

  Raised with the person at the close, not acted on: *Sources, sinks and projections* does not
  name `render_report`; and `Bash(make run:*)` with an unquoted `$(INPUT)` lets one approved
  command run another.
- [x] Per-commit gate; commit `feat(cli): …`. `make ci` green, 38 passed and 1 ignored; the
      name greps and the record scan are silent.

### Task 5 — closing

- [x] The Review section of this plan: what the plan got wrong, what was narrower or wider, what
      was deliberately not done.
- [x] A final whole-branch review of `main..feat/stage-1-cli` on the most capable model
      (`model: "fable"`, as stage 1's), with every important finding acted on.
  - Result: no critical finding, one important, six minor; ready to merge with fixes.
  - The important one: `Bash(make run:*)` auto-allowed any `make run …`, and the recipe passed
    `$(INPUT)` unquoted to the shell. An allowed `make run INPUT='x; <command>'` ran the
    command, and a `CARGO=`, `SHELL=` or `-f` override did too. The person chose to narrow and
    quote:
    - the allowance is the exact `Bash(make run)` again;
    - the recipe passes `$${INPUT:+"$$INPUT"}`.
  - Before: `make run INPUT='nope.csv; echo INJECTED'` printed `INJECTED` and exited 0. After:
    the binary answers `error: cannot open nope.csv; echo INJECTED: …` with status 3. No
    `INPUT` still gives the usage line, and a path with spaces now works. `make ci`: 38 passed,
    1 ignored. No test pins the recipe; these runs are the evidence.
  - The exact allowance, not the quotes, closes the hole: make itself expands a `$(…)` in a
    command-line `INPUT` when it exports it to the recipe.
  - The fix's review found no blocker. Taken from it: the Interfaces recipe, the stage 1 breaks
    sentence, the wording of these notes and the Review, and the `tasks/lessons.md` entry. Under
    zsh, `INPUT=~/…` now reaches the binary with a literal `~`, which README § Usage states.
  - The review also found six allowances already on `main` with the same flaw: `cargo run:*` and
    `cargo test:*` take a runner through `--config`, which runs any command, and `cargo build:*`,
    `check:*`, `clippy:*` and `tree:*` run a wrapper named by path. Asked before the next commit,
    the person chose to narrow them on this branch, in a commit of their own.
  - A probe then found `cargo metadata --config 'build.rustc=…'` running the program too, and the
    person chose to drop `cargo metadata:*` as well. The narrowing: `cargo run:*`, `cargo check:*`
    and `cargo metadata:*` are gone; `clippy`, `build`, `test` and `tree` are allowed only in the
    forms the Makefile runs; `cargo fmt:*` stays, since `cargo fmt` refuses `--config`.
  - Before: `cargo run` and `cargo test` with a `--config` runner ran the given command. After, by
    the allow list's own rule forms, none of eight such calls is allowed, while the Makefile's
    cargo forms, `make ci` and `make run` still are. No test pins an allow list; these runs are
    the evidence.
  - The fix is folded into the command line's green commit, which introduced the recipe and the
    allowance; `tasks/lessons.md` records the rule it teaches.
  - The minors are left to the person (Review).
- [x] Prompt history 0005, with the prompt-history skill, committed alone: `6c4efe4`, whose
      `git show --stat` lists only `docs/prompts/0005-stage-1-command-line.md`.
- [x] superpowers:finishing-a-development-branch, keeping the branch as it is. Report what is on
      it and stop: pushing and landing are the person's.
  - Kept. The person then asked to land it: the branch was pushed, and `main` became the squash
    commit `5ec116e`. `git diff --stat 6c4efe4 5ec116e` is empty, and CI run `38271822891` on
    `5ec116e` passed on Linux with 39 passed.

## Law evidence

One row per break, filled during execution: the law, its break, and the shrunk counterexample.
- Each break runs its own law alone, with the work staged:
  `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test --test it <law>`. A source break that loops
  would otherwise hang the runtime laws, which take every item the source yields.
- Each undo is proven by `git diff --quiet`.

| Law | Break | Shrunk counterexample |
|---|---|---|
| `prop_source_reads_every_spelling_as_the_model_reads_the_rows` | `Trim::None`: padded fields stop reading | two deposits by client 1, the first row's line padding one field with a trailing space |
| `prop_source_reads_every_spelling_as_the_model_reads_the_rows` | `flexible(false)`: a row cut before its trailing amount, or longer than the header, fails | `TooManyFields { first: "", rest: [] }` under the header `type,client,tx`: read as `Unreadable` |
| `prop_source_reads_every_spelling_as_the_model_reads_the_rows` | the length check removed: extra fields are ignored instead of counted | `TooManyFields { first: "", rest: [] }` under the header `type,client,tx`: read as a deposit |
| `prop_source_reads_every_spelling_as_the_model_reads_the_rows` | `client` read as text and parsed with `str::parse`, which refuses `0x7` | two deposits by client 1, the first row's client spelled `0x1` |
| `prop_header_lacking_type_client_or_tx_ends_the_source` | the header check leaves out `tx` | no rows under the header `type,client,amount`: no error |
| `prop_read_failure_ends_the_source` | an I/O error read as a malformed row | four client-1 rows cut mid-input: the last item is a malformed row, not `Unreadable` |
| `a_header_that_is_not_utf8_lacks_its_columns` (example) | a header that is not UTF-8 read as `Unreadable` | the fixture's own header: `Unreadable`, not `IncompleteHeader` |
| `a_report_that_cannot_be_written_is_a_write_failure` (fixture) | a failed report mapped to status `4` | the fixture's own input: status `4` where `5` is due |
| `prop_table_is_the_naive_models` | the total column prints `held` | `[deposit(1, 0, "0.1")]`: total `0.0000` where the model prints `0.1000` |
| `prop_table_that_cannot_be_written_whole_is_an_error` | `render_csv` returns without flushing | no rows, no extra account, room 0: `Ok(())` where the output took nothing; `csv::Writer`'s drop swallowed the failure |
| `prop_table_that_cannot_be_written_whole_is_an_error` | both `write_record` failures mapped to `Overflow` | no rows, 248 extra accounts, room 0: the first table past the writer's 8 KiB buffer |
| `prop_tx_ids_are_consumed_on_first_sight_across_clients` | withdrawals skip intake and are decided as `Sight::First` | `[withdrawal(1, 0, "1.50000"), withdrawal(1, 0, "0.1")]` |
| `prop_rejection_changes_no_balance` | after a rejection at a loaded footprint, `apply_at` commits `open()` | `[deposit(100, 0, "0"), deposit(100, 1, "1"), dispute(100, 0), chargeback(100, 0)]` |
| `prop_malformed_rows_are_skipped_and_counted` | `Row::Malformed` is skipped uncounted | `[Unreadable(Type("Deposit"))]` |
| `prop_report_stays_bounded` | rejections kept as a `Vec`, one entry per refused row | `[dispute(1, 0), withdrawal(1, 1, "1")]` |
| `prop_same_rows_give_the_same_summary` | a `static` run counter added to the first rejection count | `[withdrawal(1, 0, "0.1")]` |
| `prop_reordering_clients_changes_no_account` | a one-entry footprint cache keyed by the tx id alone, reset at each run | original `[deposit(1, 0, "1"), dispute(1, 0), dispute(2, 0)]`, interleaved `[deposit(1, 0, "1"), dispute(2, 0), dispute(1, 0)]`: client 2's dispute reads client 1's cached deposit |
| `prop_engine_agrees_with_the_naive_model` | `InsufficientFunds` when the amount is not below `available` | `[deposit(100, 0, "0.1"), withdrawal(100, 1, "0.1")]` |

`prop_source_error_stops_the_run_with_no_summary` keeps its synthetic source and is not re-run.
The domain, core and memory laws read no CSV, and their inputs are unchanged apart from the
malformed rows the core laws filter out.

## Commits

```text
docs(tasks): plan stage 1's command line and keep the closed plans

> The command line has a plan of record, and the stage 1 and proptest plans stay readable where
> their tickets and prompt histories point.
```

```text
test(csv_io): pin the CSV source to the model over generated text

> Every way the source may meet a row, padded, quoted, reordered, cut short or broken, is a law
> before the adapter exists, and each law fails against a source that reads nothing.
```

```text
feat(csv_io): read CSV rows into commands at the boundary

> Real CSV input streams into commands or counted malformed rows, one row at a time, and an
> unreadable input or an incomplete header stops the run.
```

```text
test(it): feed the runtime's laws CSV text through the source

> Every runtime law over rows reaches the engine through the production source, so no spelling
> of the input can hide a bug behind the model's own parser.
```

```text
test(csv_io): pin the accounts table to the naive model's

> The table's format, and the refusal to report a partial write as a success, are laws before
> the sink exists.
```

```text
feat(csv_io): render the accounts table as CSV

> The accounts view reaches the outside as the table the contract fixes, and a write that fails
> is an error, never a silent success.
```

```text
test(cli): pin the command-line contract through the binary

> The argument, the streams and the exit statuses are fixed by fixtures against the real binary,
> the worked example read from the overview itself.
```

```text
feat(cli): turn an input file into the accounts table

> make run turns an input file into the accounts table, with every failure reported on stderr
> and mapped to its own exit status.
```

```text
fix(settings): narrow the cargo allowances to the forms the Makefile runs

> An allowed cargo call can no longer run another program through --config without a prompt;
> cargo beyond fmt asks first unless it is a form the Makefile runs.
```

```text
docs(prompts): add prompt history 0005 (stage 1 command line)

> The path from the request to plan the command line, through its sign-off, to the working
> binary stays reproducible.
```

## Per-commit gate

1. `make ci` is green.
   - For a red commit: `make deps fmt-check lint build` is green, and
     `cargo test --workspace --locked` fails on exactly the tests its task lists.
2. The code-reviewer agent reviews the staged diff. Every blocker and should-fix is acted on, and
   `make ci` is re-run.
3. The two forbidden-name greps of Global constraints are both silent.
4. No record is cited by number or filename outside the decision logs:
   `git grep -n --untracked -E
   'docs/(adr|design)/[0-9]{4}|ADR ?[0-9]+|[Dd]ecision #?[0-9]+|record #?[0-9]+' -- .
   ':!docs/adr' ':!docs/design' ':!.claude/skills/architecture-decisions'` lists nothing.
5. `git status --short` lists only the commit's files, this plan ticked among them.

## Risks

- **Derived behaviours.** Some `csv` facts were derived from code rather than tests: a read
  after a UTF-8 error continues, and a first read holding only the mark reads as an empty input.
  Phase 0 runs them, and a mismatch stops the work.
- **The mark and network sources.** `csv-core` reads a first read that holds only the mark as
  the end of the input. A file never delivers one; a network source might, which is stage 4's
  to answer.
- **Interrupted reads.** An `Interrupted` read ends the input, since `csv` does not retry it: a
  failed run (status `4`), never a wrong table.
- **Drop retries the flush.** `csv::Writer`'s `Drop` retries a failed flush and discards the
  error. An output that recovers in between can receive part of the table twice; the run still
  exits `5`.
- **Closed pipes.** Rust ignores `SIGPIPE`, so `cthulhu input.csv | head -1` exits `5`, with a
  line on stderr.
- **Platform reach.** The fixtures read a directory to fail a read, which holds on Linux and
  macOS, not Windows. `/dev/full` exists only on Linux.
- **Breaks that loop.** A break that loops would hang a law, so every source law takes at most
  two items past its rows.
- **Test time.** Generating `Layout`s and their text adds time to every runtime law. The Review
  records the test time before and after.

## Deliberately deferred

- **Out of scope:** the benchmark generator, the measurements and the speed-ups (roadmap
  § Benchmarks and § 1's follow-ups).
- **Carried forward from stage 1:**
  - The runtime opens an account through `open()` and never answers `Event::Opened`. Before an
    event log lands, decide whether a step yields it, so a replay reopens the clients named only
    by rejected commands.
  - The reordering law could compare whole summaries, not only accounts: with unique tx ids the
    report is order-invariant too.
  - `prop_decide_never_changes_state` checks determinism, all that a `&`-typed decision can
    break; rename it or fold it into the table law.
  - `Account` offers `is_locked() -> bool` beside the crate-visible `lock() -> Lock`; one public
    reading, the enum, would do.
- **New from this plan:**
  - The OS reason for an unreadable input or an unwritable output. `SourceError` and `SinkError`
    carry none, and `render_csv` maps every csv error with `map_err(|_| …)`, against the
    `rust-errors` rule that such a mapping drops the cause.
  - The reader's and the writer's buffer sizes. They stay csv's 8 KiB defaults until the
    benchmarks choose them, although the `rust-wiring` skill pins every buffer size at the root.
    A read buffer under four bytes would also defeat the byte-order mark.
  - A bound on one row's size (Questions on gaps 3).
  - Other output formats, as other sinks (*The command-line contract*).

## Review

- **What the plan got wrong, which the reviews caught:**
  - `spoiled` hid three known `csv::ErrorKind`s behind its `#[expect]`. It now names all six,
    and its `_` arm builds `Spoiled::Unknown`, so no suppression is left and a kind a later csv
    adds fails `make lint`.
  - Two laws reached less than they claimed. Each flawed spelling turned up about once per run;
    `spelling::flawed()` splices up to two into the source law's input. No generated table
    passed csv's 8 KiB writer buffer; `widened` adds up to 400 opened accounts, so the
    `write_record` failures are reached.
  - The non-UTF-8 header ruling and the report's write failure had no test; review added an
    example test and a binary fixture, each shown failing once.
  - Two stage 1 breaks were first re-run in forms other than stage 1's wording: a cache that
    outlived its run, which failed the determinism law too, and a counter that added a
    rejection. Re-run as worded, each fails its own law (Law evidence).
  - The `make run` recipe and its allowance let an allowed command run another. Fixed as the
    person chose (Task 5).
- **Wider than the request:**
  - a second sink, `text_io::render_report`, which keeps the report's wording out of the domain
    and out of `main.rs` (Questions on gaps 6);
  - `Flaw` and `TooManyFields { first, rest }` in the generated rows;
  - paths with spaces through `make run`;
  - the cargo allowances on `main` narrowed to the forms the Makefile runs, as the person chose.

  **Narrower:**
  - unknown header columns follow serde's default, with no projection (Questions on gaps 1);
  - `make run` passes `INPUT` with no shell expansion, so a `~` that zsh leaves after `=` stays.
- **Commits:** red and green apart, as `CLAUDE.md` § Implementation asks. That gives the plan's
  own commit first, three red/green pairs with the runtime laws' move onto CSV text between the
  first two, the narrowed cargo allowances, and the prompt history last, as Commits lists them.
  The final review's fix is folded into the command line's green commit.
  - The plan-text sync of the sink red's review first rode in the sink's green commit, after a
    script failed inside a `;`-joined chain; at the close, the person chose to fold it back into
    the red.
- **What it cost:**
  - 26 tests became 38, plus the Linux-only `/dev/full` fixture.
  - The suite takes about 2 s instead of 0.5 s.
  - `Cargo.lock` gained `csv`, `csv-core` and `ryu`.
- **Deliberately not done:**
  - the benchmark generator, the measurements and the speed-ups;
  - the four stage 1 items carried forward;
  - the deferrals in Deliberately deferred.

  The final review's minors are left to the person:
  - *Sources, sinks and projections* names `CsvSource` and `render_csv` in its Governs line but
    not `render_report`;
  - "accounts table" in the code and docs against the records' "account table";
  - no test pins the overview's `deposit, 1, 1, "1.0"` example;
  - § Input does not say that a whitespace-only line is a malformed row;
  - one line of `CLAUDE.md` § Status runs to 143 columns;
  - `make run`'s stderr carries cargo's own lines above the report.
