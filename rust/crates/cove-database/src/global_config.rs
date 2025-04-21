use std::sync::Arc;

use redb::{Database, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::{cbor, define_table, Error};

define_table!(CONFIG, &str, &[u8]);

#[derive(Debug, Clone, uniffi::Object)]
pub struct GlobalConfigTable {
    pub db: Arc<Database>,
}

impl GlobalConfigTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(CONFIG.clone());
        Self { db }
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CONFIG.clone())?;

        match table.get(key)? {
            Some(value) => {
                let value: T = cbor::deserialize(value.value())?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CONFIG.clone())?;
            let value_bytes = cbor::serialize(value)?;
            table.insert(key, value_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CONFIG.clone())?;
            if table.get(key)?.is_some() {
                table.remove(key)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }
    
    // Common config getters/setters
    pub fn node_url(&self) -> Result<Option<String>, Error> {
        self.get("node_url")
    }
    
    pub fn set_node_url(&self, url: &str) -> Result<(), Error> {
        self.set("node_url", &url)
    }
    
    pub fn fiat_currency(&self) -> Result<String, Error> {
        self.get("fiat_currency").map(|v| v.unwrap_or_else(|| "USD".to_string()))
    }
    
    pub fn set_fiat_currency(&self, currency: &str) -> Result<(), Error> {
        self.set("fiat_currency", &currency)
    }
}

#[uniffi::export]
impl GlobalConfigTable {
    #[uniffi::method]
    pub fn get_node_url(&self) -> Option<String> {
        self.node_url().unwrap_or(None)
    }

    #[uniffi::method]
    pub fn set_node_url(&self, url: String) -> bool {
        self.set_node_url(&url).is_ok()
    }

    #[uniffi::method]
    pub fn get_fiat_currency(&self) -> String {
        self.fiat_currency().unwrap_or_else(|_| "USD".to_string())
    }

    #[uniffi::method]
    pub fn set_fiat_currency(&self, currency: String) -> bool {
        self.set_fiat_currency(&currency).is_ok()
    }
}