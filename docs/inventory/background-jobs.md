# Background jobs and commands

Source: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Evidence: static source inspection only; no migrated database or runtime route list has been verified.

Regenerate with `python scripts/schema_inventory.py`. Paths and line numbers refer to the pinned reference. The extractor is lexical: conditional branches, framework defaults and dynamic names require runtime verification.

No `app/Jobs` directory or app `ShouldQueue` implementation was found. Database queue tables exist, but that is not evidence of gameplay queue jobs.

## Scheduled work

- `game-events:sync-active`: every minute, without overlapping (`routes/console.php:12`).
- `crew:check-phase`: every fifteen minutes (`bootstrap/app.php:28`).

No scheduler process has been run. Phase advancement and event activation require dedicated runtime tests, restart behavior and timezone verification.

## Artisan commands

| Signature | Description | Source |
|---|---|---|
| `crew:advance-phase {--new-season : Start a brand-new season instead of just advancing the phase} {--phase-hours= : Set how many hours the NEXT phase lasts (sets phase_ends_at)} {--season-days= : When starting a new season, set total season length in days} ` | Advance the active crew season to the next phase (or start a new season) | [app/Console/Commands/CrewAdvancePhase.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/CrewAdvancePhase.php#L1) |
| `crew:check-phase ` | Automatically advance the crew phase if phase_ends_at has passed | [app/Console/Commands/CrewCheckPhase.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/CrewCheckPhase.php#L1) |
| `game:dump-data ` | Downloads, decompresses, and saves Ninja Sage game data as JSON files, then seeds the database. | [app/Console/Commands/DumpGameData.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/DumpGameData.php#L1) |
| `character:fix-items {--dry-run} ` | Fixes miscategorized character_items and applies currency rewards. | [app/Console/Commands/FixCharacterItems.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/FixCharacterItems.php#L1) |
| `item:fix-categories ` | Fixes item categories based on ID prefixes. | [app/Console/Commands/FixItemCategories.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/FixItemCategories.php#L1) |
| `pet:fix-prices ` | Fixes pet prices and premium status. | [app/Console/Commands/FixPetPrices.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/FixPetPrices.php#L1) |
| `pet:fix-skills ` | Fixes pet skill unlock levels based on known progressions. | [app/Console/Commands/FixPetSkills.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/FixPetSkills.php#L1) |
| `game:reseed-local ` | Decompresses all .bin files in public/game_data/ back into .json files (useful after a reset or corruption). | [app/Console/Commands/ReseedLocalGameData.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/ReseedLocalGameData.php#L1) |
| `mail:send {character_id : The ID of the character to send the mail to} {--title=Test Mail : Mail subject line} {--body=This is a test mail from the admin. : Mail body text} {--sender=Admin : Display name for the sender} {--rewards= : Comma-separated reward strings, e.g. gold_50000,tokens_15} ` | Send a mail to a character (admin utility) | [app/Console/Commands/SendMail.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/SendMail.php#L1) |
| `game-events:sync-active {--dry-run : Print what would change without saving} ` | Activate/deactivate game events based on their starts_at and ends_at schedule. | [app/Console/Commands/SyncGameEventActive.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Console/Commands/SyncGameEventActive.php#L1) |

`routes/console.php:7` also defines the non-gameplay `inspire` command. Data dump/reseed/fix commands can mutate catalogs or player state; do not treat them as harmless startup hooks. Node battle/chat timers are transport lifecycle behavior and are covered by the multiplayer audit.
