# Ninja Saga Rust migration

Sprint 0 research and an initial Rust protocol spike implementing the [migration plan](docs/migration/plan.md). **Not a playable server.**

The pinned source inventory is in [docs/inventory](docs/inventory); actual progress and remaining gates are in [docs/migration/progress.md](docs/migration/progress.md).

## Project layout

| Folder | Contents |
|---|---|
| `apps/` | Executable game, chat and PvP services |
| `crates/` | Shared infrastructure and AMF codec, with crate-local tests |
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

`/amf` and `/gateway.php` support the diagnostic `Game.getVersion` method and AMF0/AMF3-wrapped responses. Other methods return a clearly identified spike error. Authentication, persistence, gameplay, Socket.IO and the admin UI are not implemented. See [codec limits](crates/amf-codec/README.md).

## Verify and regenerate

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
python scripts/smoke_servers.py
python scripts/protocol_inventory.py
python scripts/schema_inventory.py
php crates/amf-codec/tests/generate-fixtures.php
php tests/compatibility/generate-gateway-fixtures.php
```

Generators require the [pinned reference checkout](reference/README.md). Checked-in golden fixtures allow Rust tests to run without PHP or the reference checkout.

`docker compose -f infra/compose.yaml up --build` describes the foundation services. Optional `--profile database` requires a unique `MYSQL_PASSWORD`; it creates an isolated test database on localhost port 3307. No migrations run automatically. Docker is unavailable on the current workstation, so container execution is unverified. Image digests, deployment health checks and release hardening remain pending.
