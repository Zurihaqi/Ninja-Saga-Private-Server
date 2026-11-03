# Headless gameplay regression

Run `./scripts/test-gameplay.ps1` from the configured Windows workspace. It requires the local MySQL instance and imported schema/catalogs, and never launches or controls the game window.

The Rust test sends real AMF3 envelopes through the Axum gateway and verifies MySQL changes. It covers accounts, actual AIR creation nesting/hash, character/battle data, equipment, points, leveling, mission rewards and duplicate completion, session rotation, shops, normal/advanced academy, skill presets, all four exams and concurrent promotion claims, pet ownership/rename/release/mission XP, paid NPC recruitment and mission consumption, forging/material rollback, consumable consumption, talent discovery/upgrades/reset, Sage Mode and class changes. It creates a uniquely named synthetic account and deletes it on success. Failed runs may leave their synthetic account for diagnosis. Existing player accounts are not modified.

Use `./scripts/test-gameplay.ps1 -ReferenceCatalog` to run with the original JSON catalogs. Test code is split between the gateway's account/mission regression and `apps/game-server/src/tests/extension.rs` for progression/companions/crafting.

`mission.json` is the minimal mission-definition subset required by the test, derived from the pinned `public/game_data/mission.json` entry `msn_1`. It is deliberately independent of downloaded SWF assets. The test uses enemy statistics and rewards from the test database.

For CI, set `BOOTSTRAP_GAMEPLAY_TEST_DB=1` and point `DATABASE_URL` at a dedicated database named `ninja_ci_test`. This explicitly permits schema/fixture initialization in that database only; normal local runs use their existing imported database. Hosted CI runs the same AMF and persistence assertions. Full combat rendering and damage animations remain actual-client checks.
