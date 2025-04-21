use bitcoin::OutPoint as BtcOutPoint;

use crate::key::{FromBytes, IntoBytes};

impl IntoBytes for BtcOutPoint {
    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(36);
        bytes.extend_from_slice(&self.txid.to_byte_array());
        bytes.extend_from_slice(&self.vout.to_le_bytes());
        bytes
    }
}

impl FromBytes for BtcOutPoint {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 36 {
            return None;
        }

        let txid = bitcoin::Txid::from_slice(&bytes[0..32]).ok()?;
        let vout = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);

        Some(BtcOutPoint { txid, vout })
    }
}