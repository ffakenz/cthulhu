---
name: rust-errors
description: "Use when Rust code defines an error type, converts a std or foreign error, decides whether a failure is retryable, or maps a failure to an exit code. Covers one closed enum per boundary, a single conversion point, total classifiers, the one sanctioned catch-all for non-exhaustive enums, dispositions computed then interpreted, and the lints that ban panicking shortcuts. Keywords: Result, Error, Display, From, ?, thiserror, #[error], #[from], #[source], transparent, map_err, io::Error, io::ErrorKind, non_exhaustive, wildcard_enum_match_arm, Box<dyn Error>, unwrap, expect, panic, retry, should_retry, can_retry, code, disposition, ExitCode, let_underscore_must_use."
---

# Errors in Rust

## The rule

**Classify a foreign failure exactly once, at the boundary it crosses, into a closed enum whose classifiers are total.**

## The failure this prevents

Suppose an adapter that reads orders from a `TcpStream` routed failed reads with `io::ErrorKind::TimedOut => Retry, _ => Park`. On Unix, a read that runs into its `set_read_timeout` bound typically reports `WouldBlock`, not `TimedOut` — std's documentation says so — so every timed-out read would fall into `_` and be parked: retryable work, set aside for good. Nothing would crash, and the match is legal Rust. Here `make lint` already flags that `_` (`wildcard_enum_match_arm`), and an `#[expect]` would only silence it while `_` still decides `Park`; adding the missing arm would fix this kind and not the next one. The fix is a classifier that is total by construction: `WouldBlock | TimedOut` both become the adapter enum's `TimedOut`, and the one `_` left decides nothing — it builds `Unknown`, which is retried and counted under its own `code()` instead of being parked on its first failure.

## The rules that follow

- **One closed enum per dependency.** The journal's is `JournalError`: a variant per failure mode, `Display`, and `std::error::Error`. Another dependency gets its own enum, because a shared variant ends as a guess about who failed.
- **Derive the message, not the conversion.** `#[derive(thiserror::Error)]` writes `Display` and `Error` from an `#[error("…")]` on each variant. Never `#[from]`, `#[source]`, `#[error(transparent)]` or a field named `source`, so no error type converts into another and `source()` stays `None`; and never an `#[error(…)]` on the enum itself, which thiserror copies to every variant that has none, so a variant without its own message does not compile.
- **One conversion point.** `impl From<io::Error> for JournalError` lives in the journal's adapter module; only adapter modules name `io::ErrorKind`. The journal's methods, and the port traits it implements (`OrderReader`, `OrderWriter`), return `JournalError`, never `io::Error`.
- **The one catch-all.** `io::ErrorKind` is `#[non_exhaustive]` (so is `num::IntErrorKind`, met when parsing `Config`), so its match needs a `_`; that arm builds `Unknown` from `raw_os_error()` and carries no `io` type. Its `#[expect(clippy::wildcard_enum_match_arm, reason = "…")]` sits on `fn from` — on the arm itself the lint does not see it — and is the one sanctioned wildcard per boundary. Our own enums are never `#[non_exhaustive]`, so every other match names every variant.
- **Total classifiers.** `const fn can_retry(&self) -> bool` and `const fn code(&self) -> &'static str` match every variant, so a new variant does not compile until it is classified. A metric label, a log field or the code `main` reports comes from `code()`, never from `to_string()`: `Display` is prose and may change.
- **One retry predicate.** `should_retry(&RunError) -> bool` reads the classifiers, and `disposition` is its only caller; the interpreter matches `Disposition`, never an error variant.
- **Compute the disposition, then interpret it.** `disposition(&RunError, attempt, max_attempts) -> Disposition` is pure, its retry budget pinned by `main` ([[rust-wiring]]), and one interpreter acts on its answer: `Retry(delay)` when the next attempt may succeed, `Park` to set the order aside with its error for a person, `Abort` when no later attempt can.

```rust
use std::{io, time::Duration};

/// Every way the journal can fail, as the code above its adapter sees it.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum JournalError {
    #[error("the journal did not answer in time")]
    TimedOut,
    #[error("the journal has no space left")]
    Full,
    #[error("the journal failed for an unclassified reason")]
    Unknown { os_code: Option<i32> },
}

impl From<io::Error> for JournalError {
    /// Classifies an `io::Error` by kind; every kind not named here becomes `Unknown`.
    #[expect(clippy::wildcard_enum_match_arm, reason = "io::ErrorKind is non_exhaustive")]
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Self::TimedOut,
            io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => Self::Full,
            _ => Self::Unknown { os_code: error.raw_os_error() },
        }
    }
}

impl JournalError {
    pub const fn can_retry(&self) -> bool {
        match self {
            Self::TimedOut | Self::Unknown { .. } => true,
            Self::Full => false,
        }
    }

    /// The stable label for counters, logs and exit reports.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::TimedOut => "journal_timed_out",
            Self::Full => "journal_full",
            Self::Unknown { .. } => "journal_unknown",
        }
    }
}

/// Every missing or malformed setting, reported together.
#[derive(Debug)]
pub struct ConfigErrors(pub Vec<String>);

/// Why a run failed, keyed by the layer that failed.
#[derive(Debug)]
pub enum RunError {
    Config(ConfigErrors),
    Journal(JournalError),
}

/// What to do after a failed attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum Disposition {
    Retry(Duration),
    Park,
    Abort,
}

pub const fn should_retry(error: &RunError) -> bool {
    match error {
        RunError::Journal(journal) => journal.can_retry(),
        RunError::Config(_) => false,
    }
}

/// Pure and total: the same error and attempt always give the same disposition.
pub fn disposition(error: &RunError, attempt: u32, max_attempts: u32) -> Disposition {
    match error {
        RunError::Journal(_) if should_retry(error) && attempt < max_attempts => {
            Disposition::Retry(Duration::from_secs(2_u64.saturating_pow(attempt)))
        }
        RunError::Journal(_) => Disposition::Park,
        RunError::Config(_) => Disposition::Abort,
    }
}

let journal = |kind: io::ErrorKind| RunError::Journal(io::Error::from(kind).into());
let timed_out = journal(io::ErrorKind::TimedOut);
assert_eq!(disposition(&timed_out, 1, 5), Disposition::Retry(Duration::from_secs(2)));
assert_eq!(disposition(&timed_out, 5, 5), Disposition::Park);
assert_eq!(disposition(&journal(io::ErrorKind::StorageFull), 1, 5), Disposition::Park);
assert_eq!(disposition(&RunError::Config(ConfigErrors(Vec::new())), 1, 5), Disposition::Abort);
```

- **`main` maps the final error.** One total match over which layer failed turns the final `RunError` into an `ExitCode`, one arm per layer: `RunError::Config(_) => ExitCode::from(2)`, `RunError::Journal(_) => ExitCode::from(3)`, and an arm for each layer added later, such as the boot gate's `RunError::Boot`. `main` is also the only place a `Box<dyn Error>` may appear; below it, every signature names its error enum.
- **Conversions keep the layer and the cause.** `From` goes from a foreign error into its boundary enum, never between our own enums, where `?` would move a failure into another layer with no line to show it: wrap by hand with `.map_err(RunError::Journal)`. `map_err(|_| …)` drops the cause; map into a variant that carries what a reader needs.
- **A refusal is a result.** The domain saying no — an order over its `Quantity` limit — is its own enum in the domain module, returned as a value, never a variant of a dependency's error.
- **A panic is a bug, never an error.** `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `indexing_slicing` and `string_slice` reject the panicking shortcuts, and `let_underscore_must_use` and `unused_results` the silent discards; the Banned table in rust.md § 1 names the alternative to each.

## Where the depth is

- [rust.md § 1](../functional-architecture/references/rust.md) — Make illegal states unrepresentable: `Result` plus an enum, and the Banned table (call → lint → instead).
- [rust.md § 2](../functional-architecture/references/rust.md) — Total functions only: narrow the input or return a variant; `checked_` and `saturating_` arithmetic.
- [rust.md § 3](../functional-architecture/references/rust.md) — Decisions as an enum, then interpret: `disposition`, and the one catch-all std's `#[non_exhaustive]` enums force.

## Verify

```sh
make lint                                              # the partiality lints and wildcard_enum_match_arm
git grep -nw 'ErrorKind' backend/src libs              # lists adapter modules only
git grep -nE 'Box<dyn [a-z:]*Error' backend/src libs   # lists main.rs only
git grep -nE '#\[(from|source)\]|transparent|\bsource:' -- backend/src libs   # lists nothing
```

Related: [[functional-architecture]] · [[rust-effects]] · [[rust-wiring]] · [[rust-resources]]
