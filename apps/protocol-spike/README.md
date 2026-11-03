# Polling compatibility experiment

Run `cargo run -p protocol-spike --locked`. Default bind: `127.0.0.1:8090`; override `BIND_ADDR` as needed. This separate binary is opt-in and excluded from the production runtime image and Compose services.

`/socket.io/?EIO=3&transport=polling` opens an Engine.IO 3 session. Use its SID on subsequent GET/POST requests. Only Socket.IO `/compat` connect/disconnect and the `echo` event are accepted; echo responses stay in the originating session. Client ping receives server pong. No transport upgrades are advertised.

Sessions use 256-bit OS randomness, a 128-session cap, a 45-second heartbeat deadline, one outstanding poll, a 20-second poll wait, at most 64 queued packets, an 8 KiB per-packet limit, and a 32 KiB request/aggregate queue limit. Expired sessions are reclaimed when requests access the store; the store remains capped while idle. Poll cancellation clears the exclusive polling slot. Invalid batches fail before changing namespace state. No CORS or cross-domain policy is provided by this local experiment.

The actual Socket.IO 2.5.0 development harness verifies polling, Unicode, isolation, heartbeat and reconnect. Production chat/PvP namespaces, authentication, WebSocket upgrades, binary/ack packets, Flash policy and actual Flash-client acceptance remain pending. `/health/ready` remains 503.
