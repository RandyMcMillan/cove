use bitcoin::{params::Params, NetworkKind, Network as BitcoinNetwork};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    Hash,
    Eq,
    PartialEq,
    uniffi::Enum,
    derive_more::Display,
    strum::EnumIter,
    Serialize,
    Deserialize,
)]
pub enum Network {
    Bitcoin,
    Testnet,
    Signet,
}

use strum::IntoEnumIterator;

#[uniffi::export]
pub fn network_to_string(network: Network) -> String {
    network.to_string()
}

#[uniffi::export]
pub fn all_networks() -> Vec<Network> {
    Network::iter().collect()
}

impl TryFrom<&str> for Network {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "bitcoin" | "Bitcoin" => Ok(Network::Bitcoin),
            "testnet" | "Testnet" => Ok(Network::Testnet),
            "signet" | "Signet" => Ok(Network::Signet),
            "mutinynet" | "Mutinynet" => Ok(Network::Signet),
            _ => Err(format!("Unknown network: {}", value)),
        }
    }
}

impl From<Network> for BitcoinNetwork {
    fn from(network: Network) -> Self {
        match network {
            Network::Bitcoin => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
            Network::Signet => BitcoinNetwork::Signet,
        }
    }
}

impl From<BitcoinNetwork> for Network {
    fn from(network: BitcoinNetwork) -> Self {
        match network {
            BitcoinNetwork::Bitcoin => Network::Bitcoin,
            BitcoinNetwork::Testnet => Network::Testnet,
            BitcoinNetwork::Signet => Network::Signet,
            network => panic!("unsupported network: {network:?}"),
        }
    }
}

impl From<Network> for Params {
    fn from(network: Network) -> Self {
        match network {
            Network::Bitcoin => Params::BITCOIN,
            Network::Testnet => Params::TESTNET3,
            Network::Signet => Params::SIGNET,
        }
    }
}

impl From<Network> for NetworkKind {
    fn from(network: Network) -> Self {
        match network {
            Network::Bitcoin => NetworkKind::Main,
            _ => NetworkKind::Test,
        }
    }
}