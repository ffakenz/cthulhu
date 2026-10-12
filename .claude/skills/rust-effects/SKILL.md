---
name: rust-effects
description: "Use when Rust code reads the clock, the environment or arguments, starts a thread, sleeps, writes to stdout or stderr, or keeps a static — anything that makes two runs differ. Covers the side-effect catalogue, time and configuration as values, main as the only effectful site, scoped threads, statics that hold data, deterministic iteration, and tests that never wait. Keywords: side effect, pure function, referential transparency, SystemTime::now, Instant::now, env::var, env::vars_os, env::args, LazyLock, OnceLock, static, thread::spawn, thread::scope, thread::sleep, JoinHandle, mpsc, Mutex, Clock, HashMap iteration order, BTreeMap, ExitCode, println, writeln, disallowed_methods."
---

# Effects in Rust

## The rule

**Every side effect and every source of non-determinism enters as a value or through a port, and only `main` touches the real thing.**

## The failure this prevents

Suppose an order event took its timestamp from a lazily initialised static:

```rust,no_run
use std::sync::LazyLock;
use std::time::SystemTime;

/// One instant per process, read from the clock by whichever access comes first.
static STARTED_AT: LazyLock<SystemTime> = LazyLock::new(SystemTime::now);
```

The event constructor sets `placed_at: *STARTED_AT`, so every order the process records would carry the instant of the first access — hours or days stale. The tests would pass: each one runs moments after that first access, so a "stamped just now" check holds. Nothing would crash, and the code is legal Rust. `LazyLock` runs its initialiser once, at a first touch nobody chose; a `now` parameter makes *when* explicit. This repository's gate rejects that exact line — `disallowed-methods` flags `SystemTime::now` even when it is passed as a value — but the same static fed by a file read has no lint. The rule forbids both.

## The rules that follow

- **The catalogue.** Each of these is a side effect or a source of non-determinism: the clock (`SystemTime::now`, `Instant::now`, and `elapsed()`, which reads it); the environment and arguments (`env::var`, `env::vars_os`, `env::args`, `env::current_dir`); the file system, the network and child processes; starting a thread or sleeping; stdin, stdout and stderr; `HashMap` and `HashSet` iteration order; a static initialised from any of these; interior mutability (`Cell`, `RefCell`, `Mutex`, atomics) that escapes its function. A local `let mut` that nothing outside sees is fine.
- **Three layers.** Pure functions decide; a port — a trait declared beside its consumer — carries each effect; a test double implements the port in the test crate, and the live adapter that performs the effect is built only in `main`, which hands it down. Library code is generic over its ports and reaches the real thing only through what `main` passes in.
- **Time is a parameter.** A decision takes `now`; an interpreter that has to read time takes a `Clock` port. `opt.unwrap_or(SystemTime::now())` reads the clock even when `opt` is `Some`, because arguments are evaluated first: `unwrap_or_else` defers the read, and a `now` parameter removes it.

```rust
use std::time::{Duration, SystemTime};

/// The time source an interpreter reads; `main` passes the system clock, a test a fixed one.
pub trait Clock {
    fn now(&self) -> SystemTime;
}

/// What the worker does next with an order due at `due_at`.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Accept,
    Defer(Duration),
}

/// Pure: the same two instants always give the same decision.
pub fn decide(due_at: SystemTime, now: SystemTime) -> Decision {
    match due_at.duration_since(now) {
        Ok(left) if !left.is_zero() => Decision::Defer(left),
        Ok(_) | Err(_) => Decision::Accept,
    }
}

let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
let due = now + Duration::from_secs(20);
assert_eq!(decide(due, now), Decision::Defer(Duration::from_secs(20)));
assert_eq!(decide(due, due), Decision::Accept);
```

- **Smallest capability.** A function that needs the time takes the time, not the struct of every dependency. `&` before `&mut` before owned.
- **Configuration is a value.** `main` snapshots `env::vars_os()` and `env::args_os()` once and parses them into a `Config` ([[rust-wiring]]); code below takes the field it needs. `env::set_var` is `unsafe` in edition 2024 and `unsafe_code` is forbidden, so a test passes its own snapshot instead.
- **Statics hold data.** A `const`, or a `static` whose value is fixed at compile time. A `LazyLock` or `OnceLock` may cache a pure computation, never an effect's result. No global `Mutex`: shared state is a value `main` builds and lends.
- **The gate.** `disallowed-methods` in `clippy.toml` lists the direct calls: `SystemTime::now`, `Instant::now` and `elapsed()` on either, `env::var`, `env::vars` and `env::args` with their `_os` forms, `env::current_dir`, `env::temp_dir`, `thread::spawn`, `thread::Builder::spawn`, `thread::sleep`, `process::exit` and `panic::catch_unwind`, and clap's calls that read the process's arguments, exit it or write to its streams. `main.rs` is the only file under `app/src` or `libs/` that opts out, each `#[expect(clippy::disallowed_methods, reason = "…")]` on the narrowest item: the environment snapshot in `main`, the live `Clock`'s `now`. Every file, network and process call has no lint: review keeps them in adapters and `main.rs`.
- **Structured concurrency.** `thread::scope` over `thread::spawn`: every thread ends before the scope returns. Channels (`mpsc`) over `Arc<Mutex<_>>`. Join each scoped handle: `join()` hands back the thread's panic as an `Err`, where an unjoined panic makes `scope` itself panic, without the original payload. `Mutex::lock` returns a `LockResult` — poisoning is a value: map it to a variant, or recover with `PoisonError::into_inner` once the data is re-checked; never `unwrap` it.
- **Cancellation is a message.** A std thread cannot be stopped from outside. The worker checks a channel, and `recv_timeout(delay)` is both its wait and its check; dropping the last `Sender` stops it too. The delay comes from a decision — the worker only waits it out.

```rust
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// Runs `work`, then waits the delay it returned, until a stop message arrives or every
/// sender is dropped. Returns how many times `work` ran.
pub fn run_until_stopped(stop: &Receiver<()>, mut work: impl FnMut() -> Duration) -> u32 {
    let mut runs: u32 = 0;
    loop {
        let delay = work();
        runs = runs.saturating_add(1);
        match stop.recv_timeout(delay) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => return runs,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

let (sender, receiver) = mpsc::channel();
let runs = thread::scope(|scope| {
    let worker = scope.spawn(move || run_until_stopped(&receiver, || Duration::from_secs(60)));
    drop(sender); // the last sender is gone: `recv_timeout` returns `Disconnected` at once
    worker.join()
});
assert_eq!(runs.ok(), Some(1));
```

- **Timeouts are pinned at the root.** A blocking read with no bound can wait forever; `main` chooses every timeout and the adapter applies it ([[rust-wiring]]).
- **Order is observable.** `HashMap` and `HashSet` iteration order is unspecified: it differs between processes, and between two maps with the same contents. When the order reaches output, an assertion or a hash, iterate a `BTreeMap` or `BTreeSet`, or sort first.
- **Tests never wait.** They pass `now` values and assert on the returned delay, as above. A test that sleeps is a test of the scheduler.
- **Output is an effect.** Write with `writeln!` to an injected `out: &mut impl Write`; `main` passes `&mut io::stdout().lock()`, a test passes `&mut Vec::new()`. `print!`, `println!`, `eprint!`, `eprintln!` and `dbg!` fail the gate.

## Where the depth is

- [rust.md § 5](../functional-architecture/references/rust.md) — Effects are captured: `now` as a parameter, the `Clock` port, `Config` from an environment snapshot, statics fixed at compile time, `main() -> ExitCode`.
- [rust.md § 6](../functional-architecture/references/rust.md) — One concern per layer: which module may touch which effect.

## Verify

```sh
make lint   # disallowed_methods (tests included), print_stdout, print_stderr, dbg_macro, unused_results
make test   # decisions run with fixed instants
```

Related: [[functional-architecture]] · [[rust-wiring]] · [[rust-errors]] · [[rust-resources]] · [[rust-properties]]
