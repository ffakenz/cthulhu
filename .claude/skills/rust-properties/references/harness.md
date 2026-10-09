# The property harness

The standard-library harness every property in this repo runs on: `Gen`, a seeded `SplitMix64`
generator; `for_all`, which turns a `false` or a panic into a counterexample; and the replay line
that counterexample prints. [SKILL.md](../SKILL.md) says which laws to assert and where to aim the
generator; this file is the code, and how to drive it.

## It lands with the first property

The harness is not in the tree yet. Every helper in it must be used by some test, or `dead_code`
fails `make lint`, so it arrives together with the first property:

1. Copy the block below verbatim into `backend/tests/it/support.rs`, and add `mod support;` to
   `backend/tests/it/main.rs` in the same change: a file no `mod` line reaches is never compiled.
2. Run `make lint`. `dead_code` names each helper the new property does not use (for example
   ``method `up_to` is never used``). Delete those, and copy each back from here when a later
   property needs it.
3. Leave the lint allowances alone. Clippy's test allowances cover `#[test]` functions, closures
   inside them included, and code under a `#[cfg(test)]` attribute. A helper in `support.rs` is
   neither, so `unwrap`, `expect`, `panic!` and indexing are all flagged there: the harness
   returns a `Result`, and the `#[test]` function is the one that panics. A `#[cfg(test)]` written
   on `mod support;` would silence all four for the whole module, a suppression with no reason,
   which review rejects.

## The harness

```rust,no_run
//! Property-test support: a seeded generator, and a runner that reports the case breaking a law.

use std::ffi::OsString;
use std::fmt::{self, Debug};
use std::num::NonZeroU64;
use std::panic::{self, AssertUnwindSafe};
use std::time::{SystemTime, UNIX_EPOCH};

/// How often a generator takes an edge case: one draw in four.
pub(crate) const EDGE_ODDS: NonZeroU64 = NonZeroU64::MIN.saturating_add(3);

/// A `SplitMix64` generator: the same seed gives the same draws, in the same order. Only
/// `for_all` builds one, so every draw comes from the seed a failure prints. `Clone` but not
/// `Copy`, so a generator is never duplicated by accident.
#[derive(Debug, Clone)]
pub(crate) struct Gen(u64);

impl Gen {
    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`.
    pub(crate) fn below(&mut self, bound: NonZeroU64) -> u64 {
        self.next_u64() % bound
    }

    /// `true` once in `n` draws, on average.
    pub(crate) fn one_in(&mut self, n: NonZeroU64) -> bool {
        self.below(n) == 0
    }

    /// One of `items`, or `None` when there are none.
    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let len = NonZeroU64::new(u64::try_from(items.len()).ok()?)?;
        items.get(usize::try_from(self.below(len)).ok()?)
    }

    /// Up to `max_len` items picked from `items`; each length in `0..=max_len` is equally likely.
    pub(crate) fn up_to<T: Clone>(&mut self, max_len: u64, items: &[T]) -> Vec<T> {
        let len = self.below(NonZeroU64::MIN.saturating_add(max_len));
        (0..len).filter_map(|_| self.pick(items).cloned()).collect()
    }
}

/// Why a property failed.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The law returned `false`, or panicked, on the input generated for this case.
    Counterexample { law: &'static str, case: u32, input: String, seed: u64 },
    /// `PROPERTY_SEED` is set but is not a hex `u64`, so the run it names cannot be replayed.
    UnreadableSeed(OsString),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Counterexample { law, case, input, seed } => {
                write!(
                    f,
                    "law `{law}` failed at case {case} with {input}; replay: PROPERTY_SEED=0x{seed:016x}"
                )
            }
            Self::UnreadableSeed(raw) => {
                let raw = raw.display();
                write!(
                    f,
                    "PROPERTY_SEED={raw} is not a hex u64; unset it, or copy one a failure printed"
                )
            }
        }
    }
}

/// Checks `holds` on `cases` inputs drawn by `generate`; the first input on which it returns
/// `false` or panics is the counterexample.
pub(crate) fn for_all<T: Debug>(
    law: &'static str,
    cases: u32,
    mut generate: impl FnMut(&mut Gen) -> T,
    holds: impl Fn(&T) -> bool,
) -> Result<(), Failure> {
    let seed = seed()?;
    let mut random = Gen(seed);
    for case in 1..=cases {
        let input = generate(&mut random);
        // `AssertUnwindSafe` holds: after a panic, `input` is only printed, never used again.
        #[expect(
            clippy::disallowed_methods,
            reason = "a panic in the code under test becomes a counterexample with its seed"
        )]
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| holds(&input)));
        if !matches!(outcome, Ok(true)) {
            return Err(Failure::Counterexample { law, case, input: format!("{input:?}"), seed });
        }
    }
    Ok(())
}

/// The run's seed: `PROPERTY_SEED` (hex, with or without `0x`) when set, otherwise fresh from
/// the clock.
#[expect(
    clippy::disallowed_methods,
    reason = "the harness edge draws a fresh seed per run; tests never read the environment or the clock elsewhere"
)]
fn seed() -> Result<u64, Failure> {
    let Some(pinned) = std::env::var_os("PROPERTY_SEED") else {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let nanos = elapsed.as_secs().wrapping_mul(1_000_000_000);
        return Ok(nanos.wrapping_add(u64::from(elapsed.subsec_nanos())));
    };
    let digits = pinned.to_str().map(|text| text.strip_prefix("0x").unwrap_or(text));
    match digits.map(|hex| u64::from_str_radix(hex, 16)) {
        Some(Ok(seed)) => Ok(seed),
        Some(Err(_)) | None => Err(Failure::UnreadableSeed(pinned)),
    }
}
```

- **Same seed, same inputs.** A `Gen`'s draws depend on its seed alone, and only `for_all` builds
  one, so a seed regenerates every input in the same order.
- **The first failing case ends the run.** Returning `false` and panicking are the same verdict.
  After a panic, the panic hook has already printed the message and its location, just above the
  failure line.
- **Two opt-outs, both at this edge.** `seed` reads the environment and the clock, and `for_all`
  calls `catch_unwind`; each sits under `#[expect(clippy::disallowed_methods, reason = "…")]`, and
  no test needs another.
- **A seed that does not parse stops the test.** Running a fresh seed instead would let a replay
  pass for the wrong reason.
- **A panic inside `generate` is not caught.** It fails the test with no replay line, so build
  generators from `pick`, `up_to` and `below`, which cannot panic.
- **No shrinking.** Generators draw small inputs, a few characters or a dozen steps, so the first
  counterexample is already readable.

## Writing a property

The first part of `backend/tests/it/main.rs`: `mod support;`, then a `quantity` module with both
round trips of the `Quantity` parser in [rust.md § 1](../../functional-architecture/references/rust.md).

```rust,ignore
//! Integration tests: one crate, with modules that mirror `src`.

mod support;

mod quantity {
    use backend::{Quantity, QuantityError};

    use crate::support::{self, EDGE_ODDS, Gen};

    /// Digits, and the characters the std integer parser treats specially.
    const SPELLING: [char; 8] = ['0', '1', '7', '9', '+', '-', ' ', '_'];

    /// Raw text: one draw in four an edge, otherwise up to four characters of `SPELLING`.
    fn spelling(random: &mut Gen) -> String {
        if random.one_in(EDGE_ODDS) {
            let edges = ["", "0", "4294967295", "4294967296"];
            return random.pick(&edges).copied().unwrap_or_default().to_owned();
        }
        random.up_to(4, &SPELLING).into_iter().collect()
    }

    /// A `u32`: one draw in four an edge, otherwise the low four bytes of a draw.
    fn number(random: &mut Gen) -> u32 {
        if random.one_in(EDGE_ODDS) {
            return random.pick(&[0, 1, u32::MAX]).copied().unwrap_or_default();
        }
        let [b0, b1, b2, b3, ..] = random.next_u64().to_le_bytes();
        u32::from_le_bytes([b0, b1, b2, b3])
    }

    #[test]
    fn prop_value_round_trips_through_text() {
        let law = support::for_all(
            "a quantity's text parses back to it; only zero is refused",
            500,
            number,
            |&n| match n.to_string().parse::<Quantity>() {
                Ok(quantity) => quantity.to_string().parse() == Ok(quantity),
                Err(error) => n == 0 && error == QuantityError::Zero,
            },
        );
        if let Err(failure) = law {
            panic!("{failure}");
        }
    }

    #[test]
    fn prop_spelling_that_parses_re_renders_to_itself() {
        let law = support::for_all(
            "a spelling that parses re-renders to itself",
            500,
            spelling,
            |text| match text.parse::<Quantity>() {
                Ok(quantity) => quantity.to_string() == *text,
                Err(_) => true,
            },
        );
        if let Err(failure) = law {
            panic!("{failure}");
        }
    }
}
```

- `spelling` produces raw text, and the law parses it with the production `FromStr`. No
  constructor exists only for tests, so every counterexample is a value production can receive.
- `SPELLING` mixes digits with the characters the std integer parser treats specially, and
  `EDGE_ODDS` sends one draw in four to an edge: empty, zero, `u32::MAX`, and one past it.
- The value round trip states its boundary too: zero is the one `u32` it may refuse.
- Each law panics in its `#[test]` function, where `clippy.toml` allows it.

Swap in a parser that delegates to `str::parse::<u32>` and the value round trip still passes on
every run, while the spelling law fails within a few dozen cases:

```text
law `a spelling that parses re-renders to itself` failed at case 13 with "+170"; replay: PROPERTY_SEED=0x18dcfb694eaf77f8
```

## Driving a state machine

The machine is an order's write to the journal, and `Status::after(self, Step) -> Status` is its
transition. The caller's `Submit` takes a `Draft` to `Awaiting`; the journal's `Written` takes that
to `Recorded`, and its `Refused` back to `Draft`; the caller's `Cancel` takes a `Draft` to
`Cancelled`. `Recorded` and `Cancelled` are terminal, and every other pair leaves the status as it
is. The driver is the second module of `backend/tests/it/main.rs`:

```rust,ignore
mod status_machine {
    use backend::{Status, Step};

    use crate::support::{self, Gen};

    /// The caller's commands and the journal's answers, drawn from one alphabet.
    const STEPS: [Step; 4] = [Step::Submit, Step::Cancel, Step::Written, Step::Refused];

    /// Folds `steps` from `Draft`, checking the invariants after every step; `Err` holds the
    /// status and step that first broke one.
    fn drive(steps: &[Step]) -> Result<Status, (Status, Step)> {
        steps.iter().try_fold(Status::Draft, |status, &step| {
            let next = status.after(step);
            let unawaited = match step {
                Step::Written | Step::Refused => status != Status::Awaiting,
                Step::Submit | Step::Cancel => false,
            };
            let terminal = match status {
                Status::Recorded | Status::Cancelled => true,
                Status::Draft | Status::Awaiting => false,
            };
            if (unawaited || terminal) && next != status { Err((status, step)) } else { Ok(next) }
        })
    }

    #[test]
    fn prop_unawaited_answers_and_terminal_statuses_change_nothing() {
        let law = support::for_all(
            "an answer nobody awaits changes nothing; a terminal status absorbs every step",
            500,
            |random: &mut Gen| random.up_to(12, &STEPS),
            |steps| drive(steps).is_ok(),
        );
        if let Err(failure) = law {
            panic!("{failure}");
        }
    }
}
```

- One alphabet holds the commands and the answers, so a generated sequence reaches conversations
  no recorded fixture holds: an answer before any submission, a refusal after a cancel, twelve
  submits in a row.
- `try_fold` checks the invariants after every step and stops at the first one broken; checking
  only the final status would miss a machine that breaks a rule and later recovers.
- The two laws every status machine owes: an answer nobody awaits changes nothing, and a terminal
  status absorbs every step.
- The driver lives in its own module, named for the machine.

## Replaying a failure

```sh
PROPERTY_SEED=0x18dcfb694eaf77f8 cargo test --test it prop_spelling_that_parses_re_renders_to_itself
```

- The seed is hex, with or without `0x`. Every property in the run reads it, so name the one that
  failed.
- The seed fixes every draw, and the code under test never touches the generator, so the failure
  returns at the same case with the same input until the generator itself changes. Fix the code,
  rerun until the replay passes, then run `make test` without the variable.
- Never commit a pinned seed. A test, a `Makefile` line or a CI setting that fixes `PROPERTY_SEED`
  turns the law back into one example.
- If the failing input is one the generator reaches rarely, add it to the generator's edges, so
  every run visits it.
