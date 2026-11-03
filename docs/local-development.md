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
cargo build -p game-server --bin game-server --locked
./scripts/start-local.ps1
```

Launch `.tools/client/NSCUSTOM.exe`. Synthetic local login details are in `.tools/local-account.json`. The Rust server uses `ninja_rust_test`; the original Laravel migration/seed reference uses `ninja_reference`. Server logs are `.tools/game-server.log` and `.tools/game-server.err`. Do not start a second server while port 8080 is in use.

## Test without the game window

```powershell
./scripts/test-gameplay.ps1
./scripts/test-gameplay.ps1 -ReferenceCatalog
```

This tests the saved gameplay loops listed in the [test contract](../tests/compatibility/gameplay/README.md), using a uniquely named synthetic account deleted on success. The second command reads original JSON catalogs; the first uses minimal checked-in JSON. Neither changes existing player accounts or interacts with the desktop. CI uses a separate `ninja_ci_test` schema and minimal fixtures.

## Reproduce the reference and import

Follow `reference/README.md` to materialize the pinned source, public game data and client. PHP with intl, fileinfo and zip is used only for baseline setup and fixture generation; the running game backend is Rust.

`scripts/bootstrap-reference.php` initializes the local reference credentials/databases on a newly initialized loopback MySQL, runs the original migrations and reseeds reference catalogs. It deliberately operates on `ninja_reference`, never the playable database. It uses a session-only relaxed SQL mode for upstream event inserts missing a required image field. `scripts/export-reference.php` writes the checked-in schema and ignored catalog export.

Import a fresh target explicitly, using `DATABASE_URL` for that target:

```powershell
cargo run -p game-server --bin import-reference --locked -- .tools/reference-catalogs.json
python scripts/prepare-local-client.py
```

The client preparation script copies the original client into `.tools/client`, changes four endpoint constants to loopback, and gives the test copy its own AIR storage ID. It preserves bytecode and gameplay logic and leaves the pinned source unchanged. Full assets must be available for the copy. No game assets or upstream credentials are committed to this repository.

## Current boundaries

First-mission PvE is verified in AIR. Exams, pets, NPC recruits, crafting, consumables, talents, Sage Mode, shops, academy and character progression have headless coverage. Secondary battle/reward modes, social features, PvP/chat, events and administration remain incomplete. See [progress](migration/progress.md). Readiness still reports 503 pending wider compatibility acceptance. Docker definitions have not been executed locally. Mission tokens are in memory and expire after 30 minutes; a backend restart invalidates an unfinished mission, while committed rewards and characters remain in MySQL.
