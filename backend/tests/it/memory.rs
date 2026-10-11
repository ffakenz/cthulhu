//! Laws of the in-memory stores: the store laws, each checked against a reference of what it says
//! a store holds.

use std::collections::{BTreeMap, BTreeSet};

use backend::core::evolve;
use backend::domain::{Account, Address, ClientId, Deposit, Footprint, Sight, TxId};
use backend::memory::{MemoryLedger, MemorySeenIds};
use backend::run::{LedgerStore, SeenIds};

use crate::support::generate::{self, EVEN_ODDS};
use crate::support::{self, Gen, infallible};

/// One operation on a ledger store.
#[derive(Debug, Clone, Copy)]
enum Operation {
    Commit(Address, Footprint),
    Load(Address),
    Snapshot,
}

/// What the store laws say a ledger holds: each client's last committed account, and the last
/// deposit committed at each address.
#[derive(Debug, Default)]
struct Reference {
    accounts: BTreeMap<ClientId, Account>,
    deposits: BTreeMap<Address, Deposit>,
}

impl Reference {
    /// The account under `address.client()`, then the footprint's deposit at `address`, or no
    /// deposit there when it holds none.
    fn commit(&mut self, address: Address, footprint: Footprint) {
        let _ = self.accounts.insert(address.client(), *footprint.account());
        let _ = match footprint.deposit() {
            Some(&deposit) => self.deposits.insert(address, deposit),
            None => self.deposits.remove(&address),
        };
    }

    /// The footprint at `address`; `None` when the client has no account.
    fn load(&self, address: Address) -> Option<Footprint> {
        self.accounts
            .get(&address.client())
            .map(|&account| Footprint::new(account, self.deposits.get(&address).copied()))
    }
}

/// Clients a load may name: those commits name, and 4, which none does.
const LOAD_CLIENTS: [u16; 6] = [1, 2, 3, 0, u16::MAX, 4];

/// Operations on a ledger: one time in two, a load or a snapshot of the empty ledger; then, for
/// each generated event, a commit at a generated address of what `evolve` makes of the footprint a
/// reference would load there; after a commit, one time in two a load or a snapshot.
fn operations(random: &mut Gen) -> Vec<Operation> {
    let mut reference = Reference::default();
    let mut operations = Vec::new();
    if random.one_in(EVEN_ODDS) {
        let first = if random.one_in(EVEN_ODDS) {
            Operation::Load(generate::address(random))
        } else {
            Operation::Snapshot
        };
        operations.push(first);
    }
    for event in generate::events(random) {
        let address = generate::address(random);
        let Ok(footprint) = evolve(reference.load(address), &event) else {
            continue;
        };
        reference.commit(address, footprint);
        operations.push(Operation::Commit(address, footprint));
        if random.one_in(EVEN_ODDS) {
            let next = if random.one_in(EVEN_ODDS) {
                Operation::Load(load_address(random, address))
            } else {
                Operation::Snapshot
            };
            operations.push(next);
        }
    }
    operations
}

/// The address a load names: one time in two the address just committed, otherwise a client from
/// `LOAD_CLIENTS` and a generated tx id.
fn load_address(random: &mut Gen, committed: Address) -> Address {
    if random.one_in(EVEN_ODDS) {
        return committed;
    }
    let client = random.pick(&LOAD_CLIENTS).copied().unwrap_or_default();
    Address::new(ClientId::from(client), generate::address(random).tx())
}

/// Replays `operations` on a fresh `MemoryLedger` beside a reference, and asks `check` after each.
fn replay(
    operations: &[Operation],
    check: impl Fn(&MemoryLedger, &Reference, Operation) -> bool,
) -> bool {
    let mut ledger = MemoryLedger::default();
    let mut reference = Reference::default();
    operations.iter().all(|&operation| {
        match operation {
            Operation::Commit(address, footprint) => {
                infallible(ledger.commit(address, footprint));
                reference.commit(address, footprint);
            }
            Operation::Load(_) | Operation::Snapshot => {}
        }
        check(&ledger, &reference, operation)
    })
}

/// Tx ids the sightings draw from.
const SIGHTED: [u32; 5] = [0, 1, 2, 3, u32::MAX];

#[test]
fn prop_read_returns_what_the_last_commit_wrote() {
    let law = support::for_all(
        "a load returns what the last commit wrote",
        500,
        operations,
        |operations| {
            replay(operations, |ledger, reference, operation| match operation {
                Operation::Load(address) => {
                    infallible(ledger.load(address)) == reference.load(address)
                }
                Operation::Commit(..) | Operation::Snapshot => true,
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_commit_writes_account_and_deposit_together() {
    let law = support::for_all(
        "a commit's account and deposit are read back together",
        500,
        operations,
        |operations| {
            replay(operations, |ledger, _, operation| match operation {
                Operation::Commit(address, footprint) => {
                    infallible(ledger.load(address)) == Some(footprint)
                }
                Operation::Load(_) | Operation::Snapshot => true,
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_snapshot_reflects_every_commit_before_it() {
    let law = support::for_all(
        "a snapshot reflects every commit before it, by client id",
        500,
        operations,
        |operations| {
            replay(operations, |ledger, reference, operation| match operation {
                Operation::Snapshot => infallible(ledger.snapshot())
                    .by_client()
                    .eq(reference.accounts.iter().map(|(&client, &account)| (client, account))),
                Operation::Commit(..) | Operation::Load(_) => true,
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_recorded_id_stays_recorded() {
    let law = support::for_all(
        "a recorded id stays recorded: the first sight of an id is First, every later one Repeat",
        500,
        |random: &mut Gen| random.up_to(24, &SIGHTED),
        |ids| {
            let mut seen = MemorySeenIds::default();
            let mut recorded = BTreeSet::new();
            ids.iter().all(|&id| {
                let tx = TxId::from(id);
                let expected = if recorded.insert(tx) { Sight::First } else { Sight::Repeat };
                infallible(seen.sight(tx)) == expected
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
