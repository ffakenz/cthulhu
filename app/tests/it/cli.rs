//! The command-line contract through the binary: one input path, or the help or the version on
//! stdout; the table on stdout only once the run succeeds, the counts on stderr, and an exit status
//! for every failure.

use std::fs;
use std::io;
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

/// Every rejection reason but overflow, and every malformed kind, a malformed row first. The locked
/// client 2 then repeats a tx id and withdraws more than it has: each counts under the first check
/// it fails.
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
deposit, 2, 4, 1.0
withdrawal, 2, 14, 5.0
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
2 rejected: the tx id was used before
2 rejected: the account is locked
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
const EMPTY: &[u8] = b"";

/// A header without `tx`, then a row.
const NO_TX: &[u8] = b"type, client, amount\ndeposit, 1, 1.0\n";

/// A header whose first name holds a byte that is not UTF-8, then a row.
const NOT_UTF8_HEADER: &[u8] = b"type\xff,client,tx,amount\ndeposit,1,1,1.0\n";

/// One deposit.
#[cfg(unix)]
const ONE_DEPOSIT: &str = "type,client,tx,amount\ndeposit,1,1,1.0\n";

/// The table for `HEADER_ONLY`: the header and no rows.
const HEADER_ONLY_TABLE: &str = "client,available,held,total,locked\n";

/// The help, as `-h` and `--help` print it.
const HELP: &str = "\
Reads client and dispute commands from a CSV file and prints the accounts table

Usage: cthulhu <input.csv>

Arguments:
  <input.csv>  The CSV file of commands to read

Options:
  -h, --help     Print help
  -V, --version  Print version
";

/// The version, as `-V` and `--version` print it.
const VERSION: &str = concat!("cthulhu ", env!("CARGO_PKG_VERSION"), "\n");

/// The usage error when no path is given.
const NO_ARGUMENT: &str = "\
error: the following required arguments were not provided:
  <input.csv>

Usage: cthulhu <input.csv>

For more information, try '--help'.
";

/// The usage error when the path is empty.
const EMPTY_ARGUMENT: &str = "\
error: a value is required for '<input.csv>' but none was supplied

For more information, try '--help'.
";

/// The usage error for the unknown flag `--input`.
const UNKNOWN_FLAG: &str = "\
error: unexpected argument '--input' found

  tip: to pass '--input' as a value, use '-- --input'

Usage: cthulhu <input.csv>

For more information, try '--help'.
";

/// The usage error when `extra.csv` follows the path.
const SECOND_ARGUMENT: &str = "\
error: unexpected argument 'extra.csv' found

Usage: cthulhu <input.csv>

For more information, try '--help'.
";

/// The error when the input cannot be read.
#[cfg(unix)]
const UNREADABLE: &str = "error: the input cannot be read\n";

/// The error when the header lacks `type`, `client` or `tx`.
const INCOMPLETE_HEADER: &str = "error: the header lacks type, client or tx\n";

/// The error when the table cannot be written.
#[cfg(unix)]
const UNWRITABLE: &str = "error: the output cannot be written\n";

/// Runs the binary with no arguments, under `name` as its `argv[0]`. A hard link to the binary,
/// made while other tests ran it, got those runs killed with SIGKILL on macOS: the name changes
/// here without touching a file.
#[cfg(unix)]
fn cthulhu_named(name: &str) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).arg0(name).output()
}

/// Runs the binary with `arguments`.
fn cthulhu(arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).output()
}

/// Runs the binary with `arguments`, from `directory`.
fn cthulhu_in(directory: &Path, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).current_dir(directory).args(arguments).output()
}

/// Runs the binary with `arguments`, its stdout written to `stdout`.
#[cfg(unix)]
fn cthulhu_into(stdout: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stdout(stdout).output()
}

/// Runs the binary with `arguments`, its stderr written to `stderr`.
#[cfg(unix)]
fn cthulhu_reporting_into(stderr: impl Into<Stdio>, arguments: &[&Path]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cthulhu")).args(arguments).stderr(stderr).output()
}

/// One end of a socket pair, shut for writing: every write to it fails, whatever process holds
/// the other end. On macOS, a pipe whose reading end was dropped let a write through about once in
/// sixty runs: a child another test spawned could inherit that reading end before it was marked
/// close-on-exec.
#[cfg(unix)]
fn unwritable() -> io::Result<OwnedFd> {
    let (writer, _reader) = UnixStream::pair()?;
    writer.shutdown(Shutdown::Write)?;
    Ok(OwnedFd::from(writer))
}

/// These tests' scratch directory, created if missing.
fn scratch() -> io::Result<PathBuf> {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

/// A file named `name` in the scratch directory, holding `contents`.
fn input(name: &str, contents: impl AsRef<[u8]>) -> io::Result<PathBuf> {
    let path = scratch()?.join(name);
    fs::write(&path, contents)?;
    Ok(path)
}

/// One deposit by each client from 1 to `clients`, each on a tx id of its own: the table of three
/// hundred is longer than the CSV writer's 8 KiB buffer, where the table of one fits.
#[cfg(unix)]
fn deposits(clients: u16) -> String {
    (1..=clients)
        .zip(1_u32..)
        .fold(String::from("type,client,tx,amount\n"), |text, (client, tx)| {
            text + &format!("deposit,{client},{tx},1.0\n")
        })
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
fn the_help_flags_print_the_help_on_stdout() {
    for flag in ["-h", "--help"] {
        let output = cthulhu(&[Path::new(flag)]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&output.stdout), HELP);
        assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    }
}

#[test]
fn the_version_flags_print_the_version_on_stdout() {
    for flag in ["-V", "--version"] {
        let output = cthulhu(&[Path::new(flag)]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&output.stdout), VERSION);
        assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    }
}

#[test]
fn a_wrong_argument_count_is_a_usage_error() {
    let path = input("usage.csv", HEADER_ONLY).expect("the scratch directory takes a file");
    for (arguments, message) in
        [(&[][..], NO_ARGUMENT), (&[path.as_path(), Path::new("extra.csv")][..], SECOND_ARGUMENT)]
    {
        let output = cthulhu(arguments).expect("the binary runs");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(String::from_utf8_lossy(&output.stderr), message);
    }
}

#[test]
fn an_empty_path_or_an_unknown_flag_is_a_usage_error() {
    for (argument, message) in [("", EMPTY_ARGUMENT), ("--input", UNKNOWN_FLAG)] {
        let output = cthulhu(&[Path::new(argument)]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(String::from_utf8_lossy(&output.stderr), message);
    }
}

#[test]
fn a_path_that_begins_with_a_dash_follows_a_double_dash() {
    let path = input("-dash.csv", HEADER_ONLY).expect("the scratch directory takes a file");
    let directory = path.parent().expect("the input sits in the scratch directory");
    let output =
        cthulhu_in(directory, &[Path::new("--"), Path::new("-dash.csv")]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), HEADER_ONLY_TABLE);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[cfg(unix)]
#[test]
fn the_binary_under_another_name_still_names_cthulhu() {
    let output = cthulhu_named("renamed").expect("the binary runs");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), NO_ARGUMENT);
}

#[cfg(unix)]
#[test]
fn a_help_that_cannot_be_written_is_a_write_failure() {
    let stdout = unwritable().expect("an unwritable stream opens");
    let output = cthulhu_into(stdout, &[Path::new("--help")]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("error: cannot write the help or the version: "), "{stderr}");
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

#[cfg(unix)]
#[test]
fn an_unreadable_input_stops_the_run_with_no_table() {
    let directory = scratch().expect("the scratch directory exists");
    let output = cthulhu(&[&directory]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), UNREADABLE);
}

#[test]
fn an_input_without_a_complete_header_stops_the_run_with_no_table() {
    for (name, text) in
        [("empty.csv", EMPTY), ("no-tx.csv", NO_TX), ("not-utf8.csv", NOT_UTF8_HEADER)]
    {
        let path = input(name, text).expect("the scratch directory takes a file");
        let output = cthulhu(&[&path]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
        assert_eq!(String::from_utf8_lossy(&output.stderr), INCOMPLETE_HEADER);
    }
}

#[cfg(unix)]
#[test]
fn a_table_that_cannot_be_written_is_a_write_failure() {
    for (name, text) in
        [("one-deposit.csv", ONE_DEPOSIT.to_owned()), ("deposits.csv", deposits(300))]
    {
        let path = input(name, text).expect("the scratch directory takes a file");
        let stdout = unwritable().expect("an unwritable stream opens");
        let output = cthulhu_into(stdout, &[&path]).expect("the binary runs");
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(String::from_utf8_lossy(&output.stderr), UNWRITABLE);
    }
}

#[cfg(unix)]
#[test]
fn a_report_that_cannot_be_written_is_a_write_failure() {
    let path = input("unwritable-stderr.csv", REJECTED_AND_MALFORMED)
        .expect("the scratch directory takes a file");
    let stderr = unwritable().expect("an unwritable stream opens");
    let output = cthulhu_reporting_into(stderr, &[&path]).expect("the binary runs");
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(String::from_utf8_lossy(&output.stdout), REJECTED_AND_MALFORMED_TABLE);
}
