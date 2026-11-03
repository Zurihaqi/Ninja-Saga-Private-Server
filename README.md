# Ninja Saga Rust migration

Rust backend migration with a **first playable local PvE flow**: register/login, create/select a character, enter the village, complete the first mission and receive persisted XP/gold. First-mission gameplay has been verified in the AIR client and confirmed by the user. Full feature parity remains in progress; see the [migration plan](docs/migration/plan.md).

The pinned source inventory is in [docs/inventory](docs/inventory); actual progress and remaining gates are in [docs/migration/progress.md](docs/migration/progress.md).

Start with [how to play](docs/how-to-play.md), [local setup](docs/local-development.md), and the [branch/release policy](docs/branch-policy.md). The migration is a development preview. `main` is the production destination; the Rust replacement must pass acceptance before promotion. The original implementation is preserved on [`legacy/php-node`](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/tree/legacy/php-node).

## Project layout

| Folder | Contents |
|---|---|
| `apps/` | Game, chat and PvP services plus an isolated protocol experiment |
| `crates/` | Shared infrastructure, AMF and legacy socket codecs, with crate-local tests |
| `database/` | Exported MySQL schema and minimal headless test fixtures |
| `tests/compatibility/` | Cross-service gateway fixtures and their generator |
| `docs/` | Architecture, inventories, protocol contracts and migration plan |
| `scripts/` | Development setup, source inventory and verification commands |
| `infra/` | Docker builds and Compose configuration |
| `reference/` | Pinned upstream metadata and checkout instructions |

Local toolchains live in ignored `.tools/`; build artifacts live in ignored `target/`. The legacy implementation and assets remain available through the pinned upstream history and ignored reference checkout.

## Run

Requires Rust 1.93.0. On this workstation, an isolated toolchain and portable GNU linker were installed in ignored `.tools/`; no system PATH changes were made:

```powershell
. ./scripts/dev-env.ps1
cargo test --workspace --locked
cargo run -p game-server --locked
```

On a normally configured Rust installation, omit the first line. Windows GNU needs binutils/GCC. The local helper enables Rust's bundled runtime libraries to avoid a `libgcc_eh` linker mismatch.

Game listens on `127.0.0.1:8080`; chat/PvP process shells default to ports 3002/3000. Override `BIND_ADDR` with an IP and port; use `RUST_LOG` to configure structured logs. `/health/live` reports process liveness. `/health/ready` returns 503 until database/protocol acceptance is implemented.

`/amf` and `/gateway.php` support accounts, characters, equipment/points, shops, normal and advanced academy, skill presets, mission rewards, all four rank exams through Ninja Tutor, pets with mission XP, NPC recruitment, 65 blacksmith recipes, consumables, talent discovery/upgrades/resets, Sage Mode discovery/upgrades/equipment, and Special Jounin class changes. These flows have headless AMF/MySQL coverage; first-mission gameplay is also verified in AIR. Social systems, production PvP/chat, secondary battle modes, events and administration remain incomplete. Unimplemented methods return explicit errors. See [progress](docs/migration/progress.md), [codec limits](crates/amf-codec/README.md) and [local setup](docs/local-development.md).

For the already configured workstation, `./scripts/start-local.ps1` starts the Rust backend; the test client is `.tools/client/NSCUSTOM.exe`. MySQL must be running on loopback port 3307. Build before starting the server; Windows cannot replace a running executable.

## Verify and regenerate

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
./scripts/test-gameplay.ps1
./scripts/test-gameplay.ps1 -ReferenceCatalog
cargo build --workspace --locked
python scripts/smoke_servers.py
python scripts/protocol_inventory.py
python scripts/schema_inventory.py
php crates/amf-codec/tests/generate-fixtures.php
php tests/compatibility/generate-gateway-fixtures.php
```

Generators require the [pinned reference checkout](reference/README.md). Checked-in golden fixtures allow Rust tests to run without PHP or the reference checkout.

The [headless gameplay suite](tests/compatibility/gameplay/README.md) is the default regression workflow. It replays AMF calls and checks persistence without launching, recording or interacting with the game window. Actual-client checks are reserved for visual behavior and protocol acceptance.

`docker compose -f infra/compose.yaml up --build` describes the foundation services. Optional `--profile database` requires a unique `MYSQL_PASSWORD`; it creates an isolated test database on localhost port 3307. No migrations run automatically. Docker is unavailable on the current workstation, so container execution is unverified. Image digests, deployment health checks and release hardening remain pending.

## Sprint 1 protocol experiment

`cargo run -p protocol-spike --locked` starts the isolated Engine.IO 3 polling experiment on localhost port 8090. The `/compat` namespace echoes text/JSON events. Run the [legacy client harness](tests/compatibility/socket-client/README.md) to verify Socket.IO 2.5.0 polling, Unicode, heartbeat and reconnect. This experiment has no gameplay authentication; real Flash traffic and WebSocket support remain open gates. See the [sprint backlog](docs/agent-tasks/sprint-1.md) and [transport limits](apps/protocol-spike/README.md).
