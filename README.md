# Ninja Saga Rust runtime

Rust backend for the original Ninja Saga AIR client, currently a **local PvE preview under production review**. The first mission is verified in AIR; broader progression is covered by AMF/MySQL tests. This version is not approved for production deployment.

- [How to play](docs/how-to-play.md)
- [Build and configure](docs/runtime-setup.md)
- [Progress and remaining work](docs/progress.md)
- [Production acceptance](docs/production-readiness.md)

The original PHP/Node implementation and client assets are preserved on [`legacy/php-node`](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/tree/legacy/php-node), at commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Ongoing migration work, local setup helpers, protocol experiments and reference inventories remain on [`sprint/1-protocol-compatibility`](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/tree/sprint/1-protocol-compatibility). No cutover of `main` is implied by this review branch.

## Implemented gameplay

Accounts and characters; equipment and points; shops; normal/advanced academy and skill presets; mission XP/gold; four rank exams through Ninja Tutor; pet purchasing/equipment/mission XP; NPC recruitment; 65 blacksmith recipes; consumables; talents; Sage Mode; and Special Jounin class selection.

Training/reward modes, complete TP/SS/material farming, social systems, production PvP/chat, events, pet training/combining and administration remain incomplete. Later progression has automated coverage rather than complete AIR acceptance. Unimplemented AMF methods return errors.

## Build and validate

Use Rust 1.93.0; Windows GNU builds also need a compatible linker/toolchain. MySQL 8.4 is used for compatibility tests.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --package game-server --bins --locked
```

The [MySQL regression](tests/compatibility/gameplay/README.md) runs explicitly against an isolated database. It uses real AMF envelopes without launching or recording a game window. CI includes both unit/fixture tests and this database flow.

Run `game-server` with `DATABASE_URL`, `GAME_DATA_ROOT`, `CLIENT_ASSET_ROOT` and `APP_URL` configured. See [runtime setup](docs/runtime-setup.md) for provisioning and client preparation. No data import runs automatically. `/health/live` reports process liveness; `/health/ready` remains 503 until production acceptance is implemented.

## Repository and image contents

| Path | Purpose |
|---|---|
| `apps/game-server/` | AMF gateway, gameplay and explicit catalog import command |
| `crates/` | AMF codec, authentication, MySQL persistence and shared HTTP process infrastructure |
| `database/` | Original schema, static recipes and synthetic CI fixtures |
| `tests/` | Protocol fixtures and headless gameplay data |
| `.github/` | Runtime CI |
| `infra/` | Runtime Docker/Compose definitions; execution still requires staging verification |
| `docs/` | Play instructions, operations, progress and production acceptance |

Local credentials, `.tools/`, downloaded assets, logs, build outputs, PHP fixture generators, Node polling experiments and migration task notes are excluded. Tests and CI remain to verify the runtime. The container image contains the game executable and explicit import utility; it does not include chat/PvP stubs or the experimental protocol service. PHP and Node are not required by the Rust runtime.
