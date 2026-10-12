//! The composition root: parses the arguments, opens the input they name, runs the input over
//! in-memory stores, writes the accounts table to stdout and the report to stderr, and maps every
//! failure to an exit status; asked for the help or the version, it writes that to stdout instead.

use std::convert::Infallible;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use backend::csv_io::{CsvSource, SinkError, render_csv};
use backend::text_io::render_report;
use clap::Parser;
use engine::run::{RunError, run};
use store::memory::{MemoryLedger, MemorySeenIds};

/// Runs the input the arguments name, or writes the help or the version. A failure's message goes
/// to stderr, and its exit status is returned whether or not that message could be written.
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

/// The arguments: the input's path, beside the help and version flags clap adds.
#[derive(Debug, Parser)]
#[command(
    name = "cthulhu",
    bin_name = "cthulhu",
    version,
    about = "Reads client and dispute commands from a CSV file and prints the accounts table"
)]
struct Arguments {
    /// The CSV file of commands to read.
    #[arg(value_name = "input.csv")]
    input: PathBuf,
}

/// Parses the arguments, then opens the input they name and runs it: the table to `stdout` once
/// the run succeeds, then the report to `stderr`. Asked for the help or the version, writes that
/// to `stdout` and opens nothing.
fn execute(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), Failure> {
    let path = match Arguments::try_parse_from(arguments) {
        Ok(Arguments { input }) => input,
        Err(usage) if usage.use_stderr() => return Err(Failure::Usage(usage)),
        Err(asked) => {
            return write!(stdout, "{asked}").and_then(|()| stdout.flush()).map_err(Failure::Help);
        }
    };
    let file = File::open(&path).map_err(|error| Failure::Open { path, error })?;
    let summary =
        run(CsvSource::new(file), &mut MemoryLedger::default(), &mut MemorySeenIds::default())
            .map_err(Failure::Run)?;
    render_csv(&summary.accounts, stdout).map_err(Failure::Table)?;
    render_report(&summary.report, stderr).map_err(Failure::Report)
}

/// Why the program failed, by the step that failed.
#[derive(Debug, thiserror::Error)]
enum Failure {
    /// The arguments are not one input path: clap's message, less the newline it ends with, which
    /// `main` writes.
    #[error("{}", .0.to_string().trim_end())]
    Usage(clap::Error),
    /// The help or the version could not be written.
    #[error("error: cannot write the help or the version: {0}")]
    Help(io::Error),
    /// The input could not be opened.
    #[error("error: cannot open {}: {error}", .path.display())]
    Open { path: PathBuf, error: io::Error },
    /// The run stopped.
    #[error("error: {0}")]
    Run(RunError<Infallible, Infallible>),
    /// The accounts table could not be written.
    #[error("error: {0}")]
    Table(SinkError),
    /// The report could not be written.
    #[error("error: cannot write the report: {0}")]
    Report(io::Error),
}

impl Failure {
    /// The exit status of the step that failed.
    fn code(&self) -> ExitCode {
        ExitCode::from(match self {
            Self::Usage(_) => 2,
            Self::Open { .. } => 3,
            Self::Run(_) => 4,
            Self::Help(_) | Self::Table(_) | Self::Report(_) => 5,
        })
    }
}
