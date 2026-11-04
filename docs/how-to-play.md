# How to play the Rust development build

The Rust version is a local PvE preview. The first mission was played successfully in the AIR client; later progression systems have automated AMF/MySQL coverage and still need broader client acceptance. See [current progress](migration/progress.md) for unfinished features.

## Start on the configured Windows workstation

1. Open PowerShell in the repository root. Check that MySQL is running on `127.0.0.1:3307`; [local setup](local-development.md) includes its start command.
2. If the backend is already running on port 8080, use it. Otherwise run `./scripts/start-local.ps1`. This requires the built server, ignored local database credentials and imported catalogs.
3. Launch `.tools/client/NSCUSTOM.exe`. Use the development login saved in `.tools/local-account.json`, or register a new account in the client. Keep credentials out of screenshots and commits.
4. Select your existing character, or create one and choose its starting element. Enter the village.

A fresh Git clone does not include the AIR runtime, SWF assets, local toolchain or database. Complete [local setup](local-development.md) and [reference setup](../reference/README.md) first. Running the backend without `DATABASE_URL` only provides diagnostic endpoints.

The browser game is `http://127.0.0.1:8080/app/`. The admin dashboard is `http://127.0.0.1:8080/admin`, redirecting to `/app/#admin`. Build the [web client](../apps/web-client/README.md) and start the backend first. Admin login accepts an admin account or the unique configured secret; the local launcher stores its generated secret in ignored `.tools/admin-secret.txt`.

## First session

1. Open the profile and spend the available attribute points. The total allocated plus free points must equal your character level.
2. Visit the shop to buy equipment you can afford and meet the level/premium requirements for. Equip purchases through the inventory; unwanted owned items can be sold.
3. Visit the academy to learn affordable skills for your elements, then equip the skills you want to use. Additional elements follow the free/premium slot rules. Saved skill presets require premium.
4. Open the mission room and choose the first available mission. Start the battle, defeat its enemies and finish through the result screen to save XP and gold.
5. Return to the village. Reconnect and select the character again to confirm saved progress.

Combat uses the original client's controls and animations. Mission rewards are committed when completion succeeds. Losing the connection or restarting the backend during a mission can require starting that mission again.

## Progression currently implemented

| At level | Required current rank | Exam and resulting rank |
|---|---|---|
| 20 | Genin | Chunin Exam → Chunin |
| 40 | Chunin | Jounin Exam → Jounin |
| 60 | Jounin | Special Jounin Exam → Special Jounin, with a class choice |
| 80 | Special Jounin | Ninja Tutor Exam → Senior Ninja Tutor |

Complete the unlocked exam stages in sequence, then claim promotion from its panel. Promotion and its reward package are saved together. The rank gates limit normal leveling until promotion.

Pets can be bought before use, but equipping requires Chunin or higher. Equipped pets earn XP from completed missions. NPC recruitment supports up to two paid teammates; the squad is consumed when a mission completes. Pet renaming uses Rename Badges.

The blacksmith supports 65 reference recipes. Gold forging consumes the recipe's materials and base weapon; token forging consumes the base weapon and token cost. Talents support discovery, TP upgrades/packages and paid resets. Sage Mode discovery requires level 80 and 2,000,000 gold; Sage skill upgrades spend SS. Advanced academy upgrades follow configured prerequisite chains.

Later systems have headless coverage rather than complete client acceptance. Daily rewards, markets, hunting, Shadow War, friends/mail, pet training/combining, events, realtime polling and administration now have implementation; check [current progress](migration/progress.md) for remaining clan/crew integration, staged S missions, natural resource progression and two-client multiplayer acceptance.

## Troubleshooting

| Symptom | What to check |
|---|---|
| Client cannot connect | Use the prepared local client; check MySQL and the backend, then `http://127.0.0.1:8080/health/live`. |
| Backend says database is not configured | Start through the configured local helper, or set `DATABASE_URL` explicitly. |
| Missing icons, maps or assets | Check `GAME_DATA_ROOT`, `CLIENT_ASSET_ROOT` and the materialized reference assets. |
| Session error after another login | Log in again; a new login rotates the account session key. |
| Invalid battle token | Restart the mission. Tokens expire after 30 minutes and are cleared by a backend restart. |
| Method is not implemented | Check the [progress report](migration/progress.md); record the panel/action and method name for remaining gameplay work. |
| Readiness returns 503 | Expected for this preview. Liveness and production readiness are separate; readiness acceptance is unfinished. |

For regressions, include the action, character level/rank, expected result and error text. Do not send passwords or session keys. Developers can run the [headless gameplay suite](../tests/compatibility/gameplay/README.md) without recording or controlling the game window.
