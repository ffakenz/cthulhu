# Prompt History 0007 — Workspace packages and admitted crates

Date: 2026-10-12

The session followed the command line and its sample input, both on `main` at `a31b805`. It set
out to weigh nine candidate crates, a folder for the CSV and text adapters, an engine, store and
app split like the one in another of the person's repositories, and a config module, and then to
build what the person confirmed.

**Outcome:** `feat/workspace-packages` holds thirteen commits on `a31b805`, unpushed:
- the plan of record, [tasks/todo.md](../../tasks/todo.md), with the sample input's closed plan
  kept as [tasks/sample-input-plan.md](../../tasks/sample-input-plan.md);
- three records: *Layers as packages that depend inward*; *csv, serde, rust_decimal, thiserror,
  clap, proptest and rand as dependencies*, which supersedes the proptest one; and *The
  command-line contract with help and version*, which supersedes *The command-line contract*;
- the library split into `engine`, `store` and `backend`;
- the error types' messages derived with `thiserror`, byte for byte;
- the command line on `clap`, red then green: `--help`, `--version`, and usage errors that say
  what was wrong;
- a fix for a race in the write-failure tests;
- `rand` as a dev-dependency, and the plan's Review;
- a fourth record, *Libraries and an app as packages that depend inward*, which supersedes
  *Layers as packages that depend inward*, and the rename of the package `backend` to `app`;
- this entry.

`make ci` is green, with 45 tests passed and 1 ignored on macOS. Landing the branch is the
person's.

---

### 1. Weigh crates, a layout and a config module

> this task is a follow-up on stage-1 cli.
> 1. i want to consider adding the following new deps: [...] eyre + anyhow [...] rand [...] clap
> [...] tracing + tracing-subscriber
> 2. not sure about: [...] tokio [...] async-trait [...] chrono
> 3. at the same time i would like to have: [...] csv_io and text_io into their own subfolder
> 4. look at this project [...] how app, store and engine are contain into their own namespace
> (folder)
> 5. last but not least, not sure we should have a config module where we expose settings
> eval each bullet above and explain what simplification/enhancement they can bring into the
> table so we confirm what to include and why

In plan mode, three read-only explorations mapped the code and the gate, the records and the
roadmap, and the other repository's layout with each crate's cost in the build graph. The
evaluation recommended three packages whose dependencies point inward, no new crate in this
change, `rand` with the benchmarks, `tokio` and `tracing` at the service stage, rejecting `eyre`,
`anyhow`, `async-trait` and `chrono`, and a config module only once it has a first setting.

### 2. Confirm the scope

> engine/store/app crates (Recommended)
> None: follow the verdicts (Recommended), thiserror, clap, "include ran now, benchmark generator
> will use it later"
> With its first setting (Recommended)
> Leave it untouched (Recommended)

Chosen among the options offered, the last for the accepted domain-model record whose paths the
split moves. `thiserror`, `clap` and `rand` joined the plan, two planning agents drafted the split,
the records and the clap design, and the person approved the plan.

### 3. The checkout

> Proceed in this checkout

Another session was busy in the same checkout; the person said it was read-only, so the branch was
cut there.

### 4. The records

> Accept all three (Recommended)

The three records, reviewed before the question, were committed as accepted, each alone with its
index line and the Status line of the record it supersedes.

### 5. The manual runs

> Yes, all three (Recommended)

The person's consent to run the binary through `cargo run`, given before the rest of the work:
- the split, every law and fixture passing unchanged, and an import of `store` into `engine`
  shown to fail the build;
- `thiserror`, twenty messages printed equal before and after;
- the red and the green, with clap's own output as the fixtures;
- two fixes the test runs forced:
  - the renamed binary went from a copy to a hard link, which got concurrent runs killed on macOS,
    to `argv[0]`, folded into the red;
  - the write-failure tests' closed pipe let a write through about once in sixty runs, because
    another process could hold its reading end; a socket shut for writing fixed it.

`cargo run -- transactions.csv`, `--help` and `--version` printed what the fixtures pin, and every
commit passed `make ci` and a code review whose findings were acted on.

### 6. The app's name

> ok i feel like backend/ does not reensables what it contains.
> i think backend/ should be named app/ and libs/ should be named backend.
> wdyt?

The answer agreed that `backend/`, which holds the command line, should be `app/`, and argued for
keeping `libs/`: `backend` for the libraries fits the adapters as well as the stores, and earlier
records give that name to the binary's package. The rename reverses a rejection in *Layers as
packages that depend inward*.

### 7. The names and the history

> app/ + keep libs/ (Recommended)
> Supersede the record (Recommended)

Chosen among the options offered. This entry's unpushed commit was taken back, to be committed
last again; a new record was drafted and reviewed, and the rename was made with `make ci` green.

### 8. The new record

> Accept (Recommended)
> Yes (Recommended)

The person signed off the record and agreed to one run. The record was committed alone, marking
the one it supersedes, then the rename as one refactor commit, the docs, skills and links
following it; `cargo run -- transactions.csv` printed the sample's table from the renamed package.
