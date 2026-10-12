//! Property-test support: a runner that checks a law on the inputs a strategy draws, and reports
//! the smallest input it found that breaks the law; and `run` over CSV text and fresh in-memory
//! stores.

use std::convert::Infallible;
use std::fmt;
use std::panic::Location;

use app::csv_io::CsvSource;
use engine::run::{RunError, Summary, run};
use proptest::strategy::Strategy;
use proptest::test_runner::{Config, TestCaseError, TestError, TestRunner, contextualize_config};
use store::memory::{MemoryLedger, MemorySeenIds};

/// How many inputs each law is checked on, unless `PROPTEST_CASES` says otherwise.
const CASES: u32 = 500;

/// Why a law failed.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The law returned `false`, or panicked, on `input`: the smallest failing input proptest
    /// found, written with `Debug`, and why it failed.
    Counterexample { law: &'static str, input: String, reason: String },
    /// proptest stopped before it could check the law.
    Aborted { law: &'static str, reason: String },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Counterexample { law, input, reason } => {
                write!(f, "law `{law}` failed with {input}: {reason}")
            }
            Self::Aborted { law, reason } => write!(f, "law `{law}` was not checked: {reason}"),
        }
    }
}

/// Checks `holds` on `CASES` inputs drawn by `strategy`. The first input on which it returns
/// `false` or panics is shrunk until no simpler input still fails, and that is the
/// counterexample; proptest saves its seed under `proptest-regressions/`, in a file named for the
/// caller's, and replays it first on the next run.
#[track_caller]
pub(crate) fn for_all<S: Strategy>(
    law: &'static str,
    strategy: S,
    holds: impl Fn(&S::Value) -> bool,
) -> Result<(), Failure> {
    let config =
        Config { cases: CASES, source_file: Some(Location::caller().file()), ..Config::default() };
    let mut runner = TestRunner::new(contextualize_config(config));
    let outcome = runner.run(&strategy, |input| {
        if holds(&input) { Ok(()) } else { Err(TestCaseError::fail("the law returned false")) }
    });
    outcome.map_err(|error| match error {
        TestError::Fail(reason, input) => Failure::Counterexample {
            law,
            input: format!("{input:?}"),
            reason: reason.message().to_owned(),
        },
        TestError::Abort(reason) => Failure::Aborted { law, reason: reason.message().to_owned() },
    })
}

/// What `run` makes of `text` through the CSV source, over fresh in-memory stores.
pub(crate) fn summary(text: &[u8]) -> Result<Summary, RunError<Infallible, Infallible>> {
    run(CsvSource::new(text), &mut MemoryLedger::default(), &mut MemorySeenIds::default())
}
