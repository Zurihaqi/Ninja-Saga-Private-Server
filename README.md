# Ninja Saga Rust runtime

Rust backend for the original Ninja Saga AIR client, currently a **local PvE preview under production review**. The first mission is verified in AIR; broader progression is covered by AMF/MySQL tests. This version is not approved for production deployment.

- [How to play](docs/how-to-play.md)
- [Build and configure](docs/runtime-setup.md)
- [Progress and remaining work](docs/progress.md)
- [Production acceptance](docs/production-readiness.md)

The original PHP/Node implementation and client assets are preserved on [`legacy/php-node`](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/tree/legacy/php-node), at commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Ongoing migration work, local setup helpers, protocol experiments and reference inventories remain on [`sprint/1-protocol-compatibility`](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/tree/sprint/1-protocol-compatibility). No cutover of `main` is implied by this review branch.

## Implemented gameplay

Accounts and characters; equipment and points; shops; normal/advanced academy and skill presets; mission XP/gold; four rank exams through Ninja Tutor; pet purchasing/equipment/mission XP; NPC recruitment; 65 blacksmith recipes; consumables; talents; Sage Mode; and Special Jounin class selection.

Additional implementation now includes hunting/markets, daily rewards, pet Villa/combining, food/paid pet skills, friends/mail, Shadow War, seasonal events, administrative APIs, realtime WebSocket/polling chat/PvP and a React/Ruffle browser/admin host. Later progression and realtime have headless coverage; complete player progression, clan/crew integration and actual-client multiplayer remain acceptance work. Unimplemented AMF methods return errors.

Skills/Pet Villa client repairs, corrected event and clan/crew contracts, saved clan prestige, both crew battle phases and free former real-money products are integrated. Gold/token gameplay prices remain; Recharge, permanent Emblem and package products claim locally without payment. See [client preparation](client/README.md).

## Build and validate

Use Rust 1.93.0; Windows GNU builds also need a compatible linker/toolchain. MySQL 8.4 is used for compatibility tests.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked
npm ci --prefix apps/web-client --ignore-scripts --no-audit --no-fund
npm run build --prefix apps/web-client
```

The [MySQL regression](tests/compatibility/gameplay/README.md) runs explicitly against an isolated database. It uses real AMF envelopes without launching or recording a game window. CI includes both unit/fixture tests and this database flow.

Run `game-server` with `DATABASE_URL`, `GAME_DATA_ROOT`, `CLIENT_ASSET_ROOT`, `WEB_CLIENT_ROOT` and `APP_URL` configured. The game is `/app/`; the admin dashboard is `/admin` → `/app/#admin`. Admin login uses a bcrypt-backed account with `account_type=99` or an explicitly configured `ADMIN_SECRET`; no built-in secret is accepted. See [runtime setup](docs/runtime-setup.md). No import or migration runs automatically. `/health/live` reports liveness; `/health/ready` remains 503 until production acceptance.

## Repository and image contents

| Path | Purpose |
|---|---|
| `apps/` | Game, chat and PvP services, explicit import/migration commands and browser/admin UI |
| `crates/` | AMF codec, authentication, MySQL persistence and shared HTTP process infrastructure |
| `client/` | Required build-time client repairs; no downloaded movies or workstation credentials |
| `database/` | Original schema, static recipes and synthetic CI fixtures |
| `tests/` | Protocol fixtures and headless gameplay data |
| `.github/` | Runtime CI |
| `infra/` | Runtime Docker/Compose definitions; execution still requires staging verification |
| `docs/` | Play instructions, operations, progress and production acceptance |

Local credentials, `.tools/`, downloaded original assets, build outputs, fixture generators, Node polling experiments and migration task notes are excluded. Tests and CI remain. Actual chat/PvP implementations are included; the experimental protocol service is excluded. Ruffle assets/licenses are generated from the locked npm dependency during build. The image includes three service binaries, explicit import/migration/event-provisioning utilities and built browser assets. Node is a frontend build dependency; PHP and Node services are unnecessary at runtime.
