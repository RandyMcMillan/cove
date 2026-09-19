use std::path::PathBuf;

use rbitcoin_node::{DatadirOpts, ListenOpts, MempoolOpts, NodeConfig, RpcOpts};
use rbitcoin_primitives::Network as RbitcoinNetwork;
use rbitcoin_store::HeadScale;

use cove_common::consts::ROOT_DATA_DIR;
use cove_types::network::Network;

use crate::error::LocalNodeError;

/// Build a `NodeConfig` for the given Cove network.
pub fn build_config(network: Network) -> Result<NodeConfig, LocalNodeError> {
    let rbitcoin_network = map_network(network)?;
    let datadir = datadir_for_network(network);

    let localhost = |port: u16| format!("127.0.0.1:{port}").parse().expect("hardcoded valid address");

    let mut config = NodeConfig {
        datadir: DatadirOpts { path: datadir, cold: None },
        listen: ListenOpts {
            p2p: Some(localhost(0)),
            electrum: Some(localhost(0)),
            esplora: Some(localhost(0)),
            ..ListenOpts::default()
        },
        mempool: MempoolOpts {
            persist: true,
            ..MempoolOpts::default()
        },
        rpc: RpcOpts::default(),
        network: rbitcoin_network,
        shindex: true,
        prefill_compact: true,
        ..NodeConfig::default()
    };

    // Use a smaller head scale for non-mainnet networks to reduce allocation
    if rbitcoin_network != RbitcoinNetwork::Mainnet {
        config.head_scale = HeadScale::Tiny;
    }

    Ok(config)
}

/// Return the datadir path for the given network.
pub fn datadir_for_network(network: Network) -> PathBuf {
    ROOT_DATA_DIR.join("rbitcoin").join(network_dir_name(network))
}

/// Map Cove's network to rbitcoin's network.
fn map_network(network: Network) -> Result<RbitcoinNetwork, LocalNodeError> {
    match network {
        Network::Bitcoin => Ok(RbitcoinNetwork::Mainnet),
        Network::Testnet => Ok(RbitcoinNetwork::Testnet),
        Network::Signet => Ok(RbitcoinNetwork::Signet),
        Network::Testnet4 => Err(LocalNodeError::UnsupportedNetwork(
            "testnet4 is not supported by rbitcoin v0.7.0".to_string(),
        )),
    }
}

fn network_dir_name(network: Network) -> &'static str {
    match network {
        Network::Bitcoin => "mainnet",
        Network::Testnet => "testnet",
        Network::Testnet4 => "testnet4",
        Network::Signet => "signet",
    }
}

/// Resolved URLs for a running local node.
#[derive(Debug, Clone)]
pub struct LocalNodeUrls {
    pub electrum: String,
    pub esplora: String,
}
