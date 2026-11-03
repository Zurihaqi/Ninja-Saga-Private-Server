# Sprint 1 — Protocol compatibility

Started: November 3, 2026. Branch: `sprint/1-protocol-compatibility`.

## Objective

Reduce the two protocol risks blocking gameplay migration: AMF object/trait references and legacy Engine.IO 3 / Socket.IO framing. Produce executable evidence and a bounded transport spike while the original MySQL/client baseline remains pending.

## Backlog and ownership

| Task | Owner | Acceptance criteria |
|---|---|---|
| NSR-011 AMF references | Protocol | Decode repeated objects and traits using PHP golden fixtures; reject invalid references, cycles and expansion beyond budgets; preserve existing fixtures |
| NSR-012 Legacy framing | Protocol | Bound EIO3 UTF-16 polling lengths; parse text packet kinds and Socket.IO namespace/event subset; reject malformed and unsupported binary/ack frames |
| NSR-013 Polling transport spike | Multiplayer | Isolated `/compat` echo namespace; bounded sessions/queues, namespace isolation, client heartbeat, close/reconnect and expiry; HTTP lifecycle tests |
| NSR-014 Interoperability harness | QA/integration | Exercise actual Socket.IO 2 client over polling against Rust; multibyte echo and reconnect; distinguish this evidence from Flash verification |
| NSR-015 Sprint acceptance | Integration lead | Workspace format, Clippy and tests pass; document behavior, limits and outstanding client/database gates |

## Contracts

`crates/socket-protocol` owns encoding/decoding only. The isolated `apps/protocol-spike` owns transport/session state and exposes a `/compat` echo namespace. Production chat/PvP handlers and authentication are separate future tasks. AMF code remains in `crates/amf-codec` with explicit resource budgets.

## Exit gate

The sprint can deliver a protocol proof of concept after reference and client interoperability tests pass. Phase 1 acceptance still requires the actual Flash client and viable transports for its connection settings. Original server setup, MySQL schema execution, login capture, WebSocket upgrades, Flash socket policy and gameplay remain in the backlog.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo build --workspace --locked`
- `npm ci --prefix tests/compatibility/socket-client` and `npm test --prefix tests/compatibility/socket-client` (development-only interoperability tooling)

Results and remaining gates are maintained in `docs/migration/progress.md`.

## Local acceptance evidence

NSR-011 through NSR-014 have implementations and local checks: 25 Rust tests pass, strict Clippy passes, and the Socket.IO 2.5.0 client completes its polling lifecycle. NSR-015 local review preserves the Phase 1 gate: actual Flash connectivity, database baseline and remaining AMF/transport types are still required. The work is ready for review; hosted CI and production compatibility have not been established.
