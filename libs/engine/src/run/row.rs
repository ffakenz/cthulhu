//! What a source yields, and how a run fails: a malformed row is skipped and counted; a source or
//! store failure stops the run.

use crate::domain::{AmountError, Command};

/// A row as a source delivers it: the command it reads as, or why it is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// The row reads as a command.
    Parsed(Command),
    /// The row is malformed, and opens nothing.
    Malformed(RowError),
}

/// Why a row is malformed: its kind, by which it is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum RowError {
    /// A field does not read: a byte that is not UTF-8, an unknown type, a client or tx id that is
    /// missing or is not a `u16` or a `u32`, a column other than `amount` that the row stops
    /// before, or `type`, `client`, `tx` or `amount` named twice in the header.
    #[error("a field does not read")]
    Unreadable,
    /// More fields than the header names.
    #[error("more fields than the header")]
    TooManyFields,
    /// A deposit or a withdrawal without an amount.
    #[error("no amount")]
    MissingAmount,
    /// A deposit or a withdrawal whose amount is refused, and why.
    #[error("invalid amount: {0}")]
    InvalidAmount(AmountError),
}

/// Why a source stops a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// Reading the input failed.
    #[error("the input cannot be read")]
    Unreadable,
    /// The header lacks `type`, `client` or `tx`.
    #[error("the header lacks type, client or tx")]
    IncompleteHeader,
}

/// Why a run stopped with no summary: its source failed, or one of its stores did.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RunError<L, S> {
    /// The source stopped.
    #[error("{0}")]
    Source(SourceError),
    /// The ledger store failed.
    #[error("the ledger store failed: {0}")]
    Ledger(L),
    /// The tx-id store failed.
    #[error("the tx-id store failed: {0}")]
    Seen(S),
}
