---
name: rust-properties
description: "Use when writing or reviewing a property test, a strategy, or a test that passes no matter what. Covers laws over examples, round-trips in both directions, totality as a property, laws run on proptest through support::for_all, edge-weighted strategies, labelled laws, raw spelling then parse, choices then interpretation for draws that depend on earlier ones, building instead of filtering, shrunk counterexamples, regression files and replay, and state-machine drivers with generated answers. Keywords: property, law, invariant, round trip, FromStr, Display, proptest, strategy, prop_oneof, Index, shrinking, counterexample, regression file, PROPTEST_CASES, seed, replay, edge case, zero, empty, boundary, commutative, associative, idempotent, state machine, try_fold, prop_."
---

# Rust properties

## The rule

**Assert a law, then make the strategy visit the region where the law would fail.**

A law is a sentence that holds for every input: text that parses renders back to itself; an
answer nobody awaits changes nothing. A strategy that never produces the input where the sentence
breaks turns the law into a proof about the wrong population: it passes on every run and
witnesses nothing. The strategy is the "for all" of the claim, and weighting it toward the edges
is what keeps the claim honest.

## The failure this prevents

Suppose `Quantity` implements `FromStr` by delegating to `str::parse::<u32>`. The std docs for
`u32::from_str` allow an optional `+` sign, and a leading zero is a digit like any other:

```rust
// What the std integer parser accepts, and the one spelling `Display` writes back.
assert_eq!("+7".parse::<u32>(), Ok(7));
assert_eq!("07".parse::<u32>(), Ok(7));
assert_eq!(7_u32.to_string(), "7");
```

The forward round trip, `q.to_string().parse() == Ok(q)`, holds on every run: it only feeds the
parser text that `Display` wrote, and `Display` never writes `+7` or `07`. Meanwhile a
deduplication key built from the raw text of a request sees `+7` and `7` as two different orders,
so a retry spelled differently places the order twice. The law that catches it runs over
generated spellings, near-misses included: every spelling that parses re-renders to itself.

## The rules that follow

- **State the law, not an example.** The catalogue: algebraic laws (`merge(a, b) == merge(b, a)`,
  associativity, identity); inverse pairs, the round trips below; idempotence
  (`apply(apply(s)) == apply(s)`, the law behind every dedup and every safe retry); equivalence
  (`old(x) == new(x)`, the proof of a rewrite, [[review-and-planning]]); and invariants under
  transition (a total is conserved, a status only moves forward).
- **A round trip is two properties.** Forward over values: value, text, the same value. Reverse
  over spellings, near-misses included: each one is an `Err(_)` or renders back to the same text.
  The forward law only ever sees the parser's own output, so a parser that accepts too much
  passes it forever.
- **Totality is a property.** For every generated input the function returns a value or an `Err`
  and never panics: proptest catches a panic in `holds` and reports it as a counterexample, so an
  overflow or an out-of-range index on an input nobody listed fails with that input printed. A
  panic inside a strategy is not caught, so keep its interpretation total. Its twin law, the same
  input twice gives the same answer, `Err` included, catches a hidden clock.
- **Every law is labelled.** `support::for_all(law, strategy, holds)` runs it, and a failure
  prints the law and the smallest input proptest found, as in
  ``law `a spelling that parses re-renders to itself` failed with "+1": the law returned false``.
  "assertion failed" alone sends someone bisecting.
- **Generate the raw spelling, then parse it.** The value enters through the production
  `FromStr`, never a test-only constructor, so every counterexample, shrunk or not, is a value
  production can receive. Keep sizes small (a few characters, a dozen steps): shrinking is faster
  and the counterexample shorter.
- **One generator per field, composed into the input.** Each field draws its value from its whole
  domain and its spelling from every form the input admits, and a line composes the fields.
  Failure has generators of its own: a flaw per field, composed with valid fields, makes a
  malformed input that knows how it must fail.
- **A property checks success or failure, never both.** Valid input comes from the valid
  generators and malformed input from the flaws, and each law asserts the one outcome its input
  must have: a law that expects `Ok` for some draws and a given `Err` for others is two laws. A law
  over raw text that only asks what parses to render back, as the reverse round trip does, asserts
  no failure.
- **Bias the strategy toward the edges.** `prop_oneof![3 => ordinary, 1 => edge]` sends one draw
  in four to an edge: zero, empty, `MAX` and one past it, the status just before a terminal one.
  Uniform draws over a `u32` reach zero about once in four billion. List first the arm a
  counterexample should shrink toward: proptest shrinks a `prop_oneof!` toward its earlier arms.
- **A draw that depends on earlier draws is a choice, interpreted afterwards.** Draw a list of
  plain choices, with an `Index` for every pick among what exists so far, then fold the list into
  the input. Every list proptest shrinks to still folds into an input the strategy could produce,
  and a pick with nothing to pick from falls back to a row of its own. Never `prop_flat_map` a
  sequence step by step, which shrinks poorly, and never draw picks with `prop_perturb`: what it
  draws from its generator replays unchanged while shrinking, so it never simplifies.
- **Build, never filter.** `prop_filter` and `prop_assume!` throw inputs away, and proptest stops
  the run after too many. A strategy that builds only the inputs it means never runs out.
- **A law replaces the examples it covers.** Delete the hand-picked examples it quantifies over;
  keep the black-box tests it does not reach.
- **Properties live on the pure core:** parsers, folds, arithmetic, decisions and transitions,
  where a strategy reaches further than any fixture. The black-box backbone stays primary
  ([[functional-architecture]]).
- **A state-machine driver generates the commands and the collaborator's answers.** It folds the
  sequence with `try_fold` and checks the invariant after every step: an answer when nothing is
  awaited changes nothing, and a terminal state absorbs everything. The fold is pure, so the
  collaborator is generated data, not a test double.
- **A random seed per run; a failure is saved and replayed.** proptest seeds each run from the
  operating system. A failing case prints a `cc` line and is saved to
  `app/tests/proptest-regressions/<module>.txt`, which the next run replays first. Commit that
  file with the fix of the failure it records, and never otherwise. Never pin `PROPTEST_RNG_SEED`
  in code, a `Makefile` line or CI: a pinned seed is one example.
- **Names state the law.** Properties are `prop_<law>`
  (`prop_spelling_that_parses_re_renders_to_itself`); a state-machine driver gets its own module.

## Where the depth is

- [references/proptest.md](references/proptest.md) — `support::for_all`, a worked pair of round
  trips, a strategy whose draws depend on earlier ones, a state-machine driver, and replaying a
  failure.
- [rust.md § 7](../functional-architecture/references/rust.md) — Tests: the one test crate, test
  names, and why helpers return `Result` instead of panicking.

## Verify

```sh
make test                                             # every law, 500 cases each, fresh seeds
PROPTEST_CASES=10000 cargo test --test it prop_<law>  # one law, deeper
cargo test --test it prop_<law>                       # replays its saved failures first
```

A property that never fails is suspect. Break the code on purpose once (let the parser accept a
`+`), run the law with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1` so the break leaves no regression
file, watch it catch the break, then restore the code. If the law stays green, the strategy never
reaches that region: fix the strategy, not the law.

Related: [[functional-architecture]] · [[review-and-planning]] · [[rust-effects]]
