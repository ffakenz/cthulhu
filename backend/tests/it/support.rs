//! Property-test support: a seeded generator, and a runner that reports the case breaking a law.

pub(crate) mod generate;
pub(crate) mod model;

use std::ffi::OsString;
use std::fmt::{self, Debug};
use std::num::NonZeroU64;
use std::panic::{self, AssertUnwindSafe};
use std::time::{SystemTime, UNIX_EPOCH};

/// How often a generator takes an edge case: one draw in four.
pub(crate) const EDGE_ODDS: NonZeroU64 = NonZeroU64::MIN.saturating_add(3);

/// A `SplitMix64` generator: the same seed gives the same draws, in the same order. Only
/// `for_all` builds one, so every draw comes from the seed a failure prints. `Clone` but not
/// `Copy`, so a generator is never duplicated by accident.
#[derive(Debug, Clone)]
pub(crate) struct Gen(u64);

impl Gen {
    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`.
    pub(crate) fn below(&mut self, bound: NonZeroU64) -> u64 {
        self.next_u64() % bound
    }

    /// `true` once in `n` draws, on average.
    pub(crate) fn one_in(&mut self, n: NonZeroU64) -> bool {
        self.below(n) == 0
    }

    /// One of `items`, or `None` when there are none.
    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let len = NonZeroU64::new(u64::try_from(items.len()).ok()?)?;
        items.get(usize::try_from(self.below(len)).ok()?)
    }

    /// Up to `max_len` items picked from `items`; each length in `0..=max_len` is equally likely.
    pub(crate) fn up_to<T: Clone>(&mut self, max_len: u64, items: &[T]) -> Vec<T> {
        let len = self.below(NonZeroU64::MIN.saturating_add(max_len));
        (0..len).filter_map(|_| self.pick(items).cloned()).collect()
    }
}

/// A run's seed: every draw of the run follows from it. Written as `0x` and sixteen hex digits;
/// read from hex digits, with or without `0x`, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Seed(u64);

impl Seed {
    /// The seed `text` spells, or `None` when it is not a hex `u64` after an optional `0x`.
    fn read(text: &str) -> Option<Self> {
        let digits = text.strip_prefix("0x").unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        u64::from_str_radix(digits, 16).ok().map(Self)
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

/// Why a property failed.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The law returned `false`, or panicked, on the input generated for this case.
    Counterexample { law: &'static str, case: u32, input: String, seed: Seed },
    /// `PROPERTY_SEED` is set but is not a hex `u64`, so the run it names cannot be replayed.
    UnreadableSeed(OsString),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Counterexample { law, case, input, seed } => {
                write!(
                    f,
                    "law `{law}` failed at case {case} with {input}; replay: PROPERTY_SEED={seed}"
                )
            }
            Self::UnreadableSeed(raw) => {
                let raw = raw.display();
                write!(
                    f,
                    "PROPERTY_SEED={raw} is not a hex u64; unset it, or copy one a failure printed"
                )
            }
        }
    }
}

/// Checks `holds` on `cases` inputs drawn by `generate`; the first input on which it returns
/// `false` or panics is the counterexample.
pub(crate) fn for_all<T: Debug>(
    law: &'static str,
    cases: u32,
    mut generate: impl FnMut(&mut Gen) -> T,
    holds: impl Fn(&T) -> bool,
) -> Result<(), Failure> {
    let seed = seed()?;
    let mut random = Gen(seed.0);
    for case in 1..=cases {
        let input = generate(&mut random);
        // `AssertUnwindSafe` holds: after a panic, `input` is only printed, never used again.
        #[expect(
            clippy::disallowed_methods,
            reason = "a panic in the code under test becomes a counterexample with its seed"
        )]
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| holds(&input)));
        if !matches!(outcome, Ok(true)) {
            return Err(Failure::Counterexample { law, case, input: format!("{input:?}"), seed });
        }
    }
    Ok(())
}

/// The run's seed: `PROPERTY_SEED` (hex, with or without `0x`) when set, otherwise fresh from
/// the clock.
#[expect(
    clippy::disallowed_methods,
    reason = "the harness edge draws a fresh seed per run; tests never read the environment or the clock elsewhere"
)]
fn seed() -> Result<Seed, Failure> {
    let Some(pinned) = std::env::var_os("PROPERTY_SEED") else {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let nanos = elapsed.as_secs().wrapping_mul(1_000_000_000);
        return Ok(Seed(nanos.wrapping_add(u64::from(elapsed.subsec_nanos()))));
    };
    match pinned.to_str().and_then(Seed::read) {
        Some(seed) => Ok(seed),
        None => Err(Failure::UnreadableSeed(pinned)),
    }
}
