//! Laws of the in-memory stores: the store laws, each checked against a reference of what it says
//! a store holds.

use std::collections::{BTreeMap, BTreeSet};

use engine::core::evolve;
use engine::domain::{Account, Address, ClientId, Deposit, Footprint, Sight, TxId};
use engine::run::{LedgerStore, SeenIds};
use proptest::collection::vec;
use proptest::option;
use proptest::prop_oneof;
use proptest::sample::select;
use proptest::strategy::{Just, Strategy};
use store::memory::{MemoryLedger, MemorySeenIds};

use crate::support::generate;
use crate::support::{self, infallible};

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

/// What follows a commit: a load of the address just committed or of an address drawn on its
/// own, or a snapshot.
#[derive(Debug, Clone, Copy)]
enum Probe {
    /// A load of the address just committed.
    LoadCommitted,
    /// A load of this address.
    Load(Address),
    /// A snapshot.
    Snapshot,
}

impl Probe {
    /// The operation this probe asks for after a commit at `committed`.
    fn operation(self, committed: Address) -> Operation {
        match self {
            Self::LoadCommitted => Operation::Load(committed),
            Self::Load(address) => Operation::Load(address),
            Self::Snapshot => Operation::Snapshot,
        }
    }
}

/// Operations on a ledger: one time in two, a load or a snapshot of the empty ledger; then, for
/// each generated event, a commit at a generated address of what `evolve` makes of the footprint a
/// reference would load there; after a commit, one time in two a load or a snapshot. One load in
/// two names the address just committed, the other an address drawn on its own.
fn operations() -> impl Strategy<Value = Vec<Operation>> {
    let any_first = option::of(prop_oneof![
        generate::address().prop_map(Operation::Load),
        Just(Operation::Snapshot),
    ]);
    let any_probe = option::of(prop_oneof![
        1 => Just(Probe::LoadCommitted),
        1 => load_address().prop_map(Probe::Load),
        2 => Just(Probe::Snapshot),
    ]);
    let any_commits =
        vec((generate::event(), generate::address(), any_probe), 0..=generate::MAX_EVENTS);
    (any_first, any_commits).prop_map(|(first, commits)| {
        let mut reference = Reference::default();
        let mut operations: Vec<Operation> = first.into_iter().collect();
        for (event, address, probe) in commits {
            let Some(Ok(footprint)) = event.map(|event| evolve(reference.load(address), &event))
            else {
                continue;
            };
            reference.commit(address, footprint);
            operations.push(Operation::Commit(address, footprint));
            operations.extend(probe.map(|probe| probe.operation(address)));
        }
        operations
    })
}

/// An address drawn on its own for a load: a client from `LOAD_CLIENTS` and a generated tx id.
fn load_address() -> impl Strategy<Value = Address> {
    (select(&LOAD_CLIENTS), generate::address())
        .prop_map(|(client, address)| Address::new(ClientId::from(client), address.tx()))
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
    let law =
        support::for_all("a load returns what the last commit wrote", operations(), |operations| {
            replay(operations, |ledger, reference, operation| match operation {
                Operation::Load(address) => {
                    infallible(ledger.load(address)) == reference.load(address)
                }
                Operation::Commit(..) | Operation::Snapshot => true,
            })
        });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_commit_writes_account_and_deposit_together() {
    let law = support::for_all(
        "a commit's account and deposit are read back together",
        operations(),
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
        operations(),
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
        vec(select(&SIGHTED), 0..=24),
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
