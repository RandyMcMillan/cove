use std::sync::Arc;

use bitcoin::Txid;
use serde::{Deserialize, Serialize};

use crate::{Amount, TxId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Object)]
pub struct UnsignedTransaction {
    pub txid: TxId,
    pub recipient_address: String,
    pub amount: Amount,
    pub fee: Amount,
    pub psbt: String,
}

#[uniffi::export]
impl UnsignedTransaction {
    #[uniffi::constructor(name = "fromParts")]
    pub fn new(
        txid: TxId,
        recipient_address: String,
        amount: Amount,
        fee: Amount,
        psbt: String,
    ) -> Self {
        Self {
            txid,
            recipient_address,
            amount,
            fee,
            psbt,
        }
    }
}