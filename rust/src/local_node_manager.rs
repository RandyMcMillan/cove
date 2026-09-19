use std::sync::{Arc, LazyLock};

use tokio::sync::Mutex;
use tracing::{debug, info};

use cove_types::network::Network;
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
