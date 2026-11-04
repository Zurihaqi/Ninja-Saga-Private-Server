# Migration progress — 2026-11-04

Status: **broad gameplay implementation, integration and acceptance in progress**. Work continues on `sprint/1-protocol-compatibility`; [draft PR #1](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/pull/1) reviews the runtime candidate. `main` remains the production destination. Original PHP/Node source and assets are preserved on `legacy/php-node` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`.

The first mission is verified in AIR and confirmed by the user. The [Ruffle desktop spike](../research/ruffle-client-spike.md) records the same slice running in Ruffle. A React/Ruffle browser host and admin dashboard now build successfully; broader actual-client acceptance is still open. Headless AMF/MySQL testing remains the default, with no game-window recording or automation.

## Implemented and checked headlessly

| Area | Implementation and evidence |
|---|---|
| Accounts and progression | Login/creation/loading, equipment, points, missions, reward replay protection, four exams through Ninja Tutor, normal/advanced academy, presets, talents and Sage Mode |
| Shops and inventory | Buy/sell/equip, element learning, animation buying/use, 65 blacksmith recipes, consumables, Material/Mysterious markets, Special Deals and Exotic Packages |
| Pets | Buying, equipment, rename/release, mission XP, Villa training/slots, combining, food MP and explicit paid skill learning with saved unlocks |
| PvE and daily modes | Hunting House, Eudemon Garden, daily reward/roulette/scratch, welcome login and Shadow War; S-rank mission data endpoint is implemented, but staged S-mission progression remains open |
| Social | Friends, friend recruitment and mail; clan/crew HTTP services exist and have API checks; AMF clan/crew operations still include reference-style placeholder responses |
| Realtime | Chat/global and clan channels, announcements, PvP matchmaking/rooms/combat/spectators have implementation and router tests; polling transport only, with mocked characters in realtime lifecycle tests |
| Events | Event catalog, packages, giveaways, leaderboard/PvP APIs and nine seasonal event handlers; representative database checks, not exhaustive acceptance of every event |
| Administration | REST resources, commands/configuration/seasons/chat and React dashboard at `/admin` → `/app/#admin`; bcrypt admin login and explicitly configured secret login |
| Client/build | React/Ruffle host at `/app/`; locked Ruffle assets generated at build time, excluded from Git; Docker image definitions include built web assets and runtime executables |

The pet level-setting action is restricted to administrators because its unbound client developer function has no retail cost contract. New purchased pets learn skills explicitly; existing pets retain previous level-based unlock behavior. Additive migration `database/migrations/001_pet_skills.sql` stores paid learning without changing existing player rows. Run `cargo run -p game-server --bin migrate --locked` with the target `DATABASE_URL` before starting this version on an existing database.

## Authorization and test isolation

Clan/crew token issuance now always validates the session and saves both tokens atomically. Missing/wrong sessions cannot rotate credentials. Admin and chat have no built-in secret; admin passwords require bcrypt verification, and failed token creation cannot fall back to a predictable value.

The local launcher generates an ignored `.tools/admin-secret.txt` when `ADMIN_SECRET` is unset. It can start chat/PvP with `-IncludeRealtime`. See [local setup](../local-development.md), [how to play](../how-to-play.md) and [web client instructions](../../apps/web-client/README.md).

The expanded regression changes global event/configuration fixtures. Both local test modes therefore use isolated schemas: `ninja_ci_test` for minimal fixtures and `ninja_ci_reference_test` for original catalogs. The playable `ninja_rust_test` database is not the regression target. CI uses only its disposable `ninja_ci_test` service database.

## Validation of this integration

The development workspace passes 32 regular Rust tests, strict Clippy, formatting, minimal-fixture and original-catalog MySQL regressions, and the React/Ruffle build. Live startup checks verify the game/browser host, authenticated admin overview and chat/PvP liveness. Realtime router tests now compose shared health routes exactly as process startup does, catching duplicate-route panics. Generated Ruffle assets and local secrets are excluded from Git. Hosted checks for the refreshed PR and Docker execution are separate acceptance steps.

## Still required for a fully playable release

1. Verify complete player progression and natural TP/SS/material earning through real client flows; finish staged S missions and any remaining client-called actions found by that audit.
2. Verify clan/crew panels against the implemented HTTP services and finish the required AMF bridge behavior.
3. Exercise chat and full PvP battles with two actual clients, including disconnect/reconnect, rewards and spectators. Confirm polling covers the client or implement required WebSocket transport.
4. Accept later exams, pets, events and administration in AIR/Ruffle/browser; unit tests and a successful frontend build do not prove all panels work.
5. Validate target-platform containers, migrations, asset delivery, backup/restore and rollback. Readiness still returns 503; mission/battle sessions are in memory and expire or clear on restart.

No percentage or full feature parity is claimed. The [source inventories](../inventory/) and [migration plan](plan.md) remain references; this document supersedes their historical sprint status. Complete gameplay loops take priority over lower-priority polish.
