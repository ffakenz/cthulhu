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

- [ ] `git log --oneline -1` — shows the `feat(run): …` commit.
- [ ] `make ci` — green, with the domain, core and run laws passing.

## Phase 1 — the store laws against stub stores (red)

Files: `backend/src/lib.rs`, `backend/src/memory.rs`, `backend/tests/it/main.rs`,
`backend/tests/it/support.rs`, `backend/tests/it/support/generate.rs`,
`backend/tests/it/memory.rs`.

- [ ] `pub mod memory;` in `lib.rs`; `mod memory;` in `tests/it/main.rs`.
- [ ] `memory.rs` with `MemoryLedger` and `MemorySeenIds` as `tasks/todo.md` § Interfaces gives
      them, and stubs: `load` answers `Ok(None)`, `commit` writes nothing, `snapshot` is empty,
      `sight` always answers `First`.
- [ ] `support.rs`: `pub(crate) fn infallible<T>(result: Result<T, Infallible>) -> T`, by
      `match result { Ok(value) => value, Err(never) => match never {} }`.
- [ ] `support/generate.rs`: `address`, as `tasks/todo.md` § Generators states.
- [ ] `tests/it/memory.rs`: the generator and reference below, and the four laws.
- [ ] Red: `cargo test --test it memory::` — all four fail with counterexamples (a stub that keeps
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

- [ ] `MemoryLedger { accounts: BTreeMap<ClientId, Account>, deposits: HashMap<Address, Deposit> }`:
      accounts in a `BTreeMap` because the snapshot is ordered by client id; deposits in a
      `HashMap`, only ever looked up by key. `load(address)` builds the footprint with
      `Footprint::new` from the account under `address.client()` and the deposit at `address`.
      `commit(address, footprint)` writes the account, then the footprint's deposit at `address`
      or `remove`s that key, so a charged-back deposit leaves memory. A discarded previous value is
      `let _ = …`, since discarding is the point.
- [ ] `MemorySeenIds(HashSet<TxId>)`: `sight` is `First` when `insert` returns `true`.
- [ ] Verify: `cargo test --test it memory::` — 4 passed.
- [ ] Stage this ticket's files first (`git add` with explicit paths), so a break in a new file
      shows in `git diff`. Then for each row of the store-law table: apply the break by editing,
      run `cargo test --test it <test>`, paste the failure line into Law evidence, undo the break
      by editing, and confirm `git diff --quiet` exits 0. Never `git checkout` or `git stash`
      mid-step, and never undo by copying a saved file back: the copy keeps its old modification
      time, and cargo reuses the broken build.

## Phase 3 — run's laws on the real stores; the doubles go

Files: `backend/tests/it/run.rs`, `backend/tests/it/support.rs`, delete
`backend/tests/it/support/stores.rs`.

- [ ] `summarize` builds `MemoryLedger::default()` and `MemorySeenIds::default()`.
- [ ] Delete `support/stores.rs` and its `mod` line; `git grep -n 'stores::' backend/tests` lists
      nothing.
- [ ] Verify: `cargo test --test it run::` — 8 passed.

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

- [ ] `the_overview_example_ends_with_its_documented_accounts`: `printed(&summary)` of
      `summarize(source(&overview_rows()))` equals `overview_printed()`.
- [ ] It passes on its first run: the behaviour already exists, so this is a characterisation
      test, and the ticket says so instead of manufacturing a red.
- [ ] Verify: `cargo test --test it` — 26 passed (domain 4, core 9, run 8 and the worked example,
      memory 4).

## Phase 5 — sync, close and land

- [ ] `README.md` § Status: the library is built — domain, core, the sequential runtime and the
      in-memory stores, with their laws — and the command-line runtime is next. `CLAUDE.md` §
      Status says the same.
- [ ] `CLAUDE.md` and `README.md` layouts: add `memory.rs`.
- [ ] `tasks/todo.md`: tick Task 4. Its Review section waits for Closing, which folds it into
      this commit.
- [ ] Code-reviewer agent over the staged diff; act on every blocker and should-fix.
- [ ] The two forbidden-name greps of `tasks/todo.md` § Global constraints: both silent.
- [ ] `make ci` — green; paste the test summary line here.
- [ ] Stage explicit paths, including the deleted `support/stores.rs`; commit with
      `git commit -F` as `tasks/todo.md` § Commits gives it.

## Law evidence

| Law | Break | Command | Failure line |
|---|---|---|---|
| `prop_read_returns_what_the_last_commit_wrote` | `commit` removes nothing when the footprint holds no deposit | `cargo test --test it prop_read_returns_what_the_last_commit_wrote` | |
| `prop_commit_writes_account_and_deposit_together` | `commit` skips the account when the footprint holds a deposit | `cargo test --test it prop_commit_writes_account_and_deposit_together` | |
| `prop_snapshot_reflects_every_commit_before_it` | `snapshot` leaves out accounts whose balances are all zero | `cargo test --test it prop_snapshot_reflects_every_commit_before_it` | |
| `prop_recorded_id_stays_recorded` | `sight` forgets an id when it answers `Repeat` | `cargo test --test it prop_recorded_id_stays_recorded` | |

## Blocked / handed off

- [ ] None expected.

## Review

*Filled in when the work closes.*
