# Local Node

Cove can run an embedded [rbitcoin](https://github.com/bitcoinppl/cove/tree/main/rust/rbitcoin) node locally. The node binds to random free ports on `127.0.0.1` and exposes two RPC interfaces:

- **Electrum** — plain TCP (`tcp://127.0.0.1:<port>`)
- **Esplora** — HTTP (`http://127.0.0.1:<port>`)

Ports are chosen dynamically at startup. The current URLs are shown in **Settings > Local Node** under the Status section when the node is running.

## Finding the endpoint

Open **Settings > Local Node** and look for the _Electrum_ and _Esplora_ rows in the Status section. The URLs shown there are the only way to know which ports the node is using for the current session.

Example values:

```
Electrum  tcp://127.0.0.1:51234
Esplora   http://127.0.0.1:51235
```

## Esplora (HTTP)

The Esplora endpoint speaks plain HTTP and can be queried with `curl`, `wget`, or any HTTP client.

### Block height

```bash
curl http://127.0.0.1:51235/blocks/tip/height
```

### Block hash

```bash
curl http://127.0.0.1:51235/blocks/tip/hash
```

### Address info

```bash
curl http://127.0.0.1:51235/address/<address>
```

### Address transactions

```bash
curl http://127.0.0.1:51235/address/<address>/txs
```

### UTXOs for an address

```bash
curl http://127.0.0.1:51235/address/<address>/utxo
```

### Transaction by ID

```bash
curl http://127.0.0.1:51235/tx/<txid>
```

### Broadcast a raw transaction

```bash
curl -X POST \
  -H "Content-Type: text/plain" \
  --data-binary "<hex-encoded-tx>" \
  http://127.0.0.1:51235/tx
```

## Electrum (plain TCP)

The Electrum endpoint speaks the [Electrum protocol](https://electrumx.readthedocs.io/en/latest/protocol.html) over plain TCP. `curl` and `wget` do not speak this protocol natively, so use one of the tools below.

### Using `nc` (netcat) and `jq`

Send a JSON-RPC request line and read the response:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"server.version","params":["cove","1.4"]}\n' \
  | nc 127.0.0.1 51234
```

Pretty-print with `jq`:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"server.version","params":["cove","1.4"]}\n' \
  | nc 127.0.0.1 51234 | jq .
```

Other useful methods:

```bash
# Current block height
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.headers.subscribe","params":[]}\n' \
  | nc 127.0.0.1 51234 | jq .

# Get balance for a script hash
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.scripthash.get_balance","params":["<scripthash>"]}\n' \
  | nc 127.0.0.1 51234 | jq .

# Get history for a script hash
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.scripthash.get_history","params":["<scripthash>"]}\n' \
  | nc 127.0.0.1 51234 | jq .
```

### Using `jsonrpc-cli` or `electrum-client`

If you have a dedicated Electrum client tool installed, point it at the local address:

```bash
# python-electrumx client example
python -c "
import json, socket
s = socket.create_connection(('127.0.0.1', 51234))
s.sendall(b'{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server.version\",\"params\":[\"cove\",\"1.4\"]}\n')
print(s.recv(4096).decode())
"
```

## Transaction Broadcast Path

When you tap "Send" in Cove, the transaction flows through the following layers to reach the local rbitcoin node:

### 1. Swift — `WalletManager.broadcastTransaction()`
`ios/Cove/WalletManager.swift:469`

Calls the Rust FFI wrapper via UniFFI.

### 2. Rust FFI Wrapper — `WalletManager::broadcast_transaction()`
`rust/src/manager/wallet_manager.rs:798`

Sends an async message to the `WalletActor`.

### 3. WalletActor — `broadcast_transaction()` → `start_broadcast_transaction()`
`rust/src/manager/wallet_manager/actor/transactions.rs:629`

Does two things in order:
1. `deferred_node_connection()` — obtains a `NodeClient`
2. `broadcast_to_node_with_connection()` — sends the tx

### 4. Node Connection — `start_node_connection()`
`rust/src/manager/wallet_manager/actor/node.rs:200`

Calls `resolve_selected_node()` which:
- Starts the local node if not running (`LOCAL_NODE_MANAGER.start(network)`)
- Waits up to 300s for RPC endpoints to bind (`wait_for_ready(300)`)
- Returns the resolved `Node` with the dynamic URL

Then `checked_node_client()` creates a fresh `NodeClient`:
- Probes the URL with `check_node_connection_inner()`
- Calls `NodeClient::new(node)` which opens a TCP/HTTP connection

### 5. Broadcast Execution — `broadcast_to_node_with_connection()`
`rust/src/manager/wallet_manager/actor/transactions.rs:1242`

1. Awaits the node connection setup
2. Gets the cached `node_client`
3. Calls `node_client.broadcast_transaction(transaction)`

### 6. Backend Client

**Bitcoin / Testnet (Electrum):**
`rust/src/node/client/electrum.rs:321`
```rust
client.inner.transaction_broadcast(&txn)
```
Sends Electrum JSON-RPC `blockchain.transaction.broadcast` over TCP to rbitcoin.

**Signet (Esplora):**
`rust/src/node/client/esplora.rs:136`
```rust
self.client.broadcast(&txn).await
```
Sends HTTP POST to `/tx` with the raw hex transaction.

### 7. rbitcoin Receives It

**Electrum path:** `rbitcoin/crates/rbitcoin-electrum` listens on TCP, parses the JSON-RPC, and injects the tx into the mempool.

**Esplora path:** `rbitcoin/crates/rbitcoin-esplora` listens on HTTP, routes the POST to the mempool broadcast handler.

Both paths then gossip the transaction to P2P peers.

---

### Common Failure: "Connection refused (os error 61)"

This error occurs at **step 4** when `NodeClient::new()` tries to open a TCP connection to the rbitcoin electrum port, but nothing is listening.

**Why:** rbitcoin's Electrum and Esplora listeners only start after IBD (Initial Block Download) completes. During sync, the P2P node is running but the RPC ports are **closed**.

**Fix:** Wait for the Local Node Status to show **Endpoints Ready** before broadcasting. On first launch with an empty datadir, Signet sync takes 1–3 minutes.

## Notes

- Ports change every time the node restarts. Always check **Settings > Local Node** for the current session's URLs.
- The node only accepts connections from `127.0.0.1` (localhost). It is not reachable from other devices on the network.
- The Electrum and Esplora listeners do not start until after Initial Block Download (IBD) and scripthash index materialization are complete. The UI shows "Starting…" until the endpoints are ready.
- If the node is in IBD, some Esplora/Electrum queries may return stale data until sync completes.
