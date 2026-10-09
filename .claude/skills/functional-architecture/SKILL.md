---
name: functional-architecture
description: "Use when writing or reviewing any Rust code in this repo — the shape the code must take before style matters. Covers types as propositions, illegal states made unrepresentable, total functions, decisions as an enum interpreted separately, ports and interpreters, foreign boundaries, captured effects, owned resources, one concern per layer, durable state machines, idempotency, which tests survive, contracts, and machine-checkable rules. Keywords: enum, match, exhaustive, catch-all, Option, Result, ?, newtype, From, TryFrom, FromStr, Drop, trait, generic, total function, partial function, panic, unwrap, indexing, as cast, overflow, NonZero, NonEmpty, parse don't validate, smart constructor, proposition, proof, witness, port, adapter, interpreter, decorator, composition root, effect, side effect, pure, clock, environment, state machine, idempotent, dedup, retry, resource, lifetime, leak, property test, law, round trip, black box test, contract, source of truth, lint, clippy."
---

# Functional architecture

The shape Rust code takes in this repo, before style matters; each section is one principle.
[references/rust.md](references/rust.md) spells every one in Rust and names the lint that enforces
it: read it before writing or reviewing code.

## Types are propositions, values are proofs

A signature is a claim and a total body is its proof: admit an input the body cannot honour and the
claim is false, so widen the return type or narrow the argument until it is true. The full
correspondence (evidence, case analysis, theorems, limits) is [references/first-principles.md](references/first-principles.md).

## Make illegal states unrepresentable

The type is the specification. A value that exists is valid: parse into a type that cannot hold a
bad value (private fields, with `FromStr` or `TryFrom` as the only constructor) instead of checking
a loose one later, and encode each invariant in the type instead of re-checking it at every use.
Three rules follow:

- **Absence has a type.** `Option`; never a sentinel `-1`, an empty string, or a `Default` for "unset".
- **Failure is a value.** A `Result` with an enum error; a foreign failure becomes a variant at the boundary.
- **No operation that can fail without saying so.** No `unwrap`, indexing, `as` or catch-all arm:
  the Banned table in `references/rust.md` § 1 names the lint for each.

## Total functions only

Every function returns a value for every input its type admits; when it cannot, that is a variant
in the return type, never a panic. A partial function is a lie in the signature, and the fix is a
wider return (`Option`, `Result`) or a narrower argument (`NonZeroU32`, a non-empty collection),
never an `assert!` on the way in.

## Decisions as an enum, then interpret

Split *deciding* from *doing*. A pure total function folds inputs read beforehand (`now` arrives as
a parameter, never sampled inside) into one variant of an enum; a separate interpreter turns that
variant into effects. The decision is then testable with no file, process or clock. If you cannot
name the decision enum, you have not found the decision yet.

**Exhaustive matches, never a catch-all arm.** A `_ =>` discards the compiler's only chance to tell
you a variant was added; adding one must break every match that has to change. The one
sanctioned catch-all, over a std `#[non_exhaustive]` enum at the boundary, is in `references/rust.md` § 3.

## Ports and interpreters

A capability is a trait; its implementations are interpreters of it, and they are substitutable,
which is what makes tests, metrics and dry runs possible without touching the logic. Metering,
retries and caching wrap the real implementation (`Metered<S>`) and never branch inside it: an `if`
added to the live one to pass a test means you wanted another interpreter. Declare a port beside
its consumer, and hand each consumer only the narrow role it names: [[rust-wiring]].

## The boundary proves what the compiler cannot

Wrap each foreign API exactly once, behind a port whose signatures never name its types. Parse what
crosses (a file, a variable, an argument, a reply); never trust it. Give each dependency one error
enum with total classifiers, converted from the foreign error in one place: [[rust-errors]].

## Effects are captured, never performed in passing

Anything non-deterministic or side-effecting (clock, environment, arguments, files, network,
threads, sleep, the standard streams, hash order) enters as a value or through a port. A function
that looks pure but reads the clock is not pure. Only `main` touches the real thing, and the
disallowed methods in `clippy.toml` make every other call site a lint failure: [[rust-effects]].

## A constructor never allocates what it cannot release

Whatever is acquired is owned by exactly one value: a factory (`open`) acquires and returns the
owner, a release that can fail is a consuming method called on the success path, and `Drop` is the
backstop. A leak is a lifetime nobody owns. `main` owns the long-lived handles, which drop in
reverse order; bound input and compute at the door: [[rust-resources]].

## One concern per layer

Each of these is a separate place; mixing two in one module is the first thing a review looks for. The
Rust form of each row is `references/rust.md` § 6.

| Concern | Where it lives |
|---|---|
| Lifetimes | locals of `main`, dropped in reverse order |
| Configuration | one `Config`, parsed once in `main`, never re-read |
| State | values passed in and returned; never a `static` or a shared cell |
| Logic | library modules that import no `fs`, `net`, `process` or `env` |
| Errors | one enum per boundary, beside its adapter |
| I/O | a port declared by its consumer, implemented in an adapter module |
| Mapping | `TryFrom` and `From` in the adapter; foreign types stop there |
| CLI surface | argument parsing and the exit-code mapping, in `main.rs` |

## Durable state is an explicit status machine

What must survive a restart is a record whose lifecycle is a closed enum with its terminal states
named. A transition consumes the old state (`fn ship(order: Accepted) -> Shipped`) and the persisted
change names the state it moves from, so a repeated or stale transition is a no-op, not a second
write. A terminal state absorbs every later event. Persist before acknowledging; a process holds
nothing a restart would lose.

## Idempotency by construction

Re-delivery and retry are the normal case; absorb them in the shape of the code. Dedup on a natural
key at intake: the first `OrderId` wins and a repeat is an answer, not an error. Derive the id of
every command sent out from the business key, so a retry re-sends the same command and is safe
because the receiver dedups it.

## Tests keep the black box

Write the failing test first, then the minimum implementation that passes; that part is not
optional. The backbone is a black-box test in `backend/tests`, which the compiler limits to the
public API: it drives the real surface (the binary, or the library's entry point) and asserts
behaviour. Add a unit test beside the code only for a pure fold awkward to reach from outside, such
as a `decide` or a `disposition`. Delete tests of construction, derives and field access. A
property states a law over generated values and covers the pure core: [[rust-properties]].

## The contract is a file, and it is the source of truth

When the code has an external contract, that file is the specification: the code conforms to it,
tests read it as the oracle, and editing it without the code fails the build. Error codes are part
of it: a caller branches on the code, so the set grows and never churns; the message is for people.

## Prefer the machine-checkable form of a rule

Where a rule has an encoding a tool enforces, that encoding is the authoritative statement and
prose summarizes it: the lint tables in `Cargo.toml`, `clippy.toml`, `rustfmt.toml`, `make deps`
and `.claude/settings.json`. Name the file rather than restating its contents; the restatement is
the copy that goes stale.

## Where the depth is

- [references/first-principles.md](references/first-principles.md): why the principles are one rule.
- [references/rust.md](references/rust.md): the Rust spelling, the Banned table, the review-only list.
- [[rust-effects]] (effects and time) · [[rust-errors]] (foreign failures, classified once) ·
  [[rust-resources]] (ownership and release) · [[rust-wiring]] (the composition root) ·
  [[rust-properties]] (laws, generators and the harness).

## Verify

`make ci` is green, every `#[expect]` reason is true, and a review walks the Banned table and the
review-only list in `references/rust.md` § 1 over the diff.

Related: [[architecture-decisions]] · [[review-and-planning]]
