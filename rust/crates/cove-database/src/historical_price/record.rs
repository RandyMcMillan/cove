use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct HistoricalPriceRecord {
    pub currency: String,
    pub price: f64,
    pub timestamp: u64,
}

impl HistoricalPriceRecord {
    pub fn key(&self) -> Vec<u8> {
        Self::make_key(&self.currency, self.timestamp)
    }

    pub fn make_key(currency: &str, timestamp: u64) -> Vec<u8> {
        let mut key = Vec::with_capacity(currency.len() + 8);
        key.extend_from_slice(currency.as_bytes());
        key.extend_from_slice(&timestamp.to_be_bytes());
        key
    }
}