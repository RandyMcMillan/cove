# Cove Agent Guide

This file is a reference for AI coding agents working on the Cove project.
Cove is a Bitcoin mobile wallet for iOS and Android. Read this before making
non-trivial changes.

## Project Overview

Cove is a simple-to-use yet powerful Bitcoin mobile wallet, built on top of
[BDK](https://bitcoindevkit.org/). It supports hot wallets and is optimized for
use with hardware wallets such as Coldcard, TAPSIGNER, Krux, Jade, SeedSigner,
and Foundation Passport.

Key product features:

- Create and import hot wallets, verify recovery words, SeedQR backup/restore.
- Import watch-only and hardware wallets from xpubs, public descriptors, and key
  expressions via NFC, files, or QR codes.
- Send Bitcoin with hot or hardware wallets using QR (BBQr and UR), NFC, or
  files for PSBT transfer.
- Multiple wallets across Bitcoin, Signet, Testnet, and Testnet4.
- Native Segwit, Wrapped Segwit, and Legacy address types.
- UTXO coin control, labels (BIP329), and transaction CSV export.
- Custom Electrum/Esplora node connections, including a local Bitcoin node
  (rbitcoin) on mobile.
- End-to-end encrypted Cloud Backup protected by passkeys (iCloud Drive on iOS,
  Google Drive on Android).
- PIN, biometric app lock, Wipe Data, and Decoy trick PINs.

The project is released under the MIT license. Security reports should be sent
privately to security@covebitcoinwallet.com or via GitHub's private
vulnerability reporting flow.

## Technology Stack

### Core

- **Rust** (MSRV 1.95, edition 2024) - single source of truth for wallet logic,
  networking, persistence, and hardware integrations.
- **BDK** (`bdk_wallet 3.0.0`) and `bitcoin 0.32.9` for all Bitcoin logic.
- **UniFFI** - generates cross-platform bindings for Swift and Kotlin.
- **Tokio** - async runtime, initialized once from the host app.
- **act-zero** - actor system for long-lived concurrent components.
- **redb** - non-sensitive local app persistence.
- **OS keychain/keystore** - secret storage via `cove-device` callback
  interfaces.
- **rbitcoin** - local, embedded Bitcoin node (`rust/rbitcoin/` crates).

### iOS

- **SwiftUI** with iOS 16+ deployment target.
- **Swift 6** language mode.
- **Xcode 16.0+**.
- **MijickPopups** for sheets/popups (inferred from `CoveApp.swift`).
- Generated bindings live in `ios/CoveCore/Sources/CoveCore/generated/`,
  wrapped by the `CoveCore` Swift Package.

### Android

- **Jetpack Compose** and Material 3.
- **Kotlin 2.3.10**, Java 17 target.
- **Navigation3** for type-safe navigation.
- **Android Gradle Plugin 9.0.1**, minSdk 33, targetSdk/compileSdk 36.
- Generated Kotlin bindings live in
  `android/app/src/main/java/org/bitcoinppl/cove_core/`.

### Build Orchestration

- **Just** (`justfile`) - primary build/task runner.
- **Cargo xtask** (`rust/xtask/`) - custom Rust CLI for builds, mobile
  artifacts, version bumps, and utility tasks.
- Custom build scripts in `scripts/build-ios.sh` and `scripts/build-android.sh`
  (invoked by xtask) generate UniFFI bindings and copy artifacts into the mobile
  projects.

## Repository Layout

```
.
├── AGENTS.md              # This file
├── README.md              # Project overview and feature list
├── ARCHITECTURE.md        # System design and codebase structure
├── CONTRIBUTING.md        # Development setup, workflow, commit conventions
├── SECURITY.md            # Vulnerability reporting
├── justfile               # Primary build commands
├── rust/                  # Rust core library and workspace
│   ├── Cargo.toml         # Main crate and workspace manifest
│   ├── src/               # Main crate source
│   ├── crates/            # Internal workspace crates
│   ├── rbitcoin/          # Local Bitcoin node crates
│   ├── xtask/             # Build automation CLI
│   ├── bindings/          # Temporary UniFFI output (ignored)
│   └── uniffi.toml        # UniFFI binding configuration
├── ios/                   # iOS app and Swift Package
│   ├── Cove/              # SwiftUI app source
│   ├── CoveCore/          # Swift Package wrapping generated bindings
│   ├── CoveTests/         # Unit/layout tests
│   └── CoveUITests/       # UI tests
├── android/               # Android app
│   ├── app/src/main/java/ # Kotlin source + generated cove_core bindings
│   ├── app/src/test/      # Unit tests
│   ├── app/src/androidTest/ # Instrumentation/UI tests
│   ├── build.gradle.kts   # Root build script
│   └── detekt.yml         # Detekt static analysis config
├── docs/                  # Topic-specific documentation
│   ├── cloud_backup.md
│   ├── icloud_drive.md
│   ├── ios_android_parity.md
│   ├── keyteleport.md
│   ├── passkeys.md
│   └── redb.md
├── tools/                 # Development tools
│   └── cove-swift-lint/   # Custom Swift AST lint rules
├── pubkeys/               # Security PGP public key
└── designs/               # UI design assets
```

### Rust Core Layout

Main crate modules (`rust/src/`):

- `app.rs` - `App` singleton: routing coordinator, fees/prices, network
  selection, terms acceptance. `FfiApp` is the UniFFI-exposed wrapper.
- `router.rs` - Route enum definitions.
- `database.rs` + `database/` - redb tables and persistence helpers.
- `manager/` - UI-facing managers:
  - `wallet_manager.rs` + `wallet_manager/` - Wallet actor and operations.
  - `send_flow_manager.rs` + `send_flow_manager/` - Send flow state machine.
  - `import_wallet_manager.rs` - Wallet import orchestration.
  - `coin_control_manager.rs` - UTXO selection.
  - `cloud_backup_manager.rs` + `cloud_backup_manager/` - Cloud backup.
  - `auth_manager.rs` - App lock/PIN/biometric authentication.
  - `key_teleport_manager.rs` + `key_teleport_manager/` - Secure seed
    transfer.
  - `onboarding_manager.rs` + `onboarding_manager/` - Onboarding flows.
  - `pending_wallet_manager.rs` - Pending wallet construction.
- `wallet.rs`, `wallet_lifecycle.rs`, `wallet_identity.rs` - Wallet domain
  model, lifecycle, and identity.
- `transaction.rs`, `transaction_watcher.rs` - Transaction logic.
- `tap_card.rs`, `qr_scanner.rs`, `seed_qr.rs` - Hardware and QR flows.
- `fee_client.rs`, `fiat.rs` - Fee estimation and fiat price fetching.
- `node.rs`, `node_connect.rs`, `local_node_manager.rs` - Local node support.
- `keys.rs`, `mnemonic.rs`, `xpub.rs` - Key and mnemonic handling.
- `backup.rs`, `label_manager.rs` - Backup and BIP329 labels.
- `bootstrap.rs` - App startup initialization.

Internal workspace crates (`rust/crates/`):

- `cove-bdk` - BDK wallet wrappers.
- `cove-bip39` - BIP39 mnemonic handling.
- `cove-common` - Shared constants and utilities.
- `cove-device` - Platform abstraction for keychain and device features.
- `cove-macros` - Common macros.
- `cove-nfc` - NFC communication protocols.
- `cove-tap-card` - TAPSIGNER/SATSCARD integration.
- `cove-types` - Shared type definitions.
- `cove-util` - General utilities (formatting, logging, result extensions).
- `cove-ur` - UR type implementations.
- `cove-tokio` - Tokio runtime and actor helpers.
- `cove-http` - HTTP client setup.
- `cove-rbitcoin` - Local rbitcoin node wrapper.
- `cove-bdk-progressive-scan` - Progressive full-scan helpers.
- `cove-cspp` - Crypto/secret handling primitives.
- `uniffi_cli` - Custom UniFFI CLI wrapper for binding generation.

## Architecture

### Single Source of Truth

The Rust crate is the single source of truth for wallet logic, networking,
persistence, and hardware integrations. SwiftUI and Jetpack Compose UIs talk to
Rust through platform-specific "Managers" that own the generated FFI objects,
subscribe to reconciliation callbacks, and expose platform-friendly state.

### Data Model First

Model the domain correctly before optimizing for a small patch. Durable fixes
should represent state, ownership, and invariants explicitly in Rust data
structures and persisted records instead of compensating with temporary UI
logic, string flags, or caller-specific conditionals. When the model must
change, update the UniFFI API, generated bindings, migrations, and Swift/Kotlin
call sites together so every layer shares the same contract.

### Async Runtime and Actors

The core is async-first:

- A Tokio runtime is initialized once from the host app (`FfiApp::init_on_start`).
- Long-lived concurrent components use `act-zero` actors spawned with
  `task::spawn_actor()`.
- Key actors: `WalletActor`, `WalletScanner`.
- Keep UI-facing actor handlers short: validate inputs, update authoritative
  state, emit reconcile messages, and delegate slow work to child actors/workers.
- For narrow one-shot work, use `cove_tokio::task::spawn`.
- When async work belongs to an actor and reports back, prefer `AddrLike::send_fut`
  or `send_fut_with` over raw tasks with captured addresses.

### State Reconciliation

Each manager owns a `flume` channel pair. Rust emits typed `…ReconcileMessage`
enums through the channel, and the generated FFI forwards them to platform
reconcilers. Platform managers should call `listen_for_updates` immediately
after instantiating their Rust counterpart so no messages are missed.

- Treat `state()` as an initial snapshot/bootstrap read only.
- Use typed delta `ReconcileMessage`s for ongoing UI updates.
- Compute changes under the lock, then send delta messages after releasing the
  lock.

### Dispatching

For user intents, prefer `manager.dispatch(action:)`. Keep named methods for
reads, bootstrap/lifecycle hooks, and special service-style operations.

When Rust code needs to dispatch `AppAction`, use `DeferredDispatch<T>`
(`rust/src/manager/deferred_dispatch.rs`) so dispatch happens after any locks
are released, avoiding deadlocks.

### Singletons

Many core components use singletons implemented via `OnceLock`, `LazyLock`, or
`ArcSwap`. Key singletons:

- `Database::global()` - redb access.
- `App::global()` / `FfiApp::global()` - application state and routing.
- `AUTH_MANAGER` - authentication state.
- `Keychain::global()` / `Device::global()` - platform keychain/device
  capabilities.
- `FIAT_CLIENT`, `FEE_CLIENT` - cached price/fee clients.
- `PRICES`, `FEES` - cached data via `ArcSwap`.

Platform code mirrors this with `AppManager.shared` (iOS) and
`AppManager.getInstance()` (Android).

### Concurrency Primitives

Use `parking_lot::Mutex` and `parking_lot::RwLock` everywhere instead of
`std::sync` equivalents. Prefer scoped blocks to release locks or borrows
instead of explicit `drop(...)` unless explicit drop is actually needed.

### Persistence

- Non-sensitive data: [`redb`](https://crates.io/crates/redb) (`rust/src/database.rs`).
- Secrets: OS-native keychain via the `KeychainAccess` callback interface
  implemented by iOS/Android.
- Database file: `$ROOT_DATA_DIR/cove.db` (see `cove_common::consts::ROOT_DATA_DIR`).

**redb compatibility is critical.** redb stores typed table metadata
(`TypeName`) for every table. Moving a persisted type to a different module can
change the on-disk metadata even if serialized bytes stay identical. Read
`docs/redb.md` before changing `TableDefinition`s, `Value::type_name()`
implementations, persisted database structs/enums, or module paths containing
persisted types. Add regression tests that open databases written with each
historical metadata shape.

### UniFFI Bindings

- `rust/uniffi.toml` configures the shared library name (`coveffi`) and Kotlin
  package (`org.bitcoinppl.cove_core`).
- `cargo run -p uniffi_cli` invokes the custom CLI wrapper for binding
  generation.
- Generated bindings are first emitted to `rust/bindings/` (temporary), then
  copied:
  - Swift: `rust/bindings/*.swift` -> `ios/CoveCore/Sources/CoveCore/generated/`
  - Kotlin: `rust/bindings/kotlin/` ->
    `android/app/src/main/java/org/bitcoinppl/cove_core/`
- Do not manually edit generated files.
- After changing exported Rust APIs, run `just build-ios` and/or
  `just build-android` to regenerate bindings and update call sites.

### Manager Ownership and Cleanup

Managers obtained via `app.getWalletManager()` or `app.getSendFlowManager()` are
owned by `AppManager`. Components should not call `.close()` on them or clear
them from route-level `DisposableEffect` cleanup. Only close managers created
locally (e.g., `CoinControlManager`, `ImportWalletManager`, `TapSignerManager`).
For short-lived managers like `LabelManager`, use `.use { }` or
`DisposableEffect` with `.close()` if stored in state. `Database()` returns an
`Arc` clone of a global singleton and does not need closing.

On Android, keep generated UniFFI manager handles private to platform managers;
constructors that receive them should be internal, and Rust access should go
through wrapper methods backed by the shared `RustHandleGuard`.

### iOS ↔ Android Parity

The project aims for shared structure and terminology while embracing each
platform's native idioms. See `docs/ios_android_parity.md` for detailed UI/UX
parity guidance, including opacity, colors, text auto-sizing, NFC scanning UI,
slider behavior, lifecycle mappings, and navigation patterns.

### Topic-Specific Docs

Read these before changing related code:

- `docs/cloud_backup.md` - Cloud Backup architecture and flows.
- `docs/icloud_drive.md` - iCloud Drive discovery and file coordination.
- `docs/passkeys.md` - Passkey behavior and Cloud Backup confirmation.
- `docs/keyteleport.md` - KeyTeleport security and lifecycle.
- `docs/redb.md` - redb compatibility checklist.
- `docs/ios_android_parity.md` - Cross-platform UI patterns.

## Build System and Common Commands

### Prerequisites

- Rust (stable) and `cargo-nextest`.
- Just (`cargo install just`).
- iOS: Xcode 16.0+, swiftformat, swiftlint.
- Android: Android Studio + NDK, Java 17/JDK, `ANDROID_HOME`,
  `ANDROID_SDK_ROOT`, `ANDROID_NDK_HOME`, `JAVA_HOME`.
- Optional: `bacon`, `watchexec`.

Copy `.envrc.example` to set environment variables locally.

### Essential Just Recipes

| Command | Alias | Description |
|---------|-------|-------------|
| `just build-android` | `just ba` | Build Android debug Rust FFI + Kotlin bindings (ARM64) |
| `just build-android-connected-device` | `just bad` | Build Android debug for connected device ABI |
| `just build-android-release` | `just bar` | Build Android release APK |
| `just build-ios` | `just bi` | Build iOS debug for simulator |
| `just build-ios-debug-device` | `just bidd` | Build iOS debug for device |
| `just build-ios-release` | `just bir` | Build iOS release for device |
| `just build-run-ios --udid <udid>` | `just bri` | Rebuild iOS bindings, install, and run on device |
| `just run-ios` | `just ri` | Run iOS using existing bindings |
| `just compile-ios` | - | Compile iOS without regenerating bindings |
| `just compile-android` | - | Compile Android without regenerating bindings |
| `just test` | - | Run Rust tests with nextest |
| `just test-gh` | - | Run Rust tests with `cargo test` (matches CI) |
| `just watch-test` | `just wtest` | Watch and re-run Rust tests |
| `just fmt` | - | Format Rust, Swift, and Android |
| `just lint` | - | Lint Rust, Swift, and Android |
| `just ci` | - | Run format, lint, clippy, tests, and compile |
| `just full` | `just f` | Full build and verification for all platforms |
| `just clean` | - | Remove all build artifacts |

Use `just` to list all recipes.

### Development Workflow

- **Rust-only changes**: `just bacon` or `just bcheck` for continuous feedback.
- **UI changes (no Rust API changes)**: `just compile-ios` or
  `just compile-android` for faster iteration.
- **Rust API or UniFFI changes**: `just build-ios` and/or `just build-android`
  to regenerate bindings.
- **Before committing**: run `just fmt`, then `just ci`.
- **If bindings changed**: run `just build-ios` and `just build-android` before
  committing so generated bindings stay in sync.

### Release Builds

- **iOS**: `just testflight` - bumps build number, archives, and uploads to
  App Store Connect. Requires `ASC_API_KEY_PATH`, `ASC_API_KEY_ID`, and
  `ASC_API_ISSUER_ID`.
- **Android**: `just build-android-release` then sign APK/AAB via Android
  Studio.

## Testing

### Rust Tests

- Primary runner: `cargo nextest` via `just test`.
- CI runner: `cargo test --locked --workspace` via `just test-gh`.
- Config: `rust/.config/nextest.toml`.
- Some tests require mutual exclusion on the shared root data directory and are
  grouped under `shared-root-data` (`max-threads = 1`).
- Run a specific test: `just test <test-name>`.
- Watch mode: `just watch-test`.

### iOS Tests

- Unit/layout tests: `ios/CoveTests/` (XCTest).
- UI tests: `ios/CoveUITests/`.
- Run from CLI: `just ios-ui-background` or `just ios-ui-foreground`.
- CI runs CoveTests against an iPhone 17 simulator.

### Android Tests

- Unit tests: `android/app/src/test/java/`.
- Instrumentation/UI tests: `android/app/src/androidTest/java/`.
- Screenshot tests: Compose preview screenshot tests via the Android Screenshot
  Testing library.
  - Update references: `just android-preview-screenshots-update`
  - Validate: `just android-preview-screenshots-validate`
- Manual full-launch tests: `just android-ui-manual`.
- CI runs unit tests with `./gradlew testDevDebugUnitTest`.

### CI

GitHub Actions workflows (`.github/workflows/`):

- `ci.yml` - rustfmt, Swift lint, ktlint/detekt, Android/iOS compile, Rust
  tests, clippy. Also verifies that generated bindings are up to date.
- `regenerate-bindings.yml` - Manually triggered workflow that regenerates and
  commits UniFFI bindings.
- `mobile-artifacts.yml` - Builds Android debug APK and iOS CoveCore artifacts.

## Code Style and Conventions

### Rust

- MSRV: 1.95. Edition: 2024 for the main crate; workspace edition: 2021.
- Prefer `From` implementations for error conversions; avoid standalone
  conversion functions.
- Use `cove_util::ResultExt::map_err_str` and `.map_err_prefix` instead of
  manual `.map_err(|e| Error::Variant(e.to_string()))` patterns.
- Never use `pub(in ...)` or `pub(super)`; use `pub(crate)` or `pub`.
- Do not use `mod.rs`; use `module_name.rs` and `module_name/new_module.rs`.
- Never manually edit generated UniFFI files.
- Use `parking_lot` locks everywhere.

### Formatting (Rust, Swift, Kotlin)

- Explanatory comments go immediately above the statement or arm they describe,
  separated from the previous step by a blank line.
- In function bodies, blank lines follow this priority:
  1. After a multi-line construct before the next statement or block.
  2. Between distinct logical phases (setup → validate → mutate → return).
  3. Keep related single-line statements tight when a blank line adds no value.
  4. Keep a short, single-phase body together.
  5. Do not add a blank line only before the final expression.
- Also blank:
  - After setup or result bindings before new control flow.
  - Between state mutations and final returns when those are separate phases.
  - Between multi-line `match` or `switch` arms.
- Keep tightly related consecutive single-line assignments together.

### Swift

- Swift 6 language mode.
- Use `swiftformat` (config: `.swiftformat`) and `swiftlint`
  (config: `.swiftlint.yml`).
- Custom AST lint rules in `tools/cove-swift-lint/` (config:
  `.swift-ast-lint.yml`); the `swiftui-view-body-complexity` rule limits body
  complexity to 10.
- Avoid `ToolbarItem(placement: .principal)` on iOS 26 due to a known SwiftUI
  freeze at large accessibility font sizes. Use `.navigationTitleView { }` from
  `ios/Cove/Views/NavigationTitleView.swift` instead.

### Kotlin / Android

- ktlint and detekt for linting (configs: `.editorconfig` ktlint rules,
  `android/detekt.yml`).
- Generated `cove_core/` and `uniffi/` code is excluded from ktlint and detekt.
- Compose state management: prefer callbacks (`value: T`,
  `onValueChange: (T) -> Unit`) over `MutableState<T>` parameters.
- Use Navigation3 (`NavDisplay`) for stack navigation.

### Commit Messages

- Imperative mood: "Add feature", not "Added feature".
- Capitalize the subject, no trailing period.
- Add a body when context helps.
- Example:
  ```
  Add UTXO locking for coin control

  Prevent selected UTXOs from being spent by other transactions
  while a send flow is in progress.
  ```

## Security Considerations

- Secrets live in the OS keychain/keystore, not in redb or plaintext files.
- The `KeychainAccess` trait is a UniFFI callback interface implemented by iOS
  (Keychain) and Android (Android Keystore + EncryptedSharedPreferences).
- KeyTeleport seed transfer trusts an unlocked Cove session; it does not require
  new authentication. Clipboard cleanup on Android is best effort.
- Secret zeroization is best effort; `WalletXprv` wipes its own storage, but
  underlying BDK/rust-bitcoin copies do not guarantee zeroization.
- Cloud Backup uses end-to-end encryption with passkey-protected master keys.
- Passkey registration/assertion on iOS is bounded to five minutes; the separate
  presence probe stays short (one second).
- Report suspected vulnerabilities privately to
  security@covebitcoinwallet.com. Do not open public issues until disclosure is
  coordinated.

## AI Agent Guidance

When changing code in this project:

1. Read the relevant section of `ARCHITECTURE.md` and any topic-specific doc in
   `docs/` before starting.
2. Prefer structurally correct fixes over temporary UI-side workarounds. Fix the
   data model when the current types don't match the real domain.
3. When exported Rust APIs change, regenerate bindings with `just build-ios`
   and/or `just build-android` and update all affected Swift/Kotlin call sites.
4. Before changing redb tables or persisted types, read `docs/redb.md` and add
   regression tests for historical metadata shapes.
5. Before changing Android manager ownership, generated `.rust` access, `close()`,
   or route-level `DisposableEffect` cleanup, read the manager ownership section
   above and `docs/ios_android_parity.md`.
6. Run `just fmt` and `just ci` (or the relevant subset) before considering a
   change complete.
7. Do not create commits, push branches, install apps on devices, or run release
   builds unless the user request explicitly authorizes it.
