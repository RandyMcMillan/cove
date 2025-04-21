mod database;

mod app;
mod logging;
mod router;

mod auth;
mod autocomplete;
mod bdk_store;
mod bip39;
mod build;
mod color;
mod color_scheme;
mod consts;
mod converter;
mod device;
mod encryption;
mod file_handler;
mod format;
mod hardware_export;
mod keychain;
mod keys;
mod label_manager;
mod manager;
mod mnemonic;
mod multi_format;
mod multi_qr;
mod node_connect;
mod pending_wallet;
mod psbt;
mod push_tx;
mod redb;
mod seed_qr;
mod send_flow;
mod tap_card;
mod task;
mod transaction;
mod transaction_watcher;
mod unblock;
mod wallet;
mod wallet_scanner;
mod word_validator;
mod xpub;

// re-export types from crates
pub use cove_fiat as fiat;
pub use cove_node as node;
pub use cove_transaction as transaction;
pub use cove_database as database;

::cove_tap_card::uniffi_reexport_scaffolding!();
::util::uniffi_reexport_scaffolding!();
::rust_cktap::uniffi_reexport_scaffolding!();
::cove_nfc::uniffi_reexport_scaffolding!();
::cove_types::uniffi_reexport_scaffolding!();
::cove_fiat::uniffi_reexport_scaffolding!();
::cove_node::uniffi_reexport_scaffolding!();
::cove_transaction::uniffi_reexport_scaffolding!();
::cove_database::uniffi_reexport_scaffolding!();

uniffi::setup_scaffolding!();
