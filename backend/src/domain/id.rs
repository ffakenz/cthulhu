//! Identities: a client, a transaction, and a command's address, which holds one of each.

/// A client, named by a `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(u16);

impl From<u16> for ClientId {
    fn from(id: u16) -> Self {
        Self(id)
    }
}

/// A transaction, named by a `u32`: a deposit's or a withdrawal's id, which a dispute command
/// names to reach a deposit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TxId(u32);

impl From<u32> for TxId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

/// A command's address: its client and tx id, where its footprint is loaded and committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address {
    client: ClientId,
    tx: TxId,
}

impl Address {
    /// The address of a command naming `client` and `tx`.
    pub fn new(client: ClientId, tx: TxId) -> Self {
        Self { client, tx }
    }

    /// The client the command names.
    pub fn client(self) -> ClientId {
        self.client
    }

    /// The tx id the command names.
    pub fn tx(self) -> TxId {
        self.tx
    }
}
