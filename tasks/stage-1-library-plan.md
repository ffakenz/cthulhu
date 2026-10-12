# Stage 1 library core — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans, inline, as the person
> chose at hand-off on 2026-10-10, to implement this plan task by task. Work in this checkout on
> `feat/stage-1-core`, not a new worktree: the plan and its tickets stay untracked until the
> domain commit, so a fresh worktree would not have them. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Goal:** the library core of stage 1 (domain, core, run, memory), each held to its laws, in four
commits on `feat/stage-1-core`, on top of the records commit amended with the domain-model
records and the skills that apply them.

**Architecture:** a pure decider (`decide_client`, `decide_dispute`, `evolve`) over domain types
that cannot hold a bad value; a sequential runtime that takes each row through intake, load,
open, decide, evolve and commit over two store ports; in-memory stores that keep the store laws.
`main.rs`, the CSV adapters, `csv` and `serde`, binary fixtures and benchmarks are out of scope.

**Tech stack:** Rust 1.88.0, edition 2024; the standard library and `rust_decimal`
(`default-features = false`, `std`). Tests: one black-box crate, `backend/tests/it`, on the
std-only seeded harness of `.claude/skills/rust-properties/references/harness.md`.

**Spec:** the accepted records in `docs/adr/` and `docs/design/`; `docs/overview.md` (behaviour,
worked example, laws), `docs/architecture.md` (pipeline, modules), `docs/patterns.md` (decider);
`CLAUDE.md` and the skills in `.claude/skills/`. Where code and records disagree, the records win;
a record that proves wrong stops the work for a superseding record, never an edit.

## Global constraints

- Branch `feat/stage-1-core`. Never push, merge or touch a remote unless the person asks.
- History, as the person decided on 2026-10-10: the domain-model records, the `domain-modeling`
  and `curry-howard` skills, the harness's `Seed` and their syncs amend the records commit at the
  branch's base (`docs(engine): …`), whose message stands; then one commit per ticket (domain,
  core, run, memory); then each session's prompt history alone, the planning session's and then
  the build session's. This sets aside the `architecture-decisions` skill's "commit the record and
  its index line, and nothing else" for these two records. Until a pull request opens, a later fix
  belongs in the commit it fixes: between tickets, never mid-ticket, it is a
  `git commit --fixup=<sha>` on top, and Closing squashes every fixup into its commit. Once a pull
  request is open, `CLAUDE.md` § PR reviews holds and each resolution is its own commit. Format:
  `action(scope): description`, a blank line, `> impact`; no trailer lines.
- TDD drives every piece. By the person's decision for this work, a ticket's red and green land
  in its one commit, which sets aside `CLAUDE.md` § Implementation's separate red and green
  commits.
- The records commit is already on `origin/main`. Once amended, the branch no longer descends
  from `main`, and landing it means replacing that commit there: a force-push, which
  `.claude/settings.json` denies, so it is the person's to do. Amending re-signs the commit with
  the person's key.
- Commit as the `ffakenz` identity: `origin` is `git@github-ffakenz:ffakenz/cthulhu.git`, and
  `git config --show-origin user.email` resolves from `~/.gitconfig-ffakenz`. Write the message to
  a scratch file and `git commit -F` it; stage explicit paths, never `git add -A`.
- `make ci` is green before every commit. The code-reviewer agent reviews every commit's staged
  diff first, and every blocker and should-fix is acted on.
- Dependencies: only `rust_decimal = { version = "1", default-features = false, features =
  ["std"] }`, added by hand to `backend/Cargo.toml` `[dependencies]` in the domain commit. No
  `cargo add`, no other crate, no dev-dependency.
- `rust_decimal` is named only in `backend/src/domain/money.rs`, inside `Amount` and `Balance`.
- `backend/src/main.rs` stays `fn main() {}`.
- No test-only code in production. Generators, the naive model and the store doubles live in
  `backend/tests/it`; every helper there is used, or `dead_code` fails `make lint`.
- Every law is shown failing once against a deliberately broken implementation; its failure line
  goes in its ticket's Law evidence table. A break is applied and undone by hand on uncommitted
  work: never `git checkout`, `git stash` or `git reset` mid-ticket. Stage the ticket's files
  before the first break, so a break in a new file shows in `git diff`; after each undo,
  `git diff --quiet` exits 0. Undo by editing, never by copying a saved file back: the copy keeps
  its old modification time, and cargo reuses the broken build.
- A Phase 0 check that fails stops the work: ask the person before going on.
- Never name the organisation behind the external brief or the project the decision log's style
  comes from, and never quote, restate or paraphrase the brief, in any tracked file or commit
  message. Before each commit, grep the tree and the staged diff with the forbidden-name pattern:
  `git grep -n -i -E --untracked -f <pattern file>` and
  `git diff --cached | grep -n -i -E -f <pattern file>`, both silent. The pattern file is never
  tracked: each session writes it to its scratchpad from the names its kickoff prompt points to.
- Comments follow `CLAUDE.md` § Comments: what the code is and does; never a record number, a
  plan, a ticket or a trade-off.

## Review focus

Inputs the records imply that most likely bite once the library meets real rows, and the law in
the owning ticket that pins each:

1. Amount text outside the grammar — a sign, an exponent, `_`, a bare or trailing point,
   surrounding spaces, a non-ASCII digit — is refused as malformed, never rounded and never a
   panic; inside it, forty leading zeros or thirty trailing zeros parse exactly. Pinned by the
   domain ticket's reverse round trip and totality laws, whose edges list each spelling.
2. Boundary ids — tx `0` and `u32::MAX`, client `0` and `u16::MAX` — behave as any other id.
   `Fresh` hands out tx `0` first, so ordinary deposits and withdrawals use it; tx `u32::MAX` is
   an unknown dispute target; clients `0` and `u16::MAX` send the repeated and unknown commands,
   which still open their accounts. Every core and run law meets them through the command
   generator, and the store laws address tx `0` and `u32::MAX`, and clients `0` and `u16::MAX`,
   directly.
3. A dispute command naming a withdrawal's tx or another client's deposit is `UnknownDeposit`
   and only opens the account. Pinned by the core lifecycle law and the run model law, through
   the unknown and foreign tx-id edges.
4. A dispute after a resolve, and dispute commands on a locked account, apply; deposits and
   withdrawals on it do not. Pinned by the core lifecycle law, which asks every dispute step,
   locked account or not, and to which a resolved deposit is settled again; and by the
   client-commands law, through the after-chargeback episode and the repeats a locked account's
   own client sends.
5. Many malformed rows leave every account as it was and the report one count per kind. Pinned by
   the run malformed-row and bounded-report laws, through the malformed-row edge.

## Domain model

Stage 1's instance of *A domain model in the records' language* and *Entities identified by
their keys*, classified as the `domain-modeling` skill does:

| Kind | Stage 1 |
|---|---|
| Value | `ClientId`, `TxId`, `Address`, `Amount`, `AmountError`, `Balance`, `Lock`, `Status`, `Sight`, `Reason` with `ClientReason` and `DisputeReason`; `Footprint`, entity states without their keys |
| Entity | an account, `Account`, under its `ClientId`; a deposit in history, `Deposit`, under its `Address` |
| Aggregate | a client's ledger, no domain type; the core sees it one `Footprint` at a time, at its command's `Address` |
| Message | `Command` (`ClientCommand`, `DisputeCommand`) and `Event` (`Opened`, `ClientEvent`, `DisputeEvent`) |
| View | `Accounts` and `Report` |
| Boundary type | none yet: the CSV row and the printed account arrive with the CSV adapters |

Rows, as `docs/overview.md` § Input describes them, become commands at the boundary:

| Row field | Domain | What the type rules out |
|---|---|---|
| `type` | the variant: `ClientCommand::{Deposit, Withdrawal}` or `DisputeCommand::{Dispute, Resolve, Chargeback}` | an unknown kind, which is a malformed row instead |
| `client`, `tx` | one `Address` of a `ClientId` and a `TxId` | one id passed where the other belongs; a client and a tx id taken from two commands |
| `amount` | `Amount`, on client commands only | zero, negative, five decimals, `10^15` or more; a client command without an amount, a dispute command with one |

Printed accounts, as `docs/overview.md` § Output describes them, are rendered from `Accounts`:

| Printed column | Domain | What the type rules out |
|---|---|---|
| `client` | the key: `Accounts::by_client` yields `(ClientId, Account)` | an account stored under one client and naming another |
| `available`, `held` | `Account::available` and `Account::held`, as `Balance` | `-0.0000`, or more than four decimals |
| `total` | `Account::total`, derived | a total that disagrees with available plus held |
| `locked` | `Account::is_locked`, reading a `Lock` that only `evolve` sets, on `ChargedBack` | an account locked or unlocked anywhere else |

- Identity is a key throughout: accounts under `ClientId`, deposits under their `Address`, a
  footprint at its command's `Address`. No value carries its own key, and no event carries one.
- Each decision is exact: `decide_client` answers a `ClientEvent` or a `ClientReason`,
  `decide_dispute` a `DisputeEvent` or a `DisputeReason`, as the overview's Commands table splits
  them. The runtime widens both into `Event` and `Reason`, the one place the two families meet.
- Opening is total: `open()` is the footprint `Opened` makes, so no runtime path handles an
  overflow that opening cannot cause.
- The aggregate's invariants are the overview's laws 1 to 3, held by the core ticket's laws.
- Each type the tickets add, test types included, answers the `domain-modeling` and
  `curry-howard` review questions before its commit.

## File map

| File | Ticket | Holds |
|---|---|---|
| `backend/Cargo.toml`, `Cargo.lock` | domain | the `rust_decimal` dependency |
| `Makefile` | domain | `make deps`: the build graph against the admitted crates |
| `backend/src/lib.rs` | each | one `pub mod` line per module |
| `backend/src/domain.rs` | domain | module doc, private submodules, `pub use` of every type |
| `backend/src/domain/id.rs` | domain | `ClientId`, `TxId`, `Address` |
| `backend/src/domain/money.rs` | domain | `Amount`, `AmountError`, `Balance`: the only `rust_decimal` user |
| `backend/src/domain/ledger.rs` | domain, core | `Account`, `Lock`, `Deposit`, `Status`, `Footprint`; `Account::new`, `Account::lock` and `Deposit::new` in core |
| `backend/src/domain/command.rs` | domain | `ClientCommand`, `DisputeCommand`, `Command` |
| `backend/src/domain/event.rs` | domain | `Event`, `ClientEvent`, `DisputeEvent`, `Reason`, `ClientReason`, `DisputeReason`, `Sight` |
| `backend/src/core.rs` | core | `open`, `decide_client`, `decide_dispute`, `evolve`, `Overflow` |
| `backend/src/run.rs` | run | `run` and its private steps; `pub use` of `run/*` |
| `backend/src/run/port.rs` | run | `LedgerStore`, `SeenIds` |
| `backend/src/run/row.rs` | run | `Row`, `RowError`, `SourceError`, `RunError` |
| `backend/src/run/summary.rs` | run | `Summary`, `Accounts`, `Report` |
| `backend/src/memory.rs` | memory | `MemoryLedger`, `MemorySeenIds` |
| `backend/tests/it/main.rs` | each | `mod support;` and one `mod` per source module |
| `backend/tests/it/support.rs` | domain, run, memory | the harness, verbatim; `mod` lines for its helpers; `infallible` |
| `backend/tests/it/support/generate.rs` | domain, core, run, memory | amount and balance generators; `InputRow`, its builders, `Edge`, `Fresh`, `Unique`; `address` |
| `backend/tests/it/support/model.rs` | domain, run | the amount grammar and `Units`; the naive engine and `Printed` |
| `backend/tests/it/support/stores.rs` | run, deleted in memory | store doubles |
| `backend/tests/it/{domain,core,run,memory}.rs` | each | that module's laws |

## Interfaces

Every public item of stage 1. Each derives `Debug`; the derives listed are the ones the laws and
the stores need. Doc comments are omitted here and written in the code.

```rust
// domain::id — no invariant beyond the integer, so `From` is the constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)] pub struct ClientId(u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)] pub struct TxId(u32);
impl From<u16> for ClientId {}  impl From<u32> for TxId {}
// A command's address: its client and tx id, where its footprint is loaded and committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address { client: ClientId, tx: TxId }
impl Address { pub fn new(client: ClientId, tx: TxId) -> Self; pub fn client(self) -> ClientId; pub fn tx(self) -> TxId; }

// domain::money — `FromStr` is `Amount`'s only constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub struct Amount(Decimal);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AmountError { Malformed, TooPrecise, TooLarge, Zero }  // checked in this order
impl FromStr for Amount { type Err = AmountError; }
impl Display for Amount {}                                        // exactly four decimals
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub struct Balance(Decimal);
impl Balance {
    pub const ZERO: Self;
    pub fn checked_add(self, other: Self) -> Option<Self>;
    pub fn checked_sub(self, other: Self) -> Option<Self>;
}
impl From<Amount> for Balance {}
impl Display for Balance {}                                       // four decimals, never -0.0000

// domain::ledger — fields private; readings are methods; one constructor each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Account { available: Balance, held: Balance, lock: Lock }
impl Account {
    pub(crate) fn new(available: Balance, held: Balance, lock: Lock) -> Self;  // core ticket
    pub fn available(&self) -> Balance;
    pub fn held(&self) -> Balance;
    pub fn total(&self) -> Option<Balance>;                       // available + held, checked
    pub fn is_locked(&self) -> bool;
    pub(crate) fn lock(&self) -> Lock;                            // core ticket
}
// Whether a chargeback has locked the account; nothing unlocks it. Public, so the domain commit,
// where nothing builds one yet, raises no `dead_code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Lock { Unlocked, Locked }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deposit { amount: Amount, status: Status }
impl Deposit {
    pub(crate) fn new(amount: Amount, status: Status) -> Self;    // core ticket
    pub fn amount(&self) -> Amount;
    pub fn status(&self) -> Status;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Status { Settled, Disputed }
// What one command touches, at its address (its client and tx id): the account, and the deposit
// under that tx id, `None` when nothing was recorded there or a chargeback removed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footprint { account: Account, deposit: Option<Deposit> }
impl Footprint {
    pub fn new(account: Account, deposit: Option<Deposit>) -> Self;  // evolve, and every store's load
    pub fn account(&self) -> &Account;
    pub fn deposit(&self) -> Option<&Deposit>;
}

// domain::command — every command carries its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Command { Client(ClientCommand), Dispute(DisputeCommand) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCommand { Deposit { address: Address, amount: Amount }, Withdrawal { address: Address, amount: Amount } }
impl ClientCommand { pub fn address(&self) -> Address; }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeCommand { Dispute(Address), Resolve(Address), Chargeback(Address) }
impl DisputeCommand { pub fn address(&self) -> Address; }

// domain::event — an event carries what `evolve` needs and no id: it applies at its command's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Event { Opened, Client(ClientEvent), Dispute(DisputeEvent) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ClientEvent { Deposited(Amount), Withdrawn(Amount) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DisputeEvent { Held(Amount), Released(Amount), ChargedBack(Amount) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason { Client(ClientReason), Dispute(DisputeReason), Overflow }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientReason { DuplicateTx, AccountLocked, InsufficientFunds }      // checked in this order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisputeReason { UnknownDeposit, NotSettled, NotDisputed }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Sight { First, Repeat }

// core — pure and total; the decider's initial state is `None`, a client with no account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct Overflow;  // Display, Error
pub fn open() -> Footprint;                                       // what `Opened` makes: zero, unlocked, no deposit
pub fn decide_client(account: &Account, command: &ClientCommand, sight: Sight) -> Result<ClientEvent, ClientReason>;
pub fn decide_dispute(deposit: Option<&Deposit>, command: &DisputeCommand) -> Result<DisputeEvent, DisputeReason>;
pub fn evolve(state: Option<Footprint>, event: &Event) -> Result<Footprint, Overflow>;
// `evolve(None, e)` applies `e` to `open()`; `Opened` on an open account changes nothing.

// run — the ports, declared beside their consumer: synchronous, owned values, their own error.
pub trait LedgerStore {
    type Error;
    fn load(&self, address: Address) -> Result<Option<Footprint>, Self::Error>;  // `None`: no account
    fn commit(&mut self, address: Address, footprint: Footprint) -> Result<(), Self::Error>;
    fn snapshot(&self) -> Result<Accounts, Self::Error>;
}
pub trait SeenIds { type Error; fn sight(&mut self, tx: TxId) -> Result<Sight, Self::Error>; }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Row { Parsed(Command), Malformed(RowError) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowError { Unreadable, TooManyFields, MissingAmount, InvalidAmount(AmountError) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SourceError { Unreadable, IncompleteHeader }
#[derive(Debug, PartialEq, Eq)] pub enum RunError<L, S> { Source(SourceError), Ledger(L), Seen(S) }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct Summary { pub accounts: Accounts, pub report: Report }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct Accounts(BTreeMap<ClientId, Account>);
impl Accounts { pub fn by_client(&self) -> impl Iterator<Item = (ClientId, Account)> + '_; }
impl FromIterator<(ClientId, Account)> for Accounts {}
// A reason or a kind is listed only once it has happened, so every count is at least one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report { rejected: BTreeMap<Reason, NonZeroU64>, malformed: BTreeMap<RowError, NonZeroU64> }
impl Report {
    pub fn rejected(&self) -> impl Iterator<Item = (Reason, NonZeroU64)> + '_;
    pub fn malformed(&self) -> impl Iterator<Item = (RowError, NonZeroU64)> + '_;
}
pub fn run<L: LedgerStore, S: SeenIds>(
    rows: impl IntoIterator<Item = Result<Row, SourceError>>,
    ledger: &mut L,
    seen: &mut S,
) -> Result<Summary, RunError<L::Error, S::Error>>;

// memory — `Error = Infallible`; an empty store is a real value, so `Default` builds one.
#[derive(Debug, Default)]
pub struct MemoryLedger { accounts: BTreeMap<ClientId, Account>, deposits: HashMap<Address, Deposit> }
#[derive(Debug, Default)] pub struct MemorySeenIds(HashSet<TxId>);
```

Why these shapes, where the records leave a choice:

- `Footprint` is a domain type: *State partitioned by client* defines the footprint, and the
  store, the core and the runtime all exchange it. Every command names a tx id, so a footprint
  sits at its command's `Address`, which the runtime passes to `load` and `commit`. The address
  stays outside the value: `open()` builds a footprint without an id, and no footprint can
  disagree with the address it is stored at.
- `Address` types the glossary's *address*: one value, where a client and a tx id passed apart
  could come from two commands. Commands carry it; events carry no id at all, since `evolve` reads
  none, and a log that keeps events pairs each with its address in the adapter that writes it.
- For a client command, the deposit at its address plays no part: a first sight means nothing
  was recorded under the id, and a repeat is rejected, committing at most an account it just
  opened, which has no deposits. A withdrawal's footprint therefore holds none.
- Each absence has one meaning: `Option<Footprint>` is a client with no account;
  `Option<Deposit>` is no deposit of this client under that id, whether never recorded or charged
  back, which *A per-deposit dispute lifecycle* makes the same thing.
- `evolve` takes `Option<Footprint>` so `Opened` is an event like any other while `decide_*`
  only ever see an open account; `None` is the state every client starts from. `open()` is the
  footprint `Opened` makes, and the runtime opens an account with it: it cannot fail, so no
  runtime path handles an overflow that opening cannot cause. An event that changes a deposit sets
  it from its own amount and the others leave it, so `evolve` never reads the deposit it replaces,
  and a commit writes or removes the deposit at the command's address. After a rejection the
  runtime commits only an account it just opened, whose footprint holds no deposit.
- Each decision returns exactly what it can: `decide_client` a `ClientEvent` or a `ClientReason`,
  `decide_dispute` a `DisputeEvent` or a `DisputeReason`, the overview's Commands table as two
  signatures. `Event` and `Reason` nest the two families, and the runtime widens by hand
  (`.map(Event::Client).map_err(Reason::Client)`), since `rust-errors` keeps `From` for foreign
  errors.
- `Lock` stands where a `bool` would: `Account::new(available, held, Lock::Locked)` says at the
  call site what `true` would hide.
- `Account`, `Deposit` and `Footprint` keep their fields private, as the Rust reference of
  `functional-architecture` asks, and readings are methods. Each has one constructor: `Account::new`
  and `Deposit::new` are `pub(crate)`, since only `evolve` builds them, so every account and
  deposit a test sees came from `evolve`; `Footprint::new` is public, since it pairs an account
  and a deposit its caller already holds, which is how `evolve` and every store's `load` build one.
- `RunError` is generic over the stores' errors, so `Infallible` stores make those variants
  uninhabited instead of erased.
- `Report` counts by finite keys, so its size is bounded by the number of kinds, and each count
  is a `NonZeroU64`, since a key is listed only once it has happened; `RowError` carries the
  `AmountError` so the count keeps why an amount was refused.

## Generators

Shared through `backend/tests/it/support/generate.rs`; every value enters through production
parsing, so every counterexample is input a source can deliver. One draw in four, `EDGE_ODDS`,
takes an edge.

- `amount_spelling` — raw text: an `AMOUNT_EDGES` spelling; or a number-shaped spelling (up to
  seventeen whole digits with leading zeros, an optional point, up to six fraction digits); or up
  to eight characters drawn from digits, `.`, `-`, `+`, `e`, ` `, `_`, `,` and a non-ASCII digit.
- `amount_value` — `(whole, fraction)` with `fraction < 10_000`; edges `(0, 0)`, `(0, 1)`,
  `(1, 0)`, `(0, 9_999)`, `(999_999_999_999_999, 9_999)`, `(1_000_000_000_000_000, 0)`,
  `(u64::MAX, 9_999)`; otherwise a whole part of random magnitude.
- `any_text` — for totality: up to sixteen characters mixing ASCII digits, punctuation and
  letters with multi-byte characters (`'\u{663}'`, `'\u{ff11}'`, `'\u{e9}'`, `'\u{feff}'`); edges:
  empty, sixty-four zeros then `1`, a multi-byte character after a point.
- `balance_moves(random: &mut Gen) -> Vec<(Direction, String)>`, with
  `pub(crate) enum Direction { Credit, Debit }` — up to twelve moves, each the text of a valid
  amount: `{whole}.{fraction:04}` with `whole < 1000` and not both parts zero, or one of `0.0001`
  and `999999999999999.9999`. One draw in four instead repeats the previous amount in the
  opposite direction, so a balance lands on zero from either side; and, while the balance's
  magnitude is an amount, one draw in four of the rest moves the whole balance back to zero,
  spelled without trailing zeros, so a balance whose last decimal is zero also lands on zero
  through an amount spelled with fewer decimals.
- `InputRow` (core, extended in run) — a row of the input as a source reads it, before parsing.
  Its fields stay raw on purpose (`rust-properties`): a missing amount is `None`, never `Some("")`,
  and no amount is padded with spaces, since the CSV source reads an empty field as absent and
  trims every field. The core ticket adds `Fields` and `Kind`; the run ticket adds `Unreadable`
  and `TooManyFields`:

  ```rust
  pub(crate) enum InputRow { Fields { kind: Kind, client: u16, tx: u32, amount: Option<String> }, Unreadable, TooManyFields }
  pub(crate) enum Kind { Deposit, Withdrawal, Dispute, Resolve, Chargeback }
  ```

- Row builders (core) — `deposit(client, tx, amount)`, `withdrawal(client, tx, amount)`,
  `dispute(client, tx)`, `resolve(client, tx)`, `chargeback(client, tx)`, each an
  `InputRow::Fields`. Edge episodes and fixtures are written with them, so they read like the
  overview's rows: `[deposit(2, 20, "10"), withdrawal(2, 21, "10"), dispute(2, 20), chargeback(2, 20)]`.
- `rows` (core, extended in run) — up to sixteen input rows. Ordinary draws: any kind, a client
  from `[1, 2, 3]`, a fresh tx for a deposit or withdrawal, a recorded deposit's tx (any client's)
  for a dispute command, an amount `whole[.digits]` with `whole < 100`. One draw in four takes an
  `Edge`, picked uniformly from `EDGES`:

  ```rust
  enum Edge { RepeatedTx, UnknownTx, ForeignTx, AfterChargeback, ExactBalance, BoundaryAmount, MalformedRow }
  ```

  1. `RepeatedTx`: a deposit or withdrawal reusing any earlier deposit or withdrawal tx;
  2. `UnknownTx`: a dispute command naming `u32::MAX`, `4_000_000_000` or a withdrawal's tx;
  3. `ForeignTx`: a dispute command naming another client's deposit;
  4. `AfterChargeback`: on a fresh client, two deposits, a dispute and a chargeback of the first,
     then up to three of: a deposit, a withdrawal, any dispute command on either deposit;
  5. `ExactBalance`: on a fresh client, a deposit, optionally a second deposit and its dispute,
     then a withdrawal of exactly the first amount, optionally followed by `0.0001` more;
  6. `BoundaryAmount`: a deposit or withdrawal of `0.0001`, `999999999999999.9999`,
     `1000000000000000`, `0`, `0.00001`, `1.50000` or `0001.5`;
  7. `MalformedRow` (run ticket): an unreadable row, a row with too many fields, a deposit or
     withdrawal with no amount, or one whose amount is an `AMOUNT_EDGES` spelling other than
     `""`, `" 1"` and `"1 "`, which no source delivers.

  A repeated tx id comes, one draw in two, from the client of the row it repeats, which reaches
  locked accounts; otherwise repeated and unknown edges draw the client from
  `[1, 2, 3, 0, u16::MAX]`. `Fresh { client, tx }` hands out fresh ids, clients from 100 and tx
  ids from 0, so neither collides with an edge id.
- `commands` (core) — the rows that read as commands, through `command(&InputRow) ->
  Option<Command>`; `events` (core) — up to twelve events of every kind, amounts parsed from valid
  spellings including `0.0001` and `999999999999999.9999`; `address(random: &mut Gen) -> Address`
  (memory) — one address, a client from `[1, 2, 3, 0, u16::MAX]` and a tx id from
  `[1, 2, 3, 0, u32::MAX]`, so addresses recur and the store laws meet every boundary id.
- `unique` and `interleave` (run) — `unique(&[InputRow]) -> Unique` drops malformed rows and every
  deposit or withdrawal whose tx an earlier one used; `interleave(random, &Unique) -> Unique` merges
  the per-client sequences in random order, keeping each client's order. Only these two build a
  `Unique(Vec<InputRow>)`, read through `Unique::rows(&self) -> &[InputRow]`, so the reordering
  law's hypothesis is its parameter's type.

## Commits

The amended records commit keeps its message. Each new message is written to a scratch file
outside the repository and committed with `git commit -F <file>`.

```text
feat(domain): parse amounts once into exact four-decimal money

> Every later layer receives ids, amounts and balances that cannot hold a bad value, and the
> dependency gate now proves the build graph holds only admitted crates.
```

```text
feat(core): decide and evolve ledger commands as a pure decider

> Every money rule is a total function of a footprint and a command, held by state-machine laws
> independent of any runtime or store.
```

```text
feat(run): run commands in arrival order over the store ports

> Rows flow through intake, the decider and the stores with each failure tier typed, and the
> result equals a naive model of the rules: the reference every later runtime must match.
```

```text
feat(memory): keep the ledger in memory stores held to the store laws

> The library runs end to end on in-memory stores, ready for the command-line runtime.
```

```text
docs(prompts): add prompt history 0002 (stage 1 planning)

> The path from the request to the approved plan, its domain-model records and their skills stays
> reproducible.
```

```text
docs(prompts): add prompt history 0003 (stage 1 library core)

> The path from the approved plan to the four commits of the library core stays reproducible.
```

## Per-commit gate

1. `make ci` — green; the test summary line goes in the ticket.
2. Each of the ticket's laws has its failure line in Law evidence, from its deliberate break
   rather than a stub, and every break is undone: `git diff --quiet` exits 0.
3. The code-reviewer agent reviews the staged diff; blockers and should-fixes are fixed and
   `make ci` re-run.
4. The two forbidden-name greps of Global constraints: both silent. No record cited by
   number or filename outside the decision logs: `git grep -n --untracked -E
   'docs/(adr|design)/[0-9]{4}|ADR ?[0-9]+|[Dd]ecision #?[0-9]+|record #?[0-9]+' -- .
   ':!docs/adr' ':!docs/design' ':!.claude/skills/architecture-decisions'` lists nothing.
5. `git status --short` lists only the ticket's files and `docs/prompts/0002-stage-1-planning.md`,
   untracked until Closing; the message comes from `git commit -F`.

## Risks

- Names the records fix, such as `RunError` in `run` and `MemoryLedger` in `memory`, raise no
  lint: under clippy 0.1.88 a probe with the workspace's lints passed clean, since
  `module_name_repetitions` is not in the pedantic group. Write no `#[expect]` for it; one would
  fail as an unfulfilled expectation.
- A module named `core` sits beside the `core` crate. A scratch probe under the workspace's lints
  passed clean; only a bare `core::` path at a crate root would resolve to the module, and
  neither crate writes one.
- `make deps` runs `cargo tree` online: on a cold cache it downloads every locked crate to read
  its manifest, about 42 MB with `--target all`, since `--offline` fails there.
- Until the CSV source lands, generators build `Row` values and amounts are the only free text
  fed through production parsing; the CSV ticket moves the generators onto the source, as
  *Correctness defined by laws and a model* requires.
- Boundary constructors with no production caller yet, as the person decided on 2026-10-10:
  `ClientId::from`, `TxId::from` and `Address::new` are the library's boundary API, through which
  an adapter turns parsed fields into domain values. In stage 1 only the generators call them; the
  CSV source does from the next stage. They are not helpers kept for a test, which `CLAUDE.md`
  § Tests rules out: no test reaches anything through them that the public API would otherwise
  hide.
- The views' boundary API likewise, by the build session's ruling: `FromIterator<(ClientId,
  Account)> for Accounts` is how a store's `snapshot` builds the view, and only the run ticket's
  store double calls it until `MemoryLedger` does in the memory ticket; `Accounts::by_client`,
  `Report::rejected` and `Report::malformed` are what the sinks read from the next stage, and
  until then only the laws read them. They are the views' public API, not helpers kept for a test.

## Deliberately deferred

- `Display` for `Reason`, `ClientReason` and `DisputeReason`: the report's stderr lines arrive
  with `main.rs`, their first reader.

- `main.rs`, `CsvSource` and `render_csv`, `csv` and `serde`, black-box tests through the binary,
  and benchmarks: the next stage.
- Store failures: the memory stores cannot fail (`Infallible`), so the `Ledger` and `Seen`
  variants of `RunError` are first exercised by a store that can.
- `Reason::Overflow`: unreachable below the amount cap, as *Exact decimal amounts with four
  places* records; no test reaches it without bypassing `Amount`.
- Exactness past 7.9 × 10^24: `rust_decimal`'s `checked_add` is `None` only when the whole part
  overflows, and rounds silently before that. `Balance` wraps it as it is and documents the
  bound; the same record keeps every reachable balance below it.

## Tickets

Done in the planning session, before execution:

- [x] **amend the records commit** — *A domain model in the records' language* and *Entities
      identified by their keys*, signed off (`Draft` to `Accepted`), with their index lines;
      `.claude/skills/domain-modeling/SKILL.md` and `.claude/skills/curry-howard/SKILL.md`; and
      their syncs: `CLAUDE.md` (principle and skill rows), `.claude/agents/code-reviewer.md`
      (reads both skills; a domain-model check), `.claude/skills/functional-architecture/SKILL.md`
      (links both), `.claude/skills/rust-properties/references/harness.md` (the `Seed` type, which
      the domain ticket copies verbatim), `README.md`, `docs/overview.md` (Terms gains *address*),
      `docs/architecture.md`, `docs/patterns.md`. After the per-commit gate, `git commit --amend
      --no-edit` on the base commit, which kept its message.

Each task below is one ticket and one commit, in order. Its brief is its ticket file, read whole;
its commit message is under Commits, and the Per-commit gate closes it. This plan, the four
tickets and the planning lesson land with the domain commit; each later commit ticks its task
here and fills its ticket's evidence.

### Task 1: stage-1-domain

- [x] `tasks/stage-1-domain.md`: `rust_decimal` admitted and `make deps` reading the build graph;
      ids, `Amount`, `Balance`, account and history types, commands, events; four laws.

### Task 2: stage-1-core

- [x] `tasks/stage-1-core.md`: the decider and nine laws through a state-machine driver. The
      overview's *a rejection changes no balance* is a fact of the core's types, since a rejection
      carries no event and only an event changes a footprint, so run checks it where it can fail.

### Task 3: stage-1-run

- [x] `tasks/stage-1-run.md`: ports, rows, errors, projections and the sequential runtime; eight
      laws over store doubles, the rejection law among them.

### Task 4: stage-1-memory

- [x] `tasks/stage-1-memory.md`: `MemoryLedger` and `MemorySeenIds`, four store laws, run's laws
      moved onto them, the worked example end to end.

## Closing

After Task 4, in this order:

1. The whole-branch review covers the four task commits. Its base is the amended records commit,
   not `git merge-base main HEAD`, which since the amend is the workspace commit. Each fix passes
   the Per-commit gate and becomes a `git commit --fixup=<sha>` of the commit it belongs to; then
   `git rebase --autosquash --exec 'make ci' <records commit>` squashes every fixup, those made
   between tickets included, and proves every commit green.
2. The Review section, below, is filled in and folded into the memory commit the same way.
3. The prompt histories land as the branch's last two commits, each holding its entry alone:
   first `docs/prompts/0002-stage-1-planning.md`, the planning session's, written as that session
   closed and untracked until now; then `docs/prompts/0003-stage-1-library-core.md`, the build
   session's, written with the `prompt-history` skill.
4. The branch stays as it is. Pushing, merging and replacing the records commit on `main` are the
   person's to do.

## Review

Closed on 2026-10-10, executed inline in one session. The library core of stage 1 is four commits
on `feat/stage-1-core`, on top of the amended records commit:

| Commit | Ticket | Laws | Tests after it |
|---|---|---|---|
| `feat(domain): parse amounts once into exact four-decimal money` | `tasks/stage-1-domain.md` | 4 | 4 |
| `feat(core): decide and evolve ledger commands as a pure decider` | `tasks/stage-1-core.md` | 9 | 13 |
| `feat(run): run commands in arrival order over the store ports` | `tasks/stage-1-run.md` | 8 | 21 |
| `feat(memory): keep the ledger in memory stores held to the store laws` | `tasks/stage-1-memory.md` | 4, and the worked example | 26 |

Every law failed once against its deliberate break, and each ticket's Law evidence holds the
line. Every commit passed `make ci` and a code review of its staged diff before it landed.

How the plan changed while it ran: first the scan's fixes, then rulings recorded in each ticket's
Review.

- During the first ticket, before its commit, a scan that compiled every ticket's code against
  § Interfaces found four gaps, each fixed where it lay: the File map now names `Account::lock`;
  the row builders are `pub(crate)`, and `rows` keeps the ids it draws in its own state; the
  views' boundary API is in Risks; the memory ticket's Files list names the generators.
- Domain: `make deps` reads the graph with every feature, as `make lint` builds it;
  `amount_value` yields a named value; `Display for Amount` stays.
- Core: an oracle law holds the client half of the Commands table, in place of the
  locked-account law, which left unlocked accounts unchecked; one draw in two, a repeat comes
  from the repeated row's own client, which reaches locked accounts.
- Run: the source-error law also requires that nothing after the first error is read.
- Memory: loads name the address just committed one time in two; the store laws meet clients `0`
  and `u16::MAX`; the empty ledger is read.

The final review of the whole branch found nothing critical and one important gap, fixed: the
balance law's generator brought a balance back to zero only through an amount at the scale it
was added at, so the law missed, in 10 runs of 10, a break that turns a zero reached across
scales into `-0.0000`; the code itself prints `0.0000` there. The generator now also moves the
whole balance back to zero, spelled without trailing zeros, and the law catches that break in 10
of 10. The second Review focus item, which understated the store laws, is corrected here, and the
planning session's prompt history lands in its own commit. The other eleven points the reviewer
declined to judge stand, for these reasons:

- `Account::total()`'s `None` is unreachable below the amount cap; how the CSV sink prints it is
  the next stage's to decide.
- `ClientId` and `TxId` have no reading back or `Display` until their first reader, the CSV sink,
  as the plan defers `Display` for the rejection reasons.
- A dispute row carrying an amount is untested until the CSV source exists, which must ignore the
  amount.
- Id parsing that accepts `+1`, `01` and `0x10` is a recorded cost of *Rows parsed into commands
  at the boundary*.
- `make deps` matches crates by name: a crate from another source would show in the manifest diff
  that every review reads.
- `HashMap` and `HashSet` in `memory.rs` are never iterated, so hash order never reaches output.
- The memory `Reference` and the core `World` mirror the code they check: a key-value contract
  leaves little else to write.
- `rust_decimal` rounding past 7.9 × 10^24, and `{:.4}` dropping a fifth decimal, are
  unreachable: below that bound every amount and balance has at most four decimals.
- `mod core` at the test crate root shadows nothing written: no bare `core::` path appears, and
  Risks records it.
- `cargo tree --target all` needs the network; CI runs online, as Risks records.
- `unique` keeps a dispute whose target row it dropped as a repeat: that row was a rejected repeat
  in either order.

The review's five minor findings are deferred.

Deferred:

- The runtime opens an account through `open()` and never answers `Event::Opened`. Before an
  event log lands, decide whether a step yields it, so a replay reopens the clients named only by
  rejected commands.
- The reordering law could compare whole summaries, not only accounts: with unique tx ids the
  report is order-invariant too.
- `prop_decide_never_changes_state` checks determinism, all a `&`-typed decision can break;
  rename it or fold it into the table law.
- `Account` offers `is_locked() -> bool` beside the crate-visible `lock() -> Lock`; one public
  reading, the enum, would do.
- `CLAUDE.md` § Tests describes `support.rs` alone; the support tree now holds `generate.rs` and
  `model.rs` too.
- As the person asked during the build, the tests move onto `proptest` after stage 1. That needs
  a record superseding *csv, serde and rust_decimal as dependencies*, whose set is exact, to admit
  it and the crates it builds on as dev-dependencies, and `make deps` to admit them for dev
  builds only, failing when one reaches the normal graph.
