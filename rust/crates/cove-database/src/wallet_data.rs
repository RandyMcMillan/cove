pub mod label;

use std::{path::PathBuf, sync::Arc};

use cove_types::WalletId;
use label::LabelDb;
use redb::Database;
use tracing::{error, info};

use crate::Error;

#[derive(Debug, Clone, uniffi::Object)]
pub struct WalletDataDb {
    pub labels: LabelDb,
    #[allow(dead_code)]
    db: Arc<Database>,
}

impl WalletDataDb {
    pub fn new_or_existing(wallet_id: WalletId) -> Self {
        let db_path = wallet_database_path(&wallet_id);
        ensure_parent_dir(&db_path);

        let db = if db_path.exists() {
            match Database::open(&db_path) {
                Ok(db) => db,
                Err(err) => {
                    error!("Failed to open wallet database: {err}");
                    Database::create(&db_path).expect("Failed to create wallet database")
                }
            }
        } else {
            info!("Creating new wallet database at {}", db_path.display());
            Database::create(&db_path).expect("Failed to create wallet database")
        };

        let db = Arc::new(db);
        
        // Set up database tables
        let write_txn = db.begin_write().expect("Failed to begin write transaction");
        let labels = LabelDb::new(db.clone(), &write_txn);
        write_txn.commit().expect("Failed to commit transaction");

        Self { labels, db }
    }
}

fn wallet_database_path(wallet_id: &WalletId) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("cove");
    path.push("wallet_data");
    path.push(format!("{}.db", wallet_id));
    path
}

fn ensure_parent_dir(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("Failed to create directory");
        }
    }
}