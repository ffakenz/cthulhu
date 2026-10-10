# CLAUDE.md

Conventions for working in this repository — the single source of truth for how work is done
here. Decisions live in `docs/adr/` (architecture) and `docs/design/` (how the engine treats its
input).

## Project

Cthulhu keeps per-client ledgers with one pure decider and interchangeable runtimes around it;
the first runtime is a command-line program that reads CSV.

## Status

The library is being built: the domain first, then the core, the runtime and the in-memory stores;
the command-line program follows. The workspace, its gate (`make ci`), these conventions, the
decision log, the knowledge base and the engine's design (`docs/`) are in place.

## Repository layout

```text
cthulhu/
├── Cargo.toml            virtual workspace: members, shared package fields, lints
├── Cargo.lock            locked versions; `make deps` gates the build graph, not this file
├── rust-toolchain.toml   pinned toolchain: 1.88.0 with clippy and rustfmt
├── rustfmt.toml          formatting
├── clippy.toml           lint settings and disallowed methods
├── Makefile              targets; `make ci` is the commit gate
├── README.md             setup and usage
├── CLAUDE.md             this file
├── backend/              package `backend`, binary `cthulhu`
│   ├── src/
│   │   ├── lib.rs        the library: core, runtime and adapters
│   │   ├── core.rs       the decider: `open`, `decide_client`, `decide_dispute`, `evolve`
│   │   ├── domain.rs     the domain model: re-exports every type of `domain/`
│   │   ├── domain/       ids, money, accounts and deposits, commands and events
│   │   └── main.rs       the composition root
│   └── tests/it/         the black-box test crate: laws, generators and the model
├── docs/
│   ├── overview.md       what the engine does, and the decisions behind it
│   ├── architecture.md   a picture of the architecture decisions
│   ├── patterns.md       the patterns the architecture is built from
│   ├── roadmap.md        the benchmark-gated stages ahead
│   ├── adr/              architecture decision records, their index and template
│   ├── design/           design records: how the engine treats its input
│   └── prompts/          one prompt history per working session
├── tasks/                plans, tickets and lessons
├── .github/workflows/    CI: `make ci` on every push and pull request to `main`
└── .claude/              settings.json, agents/code-reviewer.md, skills/ (see Skills and agents)
```

The library holds the functional core (logic modules that reach effects only through ports) and the
adapter modules that implement those ports. `main.rs` is the composition root: the only production
code that constructs an adapter or touches the clock, the environment, the arguments or stdio;
everything it builds is passed down as a value.

## Engineering principles

| Principle | The rule | See |
|---|---|---|
| Composition | Small functions and narrow traits, composed; a cross-cutting concern wraps a port instead of branching inside it | `functional-architecture`, `rust-wiring` |
| Immutability and referential transparency | Same input, same output, no observable side effect; `let` by default, `&` before `&mut` before owned | `functional-architecture`, `rust-effects` |
| Types are propositions — parse, don't validate | Parse input once, at the boundary, into a type that cannot hold a bad value; its existence is the proof; each type states the domain's proposition, no weaker | `functional-architecture`, `curry-howard` |
| The records' language | Every domain name comes from the records' terms; each domain type is a value, an entity or a message, and an entity's identity is its key, never a field | `domain-modeling` |
| Total functions only | A value for every input the type admits; what cannot be honoured is a variant in the return type, never a panic | `functional-architecture` |
| Errors are values | A closed enum per boundary, inside a `Result`, classified once where the failure enters; a panic is a bug | `rust-errors` |
| Absence has a type | `Option`, never a sentinel or a `Default` meaning "not set" | `functional-architecture` |
| TDD | Red, green, refactor, as Implementation states | `review-and-planning`, `rust-properties` |
| Self-documenting code | Names and types carry the meaning; comments obey Comments below | Comments |
| No dead code | Every line is reachable and used; a `dead_code` warning fails `make lint` | `make lint` |
| No test-only code in production | Tests drive the public API and the ports; nothing ships only so a test can reach it | Tests |
| Prefer the machine-checkable form | `Cargo.toml` lints, `clippy.toml`, `rustfmt.toml`, `make deps` and `.claude/settings.json` are the authoritative statements; prose summarizes them | `functional-architecture` |

## Dependencies

Standard library first. The admitted crates are `csv`, `serde` and `rust_decimal`, with the 13
crates they build on, admitted by a decision record. Never add any other dependency — no
`cargo add`, no new line in a manifest's dependency tables — without explicit approval from a
person: propose it with the rationale and the alternatives considered, wait for sign-off, then
record it as a decision record. `make deps` fails the gate when the build graph (`cargo tree`,
every dependency kind, feature and target) holds a package that is neither a workspace member
nor an admitted crate. It reads the graph, not `Cargo.lock`, which also lists optional crates
that are never built. `cargo add` and `cargo install` ask for consent.

## Build, test, run

```sh
make        # list the targets
make ci     # the commit gate: deps, fmt-check, lint, build, test
make run    # run the cthulhu binary
```

## Git

- Always create a feature branch before any changes, and **leave the work on it**. Branch
  naming: `feat/description` or `fix/description`.
- **Never merge, push, or change a remote on your own initiative.** Not because those are
  forbidden — because they are not yours to decide. Do them the moment a person asks, and never
  because the work looked finished. Branching and then fast-forwarding the default branch onto
  it an hour later is the same decision as committing to it directly, taken more slowly.
- Approving a plan, a description or a diff approves the **work**, not the landing. Say what is
  on the branch and stop; landing it is a separate ask.
- `.claude/settings.json` encodes this rather than restating it: pushing, merging, rebasing,
  amending, deleting a branch, changing a remote, `cargo add` and `cargo install` sit in `ask`,
  and other branch rewrites are left out of `allow`, so they surface for consent and never happen
  silently. Force-push in its usual spellings and `reset --hard` sit in `deny`, which is for what
  no consent makes recoverable.
- Commit format, no exceptions:
  ```text
  action(scope): description

  > value describing the business or technical impact
  ```
  Each commit is a self-contained unit of value. No trailer lines.
- Never push unless asked. All work stays local by default.
- Never close a PR. Keep the branch open and repurpose it rather than close and reopen.

## PR descriptions

- The body is exactly two sections, in this order:
  ```text
  ## Motivation
  <one short sentence: the goal>

  ### Summary
  | Decision | Reasoning |
  ```
- Motivation is ONE sentence. Never a paragraph, never an evidence dump.
- The Summary table holds decisions and their reasoning, each cell a `•` bullet list
  (`<br>`-separated). It is a high-level overview of reasoning — never an inventory of changes
  or files, which the diff already shows.
- Deployment or migration work adds a runbook below the Summary: pre-steps, release, deploy,
  post-steps and proof, rollback, incident.

## PR reviews

- Resolve review comments one at a time, not all at once.
- Each resolution is its own self-contained commit.
- Wait for approval before moving to the next comment, and replan after each one.

See `.claude/skills/review-and-planning` for the ordering rule and the legitimate carve-outs.

## Implementation

- TDD for all feature and bugfix work.
  - **Red**: write the failing test first, run it, observe the failure that demonstrates the
    bug or the missing feature, commit.
  - **Green**: the minimum implementation that makes it pass. Run, confirm, commit.
- Refactors — renames, extractions, formatting — are separate commits from red/green.
- Rust style is rustfmt plus the workspace lints. `make ci` passes before every commit, except
  that a red commit fails on its new test and on nothing else: its stub returns a wrong but
  lint-clean value, since `todo!()` and `unimplemented!()` fail `make lint`.

## Tests

- Test behaviour and properties. Never test data structures, derives, factories or
  construction.
- Do not add a test for something another test already covers.
- Fixtures and stubs live beside the test, not in its body; the test body stays the behaviours
  under test.
- Nothing in production code may exist only for testing.
- Black-box tests live in `backend/tests/it` and see only the public API: one test crate,
  `main.rs` plus modules that mirror `src`, and `support.rs` for helpers once one is needed.
- A property test is named `prop_<law>` (`rust-properties`).

## Comments

- Comments say what the code or the setting IS and how it behaves. Simple, short, true.
- Never leak decisions, alternatives, trade-offs or plan context into a comment. Those live in
  the decision records, commit messages and PR descriptions.
- A comment that records **a failure someone paid for** is the exception, and is the most
  valuable comment in the file. Keep those.
- Never reference a plan document, an ADR filename, a task number or "decision #N" in code or
  in a tracked document. Link a type or a function instead.
- The decision logs (`docs/adr/`, `docs/design/`) are the one place a record's number appears:
  their indexes, and a superseded record's `Superseded by` line.
- Doc comments (`///`, `//!`) follow the same rules.
- A lint is silenced only by `#[expect(lint, reason = "…")]`: never by a comment, never by
  `#[allow]`. The reason follows the same rules.

## Decisions

Architecture decisions are recorded in `docs/adr/` with the `architecture-decisions` skill: one
numbered record per decision, copied from `docs/adr/template.md` and indexed in
`docs/adr/README.md`. A reversed decision is superseded, never edited. A record names the code;
code never cites a record. Read the records that touch a change before making it.

Design records, how the engine treats its input, live in `docs/design/` under the same
conventions, indexed in `docs/design/README.md`. They comply with the architecture decisions,
never the reverse.

`docs/roadmap.md` is a standing document, not a plan of record: other documents may name it.

## Knowledge base

`tasks/` and `docs/prompts/` are tracked: they are the evidence of how the project was built.
Files inside them may name a specific plan, ticket or session; code and every other document
never do (see Comments). Like every tracked file, they never restate, quote or paraphrase an
external brief or specification: refer to it by a neutral name only.

- `tasks/todo.md` — the plan of record for the current piece of work: checkable items, and a
  Review section when it closes.
- `tasks/<ticket>.md` — one ticket, copied from `tasks/TEMPLATE.md`.
- `tasks/lessons.md` — append an entry after any correction.
- `docs/prompts/NNNN-kebab-case-title.md` — one prompt history per working session, written
  with the `prompt-history` skill.

## Skills and agents

Skills live in `.claude/skills/<name>/SKILL.md`; the agent in `.claude/agents/code-reviewer.md`.

| Name | When to use |
|---|---|
| `functional-architecture` | Writing or reviewing any code: the shape it must take. `references/rust.md` is its Rust spelling |
| `domain-modeling` | Naming or shaping a domain type, adding a term, choosing an id or a key, mapping a file format onto the domain |
| `curry-howard` | Designing, reading or reviewing a type or a signature as the claim it makes; strengthening a weak one; choosing a DSL |
| `rust-effects` | Code that reads the clock, the environment or the arguments, or touches files, the network, processes, threads, stdio or hash order |
| `rust-errors` | Defining an error enum, classifying a foreign failure, deciding what retries, mapping a failure to an exit code |
| `rust-resources` | Acquiring anything that must be released: files, sockets, child processes, threads, buffered writers |
| `rust-wiring` | Changing `main.rs`: building adapters, parsing configuration, composing decorators, adding a module |
| `rust-properties` | Writing a property test or a generator |
| `architecture-decisions` | Making, reversing or looking up an architecture decision; starting a record |
| `review-and-planning` | Resolving review comments, reviewing a diff, planning a multi-step change, proposing a dependency, writing a test that must fail first |
| `prompt-history` | Closing a working session: recording its prompts in `docs/prompts/` |
| `code-reviewer` (agent) | After implementing a change, before committing |

## Working style

- Incremental and small: focused changes, clarity over cleverness.
- Decide → record the decision → implement.
- Keep this file, the decision records and `README.md` in sync.
