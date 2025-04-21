use std::{
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use arc_swap::ArcSwap;
use eyre::Result;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::currency::FiatCurrency;
use macros::impl_default_for;

use super::historical::HistoricalPricesResponse;

const CURRENCY_URL: &str = "https://mempool.space/api/v1/prices";
const HISTORICAL_PRICES_URL: &str = "https://mempool.space/api/v1/historical-price";

const ONE_MIN: u64 = 60;

// Global client for getting prices
pub static FIAT_CLIENT: LazyLock<FiatClient> = LazyLock::new(FiatClient::new);

pub static PRICES: LazyLock<ArcSwap<Option<PriceResponse>>> =
    LazyLock::new(|| ArcSwap::from_pointee(None));

// Simple global cache for database access
// In a real implementation, we would use proper dependency injection
// This is just to break the circular dependency for extraction
static GLOBAL_CACHE: LazyLock<Mutex<Option<PriceResponse>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, uniffi::Object)]
pub struct FiatClient {
    url: String,
    client: reqwest::Client,
    wait_before_new_prices: u64,
}

#[derive(
    Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, uniffi::Object,
)]
#[serde(rename_all = "UPPERCASE")]
pub struct PriceResponse {
    #[serde(rename = "time")]
    pub time: u64,
    pub usd: u64,
    pub eur: u64,
    pub gbp: u64,
    pub cad: u64,
    pub chf: u64,
    pub aud: u64,
    pub jpy: u64,
}

#[uniffi::export]
impl PriceResponse {
    pub fn get(&self) -> u64 {
        // In the real implementation, this would use Database::global()
        // For now, we'll default to USD to break the circular dependency
        self.get_for_currency(FiatCurrency::Usd)
    }

    pub fn get_for_currency(&self, currency: FiatCurrency) -> u64 {
        match currency {
            FiatCurrency::Usd => self.usd,
            FiatCurrency::Eur => self.eur,
            FiatCurrency::Gbp => self.gbp,
            FiatCurrency::Cad => self.cad,
            FiatCurrency::Chf => self.chf,
            FiatCurrency::Aud => self.aud,
            FiatCurrency::Jpy => self.jpy,
        }
    }
}

impl_default_for!(FiatClient);

impl FiatClient {
    fn new() -> Self {
        Self {
            url: CURRENCY_URL.to_string(),
            client: reqwest::Client::new(),
            wait_before_new_prices: ONE_MIN,
        }
    }

    #[allow(dead_code)]
    fn new_with_url(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
            wait_before_new_prices: ONE_MIN,
        }
    }

    /// Get historical price and exchange rates for a specific timestamp
    /// - timestamp: Unix timestamp in seconds for which to get the price
    pub async fn historical_prices(
        &self,
        timestamp: u64,
    ) -> Result<HistoricalPricesResponse, reqwest::Error> {
        let url = format!("{}?timestamp={}", HISTORICAL_PRICES_URL, timestamp);

        let response = self.client.get(&url).send().await?;
        let historical_prices: HistoricalPricesResponse = response.json().await?;

        Ok(historical_prices)
    }

    /// Get historical price for Bitcoin in the requested currency at a given timestamp
    pub async fn historical_price_for_currency(
        &self,
        timestamp: u64,
        currency: FiatCurrency,
    ) -> Result<Option<f32>, reqwest::Error> {
        let historical_data = self.historical_prices(timestamp).await?;
        Ok(historical_data.for_currency(currency))
    }

    /// Convert the BTC amount to the requested currency using a historical price
    pub async fn historical_value_in_currency(
        &self,
        btc_amount: f64,
        currency: FiatCurrency,
        timestamp: u64,
    ) -> Result<Option<f64>, reqwest::Error> {
        let price = self
            .historical_price_for_currency(timestamp, currency)
            .await?;

        if price.is_none() {
            return Ok(None);
        }

        let value_in_currency = btc_amount * price.expect("price is some") as f64;
        Ok(Some(value_in_currency))
    }

    /// Get the current price for a currency
    pub async fn prices(&self) -> Result<PriceResponse, reqwest::Error> {
        if let Some(prices) = PRICES.load().as_ref() {
            let now_secs = Timestamp::now().as_second() as u64;
            if now_secs - prices.time < self.wait_before_new_prices {
                return Ok(*prices);
            }
        }

        let response = self.client.get(&self.url).send().await?;
        let prices: PriceResponse = response.json().await?;

        // update global prices
        if let Err(error) = update_prices(prices) {
            error!("unable to update prices: {error:?}");
        }

        Ok(prices)
    }

    /// Get the current price for a currency
    async fn price_for(&self, currency: FiatCurrency) -> Result<u64, reqwest::Error> {
        let prices = self.prices().await?;

        let price = match currency {
            FiatCurrency::Usd => prices.usd,
            FiatCurrency::Eur => prices.eur,
            FiatCurrency::Gbp => prices.gbp,
            FiatCurrency::Cad => prices.cad,
            FiatCurrency::Chf => prices.chf,
            FiatCurrency::Aud => prices.aud,
            FiatCurrency::Jpy => prices.jpy,
        };

        Ok(price)
    }

    /// Convert the BTC amount to the requested currency using the current price
    pub async fn current_value_in_currency<T>(
        &self,
        amount: T,
        currency: FiatCurrency,
    ) -> Result<f64, reqwest::Error>
    where
        T: AsRef<f64>,
    {
        let btc_amount = *amount.as_ref();
        let price = self.price_for(currency).await?;
        let value_in_currency = btc_amount * price as f64;

        Ok(value_in_currency)
    }
}

/// Get prices from the server, and save them in the database and cache in memory
pub async fn init_prices() -> Result<()> {
    let fiat_client = &FIAT_CLIENT;

    let prices = tryhard::retry_fn(|| fiat_client.prices())
        .retries(20)
        .exponential_backoff(Duration::from_millis(10))
        .max_delay(Duration::from_secs(5))
        .await;

    match prices {
        Ok(prices) => {
            PRICES.swap(Arc::new(Some(prices)));

            // Also store in our simple cache
            let mut cache = GLOBAL_CACHE.lock().unwrap();
            *cache = Some(prices);
        }

        Err(error) => {
            warn!("Unable to get prices: {error:?}, using last known prices");

            // Check if we have prices in our simple cache
            let cache = GLOBAL_CACHE.lock().unwrap();
            if let Some(prices) = *cache {
                PRICES.swap(Arc::new(Some(prices)));
            }
        }
    }

    Ok(())
}

/// update price in database and cache
fn update_prices(prices: PriceResponse) -> Result<()> {
    PRICES.swap(Arc::new(Some(prices)));

    // Update our simple cache
    let mut cache = GLOBAL_CACHE.lock().unwrap();
    *cache = Some(prices);

    Ok(())
}

/// Update prices if needed
pub async fn update_prices_if_needed() -> Result<()> {
    if let Some(prices) = PRICES.load().as_ref() {
        let now_secs = Timestamp::now().as_second() as u64;
        if now_secs - prices.time < ONE_MIN {
            return Ok(());
        }
    }

    let fiat_client = &FIAT_CLIENT;
    let prices = tryhard::retry_fn(|| fiat_client.prices())
        .retries(5)
        .exponential_backoff(Duration::from_millis(10))
        .max_delay(Duration::from_secs(1))
        .await?;

    update_prices(prices)?;

    Ok(())
}

mod ffi {
    use tracing::error;

    #[uniffi::export]
    pub async fn update_prices_if_needed() {
        if let Err(error) = super::update_prices_if_needed().await {
            error!("unable to update prices: {error:?}");
        }
    }
}

#[uniffi::export]
fn prices_are_equal(lhs: Arc<PriceResponse>, rhs: Arc<PriceResponse>) -> bool {
    lhs == rhs
}

