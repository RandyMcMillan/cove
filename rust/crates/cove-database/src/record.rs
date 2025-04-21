use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct Record<T> {
    pub id: u64,
    pub data: T,
    pub created_at: u64,
    pub updated_at: u64,
}

impl<T> Record<T> {
    pub fn new(id: u64, data: T, timestamp: u64) -> Self {
        Self {
            id,
            data,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    pub fn update(&mut self, data: T, timestamp: u64) {
        self.data = data;
        self.updated_at = timestamp;
    }
}