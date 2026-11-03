# Runtime progress — 2026-11-03

Status: **local PvE preview; production promotion pending**. Runtime review is based on development checkpoint `bd31536a` and subsequent documentation. Original `main` is preserved on `legacy/php-node` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. This PR has not replaced or deployed `main`.

| Flow | Evidence |
|---|---|
| Accounts, characters, village, first mission | AIR checks; user-confirmed battle/completion; headless persistence/replay checks |
| Equipment, points, shops, academy and presets | Headless ownership, transaction and save/reload checks |
| Four rank exams through Ninja Tutor | Headless stage gates and concurrent promotion claims |
| Pets and NPC recruits | Headless buying/equipment/rename/release, pet XP and squad consumption |
| Blacksmith and consumables | 65 recipes implemented; gold forging/material rollback and consumption tested |
| Advanced academy and classes | Headless prerequisite/replacement and class-change checks |
| Talents and Sage Mode | Headless discovery, upgrades, resets, TP packages, Sage equipment and persistence |

The runtime-only branch passes formatting, strict Clippy, 20 unit/fixture tests, explicit MySQL integration against isolated CI fixtures, and the optimized release build for both runtime binaries. The full development workspace previously passed 28 regular tests and MySQL integration with original catalogs as well. Excluding socket experiments accounts for the lower runtime test count. Headless success does not establish rendered/client acceptance. Hosted CI and container execution remain separate gates.

## Remaining gameplay, in priority order

1. Training/secondary battle modes supplying TP, SS and materials, completing the progression economy.
2. Friend recruitment and friends/mail/clans/crews.
3. Production PvP/chat and actual Flash transport compatibility.
4. Events, pet training/combining, secondary inventory/account features and administration.
5. Actual AIR acceptance of the implemented later progression panels and battles.

Complete gameplay loops take priority over lower-priority polish. No completion percentage is claimed. Unsupported methods return errors. Normal combat is simulated by the client, with server-side session, catalog and reward checks.

Readiness remains 503; mission tokens remain in memory. Hosted CI, containers, staging provisioning, asset delivery, backups/restore, rollback and deployment hardening require acceptance on the reviewed revision/environment. See [production acceptance](production-readiness.md).
