#[derive(Debug, thiserror::Error, derive_more::Display, uniffi::Error)]
pub enum FiatAmountError {
    /// Unable to convert to fiat amount, prices client unavailable {0}
    PricesUnavailable(String),
}