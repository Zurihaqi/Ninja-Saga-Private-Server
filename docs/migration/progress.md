# Migration progress — 2026-11-05

Status: **broad gameplay implementation, integration and acceptance in progress**. Work continues on `sprint/1-protocol-compatibility`; [draft PR #1](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/pull/1) reviews the runtime candidate. `main` remains the production destination. Original PHP/Node source and assets are preserved on `legacy/php-node` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`.

The first mission is verified in AIR and confirmed by the user. The [Ruffle desktop spike](../research/ruffle-client-spike.md) records the same slice running in Ruffle. A React/Ruffle browser host and admin dashboard now build successfully; broader actual-client acceptance is still open. Headless AMF/MySQL testing remains the default, with no game-window recording or automation.

## Implemented and checked headlessly

| Area | Implementation and evidence |
|---|---|
| Accounts and progression | Login/creation/loading, equipment, points, missions, reward replay protection, four exams through Ninja Tutor, normal/advanced academy, presets, talents and Sage Mode |
| Shops and inventory | Buy/sell/equip, element learning, animation buying/use, 65 blacksmith recipes, consumables, Material/Mysterious markets, Special Deals and Exotic Packages |
| Pets | Buying, equipment, rename/release, mission XP, Villa training/slots, combining, food MP and explicit paid skill learning with saved unlocks |
| PvE and daily modes | Hunting House, Eudemon Garden, daily reward/roulette/scratch, welcome login and Shadow War; S-rank mission data endpoint is implemented, but staged S-mission progression remains open |
| Social | Friends, friend recruitment and mail; clan/crew HTTP services exist and have API checks; the shipped REST client now has exact create/join/accept/membership routes and crew phase1/phase2 persistence/replay checks; older AMF aliases still include placeholders |
| Realtime | Chat/global and clan channels, announcements, PvP matchmaking/rooms/combat/spectators have implementation and router tests; Engine.IO 3/4 direct WebSocket and polling transports; two-client chat authentication/Unicode/heartbeat and PvP WebSocket checks pass, with mocked test characters |
| Events | Event catalog, packages, giveaways, leaderboard/PvP APIs and nine seasonal event handlers; representative database checks, not exhaustive acceptance of every event |
| Administration | REST resources, commands/configuration/seasons/chat and React dashboard at `/admin` → `/app/#admin`; bcrypt admin login and explicitly configured secret login |
| Client/build | React/Ruffle host at `/app/`; locked Ruffle assets generated at build time, excluded from Git; Docker image definitions include built web assets and runtime executables |

The pet level-setting action is restricted to administrators because its unbound client developer function has no retail cost contract. New purchased pets learn skills explicitly; existing pets retain previous level-based unlock behavior. Additive migration `database/migrations/001_pet_skills.sql` stores paid learning without changing existing player rows. Run `cargo run -p game-server --bin migrate --locked` with the target `DATABASE_URL` before starting this version on an existing database.

## Authorization and test isolation

Clan/crew token issuance now always validates the session and saves both tokens atomically. Missing/wrong sessions cannot rotate credentials. Admin and chat have no built-in secret; admin passwords require bcrypt verification, and failed token creation cannot fall back to a predictable value.

The local launcher generates an ignored `.tools/admin-secret.txt` when `ADMIN_SECRET` is unset. It can start chat/PvP with `-IncludeRealtime`. See [local setup](../local-development.md), [how to play](../how-to-play.md) and [web client instructions](../../apps/web-client/README.md).

The expanded regression changes global event/configuration fixtures. Both local test modes therefore use isolated schemas: `ninja_ci_test` for minimal fixtures and `ninja_ci_reference_test` for original catalogs. The playable `ninja_rust_test` database is not the regression target. CI uses only its disposable `ninja_ci_test` service database.

## Validation of this integration

The development workspace passes 36 regular Rust tests, strict Clippy, formatting, minimal-fixture and original-catalog MySQL regressions, and the React/Ruffle build. Live startup checks verify the game/browser host, authenticated admin overview and chat/PvP liveness. Realtime router tests now compose shared health routes exactly as process startup does, catching duplicate-route panics. Generated Ruffle assets and local secrets are excluded from Git. Hosted checks for the refreshed PR and Docker execution are separate acceptance steps.

## November 5 client repair checkpoint

Event banner loading now uses same-origin images and a five-second optional-image deadline; task completion is guarded against duplicate callbacks. The user confirmed the repaired browser reaches login/the game.

The combined local SWF is compiled from guarded repairs and re-exported to verify all 12 patched classes. Skill inventory exposes all owned skills; missing artwork no longer traps selection or equipped slots. Presets now match the client: first free, then 200 premium/600 free-account tokens, maximum four. Seven upstream skill movies remain missing, so their actual animation/content is not restored.

Pet Villa stops hidden voice timelines, cancels stale icon requests, clears reused holders and refreshes training timers safely. Backend Villa responses use catalog movie names, authoritative balances and character-level/85 caps. Talent and Sage Upgrade Max accept ActionScript floating-point flags; second Secret discovery requires rank 6.

The user confirmed skill switching and Villa model loading work, but reported pet effects overlapping during paging and continuing outside the Villa. Display-object muting did not contain independently started sound channels. The installed client now routes static previews to 141 generated silent movies, preserving sound symbol bindings and duration while removing timeline audio; original combat movie hashes remain unchanged. Binary regression tests and compiled-client verification pass. User confirmation after restarting the game remains pending.

Event fixes cover the shipped start/finish argument positions, boss IDs, reward configuration and formula fields, minigame codes/replay protection, gacha coin balances/prices and milestone quantities. `seed-events` explicitly inserts missing reference configurations while keeping unsupported panels inactive. These checks cover representative active events, not every event in the source catalog.

Real-money products are now free local claims through `Billing.claimProduct`: Recharge token packs, permanent Emblem/Trial Emblem, Ancestor and four package tiers. Claims validate ownership, deduplicate requests, retain admin roles and never deduct gameplay currency. The first free Emblem grants a Wisdom scroll. Merchant redirects and native purchase actions are removed from the prepared local client; obsolete recharge-coupon buttons are hidden. Ordinary gold/token gameplay costs remain.

Both minimal and original-catalog isolated regressions pass, including repaired skill/pet/social flows, both crew battle phases and free-product replay checks. Strict Clippy, frontend build and 36 regular Rust tests pass. Full real-client acceptance and production gates remain open.

## Still required for a fully playable release

1. Verify complete player progression and natural TP/SS/material earning through real client flows; finish staged S missions and any remaining client-called actions found by that audit.
2. Accept the repaired clan/crew panels and complete any remaining battle/shop operations found through real-client checks. Basic membership and both crew battle phases have headless REST coverage.
3. Accept complete chat and PvP battles with two real AIR/Ruffle clients, including disconnect/reconnect, rewards and spectators. The required direct WebSocket transport is now implemented and checked headlessly.
4. Accept later exams, pets, events and administration in AIR/Ruffle/browser; unit tests and a successful frontend build do not prove all panels work.
5. Validate target-platform containers, migrations, asset delivery, backup/restore and rollback. Readiness still returns 503; mission/battle sessions are in memory and expire or clear on restart.

No percentage or full feature parity is claimed. The [source inventories](../inventory/) and [migration plan](plan.md) remain references; this document supersedes their historical sprint status. Complete gameplay loops take priority over lower-priority polish.
