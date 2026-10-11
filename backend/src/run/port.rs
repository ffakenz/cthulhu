//! The store ports `run` consumes: synchronous, exchanging owned values, each with its own error.

use crate::domain::{Address, Footprint, Sight, TxId};
use crate::run::Accounts;

/// Where `run` keeps the ledger: each account under its client id, and each deposit in history
/// under its address.
pub trait LedgerStore {
    /// Why the store failed.
    type Error;

    /// The footprint at `address`, as the last commit left it: the client's account and the
    /// deposit there, if any; `None` when the client has no account.
    fn load(&self, address: Address) -> Result<Option<Footprint>, Self::Error>;

    /// Writes `footprint` at `address`: the account under the client id, and the deposit under
    /// the address, or no deposit there when the footprint holds none. Both are written, or
    /// neither is.
    fn commit(&mut self, address: Address, footprint: Footprint) -> Result<(), Self::Error>;

    /// Every account by client id, as every commit before it left them.
    fn snapshot(&self) -> Result<Accounts, Self::Error>;
}

/// Where intake records the tx ids of deposits and withdrawals.
pub trait SeenIds {
    /// Why the store failed.
    type Error;

    /// Whether `tx` was seen before; on first sight, records it.
    fn sight(&mut self, tx: TxId) -> Result<Sight, Self::Error>;
}
