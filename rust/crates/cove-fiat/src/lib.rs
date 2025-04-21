uniffi::setup_scaffolding!();

pub mod amount;
pub mod client;
pub mod currency;
pub mod error;
pub mod historical;

pub use amount::FiatAmount;
pub use client::{FiatClient, PriceResponse, init_prices, update_prices_if_needed};
pub use currency::{
    FiatCurrency, all_fiat_currencies, fiat_currency_emoji, fiat_currency_suffix,
    fiat_currency_symbol, fiat_currency_to_string, is_fiat_currency_symbol,
};
pub use error::FiatAmountError;
pub use historical::{HistoricalPrice, HistoricalPricesResponse};