# Testing

How the engine is checked: the levels of tests, what each proves and through which interface, how
the levels relate, and where a new check goes. It pictures *Correctness defined by equations
between runs* and *Runtimes proven against a sequential reference*. `docs/overview.md` lists the
laws.

## Levels

| Level | Proves | Drives | Lives in |
|---|---|---|---|
| The binary | The contract: the arguments, which stream carries what, every exit status, the formats, and the numbers and words of inputs computed by hand, the overview's worked example among them | the process: its arguments, its input file, stdout, stderr and the exit status | `app/tests/it/cli.rs` |
| The engine | The rules, after any input. Inputs the rules make equivalent leave the same accounts, and clients run independently. Any spelling reads as the canonical one, and malformed rows change nothing but their counts. Under a header that misfits them, rows are each malformed: a column named twice, one they stop before, or no amount over deposits and withdrawals. A header lacking `type`, `client` or `tx`, or a failing source, stops the run | `run` over `CsvSource` and the in-memory stores, fed generated CSV text | `app/tests/it/run.rs` |
| The domain | The fine print no run reaches: every spelling of an amount, the reason for each refusal, and a balance's arithmetic and printing, from zero to its bound | `Amount` and `Balance` | `app/tests/it/domain.rs` |
| The gate | The code's shape: admitted crates only, formatting, and lints that ban panics, `as` casts and hidden effects such as the clock or the environment. It holds what no run reaches | the sources and the build graph | `make ci`, `Cargo.toml`, `clippy.toml` |

Most checks are laws on the engine. A law over generated input replaces every example it covers.
Its domains are small, three clients and eight tx ids, so repeats, disputes and locks come up
without the generator tracking them. Each law checks 500 generated inputs, more with
`PROPTEST_CASES`, and the whole suite runs in `make test` in a few seconds once built.

## How the levels relate

```text
 the binary ──── overlaps ───► the engine ──── overlaps ───► the domain
 fixtures                      equations                     round trips, refusals,
 computed by hand              between runs                  arithmetic at the bound
```

- **Overlaps.** Two levels run the same code without fakes, and the lower one covers cases the
  upper one cannot reach. Each overlap earns its place by what only it sees:
  - The binary over the engine: an equation passes a mistake that both of its sides share, such
    as every amount read ten times too large, or a reason worded wrong. A fixture's numbers and
    words catch it.
  - The engine over the domain: every run parses and adds amounts, but some things no run can
    reach:
    - text that CSV trims or splits, such as a padded amount or one holding a comma;
    - a balance near its bound of about 7.9 × 10^24.

    And an equation compares balances as numbers, so it cannot see how a balance prints.
- **Relies on.** A level that fakes a part relies on a check of the real part.
  - The engine's source-failure example fakes a reader whose every read fails; the binary's
    unreadable input is the real read failure behind it.
  - The in-memory stores are production adapters, not fakes, so nothing else relies on a fake
    today. A second store, or a store made to fail on purpose, comes with a check of the real part
    it stands for.

## Where a new check goes

A check sits at the lowest level that can see what it pins. A higher level adds only what the lower
ones cannot see.

- **A rule about commands and accounts:** an equation in the engine's laws. Two suffixes that the
  rule makes equivalent must leave the same accounts after any prefix. The test never computes a
  balance.
- **A spelling the input admits, or a flaw that makes a row malformed:** a generator in
  `app/tests/it/support/input.rs`, one per field, composed into lines. A law checks success or
  failure, never both.
- **The fine print of a value:** a law on its domain type, with the strategy weighted toward the
  edges.
- **What no equation can see** (a number, a word, a stream or an exit status): a fixture through
  the binary, computed by hand.
- **A new runtime or store:** the same laws, run over it. A new runtime's output must also equal
  the sequential reference's.

## Order of work

- **A behaviour:** a failing test first, then the code that passes it.
- **A stage on the roadmap starts from the outside.** Its outermost checks are written first:
  output equal to the sequential reference's, which every stage owes, and the stage's own gate on
  the roadmap. The stage lands when both pass and every law still holds.
- **A law that has never failed is suspect.** Each law is shown to fail once against a
  deliberately broken implementation, in a scratch copy, never in the repository.

## Ahead

| Level | Proves | From |
|---|---|---|
| Runtime equivalence | a runtime's output equals the sequential reference's on every generated input, for every configuration it accepts | the partitioned workers |
| Benchmarks | the binary's rows per second and peak memory on 1M to 100M generated rows, against each stage's gate | the benchmarks, next |
| Store laws | the engine's laws hold over every store, generic over the store ports | a second store |
| Crash points | recovery reproduces the state exactly at every crash point the tests inject | durability |

## Left out

- **Interfaces that exist only for tests:** nothing in production exists only for a test.
- **An independently written model or simulator as the oracle:** a second engine shares the
  first's misreadings, so the laws compare the engine with itself.
- **Tests across processes or machines:** nothing runs across them before the service and
  durability stages, which bring their own checks.
