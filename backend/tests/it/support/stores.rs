//! Store doubles: a ledger and a set of seen tx ids in plain ordered collections.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

use backend::domain::{Account, Address, ClientId, Deposit, Footprint, Sight, TxId};
use backend::run::{Accounts, LedgerStore, SeenIds};

/// A ledger in two maps: accounts by client id, deposits by address.
#[derive(Debug, Default)]
pub(crate) struct Ledger {
    accounts: BTreeMap<ClientId, Account>,
    deposits: BTreeMap<Address, Deposit>,
}

impl LedgerStore for Ledger {
    type Error = Infallible;

    fn load(&self, address: Address) -> Result<Option<Footprint>, Self::Error> {
        Ok(self
            .accounts
            .get(&address.client())
            .map(|&account| Footprint::new(account, self.deposits.get(&address).copied())))
    }

    fn commit(&mut self, address: Address, footprint: Footprint) -> Result<(), Self::Error> {
        let _ = self.accounts.insert(address.client(), *footprint.account());
        let _ = match footprint.deposit() {
            Some(&deposit) => self.deposits.insert(address, deposit),
            None => self.deposits.remove(&address),
        };
        Ok(())
    }

    fn snapshot(&self) -> Result<Accounts, Self::Error> {
        Ok(self.accounts.iter().map(|(&client, &account)| (client, account)).collect())
    }
}

/// Seen tx ids in a set.
#[derive(Debug, Default)]
pub(crate) struct Seen(BTreeSet<TxId>);

impl SeenIds for Seen {
    type Error = Infallible;

    fn sight(&mut self, tx: TxId) -> Result<Sight, Self::Error> {
        Ok(if self.0.insert(tx) { Sight::First } else { Sight::Repeat })
    }
}
