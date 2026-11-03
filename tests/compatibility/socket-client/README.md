# Legacy client interoperability

This development harness starts the Rust `protocol-spike` binary on a temporary loopback port and connects two actual Socket.IO 2.5.0 clients using Engine.IO 3 polling. It verifies `/compat` namespace connection, multibyte echo, per-session isolation, client-initiated ping/pong, and reconnect with a fresh session ID. It stops its child process when finished.

```powershell
cargo build -p protocol-spike --locked
npm ci --prefix tests/compatibility/socket-client --ignore-scripts
npm test --prefix tests/compatibility/socket-client
```

Node 18+ is required for this harness. Exact development dependencies are recorded in `package-lock.json`. This tooling is excluded from production images. The original game client's compiled Flash networking library has not been tested; passing this harness supplies evidence for the selected protocol subset only. WebSocket upgrades, binary events, acknowledgements and gameplay authentication need separate validation.
