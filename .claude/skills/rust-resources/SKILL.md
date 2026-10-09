---
name: rust-resources
description: "Use when Rust code opens a file, socket, child process, thread or lock, writes a Drop impl, or holds a guard. Covers acquisition that returns the owner, explicit fallible release, drop order of locals and fields, named guards, joined threads, the leaks the borrow checker allows, and bounded input. Keywords: RAII, Drop, ownership, borrow, File, BufWriter, flush, into_inner, sync_all, TcpStream, Child, wait, MutexGuard, _guard, JoinHandle, join, thread::scope, mem::forget, Box::leak, Rc, Weak, process::exit, drop order, Read::take, with_capacity."
---

# Resources and lifetimes in Rust

## The rule

**Whatever is acquired is owned by one value, and a release that can fail is called explicitly on the success path; `Drop` is the backstop, never the only release.**

## The failure this prevents

Suppose the journal appended orders like this:

```rust,no_run
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// Appends one order line and returns `Ok(())` whether or not the line reached the file.
pub fn append(path: &str, line: &str) -> io::Result<()> {
    let mut journal = BufWriter::new(File::options().append(true).open(path)?);
    writeln!(journal, "{line}")?;
    Ok(())
}
```

The line waits in the buffer, and the real write happens when `journal` drops — but `BufWriter`'s `Drop` ignores any error it meets, as std's documentation warns. On a full disk the file would end short of lines the function reported as written, and every call would still return `Ok(())`. The tests would pass: their disks are never full. The fix is a consuming `close(self) -> Result<(), JournalError>` that calls `into_inner()`, which flushes and hands back the `File`, then `sync_all()`, which surfaces what the disk reports later, and returns the first failure. An order counts as written only once `close` has returned `Ok`.

## The rules that follow

- **Acquisition returns the owner.** `Journal::open(&config) -> Result<Journal, JournalError>`: the caller holds the handle or the reason, never a half-open value.
- **The borrow checker covers use-after-release.** A borrow cannot outlive its owner, and moving the owner moves the duty to release. The rest of this list is what it cannot check.
- **Fallible release is explicit.** A consuming `close(self)` runs each step that can fail — `BufWriter::into_inner`, `File::sync_all`, `Child::wait`, `JoinHandle::join` — and returns the first error, still running every release that does not depend on the failed one; after it, the handle no longer exists. A release that cannot fail, such as a `MutexGuard`'s, needs nothing but its `Drop`. `Child` has no `Drop` at all: a dropped child keeps running, and on Unix lingers as a zombie once it exits.

```rust
use std::fs::File;
use std::io::{self, BufWriter, IntoInnerError, Write};
use std::path::PathBuf;

/// Where the journal lives, as parsed from configuration.
#[derive(Debug)]
pub struct JournalConfig {
    pub path: PathBuf,
}

/// Every way the journal can fail, as the code above its adapter sees it.
#[derive(Debug, PartialEq, Eq)]
pub enum JournalError {
    TimedOut,
    Full,
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

/// An append-only file of order lines, owned until `close` consumes it.
#[derive(Debug)]
pub struct Journal {
    writer: BufWriter<File>,
}

impl Journal {
    pub fn open(config: &JournalConfig) -> Result<Self, JournalError> {
        let file = File::options().create(true).append(true).open(&config.path)?;
        Ok(Self { writer: BufWriter::new(file) })
    }

    /// Buffers one line; it is on disk only once `close` returns `Ok`.
    pub fn append(&mut self, line: &str) -> Result<(), JournalError> {
        writeln!(self.writer, "{line}").map_err(JournalError::from)
    }

    /// Flushes the buffer, then syncs the file's data and metadata; returns the first failure.
    pub fn close(self) -> Result<(), JournalError> {
        let file = self.writer.into_inner().map_err(IntoInnerError::into_error)?;
        file.sync_all().map_err(JournalError::from)
    }
}
```

- **`new` takes acquired values.** `OrderService::new(journal)` is infallible and does no I/O; a factory named for the act, such as `Journal::open`, does the acquiring.
- **Background work is a resource.** Prefer `thread::scope`. A thread that must outlive every scope is spawned only in `main.rs` and held by a guard whose `stop(self) -> Result<…>` sends the stop message and joins. A dropped `JoinHandle` detaches its thread: nobody waits for it, and nobody learns how it ended.
- **Drop order follows declaration.** Locals drop in reverse order of declaration; struct fields and tuple elements drop in declaration order. Order fields by release: `orders: Sender<Order>` sits above `worker`, a guard whose `Drop` joins the thread that drains `orders`, because that thread ends when the channel closes; the other way round, the join would wait on a thread that is waiting on the channel. The channel closes only when its last `Sender` drops, so `orders` must be the only one.
- **Name your guards.** A guard bound to a name holds the lock to the end of the scope — `guard`, or `_guard` when nothing reads through it — while `let _ = mutex.lock()` releases it on the same line, and rustc's deny-by-default `let_underscore_lock` rejects it. A guard created in a `match` scrutinee lives until the `match` ends. Keep the scope as small as the critical section.
- **The leaks the borrow checker allows.** `mem::forget` (the `mem_forget` lint), `Box::leak`, `Rc` cycles (hold the back edge as a `Weak`), and `process::exit`, which skips every `Drop` (the `exit` lint and `disallowed_methods`): `main` returns an `ExitCode` instead.
- **The root owns.** `main` owns every long-lived handle, directly or through the value it moved it into, and lends it out.
- **Bound the input.** Read through `Read::take` with one byte more than the limit, before `read_to_end` or `lines`, and refuse when that extra byte arrives (`read_to_end` counts it; `Take::limit()` reaches zero): `take(limit)` alone truncates without a word. Compute sizes with `checked_mul`, and size `Vec::with_capacity` only from a bounded number.

```rust
use std::io::{self, Read};

/// Reads at most `limit + 1` bytes; `None` means the input exceeds `limit` and is refused.
pub fn read_bounded(input: impl Read, limit: u64) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    let read = input.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    Ok(u64::try_from(read).is_ok_and(|len| len <= limit).then_some(bytes))
}

assert_eq!(read_bounded(&b"order"[..], 5).ok(), Some(Some(b"order".to_vec())));
assert_eq!(read_bounded(&b"orders"[..], 5).ok(), Some(None));
```

- **`Drop` never panics and never blocks without a bound.** A panic while unwinding aborts the process. A `Drop` that joins waits only for a thread already told to stop, as `worker` is once `orders` has dropped; waiting on anything else can hang the exit.

## Where the depth is

- [rust.md § 5](../functional-architecture/references/rust.md) — Effects are captured: `fn main() -> ExitCode`, because `process::exit` skips every pending destructor; statics hold data.
- [rust.md § 6](../functional-architecture/references/rust.md) — One concern per layer: lifetimes are `main` locals, and each I/O concern has a port and its own adapter module.

## Verify

```sh
make lint   # mem_forget, exit, unused_results, disallowed_methods; let_underscore_lock is deny-by-default in rustc
```

Then hold every change that acquires something to this list:

1. Acquisition returns the owner, or the reason it failed.
2. Every release that can fail is called on the success path.
3. No `new` does I/O; a factory named for the act acquires.
4. Every thread is scoped, or spawned in `main.rs` and joined.
5. Fields are declared in the order they must be released.
6. Every guard is bound to a name, never `_`.
7. No `mem::forget`, `Box::leak` or `process::exit`.
8. `main` owns the long-lived handles.
9. Every read from outside is bounded.

Related: [[functional-architecture]] · [[rust-effects]] · [[rust-errors]] · [[rust-wiring]]
