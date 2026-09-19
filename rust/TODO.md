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

**Tests:** `cargo test -p cove-rbitcoin --lib` — 6 passed

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

## Phase 6: Local Node Status Reporting (DONE)

**Goal:** Expose IBD status and tip height to the UI so users know the local node is syncing.

- [x] Add `tip_height: Arc<AtomicU32>` and `initial_block_download: Arc<AtomicBool>` to `NodeHandle`
- [x] Add `run_p2p_with_handle(handle: NodeHandle, shutdown)` so callers can inspect status atomics
- [x] Update cove-rbitcoin to use `run_p2p_with_handle` and expose `tip_height()` / `is_in_ibd()`
- [x] UniFFI-export `local_node_tip_height() -> Option<u32>`
- [x] UniFFI-export `local_node_is_in_ibd() -> Option<bool>`
- [x] Regenerate iOS Swift and Android Kotlin bindings

**Files touched (rbitcoin nested repo):**
- `crates/rbitcoin-node/src/run.rs`
- `crates/rbitcoin-node/src/lib.rs`

**Files touched (cove repo):**
- `crates/cove-rbitcoin/src/node.rs`
- `src/lib.rs`
- `src/local_node_manager.rs`

**Note:** IBD progress percentage is not yet available — rbitcoin's progress internals are `pub(crate)`. For now the UI can show tip height + "Syncing" / "Ready" based on `is_in_ibd`.

---

## Phase 7: Resource Management (DONE)

**Goal:** Guard against disk exhaustion and provide datadir cleanup.

- [x] Add pre-start disk check (1 GB minimum) using `libc::statvfs`
- [x] Add `local_node_datadir_size() -> u64` UniFFI export
- [x] Add `local_node_clear_datadir() -> Result<(), LocalNodeStartError>` UniFFI export
- [x] Export `dir_size` and `format_bytes` helpers from cove-rbitcoin
- [x] Add `InsufficientDiskSpace` and `DatadirRemove` error variants

**Files touched:**
- `crates/cove-rbitcoin/src/node.rs`
- `crates/cove-rbitcoin/src/error.rs`
- `crates/cove-rbitcoin/src/lib.rs`
- `crates/cove-rbitcoin/Cargo.toml`
- `src/lib.rs`
- `src/local_node_manager.rs`

**Note:** UI integration (Settings → Node → Local Node storage metrics) is left for the mobile frontend teams.

---

## Phase 9: Eager Local Node Start (DONE)

**Goal:** Start the local node immediately when selected or on app launch, rather than waiting for the first wallet sync.

- [x] Spawn background task in `NodeSelector::select_local_node()` to start node after selection
- [x] Spawn background task in `App::init_data()` to start node if already selected on launch
- [x] Graceful error handling — logs warnings on failure, never blocks UI

**Files touched:**
- `src/node_connect.rs`
- `src/app.rs`

---

## Phase 10: iOS Xcode Auto-Build + Mac Catalyst (DONE)

**Goal:** Ensure Xcode builds the Rust library automatically and supports Mac Catalyst.

- [x] Add inline Run Script build phase to Cove target for automatic xcframework rebuild
- [x] Handle sandbox restrictions by embedding script directly in build phase
- [x] Auto-install `just` via cargo if not available in Xcode env
- [x] Add `aarch64-apple-ios-macabi` target to xtask build
- [x] Update `compile-ios` justfile recipe to check for macabi slice
- [x] xcframework now contains: ios-arm64, ios-arm64-simulator, ios-arm64-maccatalyst

**Files touched:**
- `xtask/src/ios.rs`
- `justfile`
- `ios/Cove.xcodeproj/project.pbxproj`

---

## Phase 11: RPC Health Check (DONE)

**Goal:** Ensure the local node's RPC endpoint is actually accepting connections before wallet sync tries to use it.

- [x] Add `LocalNode::wait_for_ready(timeout_secs)` that polls electrum/esplora ports
- [x] Add `parse_addr()` helper for URL → SocketAddr parsing
- [x] Call `wait_for_ready(30)` in `resolve_selected_node()` after start
- [x] Add unit tests for `parse_addr`

**Files touched:**
- `crates/cove-rbitcoin/src/node.rs`
- `src/local_node_manager.rs`

---

## Phase 12: App Lifecycle Hooks (DONE)

**Goal:** Provide Rust hooks for mobile frontends to stop/resume the local node on app background/foreground.

- [x] Add `local_node_app_backgrounded()` UniFFI export — stops node cooperatively
- [x] Add `local_node_app_foregrounded()` UniFFI export — restarts node if selected
- [x] Regenerate iOS Swift and Android Kotlin bindings

**Files touched:**
- `src/lib.rs`
- `src/local_node_manager.rs`

**Note:** iOS/Android call sites (`applicationDidEnterBackground`, `onStop`, etc.) are left for mobile frontend teams.

---

## Phase 13: Local Node Config (DONE)

**Goal:** Allow mobile-friendly tuning of rbitcoin peer and bandwidth settings.

- [x] Add `LocalNodeConfig` struct with `max_outbound` and `blocksonly` fields
- [x] Add `build_config_with_options(network, options)` in `cove-rbitcoin`
- [x] Wire `LocalNodeConfig` through `LocalNode::start()` and `LocalNodeManager`
- [x] Mobile defaults: `max_outbound: 4`, `blocksonly: false`

**Files touched:**
- `crates/cove-rbitcoin/src/config.rs`
- `crates/cove-rbitcoin/src/node.rs`
- `crates/cove-rbitcoin/src/lib.rs`
- `src/local_node_manager.rs`

**Note:** Database persistence and UI controls for config are left for later when the frontend exposes settings.

---

## Phase 8: End-to-End Verification (PARTIAL)

**Goal:** Validate the full flow on device/simulator.

- [x] iOS Swift compilation: `just compile-ios` — BUILD SUCCEEDED
- [x] Rust lib tests: `cargo test --lib` — 1647 passed, 1 pre-existing flaky failure (fee_client)
- [x] rbitcoin-node tests: `cargo test -p rbitcoin-node --lib run::tests` — 35 passed
- [x] cove-rbitcoin tests: `cargo test -p cove-rbitcoin --lib` — 8 passed
- [ ] iOS: `just build-run-ios --udid <device>` with local node selected
- [x] Android: `just build-android` — BUILD SUCCEEDED (Kotlin bindings regenerated)
- [ ] Verify wallet sync completes against local Esplora/Electrum
- [ ] Verify app backgrounding triggers cooperative shutdown
- [ ] Verify app relaunch resumes sync from last tip

**Blockers:**
- No iOS device UDID available in this environment
- Android device E2E not yet run (Gradle build / install pending)

---

## Blockers / Notes

1. ~~**Android NDK** not installed on build machine.~~ RESOLVED — NDK installed via brew, `just build-android` passes.

2. **Global cargo target-dir** is `~/.cache/cargo`. xtask expects `./target/`.
   Workaround: symlink `rust/target -> ~/.cache/cargo`.

3. **Testnet4** intentionally excludes Local node (rbitcoin doesn't support it).

4. **rbitcoin nested repo** changes were committed separately; remember to push both repos.

5. **iOS `AppManager` environment in sheets on Mac Catalyst** — fixed by explicitly injecting `.environment(context.app)` into `SelectedWalletSheetContent` views.

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
