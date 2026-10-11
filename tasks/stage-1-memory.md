# stage-1-memory — in-memory stores that keep the store laws, and the library end to end

Plan of record: `tasks/todo.md`

## Context

The runtime's ports have only test doubles. This ticket adds `MemoryLedger` and `MemorySeenIds`,
proves them against the store laws, moves every run law onto them, deletes the doubles, and pins
the overview's worked example end to end through `run`. It closes stage 1's library and lands as
one commit, `feat(memory): …`.

Records that govern it: *Effects behind ports*, *Stores defined by their laws*, *Memory bounded by
state, not input*, *Entities identified by their keys*; design record *A per-deposit dispute
lifecycle* (a charged-back deposit leaves history); the worked example in `docs/overview.md`
§ Output. The `domain-modeling` and `curry-howard` skills apply to every type it adds, test types
included.

## Phase 0 — evidence (read-only)

- [x] `git log --oneline -1` — shows the `feat(run): …` commit.
- [x] `make ci` — green, with the domain, core and run laws passing.

## Phase 1 — the store laws against stub stores (red)

Files: `backend/src/lib.rs`, `backend/src/memory.rs`, `backend/tests/it/main.rs`,
`backend/tests/it/support.rs`, `backend/tests/it/support/generate.rs`,
`backend/tests/it/memory.rs`.

- [x] `pub mod memory;` in `lib.rs`; `mod memory;` in `tests/it/main.rs`.
- [x] `memory.rs` with `MemoryLedger` and `MemorySeenIds` as `tasks/todo.md` § Interfaces gives
      them, and stubs: `load` answers `Ok(None)`, `commit` writes nothing, `snapshot` is empty,
      `sight` always answers `First`.
- [x] `support.rs`: `pub(crate) fn infallible<T>(result: Result<T, Infallible>) -> T`, by
      `match result { Ok(value) => value, Err(never) => match never {} }`.
- [x] `support/generate.rs`: `address`, as `tasks/todo.md` § Generators states.
- [x] `tests/it/memory.rs`: the generator and reference below, and the four laws.
- [x] Red: `cargo test --test it memory::` — all four fail with counterexamples (a stub that keeps
      nothing breaks every store law); record the failure lines.

```rust
/// One operation on a ledger store.
#[derive(Debug, Clone, Copy)]
enum Operation {
    Commit(Address, Footprint),
    Load(Address),
    Snapshot,
}

/// What the store laws say a ledger holds: each client's last committed account, and the last
/// deposit committed at each address.
#[derive(Debug, Default)]
struct Reference {
    accounts: BTreeMap<ClientId, Account>,
    deposits: BTreeMap<Address, Deposit>,
}
```

- `fn operations(random: &mut Gen) -> Vec<Operation>`: takes the shared `generate::events` and,
  for each event, an address from `generate::address`, and commits `evolve(previous, &event)`
  there, where `previous` is what a reference fed the operations so far would load at that
  address (`None` before the client's first commit), skipping an event whose `evolve` overflows,
  which the amount cap rules out. A load or a snapshot follows each commit one time in two; a load
  names an address whose client is drawn from `[1, 2, 3, 4]` (4 is never committed). Footprints so
  come only from the production core over the same edge-weighted events the core laws use, commits
  at one address overwrite and remove each other's deposits, and a chargeback removes a deposit.
- `Reference::commit(address, footprint)` inserts the account under `address.client()`, then
  inserts the footprint's deposit at `address` or removes that key; `Reference::load(address)` is
  `None` without an account, else `Some(Footprint::new(account, deposit))` with the deposit at
  `address`.

| Test | Label (the law) | Holds when | Deliberate break |
|---|---|---|---|
| `prop_read_returns_what_the_last_commit_wrote` | a load returns what the last commit wrote | every `Load` equals `Reference::load` at that point | `commit` removes nothing when the footprint holds no deposit, so a charged-back deposit stays |
| `prop_commit_writes_account_and_deposit_together` | a commit's account and deposit are read back together | after each `Commit(address, footprint)`, `load(address) == Some(footprint)` | `commit` skips the account when the footprint holds a deposit |
| `prop_snapshot_reflects_every_commit_before_it` | a snapshot reflects every commit before it, by client id | every `Snapshot`'s `by_client()` equals the reference's accounts in key order | `snapshot` leaves out accounts whose balances are all zero |
| `prop_recorded_id_stays_recorded` | a recorded id stays recorded: the first sight of an id is `First`, every later one `Repeat` | over up to 24 sightings from `[0, 1, 2, 3, u32::MAX]`, each answer is `First` exactly when the reference set had not seen the id | `sight` forgets an id when it answers `Repeat` |

## Phase 2 — the stores (green)

File: `backend/src/memory.rs`.

- [x] `MemoryLedger { accounts: BTreeMap<ClientId, Account>, deposits: HashMap<Address, Deposit> }`:
      accounts in a `BTreeMap` because the snapshot is ordered by client id; deposits in a
      `HashMap`, only ever looked up by key. `load(address)` builds the footprint with
      `Footprint::new` from the account under `address.client()` and the deposit at `address`.
      `commit(address, footprint)` writes the account, then the footprint's deposit at `address`
      or `remove`s that key, so a charged-back deposit leaves memory. A discarded previous value is
      `let _ = …`, since discarding is the point.
- [x] `MemorySeenIds(HashSet<TxId>)`: `sight` is `First` when `insert` returns `true`.
- [x] Verify: `cargo test --test it memory::` — 4 passed.
- [x] Stage this ticket's files first (`git add` with explicit paths), so a break in a new file
      shows in `git diff`. Then for each row of the store-law table: apply the break by editing,
      run `cargo test --test it <test>`, paste the failure line into Law evidence, undo the break
      by editing, and confirm `git diff --quiet` exits 0. Never `git checkout` or `git stash`
      mid-step, and never undo by copying a saved file back: the copy keeps its old modification
      time, and cargo reuses the broken build.

## Phase 3 — run's laws on the real stores; the doubles go

Files: `backend/tests/it/run.rs`, `backend/tests/it/support.rs`, delete
`backend/tests/it/support/stores.rs`.

- [x] `summarize` builds `MemoryLedger::default()` and `MemorySeenIds::default()`.
- [x] Delete `support/stores.rs` and its `mod` line; `git grep -n 'stores::' backend/tests` lists
      nothing.
- [x] Verify: `cargo test --test it run::` — 8 passed.

## Phase 4 — the worked example, end to end

File: `backend/tests/it/run.rs`, with the fixtures beside the test, written with the row
builders so they read like `docs/overview.md` § Output:

```rust
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
```

- [x] `the_overview_example_ends_with_its_documented_accounts`: `printed(&summary)` of
      `summarize(source(&overview_rows()))` equals `overview_printed()`.
- [x] It passes on its first run: the behaviour already exists, so this is a characterisation
      test, and the ticket says so instead of manufacturing a red.
- [x] Verify: `cargo test --test it` — 26 passed (domain 4, core 9, run 8 and the worked example,
      memory 4).

## Phase 5 — sync, close and land

- [x] `README.md` § Status: the library is built — domain, core, the sequential runtime and the
      in-memory stores, with their laws — and the command-line runtime is next. `CLAUDE.md` §
      Status says the same.
- [x] `CLAUDE.md` and `README.md` layouts: add `memory.rs`.
- [x] `tasks/todo.md`: tick Task 4. Its Review section waits for Closing, which folds it into
      this commit.
- [x] Code-reviewer agent over the staged diff; act on every blocker and should-fix.
- [x] The two forbidden-name greps of `tasks/todo.md` § Global constraints: both silent.
- [x] `make ci` — green; paste the test summary line here. `test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
- [x] Stage explicit paths, including the deleted `support/stores.rs`; commit with
      `git commit -F` as `tasks/todo.md` § Commits gives it.

## Law evidence

| Law | Break | Command | Failure line |
|---|---|---|---|
| `prop_read_returns_what_the_last_commit_wrote` | `commit` removes nothing when the footprint holds no deposit | `cargo test --test it prop_read_returns_what_the_last_commit_wrote` | ``law `a load returns what the last commit wrote` failed at case 27 with [Commit(Address { client: ClientId(2), tx: TxId(0) }, Footpr…; replay: PROPERTY_SEED=0x18dd51bda21f8950`` |
| `prop_commit_writes_account_and_deposit_together` | `commit` skips the account when the footprint holds a deposit | `cargo test --test it prop_commit_writes_account_and_deposit_together` | ``law `a commit's account and deposit are read back together` failed at case 1 with [Load(Address { client: ClientId(2), tx: TxId(1) }), Commit(…; replay: PROPERTY_SEED=0x18dd51bddcadb1c8`` |
| `prop_snapshot_reflects_every_commit_before_it` | `snapshot` leaves out accounts whose balances are all zero | `cargo test --test it prop_snapshot_reflects_every_commit_before_it` | ``law `a snapshot reflects every commit before it, by client id` failed at case 2 with [Commit(Address { client: ClientId(0), tx: TxId(4294967295) …; replay: PROPERTY_SEED=0x18dd51be1bb158e8`` |
| `prop_recorded_id_stays_recorded` | `sight` forgets an id when it answers `Repeat` | `cargo test --test it prop_recorded_id_stays_recorded` | ``law `a recorded id stays recorded: the first sight of an id is First, every later one Repeat` failed at case 2 with [1, 3, 1, 2, 1, 3, 3, 2, 4294967295, 4294967295, 3, 3, 0, 3,…; replay: PROPERTY_SEED=0x18dd51be58dfb750`` |

Each break was applied and undone by an in-place edit of the staged `memory.rs`; `git diff
--quiet` exited 0 before and after every one. With the read law's break applied, twenty runs on
fresh seeds caught it twenty times. All were run on the final generator, after the review's
changes.

## Blocked / handed off

- [x] None.

## Review

`MemoryLedger` and `MemorySeenIds` keep the four store laws; run's eight laws now run on them; the
doubles are gone; and the overview's worked example passes end to end. Against the stubs all four
store laws failed with counterexamples; each then failed against its break, re-run on the final
generator.

Rulings:

- A load names the address just committed one time in two, otherwise a client from
  `[1, 2, 3, 0, u16::MAX, 4]` and a generated tx id. Drawn blind, as planned, the read law caught
  its break in 36 of 40 runs in the pre-flight's probe; now in 20 of 20.
- `address` draws clients from `[1, 2, 3, 0, u16::MAX]`, so the store laws meet every boundary id:
  a store losing clients 0 and `u16::MAX` passed all 26 laws before, since the run laws send those
  clients only rejected commands; it now fails at the first case.
- One time in two, a load or a snapshot of the empty ledger comes before the first commit: a
  store whose empty ledger loads an opened account now fails the read law.

The worked example passes on its first run, a characterisation test as planned; run once against
a core break, a chargeback that keeps the lock, it fails.

Code review of the staged diff: no blocker; one should-fix, acted on as ruled above; three nits,
all fixed: the empty ledger is read; the `Default` docs state what they build; `run.rs` names
`Printed` one way.

Review questions, `domain-modeling` (DM) and `curry-howard` (CH), for every type this ticket adds:

| Type | Name (DM 1) and kind (DM 2) | Proposition (CH 1, 2) | Keys, absences, derivations (DM 3–5, CH 3, 6) | What only a law states (CH 7) |
|---|---|---|---|---|
| `MemoryLedger` | *Stores defined by their laws*; a store | accounts by client id and deposits by address, its failure uninhabited | entities under their keys; a commit of no deposit removes the key, so history holds no charged-back deposit | the four store laws |
| `MemorySeenIds` | the same record; a store | the tx ids seen, its failure uninhabited | — | a recorded id stays recorded |
| `Operation` (test) | a call on the ledger port | a commit with exactly what `commit` takes, a load, or a snapshot | — | — |
| `Reference` (test) | what the store laws say a ledger holds | the last committed account per client and deposit per address | — | — |

No type here carries a file format into `domain` (DM 6) or changes a value it holds (DM 7); no
`bool`, tuple or bare primitive hides a name (CH 4); no function's return admits a case it never
produces (CH 5).
