use std::path::PathBuf;

use rbitcoin_node::{DatadirOpts, ListenOpts, MempoolOpts, NodeConfig, RpcOpts};
use rbitcoin_primitives::Network as RbitcoinNetwork;
use rbitcoin_store::HeadScale;

use cove_common::consts::ROOT_DATA_DIR;
use cove_types::network::Network;

use crate::error::LocalNodeError;

/// User-customizable settings for the local rbitcoin node.
///
/// These are tuned for mobile: low peer counts and bandwidth-saving
/// defaults. Fields are optional so callers can override only what they
/// care about and leave everything else at the mobile-friendly default.
#[derive(Debug, Clone, Default)]
pub struct LocalNodeConfig {
    /// Maximum outbound peer connections. Default 4 (mobile-friendly).
    pub max_outbound: Option<u32>,
    /// `true` disables transaction relay to/from peers, saving bandwidth.
    pub blocksonly: Option<bool>,
}

impl LocalNodeConfig {
    /// Mobile-optimized defaults.
    pub fn mobile_default() -> Self {
        Self { max_outbound: Some(4), blocksonly: Some(false) }
    }
}

/// Build a `NodeConfig` for the given Cove network.
pub fn build_config(network: Network) -> Result<NodeConfig, LocalNodeError> {
    build_config_with_options(network, None)
}

/// Build a `NodeConfig` with optional user overrides.
pub fn build_config_with_options(
    network: Network,
    options: Option<LocalNodeConfig>,
) -> Result<NodeConfig, LocalNodeError> {
    let rbitcoin_network = map_network(network)?;
    let datadir = datadir_for_network(network);
    let opts = options.unwrap_or_else(LocalNodeConfig::mobile_default);

    let localhost =
        |port: u16| format!("127.0.0.1:{port}").parse().expect("hardcoded valid address");

    let mut config = NodeConfig {
        datadir: DatadirOpts { path: datadir, cold: None },
        listen: ListenOpts {
            p2p: Some(localhost(0)),
            electrum: Some(localhost(0)),
            esplora: Some(localhost(0)),
            max_outbound: opts.max_outbound.unwrap_or(4),
            ..ListenOpts::default()
        },
        mempool: MempoolOpts { persist: true, ..MempoolOpts::default() },
        rpc: RpcOpts::default(),
        network: rbitcoin_network,
        shindex: true,
        prefill_compact: true,
        ..NodeConfig::default()
    };

    if let Some(true) = opts.blocksonly {
        config.mempool.blocksonly = true;
    }

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
        Network::Testnet4 => Ok(RbitcoinNetwork::Testnet4),
        Network::Signet => Ok(RbitcoinNetwork::Signet),
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
