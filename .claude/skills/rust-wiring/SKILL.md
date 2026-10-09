---
name: rust-wiring
description: "Use when assembling the binary — writing main, constructing adapters, parsing configuration, or adding a port and its implementations. Covers the single composition root in main.rs, factory parameters as seams, conditional construction with a fail-fast gate, decorators in one line, narrow role traits, configuration parsed once from an environment snapshot with every problem reported, and pinning defaults at the root. Keywords: main, composition root, ExitCode, run, Config, FromStr, env::vars_os, set_var, unsafe, port, trait, adapter, Live, Noop, Metered, decorator, factory, generic, impl Trait, Box<dyn Trait>, timeout, set_read_timeout, connect_timeout, default, validation, mod."
---

# Rust wiring

## The rule

**There is exactly one composition root, `main.rs`, and it is the only place a concrete adapter, the clock, the environment or stdout is touched; everything it builds is passed down as a value.**

A constructor called anywhere else is a seam no test can reach and a choice no reviewer sees.

## The failure this prevents

Picture an adapter that sends each `Order` to a peer over a `std::net::TcpStream` and waits
for the reply with `read_line`. It never calls `set_read_timeout`, so the stream keeps the std
default of `None`, and the std docs say what `None` means: reads block indefinitely. A peer
that accepts the connection and then goes silent parks the worker reading from it for good;
under load, every worker reaches that peer in turn and parks the same way. Nothing panics and
no error comes back, so the retry budget never comes into play. Each piece behaves as
documented; the outage is a default nobody chose.

The fix is one line at the root. The adapter's constructor takes its bounds as parameters, so
it cannot be built without a decision; `main` passes them next to the retry budget, where a
reviewer can check that the attempts times the timeout fit the time an order may wait.

```rust
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Connects with both bounds chosen by the caller; neither falls back to a default.
fn dial(to: SocketAddr, connect_limit: Duration, read_limit: Duration) -> io::Result<TcpStream> {
    let stream = TcpStream::connect_timeout(&to, connect_limit)?;
    stream.set_read_timeout(Some(read_limit))?; // a zero Duration is an Err here, not "no timeout"
    Ok(stream)
}
```

**A default you did not pin at the root is a decision someone else made.**

## The rules that follow

- **Seams are function-typed parameters.** The library's `run` takes factories such as
  `F: FnOnce(&JournalConfig) -> Result<J, JournalError>`. `main` passes `Journal::open`, or a
  closure that decorates it; a test passes a closure that returns a double. The test runs the
  same `run` through the same gate, so it proves the wiring, not a copy of it. There is no
  container and no registry: a seam is a generic parameter.
- **`Option` means "not provisioned", and a gate runs before any work.** An adapter is built
  only when the configuration asks for it, and the field holding it is an `Option<Journal>`
  whose doc comment says exactly when it is `Some`. Then `gate(&Config, &Provisioned)` returns
  `Result<(), BootError>` before the first order is read, so a feature that needs a missing
  adapter is a boot failure with a sentence, not a `None` met halfway through a run.
- **Decorators add one concern each and compose in one line at the root.**
  `Metered::new(Retrying::new(journal, budget))`: what is retried and what is counted
  is a line a reviewer reads. The live adapter never grows an `if` for either, and a dry run is
  the same line with a `Noop` where the live adapter was. Bounded concurrency is a decorator
  too, never a flag inside the adapter.
- **Ports are narrow role traits, declared beside their consumer.** `OrderReader` and
  `OrderWriter` over one `Journal`; each function takes only the role it uses
  (`&impl OrderReader`), so its test double fakes only the methods it calls.
- **Library code is generic over its ports and never names an adapter.** Generic parameters
  and `impl Trait` in the core; `Box<dyn OrderWriter>` only where `main` picks an
  implementation at runtime. A core module that imports `Journal`, `File` or `TcpStream` has
  moved the boundary.
- **Configuration is one typed loader over a snapshot.** `Config::from_env(vars)` takes
  `impl IntoIterator<Item = (OsString, OsString)>` and returns `Result<Config, ConfigErrors>`;
  nothing calls `expect` on a variable. `ConfigErrors` names every missing or malformed
  variable at once, so one failed boot is one fix, not a fix-and-rerun loop.
- **The environment is read once, in `main`.** Edition 2024 makes `std::env::set_var`
  `unsafe`, and `unsafe_code` is forbidden here, so no test can set a variable: configuration
  is testable only as a function of a snapshot. `main` reads `std::env::vars_os()` once, under
  `#[expect(clippy::disallowed_methods, reason = "…")]`, and passes the snapshot on. The live
  `Clock` sits in `main.rs` too, behind the same kind of opt-out.
- **Pin every timeout, buffer size and retry budget at the root.** An adapter's constructor
  takes each bound as a parameter, and configuration structs carry no `Default`, so nothing
  runs on a value nobody chose. Bounds that must agree sit on adjacent lines.
- **A file exists only once a `mod` line reaches it.** A `.rs` file under `src/` that no `mod`
  declares is not compiled, formatted or linted, so `make ci` stays green over code that would
  not build. Add the `mod` line in the same change as the file.
- **`main() -> ExitCode` maps the run's error totally.** An exhaustive `match` from `RunError`
  to an exit code, one arm per layer, never `process::exit` and never a panic; the label it
  reports comes from `code()` ([[rust-errors]]).

The root, whole: `main` reads the world once, `boot` composes, and a test calls the same `run`.

```rust,ignore
// backend/src/main.rs: the composition root, and the only file that constructs `Journal`.
fn main() -> ExitCode {
    #[expect(clippy::disallowed_methods, reason = "main takes the one environment snapshot")]
    let vars = std::env::vars_os();
    match boot(vars) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error, &mut io::stderr().lock()), // writes it, maps it to a code
    }
}

fn boot(vars: VarsOs) -> Result<(), RunError> {
    let config = Config::from_env(vars).map_err(RunError::Config)?; // every bad variable at once
    let open_journal = |journal: &JournalConfig| {
        Journal::open(journal).map(|live| Metered::new(Retrying::new(live, config.attempts)))
    };
    backend::run(&config, open_journal)
}

// backend/src/lib.rs: generic over the port; the factory runs only when a journal is configured.
pub fn run<J, F>(config: &Config, open_journal: F) -> Result<(), RunError>
where
    J: OrderWriter,
    F: FnOnce(&JournalConfig) -> Result<J, JournalError>,
{
    let journal = config.journal.as_ref().map(open_journal).transpose().map_err(RunError::Journal)?;
    let provisioned = Provisioned { journal }; // None: this configuration has no journal
    gate(config, &provisioned).map_err(RunError::Boot)?; // before the first order is read
    // … the work, over `provisioned`
}
```

Setting a variable from a test does not compile, which is why the snapshot is the only way in:

```rust,compile_fail
// Edition 2024: `set_var` is an unsafe fn, and the workspace forbids `unsafe` blocks.
std::env::set_var("CTHULHU_JOURNAL", "orders.journal");
```

## Where the depth is

- [rust.md § 4](../functional-architecture/references/rust.md) — Ports and interpreters
- [rust.md § 5](../functional-architecture/references/rust.md) — Effects are captured
- [rust.md § 6](../functional-architecture/references/rust.md) — One concern per layer

## Verify

```sh
make lint                                    # the disallowed-methods gate holds everywhere else
git grep -n 'disallowed_methods' backend/src # lists backend/src/main.rs and nothing else
```

A boot test calls `run` with a closure for `Journal::open` and a configuration that needs a
journal but names none: it must stop at the gate with a `BootError`. A configuration test hands
`Config::from_env` an array holding two bad variables: `ConfigErrors` must name both.

Related: [[rust-errors]] · [[rust-effects]] · [[rust-resources]] · [[functional-architecture]]
