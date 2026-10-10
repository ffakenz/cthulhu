---
name: curry-howard
description: "Use when designing, reading or reviewing a type or a signature as the claim it makes: propositions as types, programs as proofs. Covers the correspondence in Rust (products, sums, total functions, unit, uninhabited types, generics, return-position impl Trait, refinements), the moves that strengthen a weak proposition (fields present only in one case to sums, flags to enums, positional tuples to named products, primitives to newtypes, wide returns to exact ones, stored redundancy to derivations), decisions as a proof or a refutation, small DSLs whose sentences are only the valid ones, and where types stop and laws take over. Keywords: Curry-Howard, Curry–Howard, propositions as types, proofs as programs, type as proposition, value as proof, evidence, witness, product type, sum type, conjunction, disjunction, implication, negation, Infallible, never type, uninhabited, unit, generic, universal, existential, impl Trait, refinement, newtype, smart constructor, parse don't validate, typestate, Option, Result, bool, flag, tuple, primitive obsession, correlated fields, illegal state, total function, decision procedure, DSL, embedded language, decision table, builder, fixture, law."
---

# Curry–Howard

## The rule

**Read every type as a proposition and every value as its proof, then make the proposition the
one the domain asserts: no weaker, no stronger.**

A type that admits a value the domain forbids is unsound: the compiler accepts a proof of
something false, and every reader must remember which values to ignore. A type that cannot hold a
state the domain allows is incomplete: the code works around it. Between the two, the type says
what the domain says, and the code that handles it needs neither a comment nor a guard. Why the
rules of [[functional-architecture]] are one rule is its
[first principles](../functional-architecture/references/first-principles.md); this skill is the
correspondence itself and the moves it suggests. [[domain-modeling]] decides which concepts get a
type; this skill decides what each type claims.

## The failure this prevents

Suppose a test records each step of a run as:

```rust
struct Step {
    command: Command,
    sight: Option<Sight>,
    decision: Result<Event, Reason>,
    again: Result<Event, Reason>,
}
```

Read as a proposition: a command, and maybe a sight, and a decision, and another decision. The
domain says something sharper: a client command with its sight, or a dispute command without one.
The product admits a dispute with a sight and a client command without one, so every law that
reads `sight` re-derives which case it is in; and `again` is a conjunct that only one law needs,
paid for by every step. As a sum, the impossible cases have no spelling:

```rust
enum Intake { Client(ClientCommand, Sight), Dispute(DisputeCommand) }
```

## The correspondence, in Rust

| Logic | Type | Rust | Here |
|---|---|---|---|
| A and B | product | `struct`, tuple | a footprint: an account and at most one deposit |
| A or B | sum | `enum` | `Command`: a client command or a dispute command |
| A implies B | total function | `fn(A) -> B` | `evolve`: a footprint and an event give the next footprint, or an overflow |
| true | unit | `()` | `Ok(())` from a commit: nothing more to show |
| false | uninhabited | `Infallible`, an empty `enum` | a memory store's `Error = Infallible`: its failure cannot be built |
| not A | A implies false | `fn(A) -> Infallible` | `match never {}`: from a proof of false, anything follows |
| maybe A | A or true | `Option<A>` | `Option<Deposit>`: a deposit, or none |
| A, or why not | a decision | `Result<A, E>` | `decide_client`: a `ClientEvent` proves the command, a `ClientReason` refutes it |
| for every T | universal | generics, argument-position `impl Trait` | `run<L: LedgerStore, S: SeenIds>`: one proof for every pair of stores |
| some T | existential | return-position `impl Trait` | `Accounts::by_client`: an iterator whose type the function keeps to itself |
| the x such that P(x) | refinement | newtype, private field, one parser | `Amount`: a decimal that is positive, exact at four places and below 10^15 |

## The moves

Each move turns a weak proposition into the domain's. Make one when it removes a value the domain
forbids or names a meaning a reader would otherwise reconstruct; skip it when it only adds
ceremony.

| Weak shape | What it claims | The domain's shape |
|---|---|---|
| a field that is `Some` exactly in one case, `{ command, sight: Option<Sight> }` | A and maybe B, under a side condition no type checks | a sum whose variants carry what each case has |
| nested options, `Option<Option<T>>` or an optional field of an optional | three cases without names | an enum naming each case, or a reshape so each absence means one thing |
| a `bool` argument, `Account::new(available, held, true)`, or two flags that must never both be set | one of two anonymous cases, its meaning lost at the call site; or a pair admitting a state no value can be in | an enum with named variants; a `bool` named wherever it is read or written, `is_locked()`, or `locked: true` in a literal, reads fine |
| a positional tuple with a comment, `(i128, i128, bool) // available, held, locked` | an anonymous conjunction | a struct whose field names carry the comment |
| a bare primitive for a concept, `u32` for a tx id | any integer at all | a newtype; a refinement with a parser if not every integer qualifies |
| a field that follows from others, a stored `total` | a conjunct that can contradict the rest | a method that computes it |
| a key stored inside the value it indexes | the same fact twice, free to disagree | the key beside the value, outside it |
| a return wider than what the function produces | the function may give what it never gives | a narrower enum nested in the wider one, widened by hand where a caller needs it: `.map_err(Reason::Client)` |
| a status checked before each operation | the operation may run in any state | a type per state and transitions that consume it, when the state is known where the code is written |
| `panic!`, `unwrap`, `expect` | a proof of false: the signature lies | a variant in the return type |

## Decisions are proofs or refutations

A decision returns evidence either way. `decide_client` takes an account, a command and its sight,
and returns a `ClientEvent`, the proof that the command holds, or a `ClientReason`, the
refutation. Make both sides as precise as the domain: a function that can only refuse for three
reasons states a sharper theorem when its error type has those three, and every caller's `match`
loses the arms that could never run.

## DSLs: a language whose sentences are the valid ones

An embedded language is a sum type for its sentences and a total function for their meaning.
`Command` is the grammar of requests and `decide_client` and `decide_dispute` give their meaning;
`Event` is the grammar of facts and `evolve` gives theirs. What the grammar cannot spell cannot
happen. The same move pays in tests:

- a decision table written as an exhaustive `match` over fieldless enums, row for row as its
  record states it, is an oracle a reader checks against the record by eye and the compiler
  checks for every case, which a table kept as an array of data would give up;
- a scenario builder, `deposit(1, 10, "25.5")` then `dispute(2, 20)`, makes a fixture read like
  the overview's worked example;
- an expected result written as named structs, not positional tuples, states what is expected.

A DSL earns its place when its sentences are fewer and clearer than the code it replaces; a macro
or a builder that only renames calls is ceremony.

## Where types stop

Rust has no dependent types. "Held equals the sum of the disputed deposits" relates one value to a
collection, and no type here can state it: such a proposition is a law, checked over generated
witnesses ([[rust-properties]]). At the process boundary the propositions are erased, so a parser
proves each claim again before it becomes a type ([[rust-errors]]). For the same reason a
generator stays weak on purpose: it writes the raw spelling a source would read, so production
parsing makes every claim, and a counterexample is input someone could send.

## Review questions

1. What proposition does the type state, in one sentence, and is it the domain's?
2. Which value does it admit that the domain forbids, and which state does it miss?
3. Is a field present exactly when another field has some value?
4. Does a `bool`, a tuple or a primitive hide a name?
5. Can a function return something it never returns?
6. Is anything stored that could be derived, or held twice?
7. What can only a law state, and is that law written?

## Verify

`make ci` is green, every type the change adds has an answer to every review question, and every
proposition no type can state has its law.

Related: [[functional-architecture]] · [[domain-modeling]] · [[rust-properties]] · [[rust-errors]]
