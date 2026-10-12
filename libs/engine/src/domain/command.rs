//! Commands: requests from outside, which the core may refuse. Every command carries its address.

use crate::domain::{Address, Amount};

/// A command of either family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// The client moves money.
    Client(ClientCommand),
    /// The payment provider acts on a recorded deposit.
    Dispute(DisputeCommand),
}

/// A client command: the client moves money into or out of its account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCommand {
    /// Pays `amount` in, to be recorded in history at `address`.
    Deposit { address: Address, amount: Amount },
    /// Takes `amount` out.
    Withdrawal { address: Address, amount: Amount },
}

impl ClientCommand {
    /// Where the command applies: its client and tx id.
    pub fn address(&self) -> Address {
        match self {
            Self::Deposit { address, .. } | Self::Withdrawal { address, .. } => *address,
        }
    }
}

/// A dispute command: the payment provider acts on the deposit at its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeCommand {
    /// Holds a settled deposit's amount.
    Dispute(Address),
    /// Releases a disputed deposit's amount.
    Resolve(Address),
    /// Reverses a disputed deposit and locks the account.
    Chargeback(Address),
}

impl DisputeCommand {
    /// Where the command applies: its client and the tx id of the deposit it names.
    pub fn address(&self) -> Address {
        match self {
            Self::Dispute(address) | Self::Resolve(address) | Self::Chargeback(address) => *address,
        }
    }
}
