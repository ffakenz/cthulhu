---
name: rust-properties
description: "Use when writing or reviewing a property test, a generator, or a test that passes no matter what. Covers laws over examples, round-trips in both directions, totality as a property, a seeded std-only generator, edge-weighted generation, labelled laws, raw spelling then parse instead of shrinking, replay by seed, and state-machine drivers with generated answers. Keywords: property, law, invariant, round trip, FromStr, Display, generator, seed, SplitMix64, replay, counterexample, catch_unwind, edge case, zero, empty, boundary, commutative, associative, idempotent, state machine, try_fold, prop_."
---

# Rust properties

## The rule

**Assert a law, then make the generator visit the region where the law would fail.**

A law is a sentence that holds for every input: text that parses renders back to itself; an
answer nobody awaits changes nothing. A generator that never produces the input where the
sentence breaks turns the law into a proof about the wrong population: it passes on every run and
witnesses nothing. The generator is the "for all" of the claim, and weighting it toward the edges
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
  and never panics: the harness catches a panic and reports it as a counterexample, so an
  overflow or an out-of-range index on an input nobody listed fails with that input printed. Its
  twin law, the same input twice gives the same answer, `Err` included, catches a hidden clock.
- **Every law is labelled.** A failure prints the law, the case number, the input and the seed,
  as in ``law `a spelling that parses re-renders to itself` failed at case 13 with "+170"`` plus
  the replay line. "assertion failed" alone sends someone bisecting.
- **No shrinking: generate the raw spelling, then parse it.** The value enters through the
  production `FromStr`, never a test-only constructor, so every counterexample is a value
  production can receive. Keep sizes small (a few characters, a dozen steps) and the first
  counterexample is already readable.
- **Bias the generator toward the edges.** One draw in four, `one_in(EDGE_ODDS)`, takes an edge:
  zero, empty, `MAX` and one past it, the status just before a terminal one. Uniform draws over a
  `u32` reach zero about once in four billion.
- **A law replaces the examples it covers.** Delete the hand-picked examples it quantifies over;
  keep the black-box tests it does not reach.
- **Properties live on the pure core:** parsers, folds, arithmetic, decisions and transitions,
  where a generator reaches further than any fixture. The black-box backbone stays primary
  ([[functional-architecture]]).
- **A state-machine driver generates the commands and the collaborator's answers.** It folds the
  sequence with `try_fold` and checks the invariant after every step: an answer when nothing is
  awaited changes nothing, and a terminal state absorbs everything. The fold is pure, so the
  collaborator is generated data, not a test double.
- **A fresh seed per run, printed and replayable.** The harness draws the seed at its edge and
  every failure prints it; `PROPERTY_SEED=0x… cargo test --test it <name>` replays it. Never
  commit a pinned seed: a pinned seed is one example.
- **Names state the law.** Properties are `prop_<law>`
  (`prop_spelling_that_parses_re_renders_to_itself`); a state-machine driver gets its own module.

## Where the depth is

- [references/harness.md](references/harness.md) — the std-only harness to copy into
  `backend/tests/it/support.rs` when the first property lands, a worked pair of round trips, a
  state-machine driver, and replay.
- [rust.md § 7](../functional-architecture/references/rust.md) — Tests: the one test crate, test
  names, and why helpers return `Result` instead of panicking.

## Verify

```sh
make test                                          # every law, on a fresh seed
PROPERTY_SEED=0x… cargo test --test it prop_<law>  # a printed failure, replayed
```

A property that never fails is suspect. Break the code on purpose once (let the parser accept a
`+`) and watch the law catch it, then restore the code. If the law stays green, the generator
never reaches that region: fix the generator, not the law.

Related: [[functional-architecture]] · [[review-and-planning]] · [[rust-effects]]
