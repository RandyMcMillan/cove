pub mod sent_and_received;
pub mod unit;
pub mod fees;
pub mod ffi;
pub mod transaction_details;
pub mod unsigned_transaction;

uniffi::setup_scaffolding!();

use numfmt::{Formatter, Precision};
use serde::{Deserialize, Serialize};

use crate::unit::Unit;
use std::sync::Arc;

use bdk_chain::{
    ChainPosition as BdkChainPosition, ConfirmationBlockTime,
    bitcoin::{Sequence, Witness},
    tx_graph::CanonicalTx,
};
use bdk_wallet::bitcoin::{
    ScriptBuf, Transaction as BdkTransaction, TxIn as BdkTxIn, TxOut as BdkTxOut,
};
use bip329::Labels;
use std::cmp::Ordering;
pub use cove_types::{OutPoint, TxId, WalletId};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Object,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Mul,
    derive_more::From,
    derive_more::Into,
    derive_more::Deref,
)]
pub struct Amount(pub bitcoin::Amount);

// rust only
impl Amount {
    pub fn from_btc(btc: f64) -> Result<Self, eyre::Report> {
        Ok(Self(bitcoin::Amount::from_btc(btc)?))
    }
}

#[uniffi::export]
impl Amount {
    #[uniffi::constructor]
    pub fn from_sat(sats: u64) -> Self {
        Self(bitcoin::Amount::from_sat(sats))
    }

    #[uniffi::constructor]
    pub fn one_btc() -> Self {
        Self(bitcoin::Amount::ONE_BTC)
    }

    #[uniffi::constructor]
    pub fn one_sat() -> Self {
        Self(bitcoin::Amount::ONE_SAT)
    }

    pub fn as_btc(&self) -> f64 {
        self.0.to_btc()
    }

    pub fn fmt_string_with_unit(&self, unit: Unit) -> String {
        match unit {
            Unit::Btc => self.btc_string_with_unit(),
            Unit::Sat => self.sats_string_with_unit(),
        }
    }

    pub fn as_sats(&self) -> u64 {
        self.0.to_sat()
    }

    pub fn btc_string(&self) -> String {
        format!("{:.8}", self.as_btc())
    }

    pub fn btc_string_with_unit(&self) -> String {
        format!("{:.8} BTC", self.as_btc())
    }

    pub fn sats_string(&self) -> String {
        let mut f = Formatter::new()
            .separator(',')
            .unwrap()
            .precision(Precision::Decimals(0));

        f.fmt2(self.as_sats() as f64).to_string()
    }

    pub fn sats_string_with_unit(&self) -> String {
        format!("{} SATS", self.sats_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum TransactionState {
    Pending,
    Confirmed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, uniffi::Object)]
pub enum ChainPosition {
    Unconfirmed(u64),
    Confirmed(ConfirmationBlockTime),
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Transaction {
    Confirmed(Arc<ConfirmedTransaction>),
    Unconfirmed(Arc<UnconfirmedTransaction>),
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Object)]
pub struct ConfirmedTransaction {
    pub txid: TxId,
    pub block_height: u32,
    pub confirmed_at: jiff::Timestamp,
    pub sent_and_received: sent_and_received::SentAndReceived,
    pub fiat: Option<cove_fiat::FiatAmount>,
    pub labels: Labels,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Object)]
pub struct UnconfirmedTransaction {
    pub txid: TxId,
    pub sent_and_received: sent_and_received::SentAndReceived,
    pub last_seen: u64,
    pub fiat: Option<cove_fiat::FiatAmount>,
    pub labels: Labels,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Object)]
pub struct TxOut {
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Object)]
pub struct TxIn {
    pub previous_output: OutPoint,
    pub script_sig: ScriptBuf,
    pub sequence: Sequence,
    pub witness: Witness,
}

pub type FeeRate = fees::FeeRate;
pub type BdkAmount = bitcoin::Amount;
pub type SentAndReceived = sent_and_received::SentAndReceived;
pub type TransactionDetails = transaction_details::TransactionDetails;

impl Transaction {
    pub fn id(&self) -> TxId {
        match self {
            Transaction::Confirmed(confirmed) => confirmed.id(),
            Transaction::Unconfirmed(unconfirmed) => unconfirmed.id(),
        }
    }

    pub fn sent_and_received(&self) -> SentAndReceived {
        match self {
            Self::Unconfirmed(last_seen) => last_seen.sent_and_received,
            Self::Confirmed(confirmed) => confirmed.sent_and_received,
        }
    }
}

impl From<(BdkAmount, BdkAmount)> for TransactionDirection {
    fn from((sent, received): (BdkAmount, BdkAmount)) -> Self {
        if sent > received {
            Self::Outgoing
        } else {
            Self::Incoming
        }
    }
}

impl From<BdkTxOut> for TxOut {
    fn from(tx_out: BdkTxOut) -> Self {
        Self {
            value: Amount::from(tx_out.value),
            script_pubkey: tx_out.script_pubkey,
        }
    }
}

impl From<BdkTxIn> for TxIn {
    fn from(tx_in: BdkTxIn) -> Self {
        Self {
            previous_output: tx_in.previous_output.into(),
            script_sig: tx_in.script_sig,
            sequence: tx_in.sequence,
            witness: tx_in.witness,
        }
    }
}

impl From<BdkChainPosition<&ConfirmationBlockTime>> for ChainPosition {
    fn from(chain_position: BdkChainPosition<&ConfirmationBlockTime>) -> Self {
        match chain_position {
            BdkChainPosition::Unconfirmed { last_seen } => {
                Self::Unconfirmed(last_seen.unwrap_or_default())
            }
            BdkChainPosition::Confirmed {
                anchor: confirmation_blocktime,
                ..
            } => Self::Confirmed(*confirmation_blocktime),
        }
    }
}

impl Ord for ConfirmedTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_height.cmp(&other.block_height)
    }
}

impl PartialOrd for ConfirmedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnconfirmedTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last_seen.cmp(&other.last_seen)
    }
}

impl PartialOrd for UnconfirmedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

//  MARK: transaction impls

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        let sort = match (self, other) {
            (Self::Confirmed(confirmed), Self::Confirmed(other)) => confirmed.cmp(other),
            (Self::Unconfirmed(unconfirmed), Self::Unconfirmed(other)) => unconfirmed.cmp(other),
            (Self::Confirmed(_), Self::Unconfirmed(_)) => Ordering::Less,
            (Self::Unconfirmed(_), Self::Confirmed(_)) => Ordering::Greater,
        };

        if sort == Ordering::Equal {
            self.id().cmp(&other.id())
        } else {
            sort
        }
    }
}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}