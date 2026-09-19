use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use rbitcoin_node::{NodeError, Shutdown, run_node, run_p2p_with_handle};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use cove_types::network::Network;

use crate::config::{LocalNodeUrls, build_config, build_config_with_options, datadir_for_network};
use crate::error::LocalNodeError;

/// Minimum free disk space required to start a local node (1 GB).
const MIN_FREE_SPACE_BYTES: u64 = 1024 * 1024 * 1024;

/// Format a byte count as human-readable string (GB, MB, KB).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = UNITS[0];
    for &next in UNITS {
        unit = next;
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
    }
    format!("{size:.1} {unit}")
}

/// Running local rbitcoin node.
pub struct LocalNode {
    network: Network,
    urls: Option<LocalNodeUrls>,
    task_handle: Option<JoinHandle<Result<(), NodeError>>>,
    shutdown: Option<Arc<Shutdown>>,
    tip_height: Option<Arc<AtomicU32>>,
    initial_block_download: Option<Arc<AtomicBool>>,
}

impl LocalNode {
    pub const fn new(network: Network) -> Self {
        Self {
            network,
            urls: None,
            task_handle: None,
            shutdown: None,
            tip_height: None,
            initial_block_download: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.task_handle.as_ref().is_some_and(|h| !h.is_finished())
    }

    pub fn urls(&self) -> Option<&LocalNodeUrls> {
        self.urls.as_ref()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Best block height observed by the tip-follow loop, if the node is running.
    pub fn tip_height(&self) -> Option<u32> {
        self.tip_height.as_ref().map(|a| a.load(Ordering::Relaxed))
    }

    /// `true` while the node has not yet met minimum chain work or is still in IBD.
    pub fn is_in_ibd(&self) -> Option<bool> {
        self.initial_block_download.as_ref().map(|a| a.load(Ordering::SeqCst))
    }

    /// Total size of the rbitcoin datadir for this node's network.
    pub fn datadir_size(&self) -> Result<u64, LocalNodeError> {
        let path = datadir_for_network(self.network);
        if !path.exists() {
            return Ok(0);
        }
        dir_size(&path)
    }

    /// Remove the rbitcoin datadir for this node's network.
    ///
    /// The caller should ensure the node is stopped first.
    pub fn clear_datadir(&self) -> Result<(), LocalNodeError> {
        let path = datadir_for_network(self.network);
        if path.exists() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| LocalNodeError::DatadirRemove(format!("{}: {e}", path.display())))?;
        }
        Ok(())
    }

    /// Start the local node: find free ports, build config, check disk, and spawn `run_p2p_with_handle`.
    pub async fn start(
        &mut self,
        config_override: Option<crate::LocalNodeConfig>,
    ) -> Result<(), LocalNodeError> {
        if self.is_running() {
            warn!("local node already running");
            return Ok(());
        }

        info!("starting local rbitcoin node for {}", self.network);

        let datadir = datadir_for_network(self.network);
        let free = available_disk_space(&datadir)?;
        if free < MIN_FREE_SPACE_BYTES {
            return Err(LocalNodeError::InsufficientDiskSpace(format!(
                "{} free ({} required)",
                format_bytes(free),
                format_bytes(MIN_FREE_SPACE_BYTES)
            )));
        }

        let electrum_addr = find_free_port().await?;
        let esplora_addr = find_free_port().await?;

        debug!("local node will bind electrum={} esplora={}", electrum_addr, esplora_addr);

        let mut config = build_config_with_options(self.network, config_override)?;
        config.listen.electrum = Some(electrum_addr);
        config.listen.esplora = Some(esplora_addr);

        self.urls = Some(LocalNodeUrls {
            electrum: format!("ssl://{}", electrum_addr),
            esplora: format!("http://{}", esplora_addr),
        });

        let node_handle = run_node(config).map_err(|e| LocalNodeError::StoreOpen(e.to_string()))?;
        let tip_height = Arc::clone(&node_handle.tip_height);
        let initial_block_download = Arc::clone(&node_handle.initial_block_download);

        let shutdown = Shutdown::new();
        let shutdown_for_task = Arc::clone(&shutdown);

        let task = tokio::spawn(async move {
            let result = run_p2p_with_handle(node_handle, shutdown_for_task).await;
            if let Err(ref e) = result {
                error!("rbitcoin run_p2p exited with error: {e}");
            }
            result
        });

        self.task_handle = Some(task);
        self.shutdown = Some(shutdown);
        self.tip_height = Some(tip_height);
        self.initial_block_download = Some(initial_block_download);

        // Give the node a moment to open the store and bind listeners.
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if !self.is_running() {
            return Err(LocalNodeError::P2PStart("run_p2p task exited immediately".to_string()));
        }

        info!("local rbitcoin node started");
        Ok(())
    }

    /// Stop the local node by requesting cooperative shutdown and awaiting the task.
    pub async fn stop(&mut self) {
        if !self.is_running() {
            return;
        }

        info!("stopping local rbitcoin node (cooperative shutdown)");

        if let Some(shutdown) = self.shutdown.take() {
            shutdown.request();
        }

        if let Some(handle) = self.task_handle.take() {
            let timeout = tokio::time::Duration::from_secs(30);
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(Ok(()))) => info!("local node stopped cleanly"),
                Ok(Ok(Err(e))) => warn!("local node stopped with error: {e}"),
                Ok(Err(e)) if e.is_cancelled() => info!("local node task aborted"),
                Ok(Err(e)) => warn!("local node task panicked: {e}"),
                Err(_) => {
                    warn!("local node shutdown timed out after 30s — task may still be running");
                }
            }
        }

        self.urls = None;
        self.tip_height = None;
        self.initial_block_download = None;
    }

    /// Wait for the RPC listener to accept connections.
    ///
    /// Polls the electrum and esplora ports with a short timeout until one
    /// responds or the overall deadline expires. Returns the URL that became
    /// ready first.
    pub async fn wait_for_ready(
        &self,
        timeout_secs: u64,
    ) -> Result<&LocalNodeUrls, LocalNodeError> {
        let urls = self.urls.as_ref().ok_or_else(|| LocalNodeError::NotRunning)?;
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        let electrum_addr = parse_addr(&urls.electrum)?;
        let esplora_addr = parse_addr(&urls.esplora)?;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(LocalNodeError::P2PStart(format!(
                    "RPC did not become ready within {timeout_secs}s"
                )));
            }

            if TcpStream::connect(electrum_addr).await.is_ok() {
                debug!("electrum port {electrum_addr} is ready");
                return Ok(urls);
            }

            if TcpStream::connect(esplora_addr).await.is_ok() {
                debug!("esplora port {esplora_addr} is ready");
                return Ok(urls);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }
}

impl Drop for LocalNode {
    fn drop(&mut self) {
        if self.is_running() {
            if let Some(shutdown) = self.shutdown.take() {
                shutdown.request();
            }
            if let Some(handle) = self.task_handle.take() {
                handle.abort();
            }
        }
    }
}

/// Bind to `127.0.0.1:0` to get a free port, then return the address.
/// Parse a `host:port` address from a URL string like `ssl://127.0.0.1:50001`.
fn parse_addr(url: &str) -> Result<SocketAddr, LocalNodeError> {
    let stripped = url
        .strip_prefix("ssl://")
        .or_else(|| url.strip_prefix("tcp://"))
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);

    stripped.parse().map_err(|e| {
        LocalNodeError::Config(format!("failed to parse socket address from '{url}': {e}"))
    })
}

async fn find_free_port() -> Result<SocketAddr, LocalNodeError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| LocalNodeError::Config(format!("failed to bind to find free port: {e}")))?;

    let addr = listener
        .local_addr()
        .map_err(|e| LocalNodeError::Config(format!("failed to get local address: {e}")))?;

    drop(listener);

    Ok(addr)
}

/// Recursively calculate the total size of a directory in bytes.
pub fn dir_size(path: &Path) -> Result<u64, LocalNodeError> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| LocalNodeError::Config(format!("read_dir {}: {e}", dir.display())))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                LocalNodeError::Config(format!("dir entry in {}: {e}", dir.display()))
            })?;
            let meta = entry.metadata().map_err(|e| {
                LocalNodeError::Config(format!("metadata {}: {e}", entry.path().display()))
            })?;

            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }

    Ok(total)
}

/// Return available disk space in bytes for the filesystem containing `path`.
#[cfg(unix)]
fn available_disk_space(path: &Path) -> Result<u64, LocalNodeError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| LocalNodeError::Config(format!("invalid path: {e}")))?;

    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(LocalNodeError::Config(format!(
            "statvfs {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        )));
    }

    Ok(u64::from(stat.f_bavail) * u64::from(stat.f_frsize))
}

#[cfg(not(unix))]
fn available_disk_space(_path: &Path) -> Result<u64, LocalNodeError> {
    Err(LocalNodeError::Config("disk space check not supported on this platform".to_string()))
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

    #[test]
    fn parse_addr_parses_url_schemes() {
        let cases = [
            ("ssl://127.0.0.1:50001", "127.0.0.1:50001"),
            ("tcp://127.0.0.1:50001", "127.0.0.1:50001"),
            ("http://127.0.0.1:3000", "127.0.0.1:3000"),
            ("https://127.0.0.1:3000", "127.0.0.1:3000"),
            ("127.0.0.1:8080", "127.0.0.1:8080"),
        ];

        for (input, expected) in cases {
            let addr =
                parse_addr(input).unwrap_or_else(|e| panic!("parse_addr({input}) failed: {e}"));
            assert_eq!(addr.to_string(), expected, "for input {input}");
        }
    }

    #[test]
    fn parse_addr_rejects_invalid() {
        assert!(parse_addr("not-an-address").is_err());
        assert!(parse_addr("").is_err());
    }
}
