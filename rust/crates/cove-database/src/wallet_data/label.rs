use std::sync::Arc;

use bip329::{Label, Labels, TransactionRecord};
use bitcoin::Txid;
use redb::{Database, TableDefinition, WriteTransaction};

use crate::{cbor, define_table, Error};

define_table!(LABELS, &[u8], &[u8]);

#[derive(Debug, Clone, uniffi::Object)]
pub struct LabelDb {
    db: Arc<Database>,
}

impl LabelDb {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(LABELS.clone());
        Self { db }
    }

    pub fn all_labels_for_txn(&self, txid: Txid) -> Result<Labels, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(LABELS.clone())?;
        let key = txid.to_byte_array();

        match table.get(key.as_slice())? {
            Some(value) => {
                let labels: Vec<Label> = cbor::deserialize(value.value())?;
                Ok(labels.into())
            }
            None => Ok(Labels::default()),
        }
    }

    pub fn add_label(&self, txid: Txid, label: &str) -> Result<(), Error> {
        if label.is_empty() {
            return Ok(());
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(LABELS.clone())?;
            let key = txid.to_byte_array();
            
            let mut labels = match table.get(key.as_slice())? {
                Some(value) => cbor::deserialize::<Vec<Label>>(value.value())?,
                None => Vec::new(),
            };
            
            let new_label = Label::from(TransactionRecord {
                ref_: txid,
                label: Some(label.to_string()),
                origin: None,
            });
            
            // Don't add duplicate labels
            if !labels.iter().any(|l| l.label() == Some(label)) {
                labels.push(new_label);
                let labels_bytes = cbor::serialize(&labels)?;
                table.insert(key.as_slice(), labels_bytes.as_slice())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn remove_label(&self, txid: Txid, label: &str) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(LABELS.clone())?;
            let key = txid.to_byte_array();
            
            if let Some(value) = table.get(key.as_slice())? {
                let mut labels: Vec<Label> = cbor::deserialize(value.value())?;
                
                // Remove the label
                labels.retain(|l| l.label() != Some(label));
                
                if labels.is_empty() {
                    table.remove(key.as_slice())?;
                } else {
                    let labels_bytes = cbor::serialize(&labels)?;
                    table.insert(key.as_slice(), labels_bytes.as_slice())?;
                }
            }
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[uniffi::export]
impl LabelDb {
    #[uniffi::method]
    pub fn get_transaction_label(&self, txid_string: String) -> Option<String> {
        let txid = match bitcoin::Txid::from_str(&txid_string) {
            Ok(txid) => txid,
            Err(_) => return None,
        };
        
        match self.all_labels_for_txn(txid) {
            Ok(labels) => labels.transaction_label().map(|s| s.to_string()),
            Err(_) => None,
        }
    }

    #[uniffi::method]
    pub fn add_transaction_label(&self, txid_string: String, label: String) -> bool {
        let txid = match bitcoin::Txid::from_str(&txid_string) {
            Ok(txid) => txid,
            Err(_) => return false,
        };
        
        self.add_label(txid, &label).is_ok()
    }

    #[uniffi::method]
    pub fn remove_transaction_label(&self, txid_string: String, label: String) -> bool {
        let txid = match bitcoin::Txid::from_str(&txid_string) {
            Ok(txid) => txid,
            Err(_) => return false,
        };
        
        self.remove_label(txid, &label).is_ok()
    }
}

// Helper function for parsing txid from string
fn bitcoin::Txid::from_str(s: &str) -> Result<bitcoin::Txid, bitcoin::hashes::hex::Error> {
    use std::str::FromStr;
    bitcoin::Txid::from_str(s)
}