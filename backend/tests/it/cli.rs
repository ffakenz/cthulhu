//! The command-line contract through the binary: one argument, the table on stdout only once the
//! run succeeds, the counts on stderr, and an exit status for every failure.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use backend::csv_io::SinkError;
use backend::run::SourceError;

/// The overview, whose worked example is the contract's own.
const OVERVIEW: &str = include_str!("../../../docs/overview.md");

/// The path of the sample input, `transactions.csv` at the repository root.
const SAMPLE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../transactions.csv");

/// The table for the sample input.
const SAMPLE_TABLE: &str = "\
client,available,held,total,locked
1,50.0001,0.0000,50.0001,false
2,40.0000,0.0000,40.0000,false
3,-75.5000,0.0000,-75.5000,true
4,10.0000,5.5000,15.5000,false
";

/// The report for the sample input.
const SAMPLE_REPORT: &str = "\
1 rejected: the account is locked
1 rejected: the amount is above available
2 rejected: no such deposit for this client
1 rejected: the deposit is not disputed
";

/// Every rejection reason but overflow, and every malformed kind, a malformed row first.
const REJECTED_AND_MALFORMED: &str = "\
type, client, tx, amount
deposit, 3, 13, 0
deposit, 1, 1, 10.0
deposit, 1, 1, 5.0
withdrawal, 1, 2, 20.0
dispute, 1, 1,
dispute, 1, 1,
resolve, 1, 3,
resolve, 1, 1,
chargeback, 1, 1,
deposit, 2, 4, 1.5
dispute, 2, 4,
chargeback, 2, 4,
deposit, 2, 5, 1.0
Deposit, 3, 6, 1.0
deposit, 70000, 7, 1.0
deposit, 3, 8, 1.0, extra
withdrawal, 3, 9,
deposit, 3, 10, abc
deposit, 3, 11, 1.23456
deposit, 3, 12, 1000000000000000
";

/// The table for `REJECTED_AND_MALFORMED`: client 3, named only by malformed rows, has no account.
const REJECTED_AND_MALFORMED_TABLE: &str = "\
client,available,held,total,locked
1,10.0000,0.0000,10.0000,false
2,0.0000,0.0000,0.0000,true
";

/// The report for `REJECTED_AND_MALFORMED`.
const REJECTED_AND_MALFORMED_REPORT: &str = "\
1 rejected: the tx id was used before
1 rejected: the account is locked
1 rejected: the amount is above available
1 rejected: no such deposit for this client
1 rejected: the deposit is already disputed
1 rejected: the deposit is not disputed
2 malformed: a field does not read
1 malformed: more fields than the header
1 malformed: no amount
1 malformed: invalid amount: not digits with an optional fraction
1 malformed: invalid amount: more than four decimal places
1 malformed: invalid amount: 10^15 or more
1 malformed: invalid amount: zero
";

/// A header and no rows.
const HEADER_ONLY: &str = "type,client,tx,amount\n";

/// An empty input, which has no header.
const EMPTY: &str = "";

/// A header without `tx`, then a row.
const NO_TX: &str = "type, client, amount\ndeposit, 1, 1.0\n";

/// One deposit.
const ONE_DEPOSIT: &str = "type,client,tx,amount\ndeposit,1,1,1.0\n";

/// Runs the binary with `arguments`.
fn cthulhu(arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).output()
}

/// Runs the binary with `arguments`, its stdout written to `stdout`.
fn cthulhu_into(stdout: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stdout(stdout).output()
}

/// Runs the binary with `arguments`, its stderr written to `stderr`.
fn cthulhu_reporting_into(stderr: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stderr(stderr).output()
}

/// The writing end of a pipe whose reading end is already closed: every write to it fails.
fn closed_pipe() -> io::Result<io::PipeWriter> {
    let (reader, writer) = io::pipe()?;
    drop(reader);
    Ok(writer)
}

/// These tests' scratch directory, created if missing.
fn scratch() -> io::Result<PathBuf> {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

/// A file named `name` in the scratch directory, holding `text`.
fn input(name: &str, text: &str) -> io::Result<PathBuf> {
    let path = scratch()?.join(name);
    fs::write(&path, text)?;
    Ok(path)
}

/// The code block after the line `label` in the overview.
fn block(label: &str) -> Option<&'static str> {
    let (_, after) = OVERVIEW.split_once(&format!("{label}\n\n```text\n"))?;
    after.split_once("```").map(|(block, _)| block)
}

#[test]
fn the_overview_example_prints_its_table_and_report() {
    let (Some(text), Some(table), Some(report)) =
        (block("Input:"), block("Output:"), block("Report:"))
    else {
        panic!("docs/overview.md holds the worked example's input, output and report");
    };
    let path = input("overview.csv", text).expect("the scratch directory takes a file");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), table);
    assert_eq!(String::from_utf8_lossy(&output.stderr), report);
}

#[test]
fn the_sample_input_prints_its_table_and_report() {
    let output = cthulhu(&[Path::new(SAMPLE_PATH)]).expect("the binary runs");
    assert_eq!(String::from_utf8_lossy(&output.stderr), SAMPLE_REPORT);
    assert_eq!(String::from_utf8_lossy(&output.stdout), SAMPLE_TABLE);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn every_rejection_and_malformed_kind_is_counted_on_stderr() {
    let path = input("rejected-and-malformed.csv", REJECTED_AND_MALFORMED)
        .expect("the scratch directory takes a file");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stderr), REJECTED_AND_MALFORMED_REPORT);
    assert_eq!(String::from_utf8_lossy(&output.stdout), REJECTED_AND_MALFORMED_TABLE);
}

#[test]
fn a_wrong_argument_count_is_a_usage_error() {
    let path = input("usage.csv", HEADER_ONLY).expect("the scratch directory takes a file");
    for arguments in [&[][..], &[path.as_path(), path.as_path()][..]] {
        let output = cthulhu(arguments).expect("the binary runs");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "usage: cthulhu <input.csv>\n");
    }
}

#[test]
fn a_missing_input_cannot_be_opened() {
    let path = scratch().expect("the scratch directory exists").join("missing.csv");
    let output = cthulhu(&[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with(&format!("error: cannot open {}: ", path.display())), "{stderr}");
}

#[test]
fn an_unreadable_input_stops_the_run_with_no_table() {
    let directory = scratch().expect("the scratch directory exists");
    let output = cthulhu(&[&directory]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("error: {}\n", SourceError::Unreadable)
    );
}

#[test]
fn an_input_without_a_complete_header_stops_the_run_with_no_table() {
    for (name, text) in [("empty.csv", EMPTY), ("no-tx.csv", NO_TX)] {
        let path = input(name, text).expect("the scratch directory takes a file");
        let output = cthulhu(&[&path]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("error: {}\n", SourceError::IncompleteHeader)
        );
    }
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "/dev/full exists only on Linux")]
fn a_full_output_is_a_write_failure() {
    let path = input("full.csv", ONE_DEPOSIT).expect("the scratch directory takes a file");
    let full = fs::File::options().write(true).open("/dev/full").expect("/dev/full opens");
    let output = cthulhu_into(full, &[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("error: {}\n", SinkError::Unwritable)
    );
}

#[test]
fn a_report_that_cannot_be_written_is_a_write_failure() {
    let path = input("closed-stderr.csv", REJECTED_AND_MALFORMED)
        .expect("the scratch directory takes a file");
    let closed = closed_pipe().expect("a pipe opens");
    let output = cthulhu_reporting_into(closed, &[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(String::from_utf8_lossy(&output.stdout), REJECTED_AND_MALFORMED_TABLE);
}
