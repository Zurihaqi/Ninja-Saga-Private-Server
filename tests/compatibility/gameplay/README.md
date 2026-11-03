# Headless gameplay regression

The ignored integration test sends AMF3 envelopes through the gateway and checks MySQL state. It covers accounts/characters, points, missions and duplicate rewards, shops, normal/advanced academy, presets, all four exams, pets, NPC recruitment, crafting, consumables, talents, Sage Mode and classes. It never launches, records or controls the AIR client.

Create a dedicated empty MySQL database named `ninja_ci_test`, then set:

```powershell
$env:DATABASE_URL = 'mysql://USER:PASSWORD@127.0.0.1:3307/ninja_ci_test'
$env:BOOTSTRAP_GAMEPLAY_TEST_DB = '1'
$env:GAME_DATA_ROOT = Join-Path $PWD 'tests/compatibility/gameplay'
cargo test -p game-server --lib persistent_account_character_and_ownership_flow --locked -- --ignored --nocapture
```

Replace placeholders with isolated test credentials. Bootstrap refuses any database name other than `ninja_ci_test`; it creates schema and imports synthetic fixtures. CI runs the same command against its MySQL service on port 3306.

To test existing imported original catalogs in an isolated development database, unset `BOOTSTRAP_GAMEPLAY_TEST_DB`, set `DATABASE_URL` to that test database and point `GAME_DATA_ROOT` at the original game-data directory. Never run this against a production database.

Each run creates a uniquely named synthetic account, exercises state changes and deletes the account on success. Failed runs can leave that account for diagnosis. Existing player accounts are not changed. Account/mission tests live in `apps/game-server/src/lib.rs`; extended progression is in `apps/game-server/src/tests/extension.rs`.

The checked-in JSON and SQL are minimal synthetic data; `mission.json` follows the first reference mission's enemy shape. They do not contain SWF assets. The broader development branch retains workstation PowerShell helpers and PHP/Node fixture generators; these are excluded from runtime review. Rendering and combat animation remain actual-client acceptance checks.
