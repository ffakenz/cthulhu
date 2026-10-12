//! The in-memory stores: the ledger and the seen tx ids, which cannot fail.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::Infallible;

use engine::domain::{Account, Address, ClientId, Deposit, Footprint, Sight, TxId};
use engine::run::{Accounts, LedgerStore, SeenIds};

/// The ledger in memory: accounts by client id, in client order as the snapshot yields them, and
/// deposits by address, looked up only by key. `Default` builds the empty ledger.
#[derive(Debug, Default)]
pub struct MemoryLedger {
    accounts: BTreeMap<ClientId, Account>,
    deposits: HashMap<Address, Deposit>,
}

impl LedgerStore for MemoryLedger {
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

/// The seen tx ids in memory. `Default` builds the empty set.
#[derive(Debug, Default)]
pub struct MemorySeenIds(HashSet<TxId>);

impl SeenIds for MemorySeenIds {
    type Error = Infallible;

    fn sight(&mut self, tx: TxId) -> Result<Sight, Self::Error> {
        Ok(if self.0.insert(tx) { Sight::First } else { Sight::Repeat })
    }
}
