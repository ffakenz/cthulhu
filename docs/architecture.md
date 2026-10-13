# Architecture

A picture of the decisions in `docs/adr/`, each named by its title.

## The pipeline

*Sources, sinks and projections*, *State partitioned by client*.

```text
 source ─► intake ─► partition ─► projections ─► sink
   │         │          │              │            │
 rows in   arrival   one writer   Accounts,      CSV table
 order     order,    per client:  Report         on stdout,
           first     decide,                     report on
           sight     evolve                      stderr
```

- A malformed row is counted and skipped; a rejection is counted; an unreadable source stops the
  run before anything reaches the sink (*Three failure tiers*).
- Rows stream one at a time; only the state, the counts and bounded queues stay in memory
  (*Memory bounded by state, not input*).

### In detail

*Effects behind ports*, *A pure core*, *State changes only through events*, *Stores defined by
their laws*, *Entities identified by their keys*.

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
 main.rs ─┬─► render_csv(Accounts) ──┬─► [Write] (stdout)
          │                          └─► SinkError ──► ExitCode
          ├─► render_report(Report) ──┬─► [Write] (stderr)
          │                           └─► io::Error ──► ExitCode
          └─► RunError ──► ExitCode
```

- `[port]` marks a port; `(…)` is the implementation `main.rs` supplies: an adapter, or stdout or
  stderr for `Write`.
- The footprint is the client's account and at most one of its deposits, loaded and committed at
  the command's address: its client and tx id. Dispute commands skip `SeenIds`.
- `decide` only checks conditions; `evolve` only computes, and fails only on overflow, which
  rejects the command. A rejection changes no balance; its commit stores only an account the
  step opened.
- `Accounts` and `Report` are the projections; `render_csv` and `render_report`, the sink
  adapters, write the table and the report to `Write`.

## Wiring

*Effects behind ports*, *Runtimes proven against a sequential reference*.

```text
 main.rs
   ├─ builds    CsvSource, MemoryLedger, MemorySeenIds
   ├─ calls     run(source, stores) ─► Summary { accounts, report } | RunError
   └─ renders   render_csv(accounts) ─► stdout | SinkError;
                render_report(report) ─► stderr | io::Error
```

Besides adapters, only `run`, the runtime, varies between deployments; today it is sequential,
one partition, and the reference that every later runtime must equal.

## Modules

*A pure core*, *Effects behind ports*, *Libraries and an app as packages that depend inward*.

```text
┌────────────────────────────────────────────────────────────────┐
│ app                                                            │
│   main.rs  composition root: arguments, file, stdio, exit code │
│   csv_io   CsvSource, render_csv, SinkError                    │
│            (csv, serde)                                        │
│   text_io  render_report                                       │
├────────────────────────────────────────────────────────────────┤
│ store                                                          │
│   memory   MemoryLedger, MemorySeenIds                         │
├────────────────────────────────────────────────────────────────┤
│ engine                                                         │
│   run      the runtime; ports: source, LedgerStore, SeenIds;   │
│            Row, RowError, SourceError, RunError, Summary,      │
│            Accounts, Report                                    │
│   core     open, decide_client, decide_dispute, evolve: pure,  │
│            total; Overflow, evolve's one failure               │
│   domain   ids, Address, Amount, Balance, Account, Lock,       │
│            Deposit, Status, Footprint, commands, events,       │
│            reasons, Sight                                      │
│            (rust_decimal)                                      │
└────────────────────────────────────────────────────────────────┘
```

Each layer uses only the layers below it: `memory` implements the ports `run` declares, `csv_io`
implements the source over `Read` and renders the table onto `Write`, `text_io` renders the report
onto `Write`, and only `main.rs` touches the world. Each band is a package, and the compiler holds
the direction between them: `app` depends on `store` and `engine`, `store` on `engine`, and
`engine` on no other package of the workspace, so a module of `engine` that imports an adapter
does not build.

| Module | Package | Holds | Uses |
|---|---|---|---|
| `domain` | `engine` | `ClientId`, `TxId`, `Address`, `Amount`, `AmountError`, `Balance`, `Account`, `Lock`, `Deposit`, `Status`, `Footprint`, `ClientCommand`, `DisputeCommand`, `Command`, `Event`, `ClientEvent`, `DisputeEvent`, `Reason`, `ClientReason`, `DisputeReason`, `Sight` | `rust_decimal`, `thiserror` |
| `core` | `engine` | `open`, `decide_client`, `decide_dispute`, `evolve`, `Overflow` | `thiserror`, `domain` |
| `run` | `engine` | `run`, `RunError`, `Summary`, `Accounts`, `Report`; the ports: the source (an iterator of `Result<Row, SourceError>`, with `Row` and `RowError`), `LedgerStore`, `SeenIds` | `thiserror`, `domain`, `core` |
| `memory` | `store` | `MemoryLedger`, `MemorySeenIds` | `domain`, `run` |
| `csv_io` | `app` | `CsvRow`, `CsvSource`, `render_csv`, `SinkError` | `csv`, `serde`, `thiserror`, `domain`, `run` |
| `text_io` | `app` | `render_report` | `domain`, `run` |
| `main.rs` | `app` | the arguments, the input file, stdio, `ExitCode` | `clap`, `thiserror`, `csv_io`, `text_io`, `memory`, `run` |

- `domain` is the model in the records' terms: values, entities and messages, each entity reached
  by its key (*A domain model in the records' language*, *Entities identified by their keys*).
- No module but `main.rs` touches `std::env`, `std::fs`, stdout or stderr.
- Output is deterministic: commands apply in arrival order, `Accounts` iterates by client id, and
  nothing reads the clock or hash order.
- Verification follows *Correctness defined by equations between runs*: the laws in
  `docs/overview.md`, checked by comparing runs of the engine with each other, and fixtures through
  the binary in `app/tests/it`. `docs/testing.md` pictures the levels of tests.
