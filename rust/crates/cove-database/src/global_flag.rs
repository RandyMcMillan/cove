use std::sync::Arc;

use redb::{Database, TableDefinition, WriteTransaction};
use uniffi::Object;

use crate::{cbor, define_table, Error};

define_table!(FLAGS, &str, &[u8]);

#[derive(Debug, Clone, uniffi::Object)]
pub struct GlobalFlagTable {
    pub db: Arc<Database>,
}

impl GlobalFlagTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(FLAGS.clone());
        Self { db }
    }

    pub fn get(&self, key: &str) -> Result<bool, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(FLAGS.clone())?;

        match table.get(key)? {
            Some(value) => {
                let value: bool = cbor::deserialize(value.value())?;
                Ok(value)
            }
            None => Ok(false),
        }
    }

    pub fn set(&self, key: &str, value: bool) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FLAGS.clone())?;
            let value_bytes = cbor::serialize(&value)?;
            table.insert(key, value_bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[uniffi::export]
impl GlobalFlagTable {
    #[uniffi::method]
    pub fn get_flag(&self, key: String) -> bool {
        self.get(&key).unwrap_or(false)
    }

    #[uniffi::method]
    pub fn set_flag(&self, key: String, value: bool) -> bool {
        self.set(&key, value).is_ok()
    }
}