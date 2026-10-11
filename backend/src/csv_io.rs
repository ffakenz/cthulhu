//! The CSV adapters.

use std::io::Read;
use std::marker::PhantomData;

use crate::run::{Row, SourceError};

/// A source that drops its input and yields no row.
#[derive(Debug)]
pub struct CsvSource<R>(PhantomData<R>);

impl<R: Read> CsvSource<R> {
    /// The source over `input`.
    pub fn new(input: R) -> Self {
        drop(input);
        Self(PhantomData)
    }
}

impl<R: Read> Iterator for CsvSource<R> {
    type Item = Result<Row, SourceError>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
