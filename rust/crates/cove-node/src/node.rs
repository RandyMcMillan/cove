use cove_types::Network;
use serde::{Deserialize, Serialize};

use crate::{
    client::NodeClient,
    constants::{BITCOIN_ELECTRUM, SIGNET_ESPLORA, TESTNET_ESPLORA},
    error::Error,
};

// Node selection type - needed for conversions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeSelection {
    Preset(Node),
    Custom(Node),
}

impl From<NodeSelection> for Node {
    fn from(node: NodeSelection) -> Self {
        match node {
            NodeSelection::Preset(node) => node,
            NodeSelection::Custom(node) => node,
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Hash,
    Eq,
    PartialEq,
    derive_more::Display,
    strum::EnumIter,
    uniffi::Enum,
    Serialize,
    Deserialize,
)]
pub enum ApiType {
    Esplora,
    Electrum,
    Rpc,
}

#[derive(
    Debug, Clone, Hash, Eq, PartialEq, uniffi::Record, Serialize, Deserialize,
)]
pub struct Node {
    pub name: String,
    pub network: Network,
    pub api_type: ApiType,
    pub url: String,
}

impl Node {
    pub fn default(network: Network) -> Self {
        match network {
            Network::Bitcoin => {
                let (name, url) = BITCOIN_ELECTRUM[0];

                Self {
                    name: name.to_string(),
                    network,
                    api_type: ApiType::Electrum,
                    url: url.to_string(),
                }
            }
            Network::Testnet => {
                let (name, url) = TESTNET_ESPLORA[0];
                Self {
                    name: name.to_string(),
                    network,
                    api_type: ApiType::Electrum,
                    url: url.to_string(),
                }
            }

            Network::Signet => {
                let (name, url) = SIGNET_ESPLORA[0];
                Self {
                    name: name.to_string(),
                    network,
                    api_type: ApiType::Esplora,
                    url: url.to_string(),
                }
            }
        }
    }

    pub fn new_electrum(name: String, url: String, network: Network) -> Self {
        Self {
            name,
            network,
            api_type: ApiType::Electrum,
            url,
        }
    }

    pub fn new_esplora(name: String, url: String, network: Network) -> Self {
        Self {
            name,
            network,
            api_type: ApiType::Esplora,
            url,
        }
    }

    pub async fn check_url(&self) -> Result<(), Error> {
        let client = NodeClient::new(self).await?;
        client.check_url().await?;

        Ok(())
    }
}