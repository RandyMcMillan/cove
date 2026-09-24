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
printf '{"jsonrpc":"2.0","id":1,"method":"server.version","params":["cove","1.4"]}
' \
  | nc 127.0.0.1 51234
```

Pretty-print with `jq`:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"server.version","params":["cove","1.4"]}
' \
  | nc 127.0.0.1 51234 | jq .
```

Other useful methods:

```bash
# Current block height
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.headers.subscribe","params":[]}
' | nc 127.0.0.1 51234 | jq .

# Get balance for a script hash
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.scripthash.get_balance","params":["<scripthash>"]}
' | nc 127.0.0.1 51234 | jq .

# Get history for a script hash
printf '{"jsonrpc":"2.0","id":1,"method":"blockchain.scripthash.get_history","params":["<scripthash>"]}
' | nc 127.0.0.1 51234 | jq .
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

## Notes

- Ports change every time the node restarts. Always check **Settings > Local Node** for the current session's URLs.
- The node only accepts connections from `127.0.0.1` (localhost). It is not reachable from other devices on the network.
- If the node is in Initial Block Download (IBD), some Esplora/Electrum queries may return stale data until sync completes.
