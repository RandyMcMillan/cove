use serde::{de::DeserializeOwned, Serialize};

use crate::Error;

pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))
}

pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    serde_json::from_slice(bytes).map_err(|e| Error::Deserialization(e.to_string()))
}