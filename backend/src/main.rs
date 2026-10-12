//! The composition root: reads the one argument, opens the input it names, runs the input over
//! in-memory stores, writes the accounts table to stdout and the report to stderr, and maps every
//! failure to an exit status.

use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use backend::csv_io::{CsvSource, SinkError, render_csv};
use backend::memory::{MemoryLedger, MemorySeenIds};
use backend::run::{RunError, run};
use backend::text_io::render_report;

/// Runs the input the arguments name. A failure's line goes to stderr, and its exit status is
/// returned whether or not that line could be written.
fn main() -> ExitCode {
    #[expect(clippy::disallowed_methods, reason = "the composition root reads the arguments once")]
    let arguments = std::env::args_os();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match execute(arguments, &mut stdout, &mut stderr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => match writeln!(stderr, "{failure}") {
            Ok(()) | Err(_) => failure.code(),
        },
    }
}

/// Opens the input the arguments name and runs it: the table to `stdout` once the run succeeds,
/// then the report to `stderr`.
fn execute(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), Failure> {
    let path = input(arguments).ok_or(Failure::Usage)?;
    let file = File::open(&path).map_err(|error| Failure::Open { path, error })?;
    let summary =
        run(CsvSource::new(file), &mut MemoryLedger::default(), &mut MemorySeenIds::default())
            .map_err(Failure::Run)?;
    render_csv(&summary.accounts, stdout).map_err(Failure::Table)?;
    render_report(&summary.report, stderr).map_err(Failure::Report)
}

/// The input's path: the one argument after the program's name; `None` for any other count.
fn input(arguments: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut given = arguments.into_iter().skip(1);
    match (given.next(), given.next()) {
        (Some(path), None) => Some(PathBuf::from(path)),
        (None, _) | (Some(_), Some(_)) => None,
    }
}

/// Why the program failed, by the step that failed.
#[derive(Debug)]
enum Failure {
    /// Not exactly one argument.
    Usage,
    /// The input could not be opened.
    Open { path: PathBuf, error: io::Error },
    /// The run stopped.
    Run(RunError<Infallible, Infallible>),
    /// The accounts table could not be written.
    Table(SinkError),
    /// The report could not be written.
    Report(io::Error),
}

impl Failure {
    /// The exit status of the step that failed.
    fn code(&self) -> ExitCode {
        ExitCode::from(match self {
            Self::Usage => 2,
            Self::Open { .. } => 3,
            Self::Run(_) => 4,
            Self::Table(_) | Self::Report(_) => 5,
        })
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => f.write_str("usage: cthulhu <input.csv>"),
            Self::Open { path, error } => {
                write!(f, "error: cannot open {}: {error}", path.display())
            }
            Self::Run(error) => write!(f, "error: {error}"),
            Self::Table(error) => write!(f, "error: {error}"),
            Self::Report(error) => write!(f, "error: cannot write the report: {error}"),
        }
    }
}
