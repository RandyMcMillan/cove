# rbitcoin Local Node Integration — Implementation Tracker

## Overview

Integrate the local `rbitcoin` dependency as an embedded full node option within Cove.
Users can select "Local Node" from the node selector to run a lightweight Bitcoin node
(peer-to-peer + block filters + Esplora/Electrum RPC) directly on their device.

---

## Phase 1: rbitcoin Cooperative Shutdown (DONE)

**Goal:** Allow Cove to shut down the rbitcoin node cleanly on app backgrounding/exit.

- [x] Export `Shutdown` token and `run_p2p_with_shutdown` from `rbitcoin-node`
- [x] Keep `run_p2p` as convenience wrapper with signal handlers
- [x] Commit changes in nested `rbitcoin` repo

**Files touched (nested repo):**
- `rbitcoin/crates/rbitcoin-node/src/lib.rs`
- `rbitcoin/crates/rbitcoin-node/src/run.rs`

---

## Phase 2: cove-rbitcoin Cooperative Shutdown (DONE)

**Goal:** Wrap rbitcoin node in `cove-rbitcoin` with start/stop/URL APIs.

- [x] Add `start()` / `stop()` / `is_running()` / `urls()` to `LocalNode`
- [x] Use `run_p2p_with_shutdown` + `Shutdown` token
- [x] Implement `Drop` for automatic cleanup
- [x] Add `LocalNodeUrls` record and `LocalNodeError` enum

**Files touched:**
- `crates/cove-rbitcoin/src/node.rs`
- `crates/cove-rbitcoin/src/error.rs`
- `crates/cove-rbitcoin/src/lib.rs`

**Tests:** `cargo test -p cove-rbitcoin --lib` — 4 passed

---

## Phase 3: NodeSelection::Local + Dynamic Resolution (DONE)

**Goal:** Add `Local` as a first-class node selection option and resolve its URL dynamically.

- [x] Add `NodeSelection::Local` variant
- [x] Add `LOCAL_NODE_NAME` constant ("Local Node")
- [x] Add `SelectedNodeIsLocal` database key + `selected_node_is_local()` / `set_selected_node_is_local()`
- [x] Add `resolve_selected_node()` async helper
- [x] Add `selected_node_identity_placeholder()` for sync identity checks
- [x] Update `NodeConnectionIdentity` with `is_local` flag and custom `Eq`/`Hash`
- [x] Update `NodeClientBuilder::build()` to auto-resolve local placeholder
- [x] Update `NodeList` and `selected_node()` logic

**Files touched:**
- `src/database/global_config.rs`
- `src/node_connect.rs`
- `src/node.rs`
- `src/local_node_manager.rs`
- `src/node/client_builder.rs`

---

## Phase 4: Wallet Manager Integration (DONE)

**Goal:** Ensure all blockchain operations resolve the local node before executing.

- [x] Update `wallet_manager/actor/node.rs` — `check_node_connection`, `get_height`, `sync`
- [x] Update `wallet_manager/actor/receive_address.rs` — address validation
- [x] Update `wallet_manager/actor/transactions.rs` — transaction broadcast / fetch
- [x] Update `receive_address_watcher.rs`
- [x] Update `transaction_watcher.rs`
- [x] Update `NodeSelector::check_selected_node()` to resolve local node before URL check

**Files touched:**
- `src/manager/wallet_manager/actor/node.rs`
- `src/manager/wallet_manager/actor/receive_address.rs`
- `src/manager/wallet_manager/actor/transactions.rs`
- `src/receive_address_watcher.rs`
- `src/transaction_watcher.rs`
- `src/node_connect.rs`

---

## Phase 5: UniFFI Binding Generation + Mobile Call-Site Fixes (DONE)

**Goal:** Regenerate bindings and fix any Swift/Kotlin breakage from `NodeSelection::Local`.

- [x] Fix `local_node_start` UniFFI error type (`String` → `LocalNodeStartError`)
- [x] Regenerate iOS Swift bindings (`just build-ios`)
- [x] Regenerate Android Kotlin bindings (manual `uniffi_cli` via host dylib)
- [x] Verify Swift call sites — no exhaustive switches on `NodeSelection`, no changes needed
- [x] Verify Kotlin call sites — `NodeSettingsScreen.kt` handles `Local` via existing preset flow
- [x] Copy updated Kotlin bindings into Android project

**Files touched:**
- `src/lib.rs` (`LocalNodeStartError`)
- `src/node_connect.rs` (`check_selected_node`)
- `ios/CoveCore/Sources/CoveCore/generated/*.swift`
- `android/app/src/main/java/org/bitcoinppl/cove_core/*.kt`

---

## Phase 6: Local Node Auto-Start / Status Reporting (PENDING)

**Goal:** Expose IBD progress, tip height, and running state to the UI so users know
the local node is syncing.

- [ ] Add `local_node_ibd_progress() -> Option<f64>` UniFFI export
- [ ] Add `local_node_tip_height() -> Option<u32>` UniFFI export
- [ ] Add `local_node_status() -> LocalNodeStatus` enum (`Idle`, `Starting`, `IBD(f64)`, `Ready`)
- [ ] Bridge status into Swift/Kotlin UI (settings screen, status bar)
- [ ] Consider auto-starting local node when selected and not running

**Open questions:**
- Should selecting "Local Node" immediately trigger `local_node_start()`?
- Or should we show a "Start Local Node" toggle and let the user control it?
- How do we handle IBD on mobile (battery, storage, bandwidth)?

---

## Phase 7: Resource Management (PENDING)

**Goal:** Guard against disk exhaustion and provide datadir cleanup.

- [ ] Add pre-start disk check (require N GB free)
- [ ] Add `local_node_datadir_size() -> u64` UniFFI export
- [ ] Add `local_node_clear_datadir()` for reset / troubleshooting
- [ ] Expose storage metrics in Settings → Node → Local Node

---

## Phase 8: End-to-End Verification (PENDING)

**Goal:** Validate the full flow on device/simulator.

- [ ] iOS: `just build-run-ios --udid <device>` with local node selected
- [ ] Android: `just build-android` + install on device with local node selected
- [ ] Verify wallet sync completes against local Esplora/Electrum
- [ ] Verify app backgrounding triggers cooperative shutdown
- [ ] Verify app relaunch resumes sync from last tip

---

## Blockers / Notes

1. **Android NDK** not installed on build machine. `just build-android` fails at `cargo ndk`.
   Workaround: generate Kotlin bindings from host `libcove.dylib` via `uniffi_cli`.

2. **Global cargo target-dir** is `~/.cache/cargo`. xtask expects `./target/`.
   Workaround: symlink `rust/target -> ~/.cache/cargo`.

3. **Testnet4** intentionally excludes Local node (rbitcoin doesn't support it).

4. **rbitcoin nested repo** changes were committed separately; remember to push both repos.

---

## Quick Commands

```bash
# Rust checks
cargo check --lib
cargo test -p cove-rbitcoin --lib
cargo test -p rbitcoin-node --lib run::tests

# iOS bindings + framework
just build-ios

# Android bindings (requires NDK)
just build-android

# Regenerate bindings manually (if NDK unavailable)
cargo run --locked -p uniffi_cli -- generate \
  ~/.cache/cargo/debug/libcove.dylib \
  --library --language kotlin --no-format --out-dir ./bindings-kotlin
```
