use std::sync::{Arc, LazyLock};

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{
    database::Database,
    network::Network,
    node::{ApiType, Node},
    node_connect::LOCAL_NODE_NAME,
};
use cove_rbitcoin::{LocalNode, LocalNodeError, LocalNodeUrls};

pub static LOCAL_NODE_MANAGER: LazyLock<Arc<LocalNodeManager>> =
    LazyLock::new(|| Arc::new(LocalNodeManager::new()));

pub struct LocalNodeManager {
    node: Arc<RwLock<Option<LocalNode>>>,
    config: Option<cove_rbitcoin::LocalNodeConfig>,
}

impl LocalNodeManager {
    pub fn new() -> Self {
        Self { node: Arc::new(RwLock::new(None)), config: None }
    }

    #[allow(dead_code)]
    pub fn set_config(&mut self, config: cove_rbitcoin::LocalNodeConfig) {
        self.config = Some(config);
    }

    #[allow(dead_code)]
    pub fn clear_config(&mut self) {
        self.config = None;
    }

    pub async fn is_running(&self) -> bool {
        self.node.read().await.as_ref().is_some_and(LocalNode::is_running)
    }

    pub async fn urls(&self) -> Option<LocalNodeUrls> {
        self.node.read().await.as_ref().and_then(LocalNode::urls).cloned()
    }

    pub async fn tip_height(&self) -> Option<u32> {
        self.node.read().await.as_ref().and_then(LocalNode::tip_height)
    }

    pub async fn is_in_ibd(&self) -> Option<bool> {
        self.node.read().await.as_ref().and_then(LocalNode::is_in_ibd)
    }

    pub async fn peer_count(&self) -> Option<u32> {
        self.node.read().await.as_ref().and_then(LocalNode::peer_count)
    }

    pub async fn datadir_size(&self) -> Result<u64, LocalNodeError> {
        let network = {
            let guard = self.node.read().await;
            guard.as_ref().map(LocalNode::network)
        };
        let network =
            network.unwrap_or_else(|| Database::global().global_config.selected_network());
        let path = cove_rbitcoin::datadir_for_network(network);
        if !path.exists() {
            return Ok(0);
        }
        cove_rbitcoin::dir_size(&path)
    }

    pub async fn clear_datadir(&self) -> Result<(), LocalNodeError> {
        self.stop().await;
        let network = Database::global().global_config.selected_network();
        let path = cove_rbitcoin::datadir_for_network(network);
        if path.exists() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| LocalNodeError::DatadirRemove(format!("{}: {e}", path.display())))?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn network(&self) -> Option<Network> {
        self.node.read().await.as_ref().map(LocalNode::network)
    }

    pub async fn running_network(&self) -> Option<Network> {
        self.node.read().await.as_ref().filter(|n| n.is_running()).map(LocalNode::network)
    }

    /// Start the local node for `network` if it is not already running.
    ///
    /// The node is created under a write lock, but the lock is released while
    /// `LocalNode::start()` performs its long-running initialization so that
    /// status observers can poll `is_running()` without blocking for minutes.
    pub async fn start(&self, network: Network) -> Result<(), LocalNodeError> {
        // Remove a stale or wrong-network node before releasing the lock.
        {
            let mut guard = self.node.write().await;
            if let Some(ref node) = *guard {
                if node.is_running() && node.network() == network {
                    debug!("local node already running for {network}");
                    return Ok(());
                }

                if node.is_running() {
                    info!("stopping local node for {} to start {}", node.network(), network);
                    let mut node = guard.take().expect("node present");
                    drop(guard);
                    node.stop().await;
                } else {
                    guard.take();
                }
            }
        }

        rbitcoin_log::global_capture_logs(true);

        let mut node = LocalNode::new(network);
        node.start(self.config.clone()).await?;

        {
            let mut guard = self.node.write().await;
            *guard = Some(node);
        }

        Ok(())
    }

    pub async fn stop(&self) {
        let mut guard = self.node.write().await;
        if let Some(mut node) = guard.take() {
            node.stop().await;
        }
        rbitcoin_log::global_capture_logs(false);
    }

    /// Wait for the RPC endpoint to accept connections and return the URLs.
    pub async fn wait_for_ready(&self, timeout_secs: u64) -> Result<LocalNodeUrls, LocalNodeError> {
        let guard = self.node.read().await;
        let node = guard.as_ref().ok_or_else(|| LocalNodeError::NotRunning)?;
        node.wait_for_ready(timeout_secs).await.map(|urls| urls.clone())
    }

    /// Return true if both electrum and esplora ports are currently accepting
    /// connections.
    pub async fn are_endpoints_ready(&self) -> bool {
        let guard = self.node.read().await;
        match guard.as_ref() {
            Some(node) => node.are_endpoints_ready().await,
            None => false,
        }
    }
}

/// Resolve the currently selected node, starting the local node if necessary.
///
/// When the user has selected the local node, this ensures it is running and
/// returns a `Node` with the dynamically allocated URL. Otherwise returns the
/// persisted remote node.
pub async fn resolve_selected_node() -> Result<Node, LocalNodeError> {
    let global_config = &Database::global().global_config;
    let network = global_config.selected_network();

    if !global_config.selected_node_is_local() {
        return Ok(global_config.selected_node());
    }

    let manager = &LOCAL_NODE_MANAGER;

    {
        let guard = manager.node.read().await;
        if let Some(ref node) = *guard {
            if !node.is_running() || node.network() != network {
                drop(guard);
                manager.stop().await;
            }
        }
    }

    if !manager.is_running().await {
        manager.start(network).await?;
    }

    let urls = manager.urls().await.ok_or_else(|| LocalNodeError::NotRunning)?;

    let (api_type, url) = match network {
        Network::Bitcoin | Network::Testnet => (ApiType::Electrum, urls.electrum),
        Network::Signet => (ApiType::Esplora, urls.esplora),
        Network::Testnet4 => {
            return Err(LocalNodeError::UnsupportedNetwork(
                "testnet4 is not supported by rbitcoin".to_string(),
            ));
        }
    };

    let node = Node { name: LOCAL_NODE_NAME.to_string(), network, api_type, url };

    Ok(node)
}

/// Return the connection identity for the currently selected node without
/// starting the local node.
///
/// Used by sync code paths that only need to compare identities. If the local
/// node is selected, returns a stable placeholder identity so that in-flight
/// refreshes are not incorrectly discarded when the dynamic URL changes.
pub fn selected_node_identity_placeholder() -> Node {
    let global_config = &Database::global().global_config;
    let network = global_config.selected_network();

    if !global_config.selected_node_is_local() {
        return global_config.selected_node();
    }

    let api_type = match network {
        Network::Bitcoin | Network::Testnet => ApiType::Electrum,
        Network::Signet => ApiType::Esplora,
        Network::Testnet4 => ApiType::Esplora,
    };

    Node { name: LOCAL_NODE_NAME.to_string(), network, api_type, url: "local://".to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manager_starts_inactive() {
        let manager = LocalNodeManager::new();
        assert!(!manager.is_running().await);
        assert!(manager.urls().await.is_none());
        assert!(manager.network().await.is_none());
    }
}
