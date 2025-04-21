use std::sync::Arc;

use bdk_chain::{ChainPosition as BdkChainPosition, ConfirmationBlockTime, tx_graph::CanonicalTx};
use bdk_wallet::Wallet as BdkWallet;
use bdk_wallet::bitcoin::Transaction as BdkTransaction;
use bip329::{Label, Labels, TransactionRecord};
use bitcoin::params::Params;
use jiff::Timestamp;
use numfmt::{Formatter, Precision};

use crate::{
    Amount, FeeRate, TransactionDirection, Unit,
    sent_and_received::SentAndReceived, TxId
};

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum TransactionDetailError {
    #[error("Unable to determine fee: {0}")]
    Fee(String),

    #[error("Unable to determine fee rate: {0}")]
    FeeRate(String),

    #[error("Unable to determine address: {0}")]
    Address(String),

    #[error("Unable to get fiat amount: {0}")]
    FiatAmount(String),

    #[error("Unable to get change address: {0}")]
    ChangeAddress(String),
}

type Error = TransactionDetailError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Object)]
pub struct TransactionDetails {
    pub tx_id: TxId,
    pub address: Address,
    pub sent_and_received: SentAndReceived,
    pub fee: Option<Amount>,
    pub fee_rate: Option<FeeRate>,
    pub pending_or_confirmed: PendingOrConfirmed,
    pub labels: Labels,
    pub input_indexes: Vec<u32>,
    pub output_indexes: Vec<u32>,
    // for outgoing transactions we might have a change address
    pub change_address: Option<Address>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum PendingOrConfirmed {
    Pending(PendingDetails),
    Confirmed(ConfirmedDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct PendingDetails {
    last_seen: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ConfirmedDetails {
    block_number: u32,
    confirmation_time: u64,
}

impl PendingOrConfirmed {
    pub fn new(chain_position: &BdkChainPosition<ConfirmationBlockTime>) -> Self {
        match chain_position {
            BdkChainPosition::Unconfirmed { last_seen } => Self::Pending(PendingDetails {
                last_seen: (*last_seen).unwrap_or_default(),
            }),
            BdkChainPosition::Confirmed {
                anchor: confirmation_blocktime,
                ..
            } => Self::Confirmed(ConfirmedDetails {
                block_number: confirmation_blocktime.block_id.height,
                confirmation_time: confirmation_blocktime.confirmation_time,
            }),
        }
    }

    fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed(_))
    }
}

// Simple Address type for transaction details
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Object)]
pub struct Address {
    pub address: String,
}

impl Address {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    pub fn try_new(
        tx: &CanonicalTx<Arc<BdkTransaction>, ConfirmationBlockTime>,
        wallet: &BdkWallet,
    ) -> Result<Self, Error> {
        // Get the first output from the transaction
        let tx = &tx.tx_node.tx;
        let script = tx
            .output
            .iter()
            .find_map(|output| {
                if !wallet.is_mine(output.script_pubkey.clone()) {
                    Some(output.script_pubkey.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| tx.output[0].script_pubkey.clone());

        let network = wallet.network();

        let address = bitcoin::Address::from_script(&script, Params::from(network))
            .map_err(|e| Error::Address(e.to_string()))?;

        Ok(Self {
            address: address.to_string(),
        })
    }

    pub fn spaced_out(&self) -> String {
        // Add space to split long addresses into chunks of 4
        let mut address = self.address.clone();
        let prefix_len = address.find(':').unwrap_or(0) + 1;
        let address_part = &address[prefix_len..];
        
        let mut chunks = Vec::new();
        for i in (0..address_part.len()).step_by(4) {
            let end = i + 4.min(address_part.len() - i);
            chunks.push(&address_part[i..end]);
        }
        
        let spaced = chunks.join(" ");
        address.replace_range(prefix_len.., &spaced);
        address
    }

    pub fn preview_new() -> Self {
        Self {
            address: "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string(),
        }
    }
}

#[uniffi::export]
impl TransactionDetails {
    #[uniffi::method]
    pub fn tx_id(&self) -> TxId {
        self.tx_id
    }

    #[uniffi::method]
    pub fn address(&self) -> Address {
        self.address.clone()
    }

    #[uniffi::method]
    pub fn amount(&self) -> Amount {
        self.sent_and_received.amount()
    }

    #[uniffi::method]
    pub fn amount_fmt(&self, unit: Unit) -> String {
        self.sent_and_received.amount_fmt(unit)
    }

    #[uniffi::method]
    pub fn is_received(&self) -> bool {
        self.sent_and_received.direction() == TransactionDirection::Incoming
    }

    #[uniffi::method]
    pub fn is_sent(&self) -> bool {
        !self.is_received()
    }

    #[uniffi::method]
    pub fn sent_sans_fee(&self) -> Option<Amount> {
        if self.is_received() {
            return None;
        }

        let fee: Amount = self.fee?;
        let sent: Amount = self.amount();

        let sans_fee = sent.0.checked_sub(fee.0)?;

        Some(Amount(sans_fee))
    }

    #[uniffi::method]
    pub fn sent_sans_fee_fmt(&self, unit: Unit) -> Option<String> {
        let amount = self.sent_sans_fee()?;
        Some(amount.fmt_string_with_unit(unit))
    }

    #[uniffi::method]
    pub fn is_confirmed(&self) -> bool {
        self.pending_or_confirmed.is_confirmed()
    }

    #[uniffi::method]
    pub fn transaction_url(&self) -> String {
        format!("https://mempool.guide/tx/{}", self.tx_id.0)
    }

    #[uniffi::method]
    pub fn transaction_label(&self) -> Option<String> {
        let label = self.labels.transaction_label()?;
        if label.is_empty() {
            return None;
        }

        Some(label.to_string())
    }

    #[uniffi::method]
    pub fn block_number(&self) -> Option<u32> {
        match &self.pending_or_confirmed {
            PendingOrConfirmed::Pending(_) => None,
            PendingOrConfirmed::Confirmed(confirmed) => Some(confirmed.block_number),
        }
    }

    #[uniffi::method]
    pub fn block_number_fmt(&self) -> Option<String> {
        let block_number = self.block_number()?;

        let mut f = Formatter::new()
            .separator(',')
            .unwrap()
            .precision(Precision::Decimals(0));

        Some(f.fmt(block_number).to_string())
    }
    
    #[uniffi::method]
    pub fn address_spaced_out(&self) -> String {
        self.address.spaced_out()
    }

    #[uniffi::constructor]
    pub fn preview_new_confirmed() -> Self {
        Self {
            tx_id: TxId::preview_new(),
            address: Address::preview_new(),
            sent_and_received: SentAndReceived::preview_new(),
            fee: Some(Amount::from_sat(880303)),
            fee_rate: Some(FeeRate::preview_new()),
            pending_or_confirmed: PendingOrConfirmed::Confirmed(ConfirmedDetails {
                block_number: 840_000,
                confirmation_time: 1677721600,
            }),
            labels: Default::default(),
            input_indexes: vec![],
            output_indexes: vec![],
            change_address: None,
        }
    }

    #[uniffi::constructor]
    pub fn preview_confirmed_received() -> Self {
        let mut me = Self::preview_new_confirmed();
        me.sent_and_received = SentAndReceived::preview_incoming();
        me
    }

    #[uniffi::constructor]
    pub fn preview_confirmed_sent() -> Self {
        let mut me = Self::preview_new_confirmed();
        me.sent_and_received = SentAndReceived::preview_outgoing();
        me
    }

    #[uniffi::constructor]
    pub fn preview_pending_received() -> Self {
        let mut me = Self::preview_new_confirmed();
        me.sent_and_received = SentAndReceived::preview_incoming();
        me.pending_or_confirmed = PendingOrConfirmed::Pending(PendingDetails {
            last_seen: 1677721600,
        });

        me
    }

    #[uniffi::constructor]
    pub fn preview_pending_sent() -> Self {
        let mut me = Self::preview_new_confirmed();
        me.sent_and_received = SentAndReceived::preview_outgoing();
        me.pending_or_confirmed = PendingOrConfirmed::Pending(PendingDetails {
            last_seen: 1677721600,
        });

        me
    }

    #[uniffi::constructor(default(label = "bike payment"))]
    pub fn preview_new_with_label(label: String) -> Self {
        let mut me = Self::preview_new_confirmed();
        me.labels = vec![Label::from(TransactionRecord {
            ref_: *TxId::preview_new(),
            label: Some(label),
            origin: None,
        })]
        .into();

        me
    }
}