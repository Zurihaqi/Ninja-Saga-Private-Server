# Build and configure the runtime preview

This is a review/staging procedure. Container execution, backups/restore, deployment hardening and readiness acceptance remain open in [production acceptance](production-readiness.md).

## Prerequisites

- Rust 1.93.0 and a supported native linker.
- MySQL 8.4 with a dedicated database/user.
- Original game-data catalogs and AIR client/assets from `legacy/php-node` or the development checkout. Assets and credentials are excluded from this branch.
- Reference catalog JSON exported on the development branch using `scripts/export-reference.php`. It contains catalog rows, not player accounts. A fresh clone alone does not provision a playable game.

## Build and provision

```powershell
cargo build --release --workspace --locked
$env:DATABASE_URL = 'mysql://USER:PASSWORD@127.0.0.1:3307/DEDICATED_DATABASE'
cargo run --release --package game-server --bin import-reference --locked -- 'PATH/TO/reference-catalogs.json'
```

Replace placeholders and keep credentials outside Git and shell transcripts. The database must already exist and the importer needs schema/catalog write permissions. It creates missing tables and upserts catalog rows. Do not point it at an existing deployment as a migration shortcut; back up a database before changing its catalogs. The server does not import or seed on startup.

For an already provisioned database, explicitly apply additive migrations before starting this version:

```powershell
cargo run --release --package game-server --bin migrate --locked
```

Migration `001_pet_skills` creates paid pet-learning state without changing existing player rows. The original schema remains the baseline. No migration runs automatically on server startup.

## Configure and start

| Variable | Meaning |
|---|---|
| `DATABASE_URL` | SQLx MySQL URL. Without it, gameplay methods return a configuration error. |
| `GAME_DATA_ROOT` | Original game-data directory including `mission.json`, `library.json` and `senjutsu.json`. Served at `/game_data`. |
| `CLIENT_ASSET_ROOT` | Original client asset directory; used for static fallback delivery. |
| `WEB_CLIENT_ROOT` | Absolute path to the built `apps/web-client/dist`; served at `/app/`. |
| `ADMIN_SECRET` | Optional explicit admin secret; without it, use a bcrypt-backed account with `account_type=99`. No default secret exists. |
| `CHAT_ADMIN_SECRET` | Explicit secret for chat announcements; unset disables that administrative endpoint. |
| `CHAT_SOCKET_URL` / `PVP_SOCKET_URL` | Endpoints reachable by the player, defaulting to local ports 3002 and 3000 (`/pvp`). Set them for remote clients. |
| `APP_URL` | HTTP origin reachable by the client; supplies its CDN base URL. |
| `BIND_ADDR` | Listener address; defaults to `127.0.0.1:8080`. |
| `RUST_LOG` | Log filter, normally `info`. |

```powershell
$env:GAME_DATA_ROOT = 'PATH/TO/game_data'
$env:CLIENT_ASSET_ROOT = 'PATH/TO/NS Custom Client V1'
$env:APP_URL = 'http://127.0.0.1:8080'
$env:RUST_LOG = 'info'
cargo run --release --package game-server --bin game-server --locked
```

Check `/health/live` for liveness. `/amf` and `/gateway.php` accept AMF; empty requests return `AMF Gateway Ready`. `/health/ready` intentionally returns 503 in this preview and is not a deployment signal yet.

## Browser game and admin

```powershell
npm ci --prefix apps/web-client --ignore-scripts --no-audit --no-fund
npm run build --prefix apps/web-client
$env:WEB_CLIENT_ROOT = Join-Path $PWD 'apps/web-client/dist'
```

The prebuild step generates Ruffle assets and licenses from the locked dependency; generated files stay out of Git. Visit `/app/` for the game and `/admin` for the dashboard (`/app/#admin`). The original prepared client assets are still required at `CLIENT_ASSET_ROOT`.

Log in to admin using an admin account or an explicitly configured `ADMIN_SECRET`. The development branch's local helper creates ignored `.tools/admin-secret.txt`; no workstation helper or private secret is shipped here.

For realtime, start the separate `chat-server` and `pvp-server` binaries with the same database and appropriate listener/secret/catalog environment. Their defaults are ports 3002/3000; set `BIND_ADDR` per process. Realtime supports direct Engine.IO 3/4 WebSocket and polling. Actual-client multiplayer acceptance remains pending.

## AIR client

Prepare a separate client copy using the [client build instructions](../client/README.md). The helper redirects legacy endpoints and merchant URLs, compiles the skill/Pet Villa/free-product repairs, verifies the resulting bytecode, and preserves the original movie. Java/FFDec are preparation dependencies; they are not required by the running servers.

Use the repaired copy as `CLIENT_ASSET_ROOT`. Restart AIR or reload Ruffle after rebuilding it. Original movies/assets and local toolchains remain excluded from Git.

Former real-money products are free local claims. Apply `migrate` before starting this version on an existing schema; `003_free_products` stores deduplicated claims and `002_gear_presets` saves gear presets. Provision missing event definitions explicitly with `cargo run -p game-server --bin seed-events --locked`. Existing event settings are preserved; unsupported panels remain inactive.

## Containers

Configure `DATABASE_URL`, `APP_URL`, `GAME_DATA_PATH` and `CLIENT_ASSET_PATH`, then run:

```powershell
docker compose -f infra/compose.yaml up --build
```

The database URL must work from inside the container; `127.0.0.1` there refers to the container itself. Provisioning and additive migrations are separate. Assets are read-only mounts and host listeners default to loopback. The image includes game/chat/PvP binaries, explicit import/migration/event-provisioning utilities and built web assets, without PHP/Node services or the protocol experiment. Node is only a frontend build dependency. Docker has not been run on the development workstation.

## Restarts and diagnosis

Use one server per listener. Stop that process before rebuilding its executable on Windows. Mission tokens remain in memory, expire after 30 minutes and do not survive restarts; unfinished missions must restart. Committed progress remains in MySQL. Another login rotates the account session key.

Report errors with method/panel, action and character level/rank, excluding passwords, database URLs and session keys. See the [headless regression](../tests/compatibility/gameplay/README.md) for reproducible server checks.
