# Migration progress — 2026-11-03

Plan status: Sprint 0 underway; Phase 0 and Phase 1 gates are **not passed**. No playable release is claimed.

| Task | Current evidence | Remaining |
|---|---|---|
| NSR-001 baseline | Pinned sparse source; 406 syntax checks pass | Isolated full runtime, MySQL, assets and actual gameplay |
| NSR-002 AMF inventory | 276 gateway-resolvable public declarations, 168 helper declarations; dispatch documented | Captured client calls and full behavioral contracts |
| NSR-003 realtime audit | Event inventory, Socket.IO/Engine.IO lock versions, auth/policy differences | Live Flash handshakes |
| NSR-004 database inventory | 34 migrations, 49 models, 8 seeders; conflicts recorded | MySQL execution, authoritative schema snapshot |
| NSR-005 admin/routes/jobs | 16 resource classes, 20 pages, HTTP and scheduler inventories | Runtime permission checks |
| NSR-006 AMF spike | Bounded std-only codec, diagnostic Axum gateway, PHP golden fixtures | Object/trait refs and additional AMF types; session dispatch |
| NSR-007 Socket.IO spike | Source investigation only | Implement and validate EIO3 framing, heartbeat, namespaces, reconnect |
| NSR-008 foundation | Three binaries, logs/config/shutdown/liveness; Cargo.lock, CI and Compose definitions | SQLx pool, Docker execution, deployment hardening |
| NSR-009 fixtures | 7 golden files; 14 Rust tests, including HTTP byte equality | Captured login/gameplay, DB differential harness |
| NSR-010 review | Initial module contracts documented | Protocol gates prevent full acceptance |

Next work: legacy Engine.IO 3 spike, complete AMF references, provision the original runtime and isolated MySQL. Resolve login/session behavior using captures before porting authentication. The current chat/PvP binaries are health-only process shells, not functioning realtime servers.

Intentional spike deviations: bounded strict AMF decoding; only Game.getVersion implemented; other targets receive explicit spike error text; no global heuristic session gate; readiness always 503. The diagnostic response retains the original `Laravel Ninja Saga Revived` string for byte compatibility.

Validation: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Docker/CI definitions are supplied but have not run in their hosted environments.

Live smoke validation: `python scripts/smoke_servers.py` passed for all three binaries over loopback, including both gateway aliases and PHP fixture equality. The sandbox blocked loopback connections; rerunning the same checks outside the sandbox succeeded. Smoke processes were stopped after testing.
