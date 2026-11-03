# Build and configure the runtime preview

This is a review/staging procedure. Container execution, backups/restore, deployment hardening and readiness acceptance remain open in [production acceptance](production-readiness.md).

## Prerequisites

- Rust 1.93.0 and a supported native linker.
- MySQL 8.4 with a dedicated database/user.
- Original game-data catalogs and AIR client/assets from `legacy/php-node` or the development checkout. Assets and credentials are excluded from this branch.
- Reference catalog JSON exported on the development branch using `scripts/export-reference.php`. It contains catalog rows, not player accounts. A fresh clone alone does not provision a playable game.

## Build and provision

```powershell
cargo build --release --package game-server --bins --locked
$env:DATABASE_URL = 'mysql://USER:PASSWORD@127.0.0.1:3307/DEDICATED_DATABASE'
cargo run --release --package game-server --bin import-reference --locked -- 'PATH/TO/reference-catalogs.json'
```

Replace placeholders and keep credentials outside Git and shell transcripts. The database must already exist and the importer needs schema/catalog write permissions. It creates missing tables and upserts catalog rows. Do not point it at an existing deployment as a migration shortcut; back up a database before changing its catalogs. The server does not import or seed on startup.

## Configure and start

| Variable | Meaning |
|---|---|
| `DATABASE_URL` | SQLx MySQL URL. Without it, gameplay methods return a configuration error. |
| `GAME_DATA_ROOT` | Original game-data directory including `mission.json`, `library.json` and `senjutsu.json`. Served at `/game_data`. |
| `CLIENT_ASSET_ROOT` | Original client asset directory; used for static fallback delivery. |
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

## AIR client

Use the separate client copy prepared by `scripts/prepare-local-client.py` on the development branch. On the existing workstation it is `.tools/client/NSCUSTOM.exe` in that checkout, with development credentials stored locally in `.tools/local-account.json`.

The unmodified original SWF points at legacy hostnames. The helper rewrites endpoint constants in a copied SWF and uses a separate AIR storage ID, preserving the original source. No patching helper or runtime assets ship in this runtime branch. Follow [how to play](how-to-play.md) once connected.

## Containers

Configure `DATABASE_URL`, `APP_URL`, `GAME_DATA_PATH` and `CLIENT_ASSET_PATH`, then run:

```powershell
docker compose -f infra/compose.yaml up --build
```

The database URL must work from inside the container; `127.0.0.1` there refers to the container itself. Database provisioning is separate. Assets are read-only mounts; host port 8080 binds to loopback. The final image contains `game-server` and explicitly invoked `import-reference`, without PHP/Node or experimental services. Docker has not been run on the development workstation.

## Restarts and diagnosis

Use one server per listener. Stop that process before rebuilding its executable on Windows. Mission tokens remain in memory, expire after 30 minutes and do not survive restarts; unfinished missions must restart. Committed progress remains in MySQL. Another login rotates the account session key.

Report errors with method/panel, action and character level/rank, excluding passwords, database URLs and session keys. See the [headless regression](../tests/compatibility/gameplay/README.md) for reproducible server checks.
