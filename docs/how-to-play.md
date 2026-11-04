# How to play

This Rust version is a local PvE preview. Complete [runtime setup](runtime-setup.md) first. A clone does not include the AIR runtime, SWF assets, local credentials or catalog export.

The browser game is `/app/`; the dashboard is `/admin` → `/app/#admin`. Build the web client and set `WEB_CLIENT_ROOT`. Admin login requires an admin account (`account_type=99`) or an explicitly configured secret. On the configured development workstation, the local helper stores its unique secret in ignored `.tools/admin-secret.txt`.

## First session

1. Start the provisioned MySQL database and Rust backend. If already running, use that instance; check `/health/live` for liveness.
2. Launch the prepared AIR client. On the configured workstation, use `.tools/client/NSCUSTOM.exe` in the development checkout. An unmodified legacy client points at different hostnames.
3. Register an account or log in. Existing workstation development credentials are stored locally in `.tools/local-account.json`, outside Git.
4. Select or create a character, choose its starting element and enter the village.
5. Open the profile and allocate available attribute points. Allocated plus free points must equal your level.
6. Buy affordable shop equipment and equip it from inventory. Learn affordable academy skills for your elements, then equip them. Saved skill presets require premium.
7. Select the first available mission, defeat its enemies and finish through the result screen to save XP/gold.
8. Return to the village. Reconnect and load the character to check saved progress.

The first mission is verified in the actual AIR client. Later systems below are implemented and headless-tested; broader actual-client acceptance remains open.

## Rank progression

| Level | Current rank | Exam and promotion |
|---|---|---|
| 20 | Genin | Chunin Exam → Chunin |
| 40 | Chunin | Jounin Exam → Jounin |
| 60 | Jounin | Special Jounin Exam → Special Jounin with class choice |
| 80 | Special Jounin | Ninja Tutor Exam → Senior Ninja Tutor |

Complete unlocked stages in sequence, then claim promotion from the panel. Rank caps block further normal leveling until promotion. Rank and its reward package save together.

## Other implemented systems

- Buy, rename and release pets; equipping requires Chunin or higher. Equipped pets earn mission XP. Renaming consumes Rename Badges.
- Recruit up to two paid NPC teammates; completing a mission consumes the squad.
- Forge from 65 reference recipes. Gold forging consumes materials and a base weapon; token forging consumes tokens and the base weapon.
- Discover talents, spend TP on upgrades, buy TP packages and reset talents using the required seals.
- Discover Sage Mode at level 80 for 2,000,000 gold, then spend SS on upgrades and equip Sage skills.
- Upgrade advanced academy skills through configured prerequisite chains. Changing an existing Special Jounin class spends tokens.

Daily rewards, hunting/markets, Shadow War, friends/mail, pet training/combining, food/skill learning, animations, events, administrative APIs and realtime polling now have implementation. Complete progression earning, clan/crew integration, staged S missions and actual-client multiplayer remain acceptance work. See [progress](progress.md).

## Troubleshooting

| Symptom | Action |
|---|---|
| Cannot connect | Check MySQL/backend and that the prepared client targets the listener. |
| Database not configured | Set `DATABASE_URL` before starting the backend. |
| Missing icons/maps | Check original assets and both asset-root variables. |
| Session error | Log in again; another login rotates the key. |
| Invalid battle token | Restart the mission; tokens expire after 30 minutes or a backend restart. |
| Method not implemented | Check remaining work and report the panel/action and method name. |
| Readiness 503 | Expected in this preview; production acceptance is unfinished. |

Saved progress survives backend restarts; unfinished missions do not. Report errors with action, character level/rank and error text, excluding credentials/session keys. [Headless tests](../tests/compatibility/gameplay/README.md) check server behavior without recording or controlling the game window.
