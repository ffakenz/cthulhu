//! Laws of the CSV adapters: every spelling of the rows reads as the model reads them; a header
//! lacking a column, or a failed read, ends the source with its error; the table printed is the
//! naive model's, and one the output takes only in part is an error.

use std::io::{self, Read, Write};

use backend::csv_io::{CsvSource, SinkError, render_csv};
use backend::run::{Row, SourceError};
use proptest::arbitrary::any;
use proptest::sample::Index;

use crate::support::spelling::{self, Input};
use crate::support::{self, generate, model};

/// A header whose first name holds a byte that is not UTF-8, then a row.
const NOT_UTF8_HEADER: &[u8] = b"type\xff,client,tx\ndeposit,1,1\n";

/// What the model makes of `input`'s rows, as a source yields them.
fn expected(input: &Input) -> Vec<Result<Row, SourceError>> {
    input.rows().iter().map(|row| Ok(generate::row(row))).collect()
}

/// What the CSV source reads from `input`'s text, at most two items past its rows.
fn read(input: &Input) -> Vec<Result<Row, SourceError>> {
    let bound = input.rows().len().saturating_add(2);
    CsvSource::new(input.text().as_slice()).take(bound).collect()
}

/// What the CSV source reads from `input`'s text when reading fails at the byte `cut` picks, at
/// most two items past its rows.
fn read_broken(input: &Input, cut: Index) -> Vec<Result<Row, SourceError>> {
    let text = input.text();
    let left = text.get(..cut.index(text.len().saturating_add(1))).unwrap_or(&text);
    let bound = input.rows().len().saturating_add(2);
    CsvSource::new(Breaking { left }).take(bound).collect()
}

/// A reader that yields `left`, then fails every read.
#[derive(Debug)]
struct Breaking<'a> {
    left: &'a [u8],
}

impl Read for Breaking<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.left.is_empty() {
            return Err(io::Error::other("the input broke"));
        }
        self.left.read(buffer)
    }
}

#[test]
fn prop_source_reads_every_spelling_as_the_model_reads_the_rows() {
    let law = support::for_all(
        "the source reads each row, however it is spelled, as the model reads it, in order",
        spelling::flawed(),
        |input| read(input) == expected(input),
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_header_lacking_type_client_or_tx_ends_the_source() {
    let law = support::for_all(
        "a header lacking type, client or tx ends the source with that error, before any row",
        spelling::incomplete(),
        |input| read(input) == [Err(SourceError::IncompleteHeader)],
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn a_header_that_is_not_utf8_lacks_its_columns() {
    let items: Vec<Result<Row, SourceError>> = CsvSource::new(NOT_UTF8_HEADER).collect();
    assert_eq!(items, [Err(SourceError::IncompleteHeader)]);
}

#[test]
fn prop_read_failure_ends_the_source() {
    let law = support::for_all(
        "a read failure ends the source with Unreadable, after rows the model reads as they were",
        (spelling::unmarked(), any::<Index>()),
        |(input, cut)| match read_broken(input, *cut).split_last() {
            Some((last, before)) => {
                *last == Err(SourceError::Unreadable) && expected(input).starts_with(before)
            }
            None => false,
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

/// An output that takes at most `room` bytes: a write that does not fit is cut short, and every
/// write once it is full fails.
#[derive(Debug)]
struct Full {
    written: Vec<u8>,
    room: usize,
}

impl Write for Full {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let space = self.room.saturating_sub(self.written.len());
        if space == 0 && !bytes.is_empty() {
            return Err(io::Error::other("the output is full"));
        }
        let taken = bytes.get(..space.min(bytes.len())).unwrap_or_default();
        self.written.extend_from_slice(taken);
        Ok(taken.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn prop_table_is_the_naive_models() {
    let law = support::for_all(
        "the table printed for any input is the naive model's, header first, accounts by client id",
        spelling::input(),
        |input| {
            support::summarize(CsvSource::new(input.text().as_slice())).is_ok_and(|summary| {
                let mut table = Vec::new();
                render_csv(&summary.accounts, &mut table).is_ok()
                    && table == model::table(input.rows()).into_bytes()
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_table_that_cannot_be_written_whole_is_an_error() {
    let law = support::for_all(
        "a table, never empty, that the output takes only in part is an Unwritable error; one it takes whole succeeds",
        (spelling::input(), any::<Index>()),
        |(input, room)| {
            let Ok(summary) = support::summarize(CsvSource::new(input.text().as_slice())) else {
                return false;
            };
            let mut whole = Vec::new();
            if render_csv(&summary.accounts, &mut whole).is_err() || whole.is_empty() {
                return false;
            }
            let room = room.index(whole.len().saturating_add(1));
            let mut out = Full { written: Vec::new(), room };
            match render_csv(&summary.accounts, &mut out) {
                Ok(()) => room == whole.len() && out.written == whole,
                Err(error) => room < whole.len() && error == SinkError::Unwritable,
            }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
