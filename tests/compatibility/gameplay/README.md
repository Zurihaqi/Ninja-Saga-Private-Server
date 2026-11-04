# Headless gameplay regression

The ignored integration test sends real AMF envelopes through the gateway and checks MySQL. It covers core progression, shops/animations, pets/paid learning, hunting/markets/daily systems, friends/mail/Shadow War, representative events/admin APIs and authorization rejection. It never launches or records the client. Realtime lifecycle/router tests are separate in the chat/PvP crates and use mocked characters.

The regression changes global event/config fixtures as well as synthetic accounts. It refuses every schema except dedicated `ninja_ci_test` and `ninja_ci_reference_test`. Create an isolated test database and run:

```powershell
$env:DATABASE_URL = 'mysql://USER:PASSWORD@127.0.0.1:3307/ninja_ci_test'
$env:BOOTSTRAP_GAMEPLAY_TEST_DB = '1'
$env:GAME_DATA_ROOT = Join-Path $PWD 'tests/compatibility/gameplay'
cargo test -p game-server --lib persistent_account_character_and_ownership_flow --locked -- --ignored --nocapture
```

Replace placeholders with isolated credentials. Bootstrap creates the baseline schema, additive migrations and synthetic fixtures. CI provides MySQL on port 3306 and uses the same command. For original catalogs, import the export into `ninja_ci_reference_test`, retain bootstrap for supplementary fixtures and point `GAME_DATA_ROOT` at original JSON.

The development branch retains `scripts/test-gameplay.ps1` and its `-ReferenceCatalog` helper; those workstation scripts are excluded here. Accounts are uniquely named and deleted on success; failures can leave test accounts for diagnosis. All global fixture changes stay in the test schema. Never target a playable or production database.

Coverage lives in `apps/game-server/src/lib.rs`, `src/tests/extension.rs` and `src/tests/player_actions.rs`. Original-catalog requirements are read from their definitions rather than assumed from synthetic fixtures. Actual-client rendering, complete event coverage and full two-client multiplayer acceptance remain open.
