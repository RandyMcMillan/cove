#[derive(Debug, thiserror::Error)]
pub enum LocalNodeError {
    #[error("failed to open rbitcoin store: {0}")]
    StoreOpen(String),

    #[error("failed to start P2P: {0}")]
    P2PStart(String),

    #[error("failed to start Electrum server: {0}")]
    ElectrumStart(String),

    #[error("failed to start Esplora server: {0}")]
    EsploraStart(String),

    #[error("local node is not running")]
    NotRunning,

    #[error("unsupported network for local node: {0}")]
    UnsupportedNetwork(String),

    #[error("rbitcoin config error: {0}")]
    Config(String),

    #[error("insufficient disk space: {0}")]
    InsufficientDiskSpace(String),

    #[error("failed to remove datadir: {0}")]
    DatadirRemove(String),
}
