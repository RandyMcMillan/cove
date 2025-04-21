uniffi::setup_scaffolding!();

pub mod client;
pub mod client_builder;
pub mod constants;
pub mod node;
pub mod error;

pub use client::NodeClient;
pub use client_builder::NodeClientBuilder;
pub use constants::{
    BITCOIN_ELECTRUM, BITCOIN_ESPLORA, SIGNET_ESPLORA, TESTNET_ELECTRUM, TESTNET_ESPLORA,
};
pub use error::Error;
pub use node::{ApiType, Node, NodeSelection};