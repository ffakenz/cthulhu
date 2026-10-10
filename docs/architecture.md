# Architecture

A picture of the decisions in `docs/adr/`, each named by its title.

## The pipeline

*Sources, sinks and projections*, *State partitioned by client*.

```text
 source ─► intake ─► partition ─► projections ─► sink
   │         │          │              │            │
 rows in   arrival   one writer   Accounts,      CSV table
 order     order,    per client:  Report         on stdout
           first     decide,
           sight     evolve
```

- A malformed row is counted and skipped; a rejection is counted; an unreadable source stops the
  run before anything reaches the sink (*Three failure tiers*).
- Rows stream one at a time; only the state, the counts and bounded queues stay in memory
  (*Memory bounded by state, not input*).

### In detail

*Effects behind ports*, *A pure core*, *State changes only through events*, *Stores defined by
their laws*.

```text
 input file
     │ opened by main.rs, which supplies the (implementation) of every [port]
     ▼
 [source] (CsvSource) ──┬─► Row::Malformed(RowError) ────────────────────────────► Report
     │                  └─► SourceError ──► RunError: the run stops, stdout stays empty
     │ Row::Parsed(Command)
     ▼
 intake ─── client command ──► [SeenIds] (MemorySeenIds) ──► Sight
     │
     ▼
 load footprint ◄───────────── [LedgerStore] (MemoryLedger) ◄───────────┐
     │                                                                  │
     ▼                                                                  │
 open if absent: Opened                                                 │
     │                                                                  │
     ▼                                                                  │
 decide (footprint, command, Sight)                                     │
     ├─► event ──► evolve ──┬─► next footprint ────────────┬─► commit ──┘
     │                      └─► Overflow ──┐               │
     └─► rejection ◄───────────────────────┘               │
           ├─► footprint as loaded, Opened included ───────┘
           └─────────────────────────────────────────────────────────────────────► Report

 end of input ──► [LedgerStore] snapshot ──► Accounts ──┐
                                               Report ──┴──► Summary ──► main.rs
 main.rs ─┬─► render_csv(Accounts) ──► [Write] (stdout)
          ├─► Report ──► stderr
          └─► RunError ──► ExitCode
```

- `[port]` marks a port; `(…)` is the implementation `main.rs` supplies: an adapter, or stdout for
  `Write`.
- The footprint is the client's account and at most one of its deposits. Dispute commands skip
  `SeenIds`.
- `decide` only checks conditions; `evolve` only computes, and fails only on overflow, which
  rejects the command. A rejection changes no balance; its commit stores only an account the
  step opened.
- `Accounts` and `Report` are the projections; `render_csv`, the sink adapter, writes the table to
  `Write`.

## Wiring

*Effects behind ports*, *Runtimes proven against a sequential reference*.

```text
 main.rs
   ├─ builds    CsvSource, MemoryLedger, MemorySeenIds
   ├─ calls     run(source, stores) ─► Summary { accounts, report } | RunError
   └─ renders   render_csv(accounts) ─► stdout;  report ─► stderr
```

Besides adapters, only `run`, the runtime, varies between deployments; today it is sequential,
one partition, and the reference that every later runtime must equal.

## Modules

```text
┌────────────────────────────────────────────────────────────────┐
│ main.rs  composition root: argument, file, stdio, exit code    │
├───────────────────────────────┬────────────────────────────────┤
│ csv_io   CsvSource,           │ memory   MemoryLedger,         │
│          render_csv           │          MemorySeenIds         │
│          (csv, serde)         │                                │
├───────────────────────────────┴────────────────────────────────┤
│ run      the runtime; ports: source, LedgerStore, SeenIds;     │
│          Row, RowError, SourceError, RunError, Summary,        │
│          Accounts, Report                                      │
├────────────────────────────────────────────────────────────────┤
│ core     decide_client, decide_dispute, evolve: pure, total    │
├────────────────────────────────────────────────────────────────┤
│ domain   ids, Amount, Balance, Account, Deposit, Status,       │
│          commands, events, reasons, Sight                      │
│          (rust_decimal)                                        │
└────────────────────────────────────────────────────────────────┘
```

Each layer uses only the layers below it: `memory` implements the ports `run` declares, `csv_io`
implements the source over `Read` and renders onto `Write`, and only `main.rs` touches the world.

| Module | Holds | Uses |
|---|---|---|
| `domain` | `ClientId`, `TxId`, `Amount`, `Balance`, `Account`, `Deposit`, `Status`, `ClientCommand`, `DisputeCommand`, `Command`, `Event`, `Reason`, `Sight` | `rust_decimal` |
| `core` | `decide_client`, `decide_dispute`, `evolve` | `domain` |
| `run` | `run`, `RunError`, `Summary`, `Accounts`, `Report`; the ports: the source (an iterator of `Result<Row, SourceError>`, with `Row` and `RowError`), `LedgerStore`, `SeenIds` | `domain`, `core` |
| `memory` | `MemoryLedger`, `MemorySeenIds` | `domain`, `run` |
| `csv_io` | `CsvRow`, `CsvSource`, `render_csv` | `csv`, `serde`, `domain`, `run` |
| `main.rs` | the argument, the input file, stdio, `ExitCode` | the library |

- No module but `main.rs` touches `std::env`, `std::fs`, stdout or stderr.
- Output is deterministic: commands apply in arrival order, `Accounts` iterates by client id, and
  nothing reads the clock or hash order.
- Verification follows *Correctness defined by laws and a model*: the laws in `docs/overview.md`,
  a naive model, and fixtures through the binary in `backend/tests/it`.
