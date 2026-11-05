# Local playable development setup

The existing workspace has Rust 1.93.0, portable GNU linker tools, MySQL 8.4.7, original catalogs and a separately configured AIR client. Toolchains, credentials, downloaded assets and database files live under ignored `.tools/` and `reference/upstream/`.

This guide applies to `sprint/1-protocol-compatibility`, which retains workstation helpers. The runtime review branch excludes those tools and has its own configuration guide. For player steps, see [how to play](how-to-play.md); for promotion rules, see [branch policy](branch-policy.md).

## Start the existing setup

MySQL uses `.tools/mysql.ini`, loopback port 3307 and `.tools/mysql-data`. If it is stopped, start it in PowerShell:

```powershell
Start-Process -FilePath "$PWD/.tools/mysql-8.4.7-winx64/bin/mysqld.exe" -ArgumentList ('--defaults-file="' + "$PWD/.tools/mysql.ini" + '"') -WindowStyle Hidden
```

With the game server stopped, build and start:

```powershell
. ./scripts/dev-env.ps1
cargo build -p game-server -p chat-server -p pvp-server --bins --locked
./scripts/migrate-local.ps1
npm run build --prefix apps/web-client
./scripts/start-local.ps1 -IncludeRealtime
```

Install locked frontend dependencies with `npm ci --prefix apps/web-client --ignore-scripts` before its first build. The migration helper explicitly applies additive schema extensions to `ninja_rust_test`; it does not reimport catalogs or reset player data.

Open `http://127.0.0.1:8080/app/`, or launch `.tools/client/NSCUSTOM.exe`. Synthetic game login details are in `.tools/local-account.json`. The admin dashboard is `http://127.0.0.1:8080/admin`: use an admin account (`account_type=99`) or the unique ignored `.tools/admin-secret.txt` created by the local helper. There is no built-in admin password. Do not commit the secret. `-IncludeRealtime` starts authenticated WebSocket and polling chat/PvP on ports 3002/3000.

The playable server uses `ninja_rust_test`; original migration/seed reference uses `ninja_reference`. Logs and PID files are `.tools/<service>.log`, `.err` and `.pid`. Stop the relevant workspace process before rebuilding it on Windows; restarting drops unfinished battle sessions but preserves committed player data.

## Test without the game window

```powershell
./scripts/test-gameplay.ps1
./scripts/test-gameplay.ps1 -ReferenceCatalog
```

Both modes use dedicated schemas and create uniquely named synthetic accounts deleted on success. Minimal fixtures use `ninja_ci_test`; `-ReferenceCatalog` imports the original catalog export into `ninja_ci_reference_test`. The suite changes event/config fixtures as well as synthetic players, so the helper never targets the playable database. See the [test contract](../tests/compatibility/gameplay/README.md).

## Reproduce the reference and import

Follow `reference/README.md` to materialize the pinned source, public game data and client. PHP with intl, fileinfo and zip is used only for baseline setup and fixture generation; the running game backend is Rust.

`scripts/bootstrap-reference.php` initializes the local reference credentials/databases on a newly initialized loopback MySQL, runs the original migrations and reseeds reference catalogs. It deliberately operates on `ninja_reference`, never the playable database. It uses a session-only relaxed SQL mode for upstream event inserts missing a required image field. `scripts/export-reference.php` writes the checked-in schema and ignored catalog export.

Import a fresh target explicitly, using `DATABASE_URL` for that target:

```powershell
cargo run -p game-server --bin import-reference --locked -- .tools/reference-catalogs.json
python scripts/prepare-local-client.py
```

The client preparation script copies the original into `.tools/client`, redirects service and merchant URLs to loopback, gives the copy its own AIR storage ID, then compiles the skills, Pet Villa and free-product panel repairs. Java and `.tools/ffdec/ffdec.jar` are required for this preparation step. The pinned source remains unchanged. Full assets must be available for the copy. No game assets or upstream credentials are committed to this repository.

## Update an existing local client

```powershell
python scripts/patch-client-panels.py
python scripts/patch-client-panels.py --check
```

Optional event banners are served locally, and image loading cannot block startup indefinitely.

The helper compiles a temporary candidate, re-exports the bytecode to verify all repair markers, and replaces only the ignored local movie after verification. Restart AIR or reload the browser afterward. No game-window recording is needed.

For an existing database, apply `migrate` before startup. Provision missing event configuration explicitly with `cargo run -p game-server --bin seed-events --locked`; this preserves existing event records and leaves unsupported event panels inactive.

## Free former paid products

Recharge token packs, Emblem/Trial Emblem, Ancestor and the four package tiers now claim through `Billing.claimProduct`, with no payment service or currency deduction. Emblem is permanent; its first activation supplies a Wisdom scroll. Repeat network requests are deduplicated. Gold/token prices for ordinary gameplay purchases remain.

## Current boundaries

Broad gameplay, social/event/admin APIs and realtime WebSocket/polling now have implementation and headless coverage. Later actual-client progression, full clan/crew integration, two-client multiplayer, staged S missions and production operations remain open; see [progress](migration/progress.md). Readiness still reports 503. Docker execution is unverified. Battle sessions are in memory and expire or clear on restart.
