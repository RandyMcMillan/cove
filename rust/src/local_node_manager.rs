use std::sync::{Arc, LazyLock};

use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::{
    database::Database,
    network::Network,
    node::{ApiType, Node},
    node_connect::LOCAL_NODE_NAME,
};
use cove_rbitcoin::{LocalNode, LocalNodeError, LocalNodeUrls};

pub static LOCAL_NODE_MANAGER: LazyLock<Arc<Mutex<LocalNodeManager>>> =
    LazyLock::new(|| Arc::new(Mutex::new(LocalNodeManager::new())));

pub struct LocalNodeManager {
    node: Option<LocalNode>,
}

impl LocalNodeManager {
    pub const fn new() -> Self {
        Self { node: None }
    }

    pub fn is_running(&self) -> bool {
        self.node.as_ref().is_some_and(LocalNode::is_running)
    }

    pub fn urls(&self) -> Option<&LocalNodeUrls> {
        self.node.as_ref().and_then(LocalNode::urls)
    }

    #[allow(dead_code)]
    pub fn network(&self) -> Option<Network> {
        self.node.as_ref().map(LocalNode::network)
    }

    pub async fn start(&mut self, network: Network) -> Result<(), LocalNodeError> {
        if let Some(ref node) = self.node {
            if node.is_running() && node.network() == network {
                debug!("local node already running for {network}");
                return Ok(());
            }

            if node.is_running() {
                info!("stopping local node for {} to start {}", node.network(), network);
                self.stop().await;
            }
        }

        let mut node = LocalNode::new(network);
        node.start().await?;
        self.node = Some(node);

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(mut node) = self.node.take() {
            node.stop().await;
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

    let mut manager = LOCAL_NODE_MANAGER.lock().await;

    if let Some(ref node) = manager.node {
        if !node.is_running() || node.network() != network {
            manager.stop().await;
        }
    }

    if !manager.is_running() {
        manager.start(network).await?;
    }

    let urls = manager.urls().ok_or_else(|| {
        LocalNodeError::NotRunning
    })?;

    let (api_type, url) = match network {
        Network::Bitcoin | Network::Testnet => (ApiType::Electrum, urls.electrum.clone()),
        Network::Signet => (ApiType::Esplora, urls.esplora.clone()),
        Network::Testnet4 => {
            return Err(LocalNodeError::UnsupportedNetwork(
                "testnet4 is not supported by rbitcoin".to_string(),
            ));
        }
    };

    let node = Node {
        name: LOCAL_NODE_NAME.to_string(),
        network,
        api_type,
        url,
    };

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

    Node {
        name: LOCAL_NODE_NAME.to_string(),
        network,
        api_type,
        url: "local://".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_starts_inactive() {
        let manager = LocalNodeManager::new();
        assert!(!manager.is_running());
        assert!(manager.urls().is_none());
        assert!(manager.network().is_none());
    }
}
