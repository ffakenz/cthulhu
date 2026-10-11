//! Laws of the CSV adapters: every spelling of the rows reads as the model reads them; a header
//! lacking a column, or a failed read, ends the source with its error.

use std::io::{self, Read};

use backend::csv_io::CsvSource;
use backend::run::{Row, SourceError};
use proptest::arbitrary::any;
use proptest::sample::Index;

use crate::support::spelling::{self, Input};
use crate::support::{self, generate};

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
