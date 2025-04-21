use std::sync::Arc;

use cove_types::WalletId;
use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::{cbor, define_table, Error, Record};

define_table!(WALLETS, &[u8], &[u8]);

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct WalletInfo {
    pub id: WalletId,
    pub name: String,
    pub created_at: u64,
    pub color: String,
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct WalletsTable {
    pub db: Arc<Database>,
}

impl WalletsTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(WALLETS.clone());
        Self { db }
    }

    pub fn get(&self, id: &WalletId) -> Result<Option<Record<WalletInfo>>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(WALLETS.clone())?;
        let key = id.to_bytes();

        match table.get(key.as_slice())? {
            Some(value) => {
                let record: Record<WalletInfo> = cbor::deserialize(value.value())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    pub fn list(&self) -> Result<Vec<Record<WalletInfo>>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(WALLETS.clone())?;

        let mut wallets = Vec::new();
        for result in table.iter()? {
            let (_, value) = result?;
            let record: Record<WalletInfo> = cbor::deserialize(value.value())?;
            wallets.push(record);
        }

        // Sort by creation date (newest first)
        wallets.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(wallets)
    }

    pub fn insert(&self, wallet: WalletInfo, timestamp: u64) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(WALLETS.clone())?;
            let id = wallet.id.clone();
            let key = id.to_bytes();
            
            // Check if wallet already exists
            if table.get(key.as_slice())?.is_some() {
                return Err(Error::AlreadyExists(format!("Wallet with ID {} already exists", id)));
            }
            
            let record = Record::new(timestamp, wallet, timestamp);
            let record_bytes = cbor::serialize(&record)?;
            
            table.insert(key.as_slice(), record_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn update(&self, wallet: WalletInfo, timestamp: u64) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(WALLETS.clone())?;
            let id = wallet.id.clone();
            let key = id.to_bytes();
            
            // Check if wallet exists
            let existing = match table.get(key.as_slice())? {
                Some(value) => cbor::deserialize::<Record<WalletInfo>>(value.value())?,
                None => return Err(Error::NotFound(format!("Wallet with ID {} not found", id))),
            };
            
            // Update the record
            let mut record = existing;
            record.update(wallet, timestamp);
            
            let record_bytes = cbor::serialize(&record)?;
            table.insert(key.as_slice(), record_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn delete(&self, id: &WalletId) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(WALLETS.clone())?;
            let key = id.to_bytes();
            
            if table.get(key.as_slice())?.is_none() {
                return Err(Error::NotFound(format!("Wallet with ID {} not found", id)));
            }
            
            table.remove(key.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[uniffi::export]
impl WalletsTable {
    #[uniffi::method]
    pub fn get_wallet(&self, id: WalletId) -> Option<Record<WalletInfo>> {
        self.get(&id).unwrap_or(None)
    }

    #[uniffi::method]
    pub fn list_wallets(&self) -> Vec<Record<WalletInfo>> {
        self.list().unwrap_or_default()
    }

    #[uniffi::method]
    pub fn insert_wallet(&self, wallet: WalletInfo) -> bool {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
            
        self.insert(wallet, timestamp).is_ok()
    }

    #[uniffi::method]
    pub fn update_wallet(&self, wallet: WalletInfo) -> bool {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
            
        self.update(wallet, timestamp).is_ok()
    }

    #[uniffi::method]
    pub fn delete_wallet(&self, id: WalletId) -> bool {
        self.delete(&id).is_ok()
    }
}