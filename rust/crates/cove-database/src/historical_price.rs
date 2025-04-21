pub mod record;

use std::sync::Arc;

use redb::{Database, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::{cbor, define_table, Error};

use self::record::HistoricalPriceRecord;

define_table!(HISTORICAL_PRICES, &[u8], &[u8]);

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct HistoricalPrice {
    pub currency: String,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct HistoricalPriceTable {
    pub db: Arc<Database>,
}

impl HistoricalPriceTable {
    pub fn new(db: Arc<Database>, txn: &WriteTransaction) -> Self {
        let _ = txn.open_table(HISTORICAL_PRICES.clone());
        Self { db }
    }

    pub fn insert(&self, record: HistoricalPriceRecord) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(HISTORICAL_PRICES.clone())?;
            let key = record.key();
            let value = cbor::serialize(&record)?;
            table.insert(key.as_slice(), value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_by_timestamp(
        &self,
        currency: &str,
        timestamp: u64,
    ) -> Result<Option<HistoricalPriceRecord>, Error> {
        let key = HistoricalPriceRecord::make_key(currency, timestamp);
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(HISTORICAL_PRICES.clone())?;

        match table.get(key.as_slice())? {
            Some(value) => {
                let record: HistoricalPriceRecord = cbor::deserialize(value.value())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    pub fn get_closest_by_timestamp(
        &self,
        currency: &str,
        timestamp: u64,
        tolerance_seconds: u64,
    ) -> Result<Option<HistoricalPriceRecord>, Error> {
        // Try to get exact match first
        if let Some(record) = self.get_by_timestamp(currency, timestamp)? {
            return Ok(Some(record));
        }

        // If no exact match, find closest within tolerance
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(HISTORICAL_PRICES.clone())?;

        let mut closest_record = None;
        let mut min_diff = tolerance_seconds;

        for result in table.iter()? {
            let (_, value) = result?;
            let record: HistoricalPriceRecord = cbor::deserialize(value.value())?;
            
            if record.currency != currency {
                continue;
            }

            let time_diff = if record.timestamp > timestamp {
                record.timestamp - timestamp
            } else {
                timestamp - record.timestamp
            };

            if time_diff <= min_diff {
                min_diff = time_diff;
                closest_record = Some(record);
            }
        }

        Ok(closest_record)
    }
}

#[uniffi::export]
impl HistoricalPriceTable {
    #[uniffi::method]
    pub fn store_price(&self, currency: String, price: f64, timestamp: u64) -> bool {
        let record = HistoricalPriceRecord {
            currency,
            price,
            timestamp,
        };
        
        self.insert(record).is_ok()
    }

    #[uniffi::method]
    pub fn get_price_at_time(&self, currency: String, timestamp: u64) -> Option<HistoricalPriceRecord> {
        self.get_by_timestamp(&currency, timestamp).unwrap_or(None)
    }

    #[uniffi::method]
    pub fn get_closest_price(&self, currency: String, timestamp: u64) -> Option<HistoricalPriceRecord> {
        // Allow 12 hour tolerance
        const TWELVE_HOURS_SECONDS: u64 = 12 * 60 * 60;
        
        self.get_closest_by_timestamp(&currency, timestamp, TWELVE_HOURS_SECONDS)
            .unwrap_or(None)
    }
}