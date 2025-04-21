use crate::{client::PRICES, currency::FiatCurrency, error::FiatAmountError};

type Result<T, E = FiatAmountError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct FiatAmount {
    pub amount: f64,
    pub currency: FiatCurrency,
}

impl FiatAmount {
    /// Tries to create a new FiatAmount from a BTC amount and currency
    /// 
    /// This is the main implementation method used by external code
    pub fn try_new<T>(amount: &T, currency: FiatCurrency) -> Result<Self> 
    where 
        T: AsRef<f64>,
    {
        let btc_amount = *amount.as_ref();
        Self::try_new_from_btc(btc_amount, currency)
    }
    
    /// Tries to create a new FiatAmount from a BTC amount and currency
    pub fn try_new_from_btc(btc_amount: f64, currency: FiatCurrency) -> Result<Self> {
        let prices = PRICES.load().as_ref().ok_or_else(|| {
            // In actual code, we would trigger price update here
            FiatAmountError::PricesUnavailable("prices not available".to_string())
        })?;

        let fiat = btc_amount * prices.get_for_currency(currency) as f64;

        Ok(Self {
            amount: fiat,
            currency,
        })
    }
}

// PREVIEW ONLY
//
impl FiatAmount {
    pub fn preview_new() -> Self {
        Self {
            amount: 120.38,
            currency: FiatCurrency::Usd,
        }
    }
}

#[uniffi::export]
pub fn fiat_amount_preview_new() -> FiatAmount {
    FiatAmount::preview_new()
}

impl Eq for FiatAmount {}
impl PartialEq for FiatAmount {
    fn eq(&self, other: &Self) -> bool {
        self.amount.ceil() == other.amount.ceil() && self.currency == other.currency
    }
}