use bdk_wallet::bitcoin::Amount as BdkAmount;
use numfmt::{Formatter, Precision};
use serde::{Deserialize, Serialize};

use super::Unit;
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Object,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Mul,
    derive_more::From,
    derive_more::Into,
    derive_more::Deref,
)]
pub struct Amount(pub BdkAmount);

impl AsRef<f64> for Amount {
    fn as_ref(&self) -> &f64 {
        // This is a bit of a hack, but it works for our use case
        // We need to return a reference to a f64 value
        // Since as_btc() returns a copy, we need to store it somewhere
        // Using a thread_local for this
        thread_local! {
            static TEMP_F64: std::cell::Cell<f64> = std::cell::Cell::new(0.0);
        }

        TEMP_F64.with(|cell| {
            cell.set(self.as_btc());
            // This is unsafe but works for our specific use case
            // We're converting the Cell reference to a raw reference
            // because AsRef needs to return a reference
            unsafe { &*(cell as *const _ as *const f64) }
        })
    }
}

// rust only
impl Amount {
    pub fn from_btc(btc: f64) -> Result<Self, eyre::Report> {
        Ok(Self(bitcoin::Amount::from_btc(btc)?))
    }
}

#[uniffi::export]
impl Amount {
    #[uniffi::constructor]
    pub fn from_sat(sats: u64) -> Self {
        Self(bitcoin::Amount::from_sat(sats))
    }

    #[uniffi::constructor]
    pub fn one_btc() -> Self {
        Self(bitcoin::Amount::ONE_BTC)
    }

    #[uniffi::constructor]
    pub fn one_sat() -> Self {
        Self(bitcoin::Amount::ONE_SAT)
    }

    pub fn as_btc(&self) -> f64 {
        self.0.to_btc()
    }

    pub fn fmt_string_with_unit(&self, unit: Unit) -> String {
        match unit {
            Unit::Btc => self.btc_string_with_unit(),
            Unit::Sat => self.sats_string_with_unit(),
        }
    }

    pub fn as_sats(&self) -> u64 {
        self.0.to_sat()
    }

    pub fn btc_string(&self) -> String {
        format!("{:.8}", self.as_btc())
    }

    pub fn btc_string_with_unit(&self) -> String {
        format!("{:.8} BTC", self.as_btc())
    }

    pub fn sats_string(&self) -> String {
        let mut f = Formatter::new()
            .separator(',')
            .unwrap()
            .precision(Precision::Decimals(0));

        f.fmt(self.as_sats() as f64).to_string()
    }

    pub fn sats_string_with_unit(&self) -> String {
        format!("{} SATS", self.sats_string())
    }
}
