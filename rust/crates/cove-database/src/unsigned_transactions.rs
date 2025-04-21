use std::sync::Arc;

use cove_types::{TxId, WalletId};
use redb::{Database, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::{cbor, define_table, Error, Record};

define_table!(UNSIGNED_TRANSACTIONS, &[u8], &[u8]);

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct UnsignedTransactionInfo {
    pub txid: TxId,
    pub wallet_id: WalletId,
    pub recipient_address: String,
    pub amount: u64,
    pub fee: u64,
    pub psbt: String,
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct UnsignedTransactionsTable {
    pub db: Arc<Database>,
}

impl UnsignedTransactionsTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(UNSIGNED_TRANSACTIONS.clone());
        Self { db }
    }

    pub fn get(&self, txid: &TxId) -> Result<Option<Record<UnsignedTransactionInfo>>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(UNSIGNED_TRANSACTIONS.clone())?;
        let key = txid.to_bytes();

        match table.get(key.as_slice())? {
            Some(value) => {
                let record: Record<UnsignedTransactionInfo> = cbor::deserialize(value.value())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    pub fn list_by_wallet(&self, wallet_id: &WalletId) -> Result<Vec<Record<UnsignedTransactionInfo>>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(UNSIGNED_TRANSACTIONS.clone())?;

        let mut transactions = Vec::new();
        for result in table.iter()? {
            let (_, value) = result?;
            let record: Record<UnsignedTransactionInfo> = cbor::deserialize(value.value())?;
            if record.data.wallet_id == *wallet_id {
                transactions.push(record);
            }
        }

        // Sort by creation date (newest first)
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(transactions)
    }

    pub fn insert(&self, transaction: UnsignedTransactionInfo, timestamp: u64) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(UNSIGNED_TRANSACTIONS.clone())?;
            let txid = transaction.txid.clone();
            let key = txid.to_bytes();
            
            // Check if transaction already exists
            if table.get(key.as_slice())?.is_some() {
                return Err(Error::AlreadyExists(format!("Transaction with ID {} already exists", txid)));
            }
            
            let record = Record::new(timestamp, transaction, timestamp);
            let record_bytes = cbor::serialize(&record)?;
            
            table.insert(key.as_slice(), record_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn delete(&self, txid: &TxId) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(UNSIGNED_TRANSACTIONS.clone())?;
            let key = txid.to_bytes();
            
            if table.get(key.as_slice())?.is_none() {
                return Err(Error::NotFound(format!("Transaction with ID {} not found", txid)));
            }
            
            table.remove(key.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[uniffi::export]
impl UnsignedTransactionsTable {
    #[uniffi::method]
    pub fn get_transaction(&self, txid: TxId) -> Option<Record<UnsignedTransactionInfo>> {
        self.get(&txid).unwrap_or(None)
    }

    #[uniffi::method]
    pub fn list_transactions_by_wallet(&self, wallet_id: WalletId) -> Vec<Record<UnsignedTransactionInfo>> {
        self.list_by_wallet(&wallet_id).unwrap_or_default()
    }

    #[uniffi::method]
    pub fn insert_transaction(&self, transaction: UnsignedTransactionInfo) -> bool {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
            
        self.insert(transaction, timestamp).is_ok()
    }

    #[uniffi::method]
    pub fn delete_transaction(&self, txid: TxId) -> bool {
        self.delete(&txid).is_ok()
    }
}