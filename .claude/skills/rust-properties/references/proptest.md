# Properties on proptest

How every property in this repo runs: `support::for_all` over proptest's `TestRunner`, the
strategies in `backend/tests/it/support/generate.rs`, and the regression files proptest saves.
[SKILL.md](../SKILL.md) says which laws to assert and where to aim the strategies; this file is
the code, and how to drive it.

## The runner

`backend/tests/it/support.rs` holds the one runner every law calls:

```rust,ignore
/// How many inputs each law is checked on, unless `PROPTEST_CASES` says otherwise.
const CASES: u32 = 500;

/// Checks `holds` on `CASES` inputs drawn by `strategy`. The first input on which it returns
/// `false` or panics is shrunk until no simpler input still fails, and that is the
/// counterexample; proptest saves its seed under `proptest-regressions/`, in a file named for the
/// caller's, and replays it first on the next run.
#[track_caller]
pub(crate) fn for_all<S: Strategy>(
    law: &'static str,
    strategy: S,
    holds: impl Fn(&S::Value) -> bool,
) -> Result<(), Failure> {
    let config =
        Config { cases: CASES, source_file: Some(Location::caller().file()), ..Config::default() };
    let mut runner = TestRunner::new(contextualize_config(config));
    let outcome = runner.run(&strategy, |input| {
        if holds(&input) { Ok(()) } else { Err(TestCaseError::fail("the law returned false")) }
    });
    outcome.map_err(|error| match error {
        TestError::Fail(reason, input) => Failure::Counterexample {
            law,
            input: format!("{input:?}"),
            reason: reason.message().to_owned(),
        },
        TestError::Abort(reason) => Failure::Aborted { law, reason: reason.message().to_owned() },
    })
}
```

- `contextualize_config` applies the `PROPTEST_*` variables, so `PROPTEST_CASES` overrides
  `CASES` for one run.
- `#[track_caller]` hands proptest the calling test's file. Without a source file proptest saves
  no failure; with it, a failure lands in `backend/tests/proptest-regressions/<module>.txt`.
- A panic in `holds` is a failure like `false`, its message the reason, and it shrinks the same
  way. A panic inside a strategy, in an interpretation or in production code it calls, is not
  caught: the test fails with that panic, with no law, no counterexample and no saved seed. Keep
  interpretations total.
- `for_all` returns a `Result`, and the `#[test]` function is the one that panics. Clippy's test
  allowances cover `#[test]` functions, closures inside them included, and code under a
  `#[cfg(test)]` attribute. A helper in `support.rs` is neither, so `unwrap`, `expect`, `panic!`
  and indexing are all flagged there. A `#[cfg(test)]` written on `mod support;` would silence all
  four for the whole module, a suppression with no reason, which review rejects.

## Writing a property

Both round trips of the `Quantity` parser in
[rust.md § 1](../../functional-architecture/references/rust.md), as a `quantity` module of
`backend/tests/it/main.rs`:

```rust,ignore
mod quantity {
    use engine::{Quantity, QuantityError};
    use proptest::arbitrary::any;
    use proptest::collection::vec;
    use proptest::prop_oneof;
    use proptest::sample::select;
    use proptest::strategy::Strategy;

    use crate::support;

    /// Digits, and the characters the std integer parser treats specially.
    const SPELLING: [char; 8] = ['0', '1', '7', '9', '+', '-', ' ', '_'];

    /// Raw text: one draw in four an edge, otherwise up to four characters of `SPELLING`.
    fn spelling() -> impl Strategy<Value = String> {
        let drawn =
            vec(select(&SPELLING), 0..=4).prop_map(|characters| characters.into_iter().collect());
        prop_oneof![
            1 => select(&["", "0", "4294967295", "4294967296"]).prop_map(str::to_owned),
            3 => drawn,
        ]
    }

    /// A `u32`: one draw in four an edge, otherwise any.
    fn number() -> impl Strategy<Value = u32> {
        prop_oneof![1 => select(&[0, 1, u32::MAX]), 3 => any::<u32>()]
    }

    #[test]
    fn prop_value_round_trips_through_text() {
        let law = support::for_all(
            "a quantity's text parses back to it; only zero is refused",
            number(),
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
            spelling(),
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
  `prop_oneof!` sends one draw in four to an edge: empty, zero, `u32::MAX`, and one past it.
- The value round trip states its boundary too: zero is the one `u32` it may refuse.
- Each law panics in its `#[test]` function, where `clippy.toml` allows it.

Swap in a parser that delegates to `str::parse::<u32>` and the value round trip still passes on
every run, while the spelling law fails, shrunk to two characters:

```text
law `a spelling that parses re-renders to itself` failed with "+1": the law returned false
```

Other runs end at `"01"`: proptest stops at the first input none of whose simplifications still
fails.

## Draws that depend on earlier draws

A dispute names a deposit that came before it, so its tx id cannot be drawn on its own. The
strategy draws a choice, and the fold that builds the rows interprets it:

```rust,ignore
/// One draw: a deposit, or a dispute of the earlier deposit its `Index` picks.
#[derive(Debug, Clone, Copy)]
enum Draw {
    Deposit,
    Dispute(Index),
}

/// A row: a deposit or a dispute, by tx id.
#[derive(Debug, Clone, Copy)]
enum Row {
    Deposit(u32),
    Dispute(u32),
}

/// The item of `items` that `index` picks, `None` when there are none.
fn pick<T: Copy>(index: Index, items: &[T]) -> Option<T> {
    if items.is_empty() {
        return None;
    }
    items.get(index.index(items.len())).copied()
}

/// Up to twelve rows, each dispute naming an earlier deposit.
fn rows() -> impl Strategy<Value = Vec<Row>> {
    let any_draw = prop_oneof![
        3 => Just(Draw::Deposit),
        1 => any::<Index>().prop_map(Draw::Dispute),
    ];
    vec(any_draw, 0..=12).prop_map(|draws| {
        let mut deposits: Vec<u32> = Vec::new();
        let mut rows = Vec::new();
        for draw in draws {
            let disputed = match draw {
                Draw::Dispute(target) => pick(target, &deposits),
                Draw::Deposit => None,
            };
            if let Some(tx) = disputed {
                rows.push(Row::Dispute(tx));
            } else {
                let tx = u32::try_from(rows.len()).unwrap_or(u32::MAX);
                deposits.push(tx);
                rows.push(Row::Deposit(tx));
            }
        }
        rows
    })
}
```

- The strategy draws choices, never tx ids: a dispute's `Index` picks among the deposits built so
  far, whatever they turned out to be.
- Every list proptest shrinks to still builds rows: fewer draws, each `Index` toward the first
  deposit, each `prop_oneof!` toward its first arm. A dispute drawn before any deposit builds a
  deposit instead, so no draw is invalid.
- `Index::index` panics on an empty collection; `pick` checks first.
- `generate::rows` builds a run the same way: an ordinary draw or one of seven edge draws,
  interpreted by its `Draft` while the run holds fewer rows than draws.

## Driving a state machine

The machine is an order's write to the journal, and `Status::after(self, Step) -> Status` is its
transition. The caller's `Submit` takes a `Draft` to `Awaiting`; the journal's `Written` takes that
to `Recorded`, and its `Refused` back to `Draft`; the caller's `Cancel` takes a `Draft` to
`Cancelled`. `Recorded` and `Cancelled` are terminal, and every other pair leaves the status as it
is. The driver is another module of `backend/tests/it/main.rs`:

```rust,ignore
mod status_machine {
    use engine::{Status, Step};
    use proptest::collection::vec;
    use proptest::sample::select;

    use crate::support;

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
            vec(select(&STEPS), 0..=12),
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
- A failing sequence shrinks to the steps that reach the broken rule. Let `Submit` reopen a
  `Recorded` order and every run ends at the same three steps:

  ```text
  law `an answer nobody awaits changes nothing; a terminal status absorbs every step` failed with [Submit, Written, Submit]: the law returned false
  ```

- The driver lives in its own module, named for the machine.

## Replaying a failure

A failing law prints proptest's lines before its own:

```text
proptest: Saving this and future failures in …/backend/tests/proptest-regressions/domain.txt
proptest: If this test was run on a CI system, you may wish to add the following line to your copy of the file. (You may need to create it.)
cc 169571591a81d22cebc37381e52528a500f026f3953a773781033517f0a1f706
```

- The `cc` line is the failing case's seed. proptest saves it to the module's regression file,
  and every later run of that module's laws replays the saved seeds before drawing new ones:
  `cargo test --test it <name>` fails on it again, at once, until the code is fixed.
- A failure on CI saves nothing you keep: copy the printed `cc` line into the module's file, and
  the local run replays it.
- Fix the code, rerun until the replay passes, then commit the regression file with the fix, so
  the failure stays a test. Commit one for no other reason. Never pin `PROPTEST_RNG_SEED` in a
  test, a `Makefile` line or a CI setting: a pinned seed turns the law back into one example.
- A seed replays the same input only while the strategy draws the same way. If the failing input
  is one the strategy reaches rarely, add it to the strategy's edges, so runs visit it often.
- A deliberate break runs with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`, which saves nothing and
  loads nothing.
