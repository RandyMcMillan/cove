// Allow lints that are problematic due to uniffi requirements or too invasive
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::significant_drop_in_scrutinee)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_fields_in_debug)]
#![allow(clippy::inline_always)]
#![allow(clippy::option_option)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::unchecked_time_subtraction)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::needless_collect)]

mod database;

#[cfg(test)]
mod test_support;

mod app;
mod bootstrap;
mod router;

mod auth;
mod autocomplete;
mod backup;
mod bdk_store;
mod build;
mod converter;
mod custom_block_explorer;
mod diagnostics;
mod discovery_scanner;
mod fee_client;
mod fiat;
mod file_handler;
mod hardware_export;
mod historical_price_service;
mod key_teleport;
mod keys;
mod label_manager;
mod loading_popup;
mod local_node_manager;
mod manager;
mod mnemonic;
mod multi_format;
mod node;
mod node_connect;
mod pending_wallet;
mod push_tx;
mod qr_scanner;
mod receive_address_watcher;
mod reporting;
mod retry;
mod seed_qr;
mod send_flow;
mod signed_import;
mod tap_card;
mod transaction;
mod transaction_watcher;
mod ur;
mod wallet;
mod wallet_identity;
mod wallet_lifecycle;
mod wallet_secret;
mod word_validator;
mod word_verify_state_machine;
mod xpub;

::cove_tap_card::uniffi_reexport_scaffolding!();
::cove_util::uniffi_reexport_scaffolding!();
::cove_nfc::uniffi_reexport_scaffolding!();
::cove_types::uniffi_reexport_scaffolding!();
::cove_device::uniffi_reexport_scaffolding!();

uniffi::setup_scaffolding!();

// re-export types from crates that are are used
use cove_common::logging;
use cove_device::device;
use cove_device::keychain;
use cove_types::color;
use cove_types::color_scheme;
use cove_types::network;
use cove_types::psbt;

use std::path::PathBuf;

#[derive(Debug, Clone, uniffi::Error, thiserror::Error)]
#[uniffi(flat_error)]
pub enum InitError {
    #[error("Failed to set root data directory: {0}")]
    RootDataDirAlreadySet(String),
}

/// set root data directory before any database access
/// required for Android to specify app-specific storage path
#[uniffi::export]
fn set_root_data_dir(path: String) -> Result<(), InitError> {
    cove_common::consts::set_root_data_dir(PathBuf::from(path))
        .map_err(InitError::RootDataDirAlreadySet)
}

// Local node exports
use crate::network::Network;

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_is_running() -> bool {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.is_running()
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_esplora_url() -> Option<String> {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.urls().map(|u| u.esplora.clone())
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_electrum_url() -> Option<String> {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.urls().map(|u| u.electrum.clone())
}

#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum LocalNodeStartError {
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

    #[error("insufficient disk space: {0}")]
    InsufficientDiskSpace(String),

    #[error("failed to remove datadir: {0}")]
    DatadirRemove(String),

    #[error("rbitcoin config error: {0}")]
    Config(String),
}

impl From<cove_rbitcoin::LocalNodeError> for LocalNodeStartError {
    fn from(err: cove_rbitcoin::LocalNodeError) -> Self {
        match err {
            cove_rbitcoin::LocalNodeError::StoreOpen(s) => Self::StoreOpen(s),
            cove_rbitcoin::LocalNodeError::P2PStart(s) => Self::P2PStart(s),
            cove_rbitcoin::LocalNodeError::ElectrumStart(s) => Self::ElectrumStart(s),
            cove_rbitcoin::LocalNodeError::EsploraStart(s) => Self::EsploraStart(s),
            cove_rbitcoin::LocalNodeError::NotRunning => Self::NotRunning,
            cove_rbitcoin::LocalNodeError::UnsupportedNetwork(s) => Self::UnsupportedNetwork(s),
            cove_rbitcoin::LocalNodeError::InsufficientDiskSpace(s) => {
                Self::InsufficientDiskSpace(s)
            }
            cove_rbitcoin::LocalNodeError::DatadirRemove(s) => Self::DatadirRemove(s),
            cove_rbitcoin::LocalNodeError::Config(s) => Self::Config(s),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_start(network: Network) -> Result<(), LocalNodeStartError> {
    local_node_manager::LOCAL_NODE_MANAGER
        .lock()
        .await
        .start(network)
        .await
        .map_err(LocalNodeStartError::from)
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_stop() {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.stop().await;
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_tip_height() -> Option<u32> {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.tip_height()
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_is_in_ibd() -> Option<bool> {
    local_node_manager::LOCAL_NODE_MANAGER.lock().await.is_in_ibd()
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_datadir_size() -> Result<u64, LocalNodeStartError> {
    local_node_manager::LOCAL_NODE_MANAGER
        .lock()
        .await
        .datadir_size()
        .map_err(LocalNodeStartError::from)
}

#[uniffi::export(async_runtime = "tokio")]
async fn local_node_clear_datadir() -> Result<(), LocalNodeStartError> {
    local_node_manager::LOCAL_NODE_MANAGER
        .lock()
        .await
        .clear_datadir()
        .await
        .map_err(LocalNodeStartError::from)
}
