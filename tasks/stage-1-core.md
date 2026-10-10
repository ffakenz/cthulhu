# stage-1-core — the decider: total checks and the only computation, held to nine laws

Plan of record: `tasks/todo.md`

## Context

The domain types exist but nothing decides or computes. This ticket adds `open`, `decide_client`,
`decide_dispute` and `evolve` in `backend/src/core.rs`, and a state-machine driver in the test crate
that folds generated commands the way a runtime would and checks nine laws after every step. It
lands as one commit, `feat(core): …`.

The overview's *a rejection changes no balance* is not among them. In the core it is a fact of the
types: a rejection carries no event, and only an event changes a footprint, so the driver's step
after a rejection is the footprint it was decided on by construction, and a law there could only
restate `open`, which `prop_first_command_opens_the_account` already pins. The half that can fail
is the runtime's commit after a rejection, and the run ticket checks it.

Records that govern it: *A pure core*, *State changes only through events*, *State partitioned by
client*, *Correctness defined by laws and a model*, *A domain model in the records' language*,
*Entities identified by their keys*; design records *Accounts opened by the first command*, *A
per-deposit dispute lifecycle*, *Client commands refused on locked accounts*; the check order in
`docs/overview.md` § Commands. The `domain-modeling` and `curry-howard` skills apply to every type
it adds, test types included.

## Phase 0 — evidence (read-only)

- [ ] `git log --oneline -1` — shows the `feat(domain): …` commit.
- [ ] `make ci` — green, with the four domain laws passing.

## Phase 1 — the row generator

Files: `backend/tests/it/support/generate.rs` (extend).

- [ ] Add `InputRow` with its `Fields` variant, `Kind`, the five row builders, `Edge` with its six
      variants and `EDGES`, `Fresh`, `rows`, `command`, `commands` and `events`, exactly as
      `tasks/todo.md` § Generators states.
- [ ] Every amount enters through `Amount::from_str`, and every address through
      `Address::new(ClientId::from(client), TxId::from(tx))`; `command(&InputRow) ->
      Option<Command>` is `None` for a missing or unparseable amount, so the core only ever sees
      what a source could deliver.

## Phase 2 — `open`, the constructors, and stub decisions (red)

Files: `backend/src/lib.rs`, `backend/src/core.rs`, `backend/src/domain/ledger.rs`,
`backend/tests/it/main.rs`, `backend/tests/it/core.rs`.

- [ ] `pub mod core;` in `lib.rs`; `mod core;` in `tests/it/main.rs`.
- [ ] In `domain/ledger.rs`, the `pub(crate)` items whose first user is `evolve`, so `dead_code`
      stays quiet: `Account::new`, `Account::lock` and `Deposit::new`.
- [ ] `core.rs` with the signatures from `tasks/todo.md` § Interfaces. `open` is real from the
      start: `Footprint::new(Account::new(Balance::ZERO, Balance::ZERO, Lock::Unlocked), None)`.
      Lint-clean stubs for the rest: `decide_client` returns `Err(ClientReason::AccountLocked)`,
      `decide_dispute` returns `Err(DisputeReason::UnknownDeposit)`, `evolve` returns
      `Ok(state.unwrap_or_else(open))`.
- [ ] `tests/it/core.rs`: the driver below, the lifecycle oracle and the nine laws.
- [ ] Run `cargo test --test it core::` against the stubs: all nine pass, since every command is
      refused and an invariant holds vacuously then. Record that. Each law's red is its deliberate
      break in Phase 4: the real body with exactly the bug the law exists to catch.

The driver mirrors the runtime's order — intake, load, open, decide, evolve, commit — over plain
maps. A step records only what happened, each varying part a type of its own, and holds no oracle
value; a law is asked after every step, of the step and of the world as the step left it:

```rust
/// One command as the runtime took it.
#[derive(Debug, Clone, Copy)]
struct Step { intake: Intake, start: Start, outcome: Outcome }

/// The command as intake passed it on, with its sight for a client command.
#[derive(Debug, Clone, Copy)]
enum Intake { Client(ClientCommand, Sight), Dispute(DisputeCommand) }

/// Where the footprint the command was decided on came from: the world, or `open` for a client
/// with no account.
#[derive(Debug, Clone, Copy)]
enum Start { Loaded(Footprint), Opened }

/// What the decider answered: an event and the footprint `evolve` made from it, or a rejection.
#[derive(Debug, Clone, Copy)]
enum Outcome { Accepted { event: Event, next: Footprint }, Rejected(Reason) }

/// A client's running sums of what was deposited, withdrawn and charged back.
#[derive(Debug, Clone, Copy, Default)]
struct Books { deposited: Units, withdrawn: Units, charged_back: Units }

/// What the stores and intake would hold, with each client's history and books: the world the
/// laws read.
#[derive(Debug, Default)]
struct World {
    accounts: BTreeMap<ClientId, Account>,
    history: BTreeMap<ClientId, BTreeMap<TxId, Deposit>>,
    seen: BTreeSet<TxId>,
    books: BTreeMap<ClientId, Books>,
}
```

```rust
/// The decision for `intake` on `footprint`, routed and widened as the runtime does it.
fn decide(intake: &Intake, footprint: &Footprint) -> Result<Event, Reason> {
    match intake {
        Intake::Client(command, sight) => decide_client(footprint.account(), command, *sight)
            .map(Event::Client)
            .map_err(Reason::Client),
        Intake::Dispute(command) => decide_dispute(footprint.deposit(), command)
            .map(Event::Dispute)
            .map_err(Reason::Dispute),
    }
}

impl Intake {
    fn address(&self) -> Address {
        match self {
            Self::Client(command, _) => command.address(),
            Self::Dispute(command) => command.address(),
        }
    }
}

impl Start {
    /// The footprint the command was decided on.
    fn footprint(&self) -> Footprint {
        match *self {
            Self::Loaded(footprint) => footprint,
            Self::Opened => open(),
        }
    }
}

impl Outcome {
    /// What `decide` answered.
    fn decision(&self) -> Result<Event, Reason> {
        match *self {
            Self::Accepted { event, .. } => Ok(event),
            Self::Rejected(reason) => Err(reason),
        }
    }
}

impl Step {
    /// The footprint the step committed: the evolved one, or the one it was decided on.
    fn after(&self) -> Footprint {
        match self.outcome {
            Outcome::Accepted { next, .. } => next,
            Outcome::Rejected(_) => self.start.footprint(),
        }
    }
}

impl Books {
    /// The books after `decision`: an accepted deposit, withdrawal or chargeback moves its sum.
    fn after(self, decision: Result<Event, Reason>) -> Option<Self> {
        Some(match decision {
            Ok(Event::Client(ClientEvent::Deposited(amount))) => {
                Self { deposited: self.deposited.plus(units(amount)?), ..self }
            }
            Ok(Event::Client(ClientEvent::Withdrawn(amount))) => {
                Self { withdrawn: self.withdrawn.plus(units(amount)?), ..self }
            }
            Ok(Event::Dispute(DisputeEvent::ChargedBack(amount))) => {
                Self { charged_back: self.charged_back.plus(units(amount)?), ..self }
            }
            Ok(Event::Opened | Event::Dispute(DisputeEvent::Held(_) | DisputeEvent::Released(_)))
            | Err(_) => self,
        })
    }

    /// Whether a chargeback happened: every chargeback moves at least one unit.
    fn charged(&self) -> bool {
        self.charged_back > Units::ZERO
    }
}

impl World {
    /// One command through intake, load, open, decide, evolve and commit at its address, with the
    /// outcome booked; `None` if `evolve` overflowed or an amount did not read back as units.
    fn step(&mut self, command: Command) -> Option<Step> {
        let intake = match command {
            Command::Client(client_command) => {
                let sight = if self.seen.insert(client_command.address().tx()) {
                    Sight::First
                } else {
                    Sight::Repeat
                };
                Intake::Client(client_command, sight)
            }
            Command::Dispute(dispute_command) => Intake::Dispute(dispute_command),
        };
        let address = intake.address();
        let start = match self.accounts.get(&address.client()) {
            Some(&account) => Start::Loaded(Footprint::new(account, self.deposit(address))),
            None => Start::Opened,
        };
        let outcome = match decide(&intake, &start.footprint()) {
            Ok(event) => Outcome::Accepted { event, next: evolve(Some(start.footprint()), &event).ok()? },
            Err(reason) => Outcome::Rejected(reason),
        };
        let step = Step { intake, start, outcome };
        let after = step.after();
        let _ = self.accounts.insert(address.client(), *after.account());
        let history = self.history.entry(address.client()).or_default();
        let _ = match after.deposit() {
            Some(&deposit) => history.insert(address.tx(), deposit),
            None => history.remove(&address.tx()),
        };
        let books = self.books.entry(address.client()).or_default();
        *books = books.after(outcome.decision())?;
        Some(step)
    }

    /// The deposit in history at `address`.
    fn deposit(&self, address: Address) -> Option<Deposit> {
        self.history.get(&address.client()).and_then(|history| history.get(&address.tx())).copied()
    }

    /// The client's books; a client no command has named has none.
    fn books(&self, client: ClientId) -> Books {
        self.books.get(&client).copied().unwrap_or_default()
    }

    /// The sum of the client's `Disputed` deposits.
    fn disputed(&self, client: ClientId) -> Option<Units> {
        self.history
            .get(&client)
            .into_iter()
            .flat_map(BTreeMap::values)
            .filter(|deposit| deposit.status() == Status::Disputed)
            .try_fold(Units::ZERO, |sum, deposit| Some(sum.plus(units(deposit.amount())?)))
    }
}

/// An amount in units of 0.0001, read back through the grammar model.
fn units(amount: Amount) -> Option<Units> {
    model::amount_units(&amount.to_string()).ok().map(Units::from)
}

/// Runs `commands` through a fresh world and asks `law` after every step, of the step and the
/// world as the step left it; a step the world cannot take fails the law.
fn holds(commands: &[Command], law: impl Fn(&World, &Step) -> bool) -> bool {
    commands
        .iter()
        .try_fold(World::default(), |mut world, &command| {
            let step = world.step(command)?;
            law(&world, &step).then_some(world)
        })
        .is_some()
}

/// A law over every step of generated runs.
fn driver_law(label: &'static str, law: impl Fn(&World, &Step) -> bool) -> Result<(), Failure> {
    support::for_all(label, 500, generate::commands, |commands| holds(commands, &law))
}
```

- No list of steps is kept: each law sees one step at a time, and a failing seed replays the
  whole run.
- No helper panics: they return `Option`, `bool` or the harness's `Result`, and each `#[test]`
  panics with the harness's failure line: `if let Err(failure) = driver_law(…) { panic!("{failure}"); }`.

## Phase 3 — the real decider (green)

Files: `backend/src/core.rs`.

- [ ] `decide_client`: `Repeat` is `DuplicateTx`; then a locked account is `AccountLocked`; then a
      withdrawal above `available` (`Balance::from(amount) > account.available()`) is
      `InsufficientFunds`; otherwise `Deposited` or `Withdrawn` with the command's amount.
- [ ] `decide_dispute`: no deposit is `UnknownDeposit`; then the lifecycle table, one `match` over
      `(command, status)` with no catch-all: dispute `Settled` → `Held`; dispute `Disputed` →
      `NotSettled`; resolve `Disputed` → `Released`; chargeback `Disputed` → `ChargedBack`;
      resolve or chargeback `Settled` → `NotDisputed`. Each event carries the deposit's amount.
- [ ] `evolve`: `let footprint = state.unwrap_or_else(open);` reads the account through its
      readings, then one arm per event builds the next footprint with `Footprint::new`,
      `Account::new` and `Deposit::new`, every sum through `Balance::checked_add` /
      `checked_sub` mapped to `Overflow`. Events carry no id: the footprint already sits at the
      command's address.

  | Event | `available` | `held` | lock | `deposit` |
  |---|---|---|---|---|
  | `Opened` | | | | unchanged |
  | `Client(Deposited(amount))` | `+ amount` | | | `Some(Settled amount)` |
  | `Client(Withdrawn(amount))` | `- amount` | | | unchanged |
  | `Dispute(Held(amount))` | `- amount` | `+ amount` | | `Some(Disputed amount)` |
  | `Dispute(Released(amount))` | `+ amount` | `- amount` | | `Some(Settled amount)` |
  | `Dispute(ChargedBack(amount))` | | `- amount` | `Lock::Locked` | `None` |

- [ ] Verify: `cargo test --test it core::` — 9 passed.

## Phase 4 — the laws, and each one broken once

File: `backend/tests/it/core.rs`. Every law but one is `driver_law(label, |world, step| …)`;
`prop_evolve_is_total` draws `(generate::commands(random), generate::events(random))` through
`support::for_all` and asks `holds` itself. Below, `client` is `step.intake.address().client()`,
and a law that concerns one kind of step holds trivially for the others.

| Test | Label (the law) | Holds when, after every step | Deliberate break |
|---|---|---|---|
| `prop_decide_never_changes_state` | deciding twice on the same footprint gives the same answer | asked twice more, `decide(&step.intake, &step.start.footprint())` answers `step.outcome.decision()` both times (the `&` borrow already makes changing the footprint a compile error) | a `static` `AtomicBool` that `decide_client` flips on every call, refusing every other call |
| `prop_evolve_is_total` | evolve applies any event to any footprint, or to none, and never fails below the amount cap | for `None`, `Some(step.start.footprint())` and `Some(step.after())`, `evolve(state, &event).is_ok()` for every generated event | `Released` reads the footprint's deposit with `.expect("disputed")` |
| `prop_held_is_the_sum_of_disputed_deposits_and_never_negative` | held equals the sum of the client's disputed deposits, and is never negative | `step.after()`'s held is at least `Balance::ZERO`, and prints what `world.disputed(client)` prints | `Released` leaves `held` unchanged |
| `prop_total_is_deposited_minus_withdrawn_minus_charged_back` | total equals deposited minus withdrawn minus charged back | `step.after()`'s `total()` is `Some` and prints `deposited.minus(withdrawn).minus(charged_back)` of `world.books(client)` | `ChargedBack` leaves `held` unchanged |
| `prop_locked_exactly_when_charged_back` | an account is locked exactly when a chargeback happened | `step.after().account().is_locked() == world.books(client).charged()` | `ChargedBack` keeps the account's lock |
| `prop_only_held_makes_available_negative` | available falls below zero, or further below it, only through Held | `step.after()`'s available below zero and below `step.start.footprint()`'s implies `Outcome::Accepted { event: Event::Dispute(DisputeEvent::Held(_)), .. }` | `InsufficientFunds` compares the amount with `total` instead of `available` |
| `prop_dispute_lifecycle_allows_exactly_its_transitions` | a deposit enters history Settled, and dispute commands move it exactly as the lifecycle table says | for `Intake::Dispute(command)`, `follows_lifecycle(command, step.start.footprint().deposit(), step.outcome.decision(), step.after().deposit())`; for an accepted deposit, `step.after()` holds it `Settled` with its amount | resolve on a `Settled` deposit answers `Released` |
| `prop_locked_accounts_refuse_client_commands_only` | a locked account refuses deposits and withdrawals, and still applies dispute commands | with `step.start.footprint()` locked: `Intake::Client(_, sight)` answers `Err(Reason::Client(DuplicateTx))` on `Repeat`, else `Err(Reason::Client(AccountLocked))`; `Intake::Dispute(command)` follows the lifecycle | the lock is checked for deposits only |
| `prop_first_command_opens_the_account` | the first command naming a client opens its account with zero balances, unlocked | under `Start::Opened`, `open()` has zero available, held and total, is unlocked and holds no deposit, and `evolve(None, &Event::Opened) == Ok(open())`; `Outcome::Accepted { event, next }` gives `evolve(None, &event) == Ok(next)` | `open` locks the account |

The lifecycle oracle, written once in the test module as an exhaustive match, row for row with
*A per-deposit dispute lifecycle*'s two lists, transitions and rejections:

```rust
use backend::domain::DisputeCommand::{Chargeback, Dispute, Resolve};
use backend::domain::Status::{Disputed, Settled};

/// The dispute lifecycle: from a deposit's status, the status a dispute command leaves, `None`
/// once charged back, or the reason it is refused.
fn transition(command: DisputeCommand, from: Status) -> Result<Option<Status>, DisputeReason> {
    match (command, from) {
        (Dispute(_), Settled) => Ok(Some(Disputed)),
        (Resolve(_), Disputed) => Ok(Some(Settled)),
        (Chargeback(_), Disputed) => Ok(None),
        (Dispute(_), Disputed) => Err(DisputeReason::NotSettled),
        (Resolve(_) | Chargeback(_), Settled) => Err(DisputeReason::NotDisputed),
    }
}

/// The event a dispute command answers when the lifecycle allows it.
fn dispute_event(command: DisputeCommand, amount: Amount) -> DisputeEvent {
    match command {
        Dispute(_) => DisputeEvent::Held(amount),
        Resolve(_) => DisputeEvent::Released(amount),
        Chargeback(_) => DisputeEvent::ChargedBack(amount),
    }
}

/// Whether a dispute command answered, and left its deposit, as the lifecycle says: an unknown
/// deposit is refused, an allowed transition answers its event, and a refusal changes nothing.
fn follows_lifecycle(
    command: DisputeCommand,
    before: Option<&Deposit>,
    decision: Result<Event, Reason>,
    after: Option<&Deposit>,
) -> bool {
    let Some(&deposit) = before else {
        return decision == Err(Reason::Dispute(DisputeReason::UnknownDeposit)) && after.is_none();
    };
    match transition(command, deposit.status()) {
        Ok(to) => {
            decision == Ok(Event::Dispute(dispute_event(command, deposit.amount())))
                && after.map(|left| (left.amount(), left.status())) == to.map(|status| (deposit.amount(), status))
        }
        Err(reason) => decision == Err(Reason::Dispute(reason)) && after == Some(&deposit),
    }
}
```

- [ ] Stage this ticket's files first (`git add` with explicit paths), so a break in a new file
      shows in `git diff`. Then for each row: apply the break by editing, run
      `cargo test --test it <test>`, paste the failure line into Law evidence, undo the break by
      editing, and confirm `git diff --quiet` exits 0. Never `git checkout` or `git stash`
      mid-step, and never undo by copying a saved file back: the copy keeps its old modification
      time, and cargo reuses the broken build.
- [ ] Verify: `cargo test --test it core::` — 9 passed; `make ci` — green.

## Phase 5 — sync and land

- [ ] `docs/architecture.md`: the `core` row of the module table and the module box, which the
      records commit already brought to `open`, `decide_client`, `decide_dispute`, `evolve` and
      `Overflow`, still match `backend/src/core.rs`.
- [ ] `CLAUDE.md` and `README.md` layouts: add `core.rs`.
- [ ] Code-reviewer agent over the staged diff; act on every blocker and should-fix.
- [ ] The two forbidden-name greps of `tasks/todo.md` § Global constraints: both silent.
- [ ] `make ci` — green; paste the test summary line here.
- [ ] Tick this ticket's items and its line in `tasks/todo.md`; stage explicit paths; commit with
      `git commit -F` as `tasks/todo.md` § Commits gives it.

## Law evidence

| Law | Break | Command | Failure line |
|---|---|---|---|
| `prop_decide_never_changes_state` | a `static` `AtomicBool` that `decide_client` flips on every call | `cargo test --test it prop_decide_never_changes_state` | |
| `prop_evolve_is_total` | `Released` reads the footprint's deposit with `.expect("disputed")` | `cargo test --test it prop_evolve_is_total` | |
| `prop_held_is_the_sum_of_disputed_deposits_and_never_negative` | `Released` leaves `held` unchanged | `cargo test --test it prop_held_is_the_sum_of_disputed_deposits_and_never_negative` | |
| `prop_total_is_deposited_minus_withdrawn_minus_charged_back` | `ChargedBack` leaves `held` unchanged | `cargo test --test it prop_total_is_deposited_minus_withdrawn_minus_charged_back` | |
| `prop_locked_exactly_when_charged_back` | `ChargedBack` keeps the account's lock | `cargo test --test it prop_locked_exactly_when_charged_back` | |
| `prop_only_held_makes_available_negative` | `InsufficientFunds` compares with `total` | `cargo test --test it prop_only_held_makes_available_negative` | |
| `prop_dispute_lifecycle_allows_exactly_its_transitions` | resolve on a `Settled` deposit answers `Released` | `cargo test --test it prop_dispute_lifecycle_allows_exactly_its_transitions` | |
| `prop_locked_accounts_refuse_client_commands_only` | the lock is checked for deposits only | `cargo test --test it prop_locked_accounts_refuse_client_commands_only` | |
| `prop_first_command_opens_the_account` | `open` locks the account | `cargo test --test it prop_first_command_opens_the_account` | |

## Blocked / handed off

- [ ] None expected.

## Review

*Filled in when the work closes.*
