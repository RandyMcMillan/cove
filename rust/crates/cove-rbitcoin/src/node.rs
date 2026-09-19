use std::net::SocketAddr;

use rbitcoin_node::{run_p2p, NodeError};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use cove_types::network::Network;

use crate::config::{build_config, LocalNodeUrls};
use crate::error::LocalNodeError;

/// Running local rbitcoin node.
pub struct LocalNode {
    network: Network,
    urls: Option<LocalNodeUrls>,
    task_handle: Option<JoinHandle<Result<(), NodeError>>>,
}

impl LocalNode {
    pub const fn new(network: Network) -> Self {
        Self { network, urls: None, task_handle: None }
    }

    pub fn is_running(&self) -> bool {
        self.task_handle
            .as_ref()
            .is_some_and(|h| !h.is_finished())
    }

    pub fn urls(&self) -> Option<&LocalNodeUrls> {
        self.urls.as_ref()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Start the local node: find free ports, build config, and spawn `run_p2p`.
    pub async fn start(&mut self) -> Result<(), LocalNodeError> {
        if self.is_running() {
            warn!("local node already running");
            return Ok(());
        }

        info!("starting local rbitcoin node for {}", self.network);

        let electrum_addr = find_free_port().await?;
        let esplora_addr = find_free_port().await?;

        debug!(
            "local node will bind electrum={} esplora={}",
            electrum_addr, esplora_addr
        );

        let mut config = build_config(self.network)?;
        config.listen.electrum = Some(electrum_addr);
        config.listen.esplora = Some(esplora_addr);

        self.urls = Some(LocalNodeUrls {
            electrum: format!("ssl://{}", electrum_addr),
            esplora: format!("http://{}", esplora_addr),
        });

        let task = tokio::spawn(async move {
            let result = run_p2p(config).await;
            if let Err(ref e) = result {
                error!("rbitcoin run_p2p exited with error: {e}");
            }
            result
        });

        self.task_handle = Some(task);

        // Give the node a moment to open the store and bind listeners.
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if !self.is_running() {
            return Err(LocalNodeError::P2PStart(
                "run_p2p task exited immediately".to_string(),
            ));
        }

        info!("local rbitcoin node started");
        Ok(())
    }

    /// Stop the local node by aborting the background task.
    ///
    /// NOTE: rbitcoin v0.7.0 does not expose the `Shutdown` flag used by
    /// `run_p2p`, so we cannot request a graceful shutdown. Task abort is
    /// the only available mechanism. Clean store flush requires upstream
    /// changes (export `Shutdown` or add a `no_signal_handlers` mode).
    pub async fn stop(&mut self) {
        if !self.is_running() {
            return;
        }

        info!("stopping local rbitcoin node (task abort)");

        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let timeout = tokio::time::Duration::from_secs(10);
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(Ok(()))) => info!("local node stopped cleanly"),
                Ok(Ok(Err(e))) => warn!("local node stopped with error: {e}"),
                Ok(Err(e)) if e.is_cancelled() => info!("local node task aborted"),
                Ok(Err(e)) => warn!("local node task panicked: {e}"),
                Err(_) => warn!("local node shutdown timed out"),
            }
        }

        self.urls = None;
    }
}

impl Drop for LocalNode {
    fn drop(&mut self) {
        if self.is_running() && let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// Bind to `127.0.0.1:0` to get a free port, then return the address.
async fn find_free_port() -> Result<SocketAddr, LocalNodeError> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
        LocalNodeError::Config(format!("failed to bind to find free port: {e}"))
    })?;

    let addr = listener.local_addr().map_err(|e| {
        LocalNodeError::Config(format!("failed to get local address: {e}"))
    })?;

    drop(listener);

    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn find_free_port_returns_valid_localhost() {
        let addr = find_free_port().await.expect("should find a free port");
        assert!(addr.ip().is_loopback());
        assert!(addr.port() > 0);
    }

    #[test]
    fn local_node_starts_inactive() {
        let node = LocalNode::new(Network::Signet);
        assert!(!node.is_running());
        assert!(node.urls().is_none());
    }

    #[test]
    fn build_config_maps_networks() {
        for network in [Network::Bitcoin, Network::Testnet, Network::Signet] {
            let config = build_config(network).expect("should build config");
            assert!(config.shindex);
            assert!(config.prefill_compact);
        }
    }

    #[test]
    fn build_config_rejects_testnet4() {
        let result = build_config(Network::Testnet4);
        assert!(matches!(result, Err(LocalNodeError::UnsupportedNetwork(_))));
    }
}
