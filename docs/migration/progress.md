# Migration progress — 2026-11-03

Plan status: **core playability in progress** on the existing `sprint/1-protocol-compatibility` branch. The first local PvE slice works in the actual AIR client; the user confirmed battling and completing the first mission. This is an early playable slice, not full game parity. No new branch is created per sprint.

Runtime checkpoint: `bd31536a` implements persistent PvE progression and protocol compatibility. Release preparation separates runtime review from ongoing development: original `main` is preserved as `legacy/php-node`, and `release/rust-runtime` excludes workstation tools and protocol experiments. The replacement PR remains draft; `main` has not been cut over. See [branch policy](../branch-policy.md) and [how to play](../how-to-play.md).

Review: [draft PR #1](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/pull/1). Runtime checkpoint `05045df4` passes 20 unit/fixture tests, isolated MySQL integration, formatting, strict Clippy and an optimized release build. Development documentation and tooling remain on this branch; the runtime PR retains tests/CI and operating docs. Hosted CI, staging/container operation and production acceptance remain open.

## Current priority: complete gameplay loops

User direction: prioritize a fully playable game before polishing lower-priority edge cases and visual details. Use headless AMF/MySQL regression tests by default; actual-client checks are occasional acceptance checks. Authentication, transactional purchases and duplicate-reward protection remain part of normal implementation.

| Gameplay loop | Status |
|---|---|
| Register/login, create/select character, village | Verified in AIR client and automated MySQL flow |
| First mission, battle and XP/gold reward | Verified in AIR client and confirmed by user; headless persistence/replay checks pass |
| Attribute allocation/reallocation | Implemented after user-reported missing method; headless save/reload and level-up-budget tests pass |
| Normal shop: buy, equip and sell | Implemented; headless transaction and persistence flow passes |
| Academy: learn/equip skills and element acquisition | Implemented; headless flow passes |
| Skill presets | Get/create/save implemented with source premium/cost rules; headless flow passes |
| Rank exams through Ninja Tutor | Four exams implemented; headless stage unlock, level/rank gates, promotion and concurrent claim checks pass |
| Pets and NPC recruits | Buy/equip/rename/release pets, mission XP, paid NPC squads and mission consumption implemented; headless persistence checks pass |
| Blacksmith and consumables | 65 pinned recipes, gold/material and token forging, battle items, TP consumables and reward packs implemented; headless rollback/consumption checks pass |
| Advanced academy and Special Jounin classes | Skill-chain upgrades, equipped skill/preset replacement and class changes implemented; headless flow passes |
| Talents and Sage Mode | Discovery, upgrades, talent resets/TP packages and Sage skill equipment implemented; headless flow passes |
| Friends/mail/clans/crews | Pending |
| PvP/chat production integration | Pending; text-polling experiment only |
| Events, secondary modes and full admin | Pending |

The normal progression API flow now covers gear, skills, points, missions, all four rank gates, pets, NPC recruits, crafting, talents and Sage Mode with saved progress. Actual AIR acceptance beyond the first mission remains open. Next core work: training/secondary battle modes that supply TP/SS/materials, friend recruitment and social systems, then production PvP/chat and event loops. Pet villa/combining and secondary inventory/account features also remain. No percentage is reported because method counts do not measure gameplay completeness.

## Foundation inventory (historical Sprint 0/1 scope)

| Task | Current evidence | Remaining |
|---|---|---|
| NSR-001 baseline | Pinned source, original locked PHP dependencies, MySQL 8.4.7 migrated/seeded; actual client runs against Rust | Full original-server gameplay comparison |
| NSR-002 AMF inventory | 276 gateway-resolvable public declarations, 168 helper declarations; dispatch documented | Captured client calls and full behavioral contracts |
| NSR-003 realtime audit | Event inventory, Socket.IO/Engine.IO lock versions, auth/policy differences | Live Flash handshakes |
| NSR-004 database inventory | 34 migrations executed with documented event-insert workaround; schema exported; 8,602 catalog rows imported | Wider integrity and rollback verification |
| NSR-005 admin/routes/jobs | 16 resource classes, 20 pages, HTTP and scheduler inventories | Runtime permission checks |
| NSR-006 AMF spike | Bounded codec, acyclic references, ByteArray and actual AIR account/mission calls | Additional types and full method coverage |
| NSR-007 Socket.IO spike | EIO3 text framing and isolated polling `/compat` namespace; Socket.IO 2.5.0 client checks pass | Flash networking, WebSocket upgrades, production auth/events |
| NSR-008 foundation | Binaries, SQLx pool, logs/config/shutdown/liveness; lockfile, CI and Compose | Docker execution and deployment hardening |
| NSR-009 fixtures | PHP serializer/deserializer fixtures and legacy JS parser fixture; 28 regular Rust tests plus explicit MySQL integration | Captured login/gameplay, DB differential harness |
| NSR-010 review | Initial module contracts documented | Protocol gates prevent full acceptance |

The current chat/PvP binaries remain health-only process shells; the polling experiment is separate. Full protocol acceptance, particularly Flash realtime transport, remains open while core AMF/PvE playability advances under the user's revised priority.

Intentional differences: bounded strict AMF decoding; explicit ownership checks rather than the upstream gateway's session-string heuristic; point budgets enforced using the actual profile's level-based rule; purchases and mission rewards are transactional. Unsupported methods return explicit errors. Readiness still reports 503 pending broader acceptance. The diagnostic response retains the original `Laravel Ninja Saga Revived` string for byte compatibility.

Validation: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Docker/CI definitions are supplied but have not run in their hosted environments.

Live smoke validation: `python scripts/smoke_servers.py` passed for all three binaries over loopback, including both gateway aliases and PHP fixture equality. The sandbox blocked loopback connections; rerunning the same checks outside the sandbox succeeded. Smoke processes were stopped after testing.

Current validation includes 28 regular Rust tests and one explicitly invoked AMF/MySQL gameplay integration test covering the loops above. Both the populated development database (including original JSON catalogs) and minimal CI fixture database pass. Formatting and strict Clippy pass. The Socket.IO 2.5.0 harness supplies polling evidence; actual Flash realtime networking is still pending. CI definitions include MySQL-backed gameplay checks; hosted execution is pending.
