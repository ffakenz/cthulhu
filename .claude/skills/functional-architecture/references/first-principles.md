# First principles

Why the rules in [SKILL.md](../SKILL.md) are one rule. The doctrine is the propositions-as-types
correspondence taken as an engineering practice: a type is a proposition, a program is its proof,
and the compiler is the referee.

## A signature is a claim; a total implementation is its proof

`fn parse(text: &str) -> Order` claims that every string is an order. That claim is false, and the
body will prove it false at runtime, on someone else's input. `fn parse(text: &str) -> Option<Order>`
is a claim the body can actually prove, and `Result<Order, ParseError>` proves it while saying why
an input was refused. Every partial function is a theorem stated wrong: the domain is narrower than
declared (narrow the argument type), or the codomain is wider than declared (widen the return
type). The fix is never a guard that panics: a panic is the proof giving up while the
signature keeps claiming.

Totality is not a style preference. It is the property that makes local reasoning sound: a caller
who has a value of the return type needs to know nothing about how the function got it.

## A value's existence is evidence

A smart constructor discharges a proof obligation exactly once. `OrderId::from_str` checks the
grammar; every function that later receives an `OrderId` holds a *witness* that the check happened
and needs no defensive re-check. This is why "parse, don't validate" is a proof technique, not a
taste: validation asserts a fact and drops the evidence; parsing turns the fact into a type, and the
type carries the evidence to every use site. Private fields keep the witness honest: a `pub` field
is a second constructor that skips the proof.

The contrapositive is the review heuristic: a re-check deep in the core (an `is_empty` guard, a
re-validation, a defensive clamp) is the code confessing that some path constructs the value without
proving it. Find that path; do not add the guard.

## An exhaustive match is case analysis

An enum is a finished enumeration of the possible worlds; a `match` over it is a proof by cases. A
missing arm is an unproven theorem the compiler is offering to name, and a `_` arm declines the
offer. When a variant is added, every match without a catch-all becomes a list of exactly the places
the proof must be extended. A catch-all converts that list to zero findings and ships the unproven
case to production.

## A property test is a theorem over witnesses

An example-based test proves the theorem for one witness. A property states the theorem
(`∀ a b: total(a + b) = total(b + a)`) and checks it over witnesses a generator supplies. The
generator is the population the theorem is quantified over, which is why generator design is where
the honesty lives: a law checked only over inputs that cannot fail is a proof about the wrong
population. Bias the generator toward the region where the proof would break: the zero divisor, the
empty list, the boundary of the representable.

Round-trips are the theorems most worth stating, and they are two theorems, not one:
`x.to_string().parse() == Ok(x)` for every generated `x`, and `s.parse().map(|x| x.to_string()) == Ok(s)`
for every canonical spelling `s`, with every other spelling refused. An implementation can satisfy
the first by refusing or distorting everything the generator never produces; the second closes that
door.

## The limit: erased propositions

At runtime the propositions are gone. A file, an environment variable, an argument, a reply from
another process: each arrives as bytes with no proof attached. The correspondence does not extend
past the process boundary, so the boundary must re-prove every claim before it becomes a type: parse
the bytes, re-check the invariant, convert the foreign error into a variant. That discipline is
[[rust-errors]], and it is why "the compiler checked it" is never an argument about data that
crossed a process boundary.

## What this buys

Review stops being taste. "This function is partial", "this value is unproven here", "this match
declines case analysis", "this law is quantified over the wrong population" are findings with a
shape, and the fix is mechanical: widen, narrow, parse, enumerate. The rules in
[SKILL.md](../SKILL.md) are the same finding stated for each place it appears.
