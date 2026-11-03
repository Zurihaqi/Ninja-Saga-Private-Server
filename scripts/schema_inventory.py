"""Generate source inventories; no PHP execution or database changes.

This is a lexical index, not a PHP parser or a substitute for migrated SQL inspection.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "reference/upstream/Database/Laragon/ninjasage"
OUT = ROOT / "docs/inventory"
COMMIT = "ecf9012d885d1c89fd5f42791eed4f37f90f2edd"


def read(p):
    return p.read_text(encoding="utf-8-sig")


def ref(p, line=1):
    rel = p.relative_to(SRC).as_posix()
    return f"[{rel}:{line}](../../reference/upstream/Database/Laragon/ninjasage/{rel}#L{line})"


def matches(p, pattern):
    text = read(p)
    return [(m, text.count("\n", 0, m.start()) + 1) for m in re.finditer(pattern, text, re.M)]


def save(name, title, content):
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / name).write_text(
        f"# {title}\n\nSource: `{COMMIT}`. Evidence: static source inspection only; no migrated database or runtime route list has been verified.\n\n"
        "Regenerate with `python scripts/schema_inventory.py`. Paths and line numbers refer to the pinned reference. The extractor is lexical: conditional branches, framework defaults and dynamic names require runtime verification.\n\n"
        + content + "\n", encoding="utf-8")


def database():
    migrations = sorted((SRC / "database/migrations").glob("*.php"))
    models = sorted((SRC / "app/Models").glob("*.php"))
    seeders = sorted((SRC / "database/seeders").glob("*.php"))
    s = f"{len(migrations)} migrations, {len(models)} models and {len(seeders)} seeders.\n\n"
    s += "## Schema hazards and ownership\n\n"
    s += "- Two migrations define the clan tables. The February 22 creation runs first; the February 23 conditional creation skips existing tables, leaving different defaults, nullability and foreign keys than its declarations suggest. Both rollback methods drop the same tables. Preserve the actual migrated schema, not whichever file looks newer.\n"
    s += "- `chat_messages` is created by the Node chat service, outside Laravel migrations; include its DDL in the eventual schema baseline.\n"
    s += "- Some migrations mutate event and inventory data. Replaying only CREATE TABLE statements is insufficient.\n"
    s += "- `GameDataSeeder` deletes catalog/config rows before loading local JSON; missing JSON returns an empty array. Never run it against valuable data. Its local disk path is `storage/app/private/game_data`, whereas `TalentSeeder` reads `public/game_data/talents.json`.\n"
    s += "- No MySQL schema dump, row counts, collation inspection, migration execution or foreign-key integrity checks have been captured yet.\n\n"
    s += "## Migration declarations in execution order\n\n"
    for p in migrations:
        s += f"### {p.name}\n\n{ref(p)}\n\n```php\n"
        # Only up() declarations; exclude rollback from the forward inventory.
        up = read(p).split("public function down", 1)[0]
        for n, line in enumerate(up.splitlines(), 1):
            if re.search(r"Schema::|\$table->|DB::table", line):
                s += f"// L{n}\n{line.strip()}\n"
        s += "```\n\n"
    p = SRC / "chat-server/src/messageStore.js"
    ddl = re.search(r"const CREATE_TABLE = `(.+?)`;", read(p), re.S).group(1)
    s += f"## Chat-owned schema\n\n{ref(p, 6)}\n\n```sql\n{ddl.strip()}\n```\n\n"
    s += "## Models and declared relationships\n\n| Model | Explicit table / relationship declarations | Source |\n|---|---|---|\n"
    for p in models:
        declarations = []
        for m, n in matches(p, r"(?:protected \$table\s*=.*?;|return \$this->(?:hasMany|hasOne|belongsTo|belongsToMany|morphMany|morphTo)\([^;]+;)"):
            declarations.append(f"L{n}: `{m.group(0).strip()}`")
        s += f"| {p.stem} | {'<br>'.join(declarations) or 'Framework table inference; no relationship matched'} | {ref(p)} |\n"
    s += "\n## Seeders\n\nDatabaseSeeder calls AdminUser, EudemonGarden, GameData, XP, Talent, GameEvent, SpecialDeals in that order. AdminUserSeeder contains development account provisioning; do not reuse its credential defaults.\n\n"
    for p in seeders:
        model_names = re.findall(r"use App\\Models\\(\w+);", read(p))
        loads = re.findall(r"loadJson\('([^']+)'\)", read(p))
        s += f"- **{p.stem}** — models: {', '.join(model_names) or 'none imported'}; JSON catalogs: {', '.join(loads) or 'none via loadJson'}. {ref(p)}\n"
    save("database-schema.md", "Database schema inventory", s)


def routes():
    p = SRC / "routes/web.php"
    s = "## Registration and middleware\n\n"
    s += "`bootstrap/app.php` registers `routes/web.php`, `routes/console.php`, and health `/up`. It does not register `routes/clan.php`; that file is an alternate/unwired definition, not an additional active route set. Filament dynamically registers `/admin` routes (see admin inventory).\n\n"
    s += "CSRF exceptions cover `amf`, `gateway.php`, `clan/*`, `crew/*`. The root-level crew aliases are registered through the web middleware but are not exempted: POST requests without CSRF tokens may fail there even when their `/crew/*` equivalents succeed. This is source-derived and needs a runtime regression fixture.\n\n"
    s += "Clan protected routes use `ClanAuthMiddleware`; crew protected routes use `CrewAuthMiddleware`. Public login/season routes still inherit web middleware. Every crew row below also has a root alias obtained by removing `/crew`. No hostname constraint appears on those aliases.\n\n"
    s += "| Method | Registered path | Handler | Auth | Source |\n|---|---|---|---|---|\n"
    for m, n in matches(p, r"Route::(get|post|any)\('([^']+)'\s*,\s*(?:\[(\w+)::class,\s*'([^']+)'\]|function)"):
        method, uri, controller, action = m.groups()
        prefix = "/clan" if controller == "ClanApiController" else "/crew" if controller == "CrewApiController" else ""
        auth = "public" if not controller or action in ("handle", "login", "season") else ("clan.auth" if controller == "ClanApiController" else "crew.auth")
        s += f"| {method.upper()} | `{prefix}{uri}` | {controller + '::' + action if controller else 'closure'} | {auth} | {ref(p,n)} |\n"
    s += "| GET | `/up` | Laravel health | public | " + ref(SRC / "bootstrap/app.php", 11) + " |\n"
    s += "\n## Static asset behavior\n\n`/enemy/{filename}` accepts an alphanumeric/underscore/hyphen `.swf` filename and returns 404 for missing base directories/files. `/skills/{filename}` first checks the exact file, then a database `skills.swf` alias or `SKILL_DEFAULT_SWF` (default `skill_01.swf`), then 404. Both return `application/x-shockwave-flash`. Base directories are controlled by `ENEMY_SWF_PATH` / `SKILL_SWF_PATH`. Static web-server/public assets are not exhaustively represented by application routes.\n"
    save("http-routes.md", "HTTP route inventory", s)


def admin():
    s = "## Access boundary\n\n`AdminPanelProvider` configures `/admin`, login, session/CSRF middleware and Filament Authenticate. `User::canAccessPanel` allows account type 99 **or** email `admin@ninjasage.test`. This email bypass must be recorded as an upstream security defect, not silently treated as role-based authorization. No app Policies directory or explicit resource canCreate/canEdit/canDelete overrides were observed. Framework authorization defaults are not verified.\n\n"
    s += f"Sources: {ref(SRC / 'app/Models/User.php',58)}, {ref(SRC / 'app/Providers/Filament/AdminPanelProvider.php',25)}.\n\n"
    resources = sorted((SRC / "app/Filament/Resources").glob("*/*Resource.php"))
    pages = sorted((SRC / "app/Filament/Pages").glob("*.php"))
    s += f"{len(resources)} resource classes across {len(list((SRC / 'app/Filament/Resources').iterdir()))} resource directories; {len(pages)} standalone page classes.\n\n## Resource classes\n\n"
    for p in resources:
        text = read(p)
        model = re.search(r"\$model\s*=\s*(\w+)::class", text)
        s += f"- **{p.stem}** — model: `{model.group(1) if model else 'see source'}`. {ref(p)}\n"
        for m, n in matches(p, r"'[^']+'\s*=>\s*(?:Pages\\)?\w+::route\('[^']*'\)"):
            s += f"  - `{m.group(0)}` (L{n})\n"
    s += "\n## Standalone pages and callable methods\n\n| Page | Public methods | Source |\n|---|---|---|\n"
    for p in pages:
        methods = [f"{m.group(1)} (L{n})" for m, n in matches(p, r"public\s+(?:static\s+)?function\s+(\w+)\s*\(")]
        s += f"| {p.stem} | {', '.join(methods)} | {ref(p)} |\n"
    s += "\n## Actions, forms and relationship managers\n\nThis index includes declared action and field identifiers, not implicit framework actions or rendered UI verification. Validate permission, validation and database effects per operation before replacing it.\n\n"
    for p in sorted((SRC / "app/Filament").rglob("*.php")):
        entries = [(m, n) for m, n in matches(p, r"\b(\w+)::make\((?:'([^']*)')?\)") if m.group(1).endswith("Action") or m.group(2)]
        if not entries:
            continue
        s += f"### {p.relative_to(SRC / 'app/Filament').as_posix()}\n\n{ref(p)}\n\n"
        s += ", ".join(f"`{m.group(1)}({m.group(2) or ''})` L{n}" for m, n in entries) + ".\n\n"
    save("admin-features.md", "Administration inventory", s)


def jobs():
    s = "No `app/Jobs` directory or app `ShouldQueue` implementation was found. Database queue tables exist, but that is not evidence of gameplay queue jobs.\n\n## Scheduled work\n\n- `game-events:sync-active`: every minute, without overlapping (`routes/console.php:12`).\n- `crew:check-phase`: every fifteen minutes (`bootstrap/app.php:28`).\n\nNo scheduler process has been run. Phase advancement and event activation require dedicated runtime tests, restart behavior and timezone verification.\n\n## Artisan commands\n\n| Signature | Description | Source |\n|---|---|---|\n"
    for p in sorted((SRC / "app/Console/Commands").glob("*.php")):
        t = read(p)
        sig = re.search(r"\$signature\s*=\s*'([^']+)'", t)
        desc = re.search(r"\$description\s*=\s*'([^']+)'", t)
        s += f"| `{' '.join(sig.group(1).split()) if sig else '?'} ` | {desc.group(1) if desc else '?'} | {ref(p)} |\n"
    s += "\n`routes/console.php:7` also defines the non-gameplay `inspire` command. Data dump/reseed/fix commands can mutate catalogs or player state; do not treat them as harmless startup hooks. Node battle/chat timers are transport lifecycle behavior and are covered by the multiplayer audit.\n"
    save("background-jobs.md", "Background jobs and commands", s)


def gameplay():
    s = "This is a migration scope index, not a claim that each service works. AMF dispatch names and callable methods are cataloged separately in `amf-methods.md`. Per-method response, mutation and failure contracts remain required before porting gameplay.\n\n"
    s += "| Service / helper | Directly imported models | Source |\n|---|---|---|\n"
    for p in sorted((SRC / "app/Services").rglob("*.php")):
        models = re.findall(r"use App\\Models\\(\w+);", read(p))
        s += f"| {p.relative_to(SRC / 'app/Services').with_suffix('').as_posix()} | {', '.join(models) or 'none directly imported'} | {ref(p)} |\n"
    s += "\n## Existing test inventory\n\nPresence does not imply passing tests; Laravel dependencies/database were not initialized for this inventory.\n\n"
    for p in sorted((SRC / "tests").rglob("*.php")):
        s += f"- {ref(p)}\n"
    save("gameplay-systems.md", "Gameplay implementation scope", s)


if __name__ == "__main__":
    if not SRC.is_dir():
        raise SystemExit(f"Missing pinned reference checkout: {SRC}")
    database()
    routes()
    admin()
    jobs()
    gameplay()
    print("Generated five source inventories.")
