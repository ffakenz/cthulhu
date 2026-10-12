# Rust

How the principles in [SKILL.md](../SKILL.md) are spelled in Rust. The compiler already proves
absence (`Option`), failure (`Result`), exhaustiveness (a missing match arm is error E0004) and
lifetimes (the borrow checker); this file covers what it leaves open and names the lint that closes
each gap. The lint tables in the root `Cargo.toml` (`[workspace.lints]`) and `clippy.toml` are the
authoritative statement: this file names lints and never restates their levels. The nouns are
examples; the shapes are the point.

## 0. Baseline

- Lints are set once, in the root `Cargo.toml`, and every crate inherits them through
  `[lints] workspace = true`; `clippy.toml` holds the disallowed methods and the test allowances.
  No source file carries an inner `#![deny(…)]`, `#![warn(…)]` or `#![allow(…)]`.
- The only suppression is `#[expect(lint, reason = "…")]` on the narrowest item, with a reason that
  is true. `allow_attributes` rejects `#[allow]`, `allow_attributes_without_reason` rejects a
  suppression with no reason, and `unfulfilled_lint_expectations` fails the gate once the lint
  stops firing, so a suppression cannot outlive its cause.
- `unsafe` is forbidden (`unsafe_code`), and no attribute can lift it.
- `make lint` runs clippy on every target and fails on any warning. Nothing below holds unless it
  is green.

## 1. Make illegal states unrepresentable

- **Absence is `Option`.** No sentinel (`-1`, `0`, `""`, `u32::MAX`) for "not set", and no
  `Default` standing in for it: `#[derive(Default)]` on a domain type whose zero value is invalid
  hands every caller a value no parser checked.
- **Failure is a `Result` whose error is an enum**, one variant per way the operation can fail,
  implementing `Display` and `error::Error` ([[rust-errors]]).
- **Parse, don't validate.** Fields are private and `FromStr` or `TryFrom` is the only constructor,
  so a value that exists passed the parser and no later caller re-checks it:

```rust
use std::num::NonZeroU32;
use std::str::FromStr;

/// How many units an order asks for: never zero, written in plain digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity(NonZeroU32);

/// Why a spelling or a number is not a quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityError {
    /// Empty, signed, led by a zero, or holding a non-digit.
    Malformed,
    /// Zero units.
    Zero,
    /// More units than a `u32` holds.
    TooLarge,
}

impl FromStr for Quantity {
    type Err = QuantityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
            return Err(QuantityError::Malformed);
        }
        let units = text.chars().try_fold(0_u32, |value, c| {
            let digit = c.to_digit(10).ok_or(QuantityError::Malformed)?;
            let shifted = value.checked_mul(10).ok_or(QuantityError::TooLarge)?;
            shifted.checked_add(digit).ok_or(QuantityError::TooLarge)
        })?;
        NonZeroU32::new(units).map(Self).ok_or(QuantityError::Zero)
    }
}
```

A number that arrives already parsed gets `impl TryFrom<u32> for Quantity` with the same zero check;
`Quantity` gets `Display`, writing the plain digits, and `QuantityError` gets `Display` and
`error::Error` like every error enum.

- **Newtypes for ids and units.** `OrderId(String)` and `Quantity(NonZeroU32)` cannot be swapped at
  a call site; a bare `String` and a bare `u32` can. Use `NonZeroU32` and its siblings wherever zero
  is not a value.
- **Non-empty is a type.** `NonEmpty<T> { head: T, tail: Vec<T> }`, fields private:
  `from_vec(Vec<T>) -> Option<Self>` is the one place emptiness is checked, and
  `first(&self) -> &T` is total because there always is a head.
- **An enum instead of bools.** `ship(&order, true)` hides the decision at the call site, and two
  flags that must never both be set (`shipped`, `cancelled`) admit an order no order can be;
  `enum Status { Accepted, Shipped, Cancelled }` admits only real ones. Pedantic
  `fn_params_excessive_bools` and `struct_excessive_bools` flag the extreme; review catches the rest.
- **Immutable and borrowed by default.** `let` unless a fold needs an accumulator; take `&T`, then
  `&mut T` only when the function must change the value, then `T` only when it must keep it.
- **No interior mutability or `pub` fields in domain types.** `Cell`, `RefCell` and `Mutex` let a
  shared reference change the value, hiding a state change from every signature; a `pub` field on a
  validated type is a second constructor that skips the parser. Read access is a method.

### Banned

The lint in each row rejects the call; the last column is what to write instead.

| Call | Lint | Instead |
|---|---|---|
| `.unwrap()`, `.expect(…)` | `unwrap_used`, `expect_used` | `?`, `match`, `let … else`, `ok_or` |
| `panic!`, `todo!`, `unimplemented!`, `unreachable!` | `panic`, `todo`, `unimplemented`, `unreachable` | a variant in the return type |
| `v[i]`, `&v[a..b]` | `indexing_slicing` | `get`, `first`, `split_first`, an iterator |
| `&s[a..b]` on a `str` | `string_slice` | `split_once`, `strip_prefix`, `char_indices` |
| `x as T` | `as_conversions` | `From`, `TryFrom` |
| `_ =>` or `other =>` on an enum | `wildcard_enum_match_arm` | name every variant (§ 3) |
| `let _ = fallible();` on a `#[must_use]` value | `let_underscore_must_use` | `?`, or `match` on both arms |
| `map.insert(k, v);` with the result unread | `unused_results` | read it; `let _ = …` only when discarding is the point |
| `unsafe` | `unsafe_code` | the safe std API; anything std lacks is a dependency, which needs approval and a decision record |
| `process::exit(…)` | `exit`, `disallowed_methods` | return an `ExitCode` from `main` |
| `mem::forget(…)` | `mem_forget` | let `Drop` run |
| `print!`, `println!`, `eprint!`, `eprintln!`, `dbg!` | `print_stdout`, `print_stderr`, `dbg_macro` | `writeln!` to an injected `impl Write` |
| `#[allow(…)]` | `allow_attributes`, `allow_attributes_without_reason` | `#[expect(…, reason = "…")]` |
| `SystemTime::now`, `Instant::now`, `elapsed` on either, `env::var*`, `env::args*`, `env::current_dir`, `env::temp_dir`, `thread::spawn`, `thread::Builder::spawn`, `thread::sleep`, `panic::catch_unwind` | `disallowed_methods` | a port, or a value passed in; `clippy.toml` names the alternative for each |
| clap's `Parser::parse`, `try_parse`, `parse_from`, `update_from`; `Command::get_matches`, `get_matches_mut`, `try_get_matches`, `get_matches_from`, `print_help`, `print_long_help`; `clap::Error::exit`, `print` | `disallowed_methods` | `try_parse_from` over `main`'s argument snapshot, writing the `clap::Error` through the handle `main` locked |

### Review-only

No lint catches these; review does.

- `assert!` or `debug_assert!` in production code: a panic with a nicer message (§ 2).
- `Box<dyn Error>` below `main`: it erases the enum a caller would match on.
- `#[derive(Default)]` on a domain type: a zero value no parser checked.
- `pub` fields on a validated type: a constructor that skips the parser.
- `LazyLock` or `OnceLock` holding an effect's result: the first caller's clock or environment,
  served to every later caller.
- `HashMap` or `HashSet` iteration that reaches output: the order changes from run to run.
- `#[non_exhaustive]` on our own enums: it forces a catch-all on every other crate that matches
  them (§ 3).
- `matches!`, `if let` or `let … else` over one of our own enums where every variant must be
  classified: an implicit catch-all that `wildcard_enum_match_arm` cannot see; write the `match`.
- `Arc<Mutex<_>>` as shared state in the core: state is a value passed in and returned (§ 6).

## 2. Total functions only

```rust,no_run
// partial: panics on an empty batch, and the signature does not say so
pub fn oldest<T>(batch: &[T]) -> &T {
    &batch[0]
}
```

```rust
// total: the empty batch is a case in the return type
pub fn oldest<T>(batch: &[T]) -> Option<&T> {
    batch.first()
}
```

Or narrow the argument instead of widening the return: `fn oldest(batch: &NonEmpty<Order>) -> &Order`
returns `batch.first()` and has no empty case to report. An `assert!` at the top of a function is
the same lie with a nicer message: the signature still claims every input. If the argument can be
wrong, the argument's type is wrong.

The reverse lie is an `Option` or `Result` that is never `None` or `Err`: it claims a failure that
cannot happen and makes every caller handle it. Pedantic `unnecessary_wraps` flags it on private
functions and spares exported ones, so review catches those.

Integer overflow panics in a debug build and wraps in a release build: two behaviours, neither in
the signature. Wherever overflow is reachable, name the one you mean: `checked_add` (an `Option`),
`saturating_add` or `wrapping_add`. The backoff in § 3, `2_u64.saturating_pow(attempt)`, is total
for every `attempt`.

## 3. Decisions as an enum, then interpret

A pure total function folds what was read beforehand, `now` included, into one variant; a separate
interpreter performs the effects the variant names.

```rust
use std::time::{Duration, SystemTime};

/// An order as the decision sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    placed_at: SystemTime,
}

/// What happens to an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Append it to the journal.
    Accept,
    /// Refuse it: it is more than a day old.
    Expire,
    /// The sender's clock runs ahead; look again after the delay.
    Defer(Duration),
}

const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Pure and total: reads no clock and performs no I/O; `now` is an argument.
pub fn decide(order: &Order, now: SystemTime) -> Decision {
    match now.duration_since(order.placed_at) {
        Ok(age) if age > MAX_AGE => Decision::Expire,
        Ok(_) => Decision::Accept,
        Err(ahead) => Decision::Defer(ahead.duration()),
    }
}
```

The interpreter, `fn interpret(decision: Decision, order: &Order, writer: &mut impl OrderWriter)`,
is the only part that touches a port; `decide` is tested with any `now` and no test double. The
fullest form is one enum in, one enum out: a failure folds into what to do about it, and only the
`Disposition` drives the interpreter.

```rust
use std::time::Duration;

/// Why one attempt to append an order failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendError {
    /// The journal did not answer in time.
    TimedOut,
    /// A journal line does not parse; carries its line number.
    Corrupt(u64),
    /// The journal file does not exist.
    Missing,
}

/// What the interpreter does with a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Try again after the delay.
    Retry(Duration),
    /// Set the order aside, with its error, for a person.
    Park,
    /// Stop the run: no later attempt can succeed.
    Abort,
}

/// One enum in, one enum out: pure, total, and exhaustive with no catch-all. The retry budget
/// is an argument, pinned by `main` next to the timeouts.
pub fn disposition(error: &AppendError, attempt: u32, max_attempts: u32) -> Disposition {
    match error {
        AppendError::TimedOut if attempt < max_attempts => {
            Disposition::Retry(Duration::from_secs(2_u64.saturating_pow(attempt)))
        }
        AppendError::TimedOut | AppendError::Corrupt(_) => Disposition::Park,
        AppendError::Missing => Disposition::Abort,
    }
}
```

**How exhaustiveness is enforced.** A `match` that misses a variant does not compile: delete the
`AppendError::Missing` arm above and the build fails with E0004, naming the variant. Adding a variant
therefore lists every match that must change. The remaining hole is the catch-all: `_ =>`, or a bare
binding such as `other =>`, compiles forever and swallows the next variant. `wildcard_enum_match_arm`
flags both. Name every variant, and join the ones that share an arm with `|`.

**The honest limit.** std marks some enums `#[non_exhaustive]` (`io::ErrorKind` is the one an
adapter meets), and a match on them must end in a catch-all. It sits once, in the adapter that
classifies the foreign error, under
`#[expect(clippy::wildcard_enum_match_arm, reason = "io::ErrorKind is non_exhaustive")]`, and maps
every kind it does not name to a variant that says so ([[rust-errors]]). Never put
`#[non_exhaustive]` on our own enums: the packages, the binary and the test crate are separate
crates, and every match outside an enum's own crate would be forced into the catch-all this
section bans.

## 4. Ports and interpreters

- **Declared by the consumer.** The trait lives in the module that calls it and names only what
  that module calls: the reading code's `OrderReader`, the accepting code's `OrderWriter`. One
  `Journal` adapter implements both.
- **Interpreters wrap; they never branch.** `Live` is the real adapter, `Noop` the dry run (every
  method answers `Ok`), and `Metered<S>` wraps any `S` that implements the same trait. An
  `if metrics_enabled` inside `Live` means you wanted another interpreter.

```rust
/// An order the core has accepted.
#[derive(Debug)]
pub struct Order;

/// Why the journal refused an append.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalError {
    /// The device is out of space.
    Full,
}

/// What the accepting code needs; adapters implement it elsewhere.
pub trait OrderWriter {
    /// Appends `order`, returning once it is durable.
    fn append(&mut self, order: &Order) -> Result<(), JournalError>;
}

/// Counts successful appends and answers whatever the wrapped writer answered.
#[derive(Debug)]
pub struct Metered<S> {
    inner: S,
    appended: u64,
}

impl<S: OrderWriter> Metered<S> {
    /// Wraps `inner` with a count of zero.
    pub fn new(inner: S) -> Self {
        Self { inner, appended: 0 }
    }

    /// How many appends succeeded.
    pub fn appended(&self) -> u64 {
        self.appended
    }
}

impl<S: OrderWriter> OrderWriter for Metered<S> {
    fn append(&mut self, order: &Order) -> Result<(), JournalError> {
        self.inner.append(order)?;
        self.appended = self.appended.saturating_add(1);
        Ok(())
    }
}

/// Core logic, generic over the port: it never names an adapter.
pub fn accept_all(orders: &[Order], writer: &mut impl OrderWriter) -> Result<(), JournalError> {
    orders.iter().try_for_each(|order| writer.append(order))
}
```

- **Static dispatch in the core.** Library functions are generic over their ports;
  `Box<dyn OrderWriter>` appears only in `main`, where `Config` chooses an implementation at run
  time.
- **Associated types** when what an implementation returns varies with the implementation: an
  `OrderReader` with `type Orders: Iterator<Item = Result<Order, JournalError>>` reads lines from a
  file in `Journal` and walks a `Vec` in a test double.
- **No mocking crate.** A test double is a struct in the test crate that implements the trait and
  answers or records what the test needs.

## 5. Effects are captured

The catalogue (clock, environment and arguments, files, network, processes, threads, sleep, the
standard streams, hash order, and statics fed by any of these) lives in [[rust-effects]]. The Rust
mechanics:

- **`now` is a parameter** of every decision (§ 3). An interpreter that must read time again takes a
  `Clock` port, and only `main.rs` implements it over the real clock, because `SystemTime::now` is a
  disallowed method everywhere else:

```rust
use std::time::SystemTime;

/// Where an interpreter reads the current time.
pub trait Clock {
    /// The current time.
    fn now(&self) -> SystemTime;
}

/// The real clock; it lives in `main.rs`, the one module allowed to read it.
#[derive(Debug, Clone, Copy)]
struct SystemClock;

impl Clock for SystemClock {
    #[expect(clippy::disallowed_methods, reason = "the composition root owns the real clock")]
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}
```

- **Configuration is a function of a snapshot.** `main` takes one `env::vars_os()` snapshot and
  parses it into a `Config` (`Config::from_env(vars) -> Result<Config, ConfigErrors>`, over any
  `IntoIterator<Item = (OsString, OsString)>`); nothing else reads the environment. `std::env::set_var` is `unsafe` in edition 2024 and `unsafe_code` is
  forbidden, so a test cannot change the process environment: it passes another snapshot, or sets a
  child's environment with `Command::env`.
- **Statics hold data.** A `static`'s value is fixed at compile time, so no code runs before `main`. `LazyLock`
  and `OnceLock` initialise on first use and never hold an effect's result: a
  `LazyLock<SystemTime>` freezes the first caller's clock for every caller after it.
- **`fn main() -> ExitCode` is the root shape.** `process::exit` skips every destructor still
  pending, and `main() -> Result<(), E>` reports the error through its `Debug` form. `main` maps each
  `RunError` to an exit code with a total `match` ([[rust-wiring]]).

## 6. One concern per layer

| Concern | Rust form |
|---|---|
| Lifetimes | locals of `main` in `main.rs`, dropped in reverse order of declaration |
| Configuration | one `Config`, parsed in `main` from an environment snapshot and passed down |
| State | values threaded through calls and returned; a `static mut` cannot be touched without `unsafe` |
| Logic | library modules that import no `fs`, `net`, `process` or `env` |
| Errors | one enum per boundary, beside its adapter |
| I/O | a port declared by its consumer, plus an adapter module that implements it |
| Mapping | `TryFrom` and `From` in a per-boundary adapter module; foreign types stop there |
| CLI surface | a `clap` parser over `main`'s argument snapshot (`try_parse_from`), each argument a `FromStr` type, and the exit-code mapping, in `main.rs` |

**The compiler-checked form.** Review keeps logic modules free of adapters. When the core must
never reach an adapter, the core becomes its own workspace crate with no dependency on the adapter
crates: the import then does not resolve, and the layering is a build failure instead of a review
finding. Here that crate is `engine`, under `libs/`, and `store` and `backend` depend on it. A
workspace path crate is not a dependency, so `make deps` still passes; the split itself is a
decision record ([[architecture-decisions]]).

## 7. Tests

- **Red first.** The failing test lands before the code that makes it pass.
- **The black box is a compiler fact.** `backend/tests` is a separate crate that sees only the
  public APIs of `engine`, `store` and `backend`, so it cannot pin a private function.
- **The backbone drives the real surface:** the binary through
  `Command::new(env!("CARGO_BIN_EXE_cthulhu"))`, or `engine`'s entry point, `run`, with in-memory
  ports.
- **One test crate.** Cargo builds each file directly under `tests/` as a separate crate, so the
  tests live in one: `backend/tests/it/main.rs`, with a module per library module (`it/order.rs`
  tests the module `order`, whichever package holds it) and shared helpers in `it/support.rs`.
- **Unit tests only for pure folds** that are awkward to reach from outside (a `decide`, a
  `disposition`, a parser's edge), in a `#[cfg(test)] mod tests` beside the code.
- **Names state the behaviour:** `refuses_a_quantity_with_a_leading_zero`, never `test_parse_2`.
  Properties are `prop_<law>` (`prop_quantity_round_trips`), and a state-machine driver gets its
  own module ([[rust-properties]]).
- **Panicking is the test's job, not the helper's.** `clippy.toml` allows `expect`, `unwrap`,
  indexing and `panic!` inside `#[test]` functions and `#[cfg(test)]` modules, so a test writes
  `expect("what must hold")`. A helper in `tests/it/support.rs` is neither, and `unwrap_used` fires
  there: helpers return `Result` or `Option`, and are `pub(crate)` (`unreachable_pub`).
- **No `#[should_panic]` for a domain failure.** A domain failure is an `Err` the test asserts on;
  `should_panic` passes on any panic, the wrong one included.
- **`#[ignore = "reason"]`**, never a bare `#[ignore]` (pedantic `ignore_without_reason`).
- **Delete** tests of `Debug` output, derives, constructors and field access: they pin the
  implementation, not the behaviour.

## Conventions

- **Naming.** `UpperCamelCase` for types, traits and variants; `snake_case` for functions, modules
  and variables; `SCREAMING_SNAKE_CASE` for constants and statics. rustc's `nonstandard_style`
  lints warn on the rest. A module is named for the concept it holds (`order`, `journal`), never
  `utils`, `common` or `helpers`; test modules mirror source modules.
- **Imports.** std through its module where the bare name is ambiguous (`io::Error`, `fmt::Result`,
  `error::Error`); crate items by name (`use crate::order::Quantity;`). No glob imports except
  `use super::*` in a unit-test module, which pedantic `wildcard_imports` exempts.
- **Text and bytes.** `&str` in, `String` out; `Arc<str>` for text with several owners (`rc_buffer`
  flags `Arc<String>`); `&[u8]` in and `Vec<u8>` out for bytes; `Path` and `PathBuf` for paths.
  `BTreeMap` and `BTreeSet` wherever order is observable: output, files, anything compared across
  runs.
- **Logging.** std has none, and `println!` is banned (§ 1). An event is a value, an enum whose
  variants carry the context fields (`Event::Accepted { id: OrderId, quantity: Quantity }`), sent
  through a port; the adapter formats it.
