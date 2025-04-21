use std::sync::Arc;

use redb::{Database, TableDefinition, WriteTransaction};

use crate::{cbor, define_table, Error};

define_table!(CACHE, &str, &[u8]);

#[derive(Debug, Clone, uniffi::Object)]
pub struct GlobalCacheTable {
    pub db: Arc<Database>,
}

impl GlobalCacheTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(CACHE.clone());
        Self { db }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CACHE.clone())?;

        match table.get(key)? {
            Some(value) => {
                let value: String = cbor::deserialize(value.value())?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CACHE.clone())?;
            let value_bytes = cbor::serialize(&value)?;
            table.insert(key, value_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CACHE.clone())?;
            if table.get(key)?.is_some() {
                table.remove(key)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[uniffi::export]
impl GlobalCacheTable {
    #[uniffi::method]
    pub fn get_value(&self, key: String) -> Option<String> {
        self.get(&key).unwrap_or(None)
    }

    #[uniffi::method]
    pub fn set_value(&self, key: String, value: String) -> bool {
        self.set(&key, &value).is_ok()
    }

    #[uniffi::method]
    pub fn remove_value(&self, key: String) -> bool {
        self.remove(&key).is_ok()
    }
}