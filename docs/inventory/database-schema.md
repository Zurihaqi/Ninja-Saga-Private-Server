# Database schema inventory

Source: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Evidence: static source inspection only; no migrated database or runtime route list has been verified.

Regenerate with `python scripts/schema_inventory.py`. Paths and line numbers refer to the pinned reference. The extractor is lexical: conditional branches, framework defaults and dynamic names require runtime verification.

34 migrations, 49 models and 8 seeders.

## Schema hazards and ownership

- Two migrations define the clan tables. The February 22 creation runs first; the February 23 conditional creation skips existing tables, leaving different defaults, nullability and foreign keys than its declarations suggest. Both rollback methods drop the same tables. Preserve the actual migrated schema, not whichever file looks newer.
- `chat_messages` is created by the Node chat service, outside Laravel migrations; include its DDL in the eventual schema baseline.
- Some migrations mutate event and inventory data. Replaying only CREATE TABLE statements is insufficient.
- `GameDataSeeder` deletes catalog/config rows before loading local JSON; missing JSON returns an empty array. Never run it against valuable data. Its local disk path is `storage/app/private/game_data`, whereas `TalentSeeder` reads `public/game_data/talents.json`.
- No MySQL schema dump, row counts, collation inspection, migration execution or foreign-key integrity checks have been captured yet.

## Migration declarations in execution order

### 0001_01_01_000000_create_users_table.php

[database/migrations/0001_01_01_000000_create_users_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/0001_01_01_000000_create_users_table.php#L1)

```php
// L14
Schema::create('users', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->string('name');
// L17
$table->string('username')->unique();
// L18
$table->string('email')->unique();
// L19
$table->tinyInteger('account_type')->default(0); // 0: Free, 1: Premium, 99: Admin
// L20
$table->integer('tokens')->default(0);
// L21
$table->timestamp('email_verified_at')->nullable();
// L22
$table->string('password');
// L23
$table->string('session_key', 64)->nullable()->index();
// L24
$table->rememberToken();
// L25
$table->timestamps();
// L28
Schema::create('password_reset_tokens', function (Blueprint $table) {
// L29
$table->string('email')->primary();
// L30
$table->string('token');
// L31
$table->timestamp('created_at')->nullable();
// L34
Schema::create('sessions', function (Blueprint $table) {
// L35
$table->string('id')->primary();
// L36
$table->foreignId('user_id')->nullable()->index();
// L37
$table->string('ip_address', 45)->nullable();
// L38
$table->text('user_agent')->nullable();
// L39
$table->longText('payload');
// L40
$table->integer('last_activity')->index();
```

### 0001_01_01_000001_create_cache_table.php

[database/migrations/0001_01_01_000001_create_cache_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/0001_01_01_000001_create_cache_table.php#L1)

```php
// L14
Schema::create('cache', function (Blueprint $table) {
// L15
$table->string('key')->primary();
// L16
$table->mediumText('value');
// L17
$table->integer('expiration');
// L20
Schema::create('cache_locks', function (Blueprint $table) {
// L21
$table->string('key')->primary();
// L22
$table->string('owner');
// L23
$table->integer('expiration');
```

### 0001_01_01_000002_create_jobs_table.php

[database/migrations/0001_01_01_000002_create_jobs_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/0001_01_01_000002_create_jobs_table.php#L1)

```php
// L14
Schema::create('jobs', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->string('queue')->index();
// L17
$table->longText('payload');
// L18
$table->unsignedTinyInteger('attempts');
// L19
$table->unsignedInteger('reserved_at')->nullable();
// L20
$table->unsignedInteger('available_at');
// L21
$table->unsignedInteger('created_at');
// L24
Schema::create('job_batches', function (Blueprint $table) {
// L25
$table->string('id')->primary();
// L26
$table->string('name');
// L27
$table->integer('total_jobs');
// L28
$table->integer('pending_jobs');
// L29
$table->integer('failed_jobs');
// L30
$table->longText('failed_job_ids');
// L31
$table->mediumText('options')->nullable();
// L32
$table->integer('cancelled_at')->nullable();
// L33
$table->integer('created_at');
// L34
$table->integer('finished_at')->nullable();
// L37
Schema::create('failed_jobs', function (Blueprint $table) {
// L38
$table->id();
// L39
$table->string('uuid')->unique();
// L40
$table->text('connection');
// L41
$table->text('queue');
// L42
$table->longText('payload');
// L43
$table->longText('exception');
// L44
$table->timestamp('failed_at')->useCurrent();
```

### 2026_01_01_000001_create_items_skills_pets_table.php

[database/migrations/2026_01_01_000001_create_items_skills_pets_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_01_000001_create_items_skills_pets_table.php#L1)

```php
// L15
Schema::create('items', function (Blueprint $table) {
// L16
$table->id();
// L17
$table->string('item_id')->unique(); // e.g. wpn_01
// L18
$table->string('name');
// L19
$table->integer('level')->default(1);
// L20
$table->integer('price_gold')->default(0);
// L21
$table->integer('price_tokens')->default(0);
// L22
$table->string('category'); // weapon, back, set, etc.
// L23
$table->boolean('premium')->default(false);
// L24
$table->string('icon')->nullable();
// L25
$table->timestamps();
// L29
Schema::create('skills', function (Blueprint $table) {
// L30
$table->id();
// L31
$table->string('skill_id')->unique();
// L32
$table->string('name');
// L33
$table->integer('level')->default(1);
// L34
$table->tinyInteger('element')->default(0); // 1:Wind, 2:Fire, etc.
// L35
$table->integer('price_gold')->default(0);
// L36
$table->integer('price_tokens')->default(0);
// L37
$table->boolean('premium')->default(false);
// L38
$table->string('icon')->nullable();
// L39
$table->timestamps();
// L43
Schema::create('pets', function (Blueprint $table) {
// L44
$table->id();
// L45
$table->string('pet_id')->unique(); // Kept as string for compatibility with existing IDs like 'pet_01' if any, but data will be numeric '1'
// L46
$table->string('name');
// L47
$table->string('swf');
// L48
$table->integer('price_gold')->default(0);
// L49
$table->integer('price_tokens')->default(0);
// L50
$table->boolean('premium')->default(false);
// L51
$table->string('icon')->nullable();
// L52
$table->json('skills')->nullable();
// L53
$table->timestamps();
```

### 2026_01_01_000002_create_characters_table.php

[database/migrations/2026_01_01_000002_create_characters_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_01_000002_create_characters_table.php#L1)

```php
// L14
Schema::create('characters', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->foreignId('user_id')->constrained()->onDelete('cascade');
// L17
$table->string('name');
// L18
$table->integer('level')->default(1);
// L19
$table->bigInteger('xp')->default(0);
// L20
$table->tinyInteger('gender')->default(0); // 0: Male, 1: Female
// L21
$table->integer('rank')->default(1); // 1: Genin, 3: Chunin, 5: Jounin, 7: Sp. Jounin, 8: Sannin, 10: Kage
// L22
$table->string('class')->nullable();
// L24
$table->integer('gold')->default(1000);
// L25
$table->integer('tp')->default(0);
// L26
$table->integer('prestige')->default(0);
// L27
$table->integer('ss')->default(0); // Senjutsu Stones?
// L30
$table->integer('element_1')->default(0);
// L31
$table->integer('element_2')->default(0);
// L32
$table->integer('element_3')->default(0);
// L35
$table->string('talent_1')->nullable();
// L36
$table->string('talent_2')->nullable();
// L37
$table->string('talent_3')->nullable();
// L40
$table->string('hair_style')->nullable();
// L41
$table->string('hair_color')->nullable();
// L42
$table->string('skin_color')->nullable();
// L45
$table->string('equipment_weapon')->nullable();
// L46
$table->string('equipment_back')->nullable();
// L47
$table->string('equipment_clothing')->nullable();
// L48
$table->string('equipment_accessory')->nullable();
// L49
$table->text('equipment_skills')->nullable();
// L50
$table->string('equipment_pet')->nullable();
// L53
$table->text('claimed_welcome_rewards')->nullable();
// L56
$table->integer('point_wind')->default(0);
// L57
$table->integer('point_fire')->default(0);
// L58
$table->integer('point_lightning')->default(0);
// L59
$table->integer('point_water')->default(0);
// L60
$table->integer('point_earth')->default(0);
// L61
$table->integer('point_free')->default(0);
// L64
$table->string('chunin_exam_progress')->default('1,0,0,0,0');
// L65
$table->boolean('chunin_claimed')->default(false);
// L66
$table->string('jounin_exam_progress')->default('1,0,0,0,0');
// L67
$table->boolean('jounin_claimed')->default(false);
// L68
$table->string('special_jounin_exam_progress')->default('1,0,0,0,0,0,0,0,0,0,0,0,0');
// L69
$table->boolean('special_jounin_claimed')->default(false);
// L70
$table->string('ninja_tutor_exam_progress')->default('1,0,0,0,0,0');
// L71
$table->boolean('ninja_tutor_claimed')->default(false);
// L74
$table->timestamp('daily_token_claimed_at')->nullable();
// L75
$table->timestamp('daily_xp_claimed_at')->nullable();
// L76
$table->timestamp('daily_scroll_claimed_at')->nullable();
// L77
$table->timestamp('double_xp_expire_at')->nullable();
// L78
$table->integer('xp_bonus_rate')->default(0);
// L81
$table->date('daily_scratch_date')->nullable();
// L82
$table->integer('daily_scratch_consecutive')->default(1);
// L83
$table->integer('daily_scratch_count')->default(0);
// L84
$table->unsignedInteger('scratch_grand_progress')->default(0);
// L85
$table->unsignedInteger('scratch_rare_progress')->default(0);
// L87
$table->date('daily_roulette_date')->nullable();
// L88
$table->integer('daily_roulette_consecutive')->default(1);
// L89
$table->integer('daily_roulette_count')->default(0);
// L92
$table->json('attendance_days')->nullable();
// L93
$table->json('attendance_rewards')->nullable();
// L94
$table->date('attendance_last_reset')->nullable();
// L97
$table->text('talent_skills')->nullable();
// L98
$table->text('senjutsu_skills')->nullable();
// L99
$table->string('senjutsu_type')->nullable();
// L100
$table->text('senjutsu_equipped_skills')->nullable();
// L103
$table->string('hunting_house_tries')->nullable();
// L104
$table->date('hunting_house_date')->nullable();
// L105
$table->string('eudemon_garden_tries')->nullable();
// L106
$table->date('eudemon_garden_date')->nullable();
// L107
$table->boolean('recruitable')->default(true);
// L110
$table->integer('pvp_played')->default(0);
// L111
$table->integer('pvp_won')->default(0);
// L112
$table->integer('pvp_lost')->default(0);
// L113
$table->integer('pvp_points')->default(0);
// L114
$table->integer('pvp_trophy')->default(0);
// L116
$table->timestamps();
```

### 2026_01_01_000003_create_character_relations_table.php

[database/migrations/2026_01_01_000003_create_character_relations_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_01_000003_create_character_relations_table.php#L1)

```php
// L15
Schema::create('character_items', function (Blueprint $table) {
// L16
$table->id();
// L17
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L18
$table->string('item_id');
// L19
$table->integer('quantity')->default(1);
// L20
$table->string('category')->nullable();
// L21
$table->timestamps();
// L25
Schema::create('character_skills', function (Blueprint $table) {
// L26
$table->id();
// L27
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L28
$table->string('skill_id');
// L29
$table->timestamps();
// L33
Schema::create('character_pets', function (Blueprint $table) {
// L34
$table->id();
// L35
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L36
$table->string('pet_id'); // e.g. "1" or "pet_01"
// L37
$table->integer('level')->default(1);
// L38
$table->bigInteger('xp')->default(0);
// L39
$table->string('name')->nullable();
// L40
$table->timestamps();
// L44
Schema::create('character_recruits', function (Blueprint $table) {
// L45
$table->id();
// L46
$table->unsignedBigInteger('character_id'); // Manual foreign key if needed or implicit
// L47
$table->string('recruit_id');
// L48
$table->timestamps();
// L52
Schema::create('character_skill_sets', function (Blueprint $table) {
// L53
$table->id();
// L54
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L55
$table->integer('preset_index');
// L56
$table->text('skills')->nullable();
// L57
$table->timestamps();
```

### 2026_01_01_000004_create_game_data_table.php

[database/migrations/2026_01_01_000004_create_game_data_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_01_000004_create_game_data_table.php#L1)

```php
// L15
Schema::create('missions', function (Blueprint $table) {
// L16
$table->id();
// L17
$table->string('mission_id')->unique();
// L18
$table->integer('req_lvl')->default(1);
// L19
$table->integer('xp')->default(0);
// L20
$table->integer('gold')->default(0);
// L21
$table->timestamps();
// L25
Schema::create('x_p_s', function (Blueprint $table) {
// L26
$table->id();
// L27
$table->integer('level')->unique();
// L28
$table->integer('character_xp'); // Renamed from xp_required
// L29
$table->integer('pet_xp')->default(999999999);
// L30
$table->timestamps();
// L34
Schema::create('npcs', function (Blueprint $table) {
// L35
$table->id();
// L36
$table->string('npc_id')->unique();
// L37
$table->string('name');
// L38
$table->integer('level');
// L39
$table->integer('rank');
// L40
$table->integer('hp');
// L41
$table->integer('cp');
// L42
$table->integer('agility');
// L43
$table->integer('dodge');
// L44
$table->integer('critical');
// L45
$table->integer('accuracy');
// L46
$table->integer('purify');
// L47
$table->text('description')->nullable();
// L48
$table->json('attacks')->nullable();
// L49
$table->integer('price_gold')->default(0);
// L50
$table->integer('price_tokens')->default(0);
// L51
$table->boolean('premium')->default(false);
// L52
$table->timestamps();
// L56
Schema::create('enemies', function (Blueprint $table) {
// L57
$table->id();
// L58
$table->string('enemy_id')->unique();
// L59
$table->string('name');
// L60
$table->integer('level');
// L61
$table->integer('hp');
// L62
$table->integer('cp');
// L63
$table->integer('agility');
// L64
$table->json('attacks')->nullable();
// L65
$table->timestamps();
// L69
Schema::create('talents', function (Blueprint $table) {
// L70
$table->id();
// L71
$table->string('talent_id')->unique();
// L72
$table->string('name');
// L73
$table->text('description')->nullable();
// L74
$table->json('skills')->nullable();
// L75
$table->integer('price_gold')->default(0);
// L76
$table->integer('price_tokens')->default(0);
// L77
$table->boolean('is_emblem')->default(false);
// L78
$table->timestamps();
// L82
Schema::create('senjutsus', function (Blueprint $table) {
// L83
$table->id();
// L84
$table->string('senjutsu_id')->unique();
// L85
$table->string('name');
// L86
$table->text('description')->nullable();
// L87
$table->json('effects')->nullable();
// L88
$table->timestamps();
// L92
Schema::create('game_configs', function (Blueprint $table) {
// L93
$table->id();
// L94
$table->string('key')->unique();
// L95
$table->json('value');
// L96
$table->timestamps();
// L100
Schema::create('game_events', function (Blueprint $table) {
// L101
$table->id();
// L102
$table->string('title');
// L103
$table->string('type')->default('seasonal');
// L104
$table->string('image_url');
// L105
$table->text('description')->nullable();
// L106
$table->boolean('active')->default(true);
// L107
$table->json('data')->nullable();
// L108
$table->timestamps();
```

### 2026_01_16_235238_create_character_talents_table.php

[database/migrations/2026_01_16_235238_create_character_talents_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_16_235238_create_character_talents_table.php#L1)

```php
// L14
if (!Schema::hasTable('character_talents')) {
// L15
Schema::create('character_talents', function (Blueprint $table) {
// L16
$table->id();
// L17
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L18
$table->string('skill_id');
// L19
$table->integer('level')->default(1);
// L20
$table->timestamps();
```

### 2026_01_18_000001_create_character_friends_table.php

[database/migrations/2026_01_18_000001_create_character_friends_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_18_000001_create_character_friends_table.php#L1)

```php
// L14
Schema::create('character_friends', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->foreignId('character_id')->constrained('characters')->onDelete('cascade');
// L17
$table->foreignId('friend_id')->constrained('characters')->onDelete('cascade');
// L18
$table->boolean('is_favorite')->default(false);
// L19
$table->timestamps();
// L21
$table->unique(['character_id', 'friend_id']);
// L24
Schema::create('friend_requests', function (Blueprint $table) {
// L25
$table->id();
// L26
$table->foreignId('character_id')->constrained('characters')->onDelete('cascade');
// L27
$table->foreignId('requester_id')->constrained('characters')->onDelete('cascade');
// L28
$table->timestamps();
// L30
$table->unique(['character_id', 'requester_id']);
```

### 2026_01_19_000001_create_shadow_war_tables.php

[database/migrations/2026_01_19_000001_create_shadow_war_tables.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_19_000001_create_shadow_war_tables.php#L1)

```php
// L14
Schema::create('shadow_war_seasons', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->integer('num')->index();
// L17
$table->dateTime('start_at');
// L18
$table->dateTime('end_at');
// L19
$table->boolean('active')->default(false)->index();
// L20
$table->timestamps();
// L23
Schema::create('shadow_war_squads', function (Blueprint $table) {
// L24
$table->id();
// L25
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L26
$table->foreignId('season_id')->constrained('shadow_war_seasons')->onDelete('cascade');
// L27
$table->unsignedTinyInteger('squad');
// L28
$table->integer('rank')->default(0);
// L29
$table->integer('trophy')->default(0);
// L30
$table->integer('energy')->default(100);
// L31
$table->date('energy_last_reset')->nullable();
// L32
$table->integer('energy_refills_today')->default(0);
// L33
$table->timestamps();
// L35
$table->unique(['character_id', 'season_id']);
```

### 2026_01_19_000003_create_shadow_war_battles_table.php

[database/migrations/2026_01_19_000003_create_shadow_war_battles_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_19_000003_create_shadow_war_battles_table.php#L1)

```php
// L14
Schema::create('shadow_war_battles', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->foreignId('season_id')->constrained('shadow_war_seasons')->onDelete('cascade');
// L17
$table->string('battle_code')->nullable()->index();
// L18
$table->foreignId('character_id')->constrained('characters')->onDelete('cascade');
// L19
$table->unsignedBigInteger('enemy_id')->nullable();
// L20
$table->unsignedTinyInteger('character_squad')->nullable();
// L21
$table->unsignedTinyInteger('enemy_squad')->nullable();
// L22
$table->integer('character_level')->default(0);
// L23
$table->integer('enemy_level')->default(0);
// L24
$table->integer('character_rank')->default(0);
// L25
$table->integer('enemy_rank')->default(0);
// L26
$table->integer('character_trophy_before')->default(0);
// L27
$table->integer('enemy_trophy_before')->default(0);
// L28
$table->integer('character_trophy_after')->default(0);
// L29
$table->integer('enemy_trophy_after')->default(0);
// L30
$table->integer('trophy_delta')->default(0);
// L31
$table->integer('total_damage')->default(0);
// L32
$table->boolean('won')->default(false);
// L33
$table->json('battle_data')->nullable();
// L34
$table->integer('energy_cost')->default(0);
// L35
$table->integer('energy_before')->default(0);
// L36
$table->integer('energy_after')->default(0);
// L37
$table->integer('refills_used_today')->default(0);
// L38
$table->timestamps();
```

### 2026_01_19_070000_create_pvp_battles_table.php

[database/migrations/2026_01_19_070000_create_pvp_battles_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_19_070000_create_pvp_battles_table.php#L1)

```php
// L11
Schema::create('pvp_battles', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->unsignedBigInteger('host_id');
// L14
$table->unsignedBigInteger('enemy_id');
// L15
$table->string('mode')->nullable();
// L16
$table->boolean('host_won')->default(false);
// L17
$table->integer('trophy_delta')->default(0);
// L18
$table->integer('host_trophy_before')->default(0);
// L19
$table->integer('host_trophy_after')->default(0);
// L20
$table->integer('enemy_trophy_before')->default(0);
// L21
$table->integer('enemy_trophy_after')->default(0);
// L22
$table->integer('host_level')->default(1);
// L23
$table->integer('enemy_level')->default(1);
// L24
$table->integer('host_rank')->default(1);
// L25
$table->integer('enemy_rank')->default(1);
// L26
$table->json('host_snapshot')->nullable();
// L27
$table->json('enemy_snapshot')->nullable();
// L28
$table->json('battle_data')->nullable();
// L29
$table->timestamps();
// L31
$table->index(['host_id']);
// L32
$table->index(['enemy_id']);
// L33
$table->index(['created_at']);
```

### 2026_01_19_070001_create_pvp_bug_reports_table.php

[database/migrations/2026_01_19_070001_create_pvp_bug_reports_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_01_19_070001_create_pvp_bug_reports_table.php#L1)

```php
// L11
Schema::create('pvp_bug_reports', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->unsignedBigInteger('character_id');
// L14
$table->string('title');
// L15
$table->text('description');
// L16
$table->timestamps();
// L18
$table->index(['character_id']);
// L19
$table->index(['created_at']);
```

### 2026_02_22_000001_add_catalog_fields_to_game_events_table.php

[database/migrations/2026_02_22_000001_add_catalog_fields_to_game_events_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_22_000001_add_catalog_fields_to_game_events_table.php#L1)

```php
// L17
Schema::table('game_events', function (Blueprint $table) {
// L20
$table->string('panel')->nullable()->after('type');
// L25
$table->string('date')->nullable()->after('panel');
// L29
$table->string('icon')->nullable()->after('date');
// L33
$table->boolean('inside')->default(false)->after('icon');
// L36
$table->unsignedSmallInteger('sort_order')->default(0)->after('inside');
```

### 2026_02_22_000001_create_clan_system_tables.php

[database/migrations/2026_02_22_000001_create_clan_system_tables.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_22_000001_create_clan_system_tables.php#L1)

```php
// L11
Schema::create('clan_seasons', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->unsignedInteger('number')->default(1);
// L14
$table->boolean('active')->default(false);
// L15
$table->timestamp('started_at')->nullable();
// L16
$table->timestamp('ended_at')->nullable();
// L17
$table->timestamps();
// L20
Schema::create('clans', function (Blueprint $table) {
// L21
$table->id();
// L22
$table->unsignedBigInteger('season_id')->nullable();
// L23
$table->string('name', 50)->unique();
// L24
$table->unsignedBigInteger('master_id'); // character_id of the master
// L25
$table->unsignedInteger('prestige')->default(0);
// L26
$table->unsignedInteger('max_members')->default(10);
// L27
$table->unsignedBigInteger('gold')->default(0);
// L28
$table->unsignedBigInteger('tokens')->default(0);
// L29
$table->text('announcement_draft')->nullable();
// L30
$table->text('announcement_published')->nullable();
// L31
$table->json('buildings')->nullable();
// L32
$table->timestamps();
// L34
$table->foreign('season_id')->references('id')->on('clan_seasons')->nullOnDelete();
// L37
Schema::create('clan_members', function (Blueprint $table) {
// L38
$table->id();
// L39
$table->unsignedBigInteger('clan_id');
// L40
$table->unsignedBigInteger('character_id');
// L41
$table->enum('role', ['master', 'elder', 'member'])->default('member');
// L42
$table->unsignedInteger('stamina')->default(5);
// L43
$table->unsignedInteger('max_stamina')->default(5);
// L44
$table->unsignedBigInteger('donated_golds')->default(0);
// L45
$table->unsignedBigInteger('donated_tokens')->default(0);
// L46
$table->timestamps();
// L48
$table->unique(['clan_id', 'character_id']);
// L49
$table->unique('character_id'); // a character can only be in one clan
// L50
$table->foreign('clan_id')->references('id')->on('clans')->onDelete('cascade');
// L53
Schema::create('clan_requests', function (Blueprint $table) {
// L54
$table->id();
// L55
$table->unsignedBigInteger('clan_id');
// L56
$table->unsignedBigInteger('character_id');
// L57
$table->enum('status', ['pending', 'accepted', 'rejected'])->default('pending');
// L58
$table->timestamps();
// L60
$table->foreign('clan_id')->references('id')->on('clans')->onDelete('cascade');
// L63
Schema::create('clan_auth_tokens', function (Blueprint $table) {
// L64
$table->id();
// L65
$table->unsignedBigInteger('user_id');
// L66
$table->unsignedBigInteger('character_id');
// L67
$table->string('token', 64)->unique();
// L68
$table->timestamp('expires_at')->nullable();
// L69
$table->timestamps();
// L72
Schema::create('clan_battles', function (Blueprint $table) {
// L73
$table->id();
// L74
$table->unsignedBigInteger('attacker_clan_id');
// L75
$table->unsignedBigInteger('defender_clan_id');
// L76
$table->unsignedBigInteger('season_id')->nullable();
// L77
$table->boolean('attacker_won')->default(false);
// L78
$table->json('battle_data')->nullable();
// L79
$table->timestamps();
```

### 2026_02_22_000002_create_character_mails_table.php

[database/migrations/2026_02_22_000002_create_character_mails_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_22_000002_create_character_mails_table.php#L1)

```php
// L11
Schema::create('character_mails', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L14
$table->string('title');
// L15
$table->string('sender')->default('System');
// L16
$table->text('body')->nullable();
// L17
$table->string('type')->default('system');
// L18
$table->json('rewards')->nullable();
// L19
$table->boolean('claimed')->default(false);
// L20
$table->boolean('viewed')->default(false);
// L21
$table->timestamps();
```

### 2026_02_23_000001_create_clan_tables.php

[database/migrations/2026_02_23_000001_create_clan_tables.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_23_000001_create_clan_tables.php#L1)

```php
// L11
if (!Schema::hasTable('clan_seasons')) {
// L12
Schema::create('clan_seasons', function (Blueprint $table) {
// L13
$table->id();
// L14
$table->unsignedInteger('number')->default(1);
// L15
$table->boolean('active')->default(false)->index();
// L16
$table->timestamp('started_at')->nullable();
// L17
$table->timestamp('ended_at')->nullable();
// L18
$table->timestamps();
// L22
if (!Schema::hasTable('clans')) {
// L23
Schema::create('clans', function (Blueprint $table) {
// L24
$table->id();
// L25
$table->foreignId('season_id')->constrained('clan_seasons')->onDelete('cascade');
// L26
$table->string('name', 50)->unique();
// L27
$table->unsignedBigInteger('master_id')->index();
// L28
$table->integer('prestige')->default(0);
// L29
$table->integer('max_members')->default(20);
// L30
$table->integer('gold')->default(0);
// L31
$table->integer('tokens')->default(0);
// L32
$table->json('buildings')->nullable();
// L33
$table->text('announcement_published')->nullable();
// L34
$table->text('announcement_draft')->nullable();
// L35
$table->timestamps();
// L39
if (!Schema::hasTable('clan_members')) {
// L40
Schema::create('clan_members', function (Blueprint $table) {
// L41
$table->id();
// L42
$table->foreignId('clan_id')->constrained()->onDelete('cascade');
// L43
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L44
$table->string('role', 20)->default('member');
// L45
$table->integer('stamina')->default(5);
// L46
$table->integer('max_stamina')->default(5);
// L47
$table->integer('donated_golds')->default(0);
// L48
$table->integer('donated_tokens')->default(0);
// L49
$table->timestamps();
// L51
$table->unique('character_id');
// L55
if (!Schema::hasTable('clan_requests')) {
// L56
Schema::create('clan_requests', function (Blueprint $table) {
// L57
$table->id();
// L58
$table->foreignId('clan_id')->constrained()->onDelete('cascade');
// L59
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L60
$table->string('status', 20)->default('pending');
// L61
$table->timestamps();
// L63
$table->unique(['clan_id', 'character_id']);
// L67
if (!Schema::hasTable('clan_battles')) {
// L68
Schema::create('clan_battles', function (Blueprint $table) {
// L69
$table->id();
// L70
$table->foreignId('season_id')->constrained('clan_seasons')->onDelete('cascade');
// L71
$table->foreignId('attacker_clan_id')->constrained('clans')->onDelete('cascade');
// L72
$table->foreignId('defender_clan_id')->constrained('clans')->onDelete('cascade');
// L73
$table->boolean('attacker_won')->default(false);
// L74
$table->json('battle_data')->nullable();
// L75
$table->timestamps();
// L79
if (!Schema::hasTable('clan_auth_tokens')) {
// L80
Schema::create('clan_auth_tokens', function (Blueprint $table) {
// L81
$table->id();
// L82
$table->foreignId('user_id')->constrained()->onDelete('cascade');
// L83
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L84
$table->string('token', 64)->unique()->index();
// L85
$table->timestamp('expires_at');
// L86
$table->timestamps();
```

### 2026_02_23_000003_create_admin_commands_table.php

[database/migrations/2026_02_23_000003_create_admin_commands_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_23_000003_create_admin_commands_table.php#L1)

```php
// L11
Schema::create('admin_commands', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->string('name');
// L14
$table->text('description')->nullable();
// L15
$table->string('command_type');
// L16
$table->json('params')->nullable();
// L17
$table->boolean('active')->default(true);
// L18
$table->timestamps();
```

### 2026_02_23_100001_add_prestige_boost_to_clan_members.php

[database/migrations/2026_02_23_100001_add_prestige_boost_to_clan_members.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_23_100001_add_prestige_boost_to_clan_members.php#L1)

```php
// L11
Schema::table('clan_members', function (Blueprint $table) {
// L12
$table->timestamp('prestige_boost_expires_at')->nullable()->after('donated_tokens');
```

### 2026_02_23_200001_create_crew_tables.php

[database/migrations/2026_02_23_200001_create_crew_tables.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_23_200001_create_crew_tables.php#L1)

```php
// L11
if (!Schema::hasTable('crew_seasons')) {
// L12
Schema::create('crew_seasons', function (Blueprint $table) {
// L13
$table->id();
// L14
$table->unsignedInteger('number')->default(1);
// L15
$table->boolean('active')->default(false)->index();
// L16
$table->unsignedTinyInteger('phase')->default(1);  // 1=attack, 2=defend
// L17
$table->timestamp('started_at')->nullable();
// L18
$table->timestamp('ended_at')->nullable();
// L19
$table->timestamps();
// L23
if (!Schema::hasTable('crews')) {
// L24
Schema::create('crews', function (Blueprint $table) {
// L25
$table->id();
// L26
$table->foreignId('season_id')->constrained('crew_seasons')->onDelete('cascade');
// L27
$table->string('name', 50)->unique();
// L28
$table->unsignedBigInteger('master_id')->index();
// L29
$table->integer('prestige')->default(0);
// L30
$table->integer('max_members')->default(20);
// L31
$table->integer('gold')->default(0);
// L32
$table->integer('tokens')->default(0);
// L33
$table->json('buildings')->nullable();
// L34
$table->text('announcement_published')->nullable();
// L35
$table->text('announcement_draft')->nullable();
// L36
$table->timestamps();
// L40
if (!Schema::hasTable('crew_members')) {
// L41
Schema::create('crew_members', function (Blueprint $table) {
// L42
$table->id();
// L43
$table->foreignId('crew_id')->constrained()->onDelete('cascade');
// L44
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L45
$table->string('role', 20)->default('member'); // master, elder, member
// L46
$table->unsignedTinyInteger('battle_role')->default(1); // 1=attacker, 2=defender
// L47
$table->string('role_limit_at', 50)->default('');
// L48
$table->integer('stamina')->default(50);
// L49
$table->integer('max_stamina')->default(50);
// L50
$table->integer('donated_golds')->default(0);
// L51
$table->integer('donated_tokens')->default(0);
// L52
$table->integer('minigame_energy')->default(3);
// L53
$table->timestamp('prestige_boost_expires_at')->nullable();
// L54
$table->timestamps();
// L56
$table->unique('character_id');
// L60
if (!Schema::hasTable('crew_requests')) {
// L61
Schema::create('crew_requests', function (Blueprint $table) {
// L62
$table->id();
// L63
$table->foreignId('crew_id')->constrained()->onDelete('cascade');
// L64
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L65
$table->string('status', 20)->default('pending');
// L66
$table->timestamps();
// L68
$table->unique(['crew_id', 'character_id']);
// L72
if (!Schema::hasTable('crew_battles')) {
// L73
Schema::create('crew_battles', function (Blueprint $table) {
// L74
$table->id();
// L75
$table->foreignId('season_id')->constrained('crew_seasons')->onDelete('cascade');
// L76
$table->unsignedBigInteger('castle_id')->nullable();
// L77
$table->foreignId('attacker_crew_id')->constrained('crews')->onDelete('cascade');
// L78
$table->unsignedBigInteger('defender_crew_id')->nullable();
// L79
$table->boolean('attacker_won')->default(false);
// L80
$table->json('battle_data')->nullable();
// L81
$table->timestamps();
// L85
if (!Schema::hasTable('crew_castles')) {
// L86
Schema::create('crew_castles', function (Blueprint $table) {
// L87
$table->id();
// L88
$table->foreignId('season_id')->constrained('crew_seasons')->onDelete('cascade');
// L89
$table->unsignedTinyInteger('castle_index'); // 0-6
// L90
$table->string('name', 100)->default('');
// L91
$table->unsignedBigInteger('owner_crew_id')->nullable()->index();
// L92
$table->unsignedInteger('wall_hp')->default(100);
// L93
$table->unsignedInteger('defender_hp')->default(100);
// L94
$table->timestamp('last_recovery_at')->nullable();
// L95
$table->timestamps();
// L97
$table->unique(['season_id', 'castle_index']);
// L101
if (!Schema::hasTable('crew_auth_tokens')) {
// L102
Schema::create('crew_auth_tokens', function (Blueprint $table) {
// L103
$table->id();
// L104
$table->foreignId('user_id')->constrained()->onDelete('cascade');
// L105
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L106
$table->string('token', 64)->unique()->index();
// L107
$table->timestamp('expires_at');
// L108
$table->timestamps();
```

### 2026_02_24_000001_add_phase_ends_at_to_crew_seasons.php

[database/migrations/2026_02_24_000001_add_phase_ends_at_to_crew_seasons.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_24_000001_add_phase_ends_at_to_crew_seasons.php#L1)

```php
// L11
Schema::table('crew_seasons', function (Blueprint $table) {
// L13
$table->timestamp('phase_ends_at')->nullable()->after('ended_at');
```

### 2026_02_24_000001_create_pet_villa_slots_table.php

[database/migrations/2026_02_24_000001_create_pet_villa_slots_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_24_000001_create_pet_villa_slots_table.php#L1)

```php
// L14
Schema::create('pet_villa_slots', function (Blueprint $table) {
// L15
$table->id();
// L16
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L17
$table->tinyInteger('slot_index'); // 0–3
// L19
$table->tinyInteger('status')->default(1);
// L20
$table->unsignedBigInteger('pet_instance_id')->nullable(); // FK to character_pets.id
// L21
$table->unsignedInteger('training_ends_at')->nullable(); // Unix timestamp
// L22
$table->unsignedInteger('gold_spent')->default(0);
// L23
$table->timestamps();
// L25
$table->unique(['character_id', 'slot_index']);
// L26
$table->foreign('pet_instance_id')->references('id')->on('character_pets')->nullOnDelete();
```

### 2026_02_24_000002_add_maturity_points_to_character_pets_table.php

[database/migrations/2026_02_24_000002_add_maturity_points_to_character_pets_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_24_000002_add_maturity_points_to_character_pets_table.php#L1)

```php
// L11
Schema::table('character_pets', function (Blueprint $table) {
// L12
$table->unsignedInteger('maturity_points')->default(0)->after('xp');
```

### 2026_02_24_100001_create_mysterious_markets_table.php

[database/migrations/2026_02_24_100001_create_mysterious_markets_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_24_100001_create_mysterious_markets_table.php#L1)

```php
// L11
Schema::create('mysterious_markets', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->boolean('active')->default(false)->index();
// L14
$table->timestamp('ends_at');
// L15
$table->string('discount', 10)->default('0'); // Emblem discount % shown as string
// L16
$table->unsignedInteger('refresh_cost')->default(50);
// L17
$table->unsignedInteger('refresh_max')->default(3);
// L19
$table->json('items')->nullable();
// L21
$table->json('all_packages')->nullable();
// L22
$table->timestamps();
// L25
Schema::create('character_mysterious_markets', function (Blueprint $table) {
// L26
$table->id();
// L27
$table->foreignId('character_id')->constrained()->onDelete('cascade');
// L28
$table->foreignId('market_id')->constrained('mysterious_markets')->onDelete('cascade');
// L30
$table->unsignedInteger('refresh_count')->default(0);
// L32
$table->json('custom_items')->nullable();
// L33
$table->timestamps();
// L34
$table->unique(['character_id', 'market_id']);
```

### 2026_02_24_300001_create_character_event_data_table.php

[database/migrations/2026_02_24_300001_create_character_event_data_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_24_300001_create_character_event_data_table.php#L1)

```php
// L18
Schema::create('character_event_data', function (Blueprint $table) {
// L19
$table->id();
// L20
$table->unsignedBigInteger('character_id');
// L21
$table->string('event_key', 100);
// L24
$table->unsignedSmallInteger('energy')->default(0);
// L27
$table->unsignedInteger('battles')->default(0);
// L30
$table->json('milestones_claimed')->nullable();
// L33
$table->boolean('bought')->default(false);
// L36
$table->json('extra')->nullable();
// L38
$table->timestamps();
// L40
$table->unique(['character_id', 'event_key'], 'ced_char_event_unique');
// L41
$table->foreign('character_id')->references('id')->on('characters')->cascadeOnDelete();
```

### 2026_02_27_000001_seed_dragon_gacha_event_data.php

[database/migrations/2026_02_27_000001_seed_dragon_gacha_event_data.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_27_000001_seed_dragon_gacha_event_data.php#L1)

```php
// L15
$existing = DB::table('game_events')->where('panel', 'DragonGacha')->first();
// L77
DB::table('game_events')
// L82
DB::table('game_events')->insert([
```

### 2026_02_27_000003_seed_daily_gacha_event_data.php

[database/migrations/2026_02_27_000003_seed_daily_gacha_event_data.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_27_000003_seed_daily_gacha_event_data.php#L1)

```php
// L80
$existing = DB::table('game_events')->where('panel', 'DailyGacha')->first();
// L86
DB::table('game_events')
// L91
DB::table('game_events')->insert([
```

### 2026_02_27_000004_fix_daily_gacha_pool_invalid_entries.php

[database/migrations/2026_02_27_000004_fix_daily_gacha_pool_invalid_entries.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_27_000004_fix_daily_gacha_pool_invalid_entries.php#L1)

```php
// L20
$event = DB::table('game_events')->where('panel', 'DailyGacha')->first();
// L57
DB::table('game_events')
```

### 2026_02_27_000005_create_giveaways_table.php

[database/migrations/2026_02_27_000005_create_giveaways_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_27_000005_create_giveaways_table.php#L1)

```php
// L11
Schema::create('giveaways', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->string('title');
// L14
$table->text('description')->nullable();
// L15
$table->json('prizes')->nullable();
// L18
$table->json('requirements')->nullable();
// L19
$table->timestamp('ends_at');
// L21
$table->json('winners')->nullable();
// L22
$table->timestamps();
```

### 2026_02_27_000006_create_giveaway_participants_table.php

[database/migrations/2026_02_27_000006_create_giveaway_participants_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_27_000006_create_giveaway_participants_table.php#L1)

```php
// L11
Schema::create('giveaway_participants', function (Blueprint $table) {
// L12
$table->id();
// L13
$table->foreignId('giveaway_id')->constrained()->cascadeOnDelete();
// L14
$table->unsignedBigInteger('character_id');
// L15
$table->timestamps();
// L17
$table->unique(['giveaway_id', 'character_id']);
// L18
$table->index('character_id');
```

### 2026_02_28_000001_fix_anniversary_2025_panel.php

[database/migrations/2026_02_28_000001_fix_anniversary_2025_panel.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_28_000001_fix_anniversary_2025_panel.php#L1)

```php
// L27
$anniversary = DB::table('game_events')
// L71
DB::table('game_events')
```

### 2026_02_28_000002_fix_character_items_slot_category.php

[database/migrations/2026_02_28_000002_fix_character_items_slot_category.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_02_28_000002_fix_character_items_slot_category.php#L1)

```php
// L36
DB::table('character_items')
```

### 2026_03_04_000001_add_swf_to_skills_table.php

[database/migrations/2026_03_04_000001_add_swf_to_skills_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_03_04_000001_add_swf_to_skills_table.php#L1)

```php
// L11
Schema::table('skills', function (Blueprint $table) {
// L16
$table->string('swf')->nullable()->after('icon');
```

### 2026_03_05_000001_add_schedule_to_game_events_table.php

[database/migrations/2026_03_05_000001_add_schedule_to_game_events_table.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/migrations/2026_03_05_000001_add_schedule_to_game_events_table.php#L1)

```php
// L19
Schema::table('game_events', function (Blueprint $table) {
// L20
$table->timestamp('starts_at')->nullable()->after('sort_order');
// L21
$table->timestamp('ends_at')->nullable()->after('starts_at');
```

## Chat-owned schema

[chat-server/src/messageStore.js:6](../../reference/upstream/Database/Laragon/ninjasage/chat-server/src/messageStore.js#L6)

```sql
CREATE TABLE IF NOT EXISTS chat_messages (
  id                BIGINT UNSIGNED   AUTO_INCREMENT PRIMARY KEY,
  channel           VARCHAR(32)       NOT NULL,
  character_id      INT UNSIGNED      NOT NULL,
  character_name    VARCHAR(64)       NOT NULL,
  character_level   SMALLINT UNSIGNED NOT NULL DEFAULT 1,
  character_rank    TINYINT UNSIGNED  NOT NULL DEFAULT 1,
  character_premium TINYINT UNSIGNED  NOT NULL DEFAULT 0,
  message           TEXT              NOT NULL,
  created_at        TIMESTAMP         DEFAULT CURRENT_TIMESTAMP,
  INDEX idx_channel_id (channel, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
```

## Models and declared relationships

| Model | Explicit table / relationship declarations | Source |
|---|---|---|
| AdminCommand | Framework table inference; no relationship matched | [app/Models/AdminCommand.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/AdminCommand.php#L1) |
| Character | L108: `return $this->belongsTo(User::class);`<br>L113: `return $this->hasMany(CharacterSkillSet::class);`<br>L118: `return $this->hasMany(CharacterItem::class);`<br>L123: `return $this->hasMany(CharacterSkill::class);`<br>L128: `return $this->hasMany(CharacterPet::class);` | [app/Models/Character.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Character.php#L1) |
| CharacterEventData | L9: `protected $table = 'character_event_data';`<br>L31: `return $this->belongsTo(Character::class);` | [app/Models/CharacterEventData.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterEventData.php#L1) |
| CharacterFriend | Framework table inference; no relationship matched | [app/Models/CharacterFriend.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterFriend.php#L1) |
| CharacterItem | L21: `return $this->belongsTo(Character::class);`<br>L26: `return $this->belongsTo(Item::class, 'item_id', 'item_id');` | [app/Models/CharacterItem.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterItem.php#L1) |
| CharacterMail | L28: `return $this->belongsTo(Character::class);` | [app/Models/CharacterMail.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterMail.php#L1) |
| CharacterMysteriousMarket | L26: `return $this->belongsTo(Character::class);`<br>L31: `return $this->belongsTo(MysteriousMarket::class, 'market_id');` | [app/Models/CharacterMysteriousMarket.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterMysteriousMarket.php#L1) |
| CharacterPet | L23: `return $this->belongsTo(Character::class);`<br>L28: `return $this->belongsTo(Pet::class, 'pet_id', 'pet_id');` | [app/Models/CharacterPet.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterPet.php#L1) |
| CharacterRecruit | Framework table inference; no relationship matched | [app/Models/CharacterRecruit.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterRecruit.php#L1) |
| CharacterSkill | L19: `return $this->belongsTo(Character::class);`<br>L24: `return $this->belongsTo(Skill::class, 'skill_id', 'skill_id');` | [app/Models/CharacterSkill.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterSkill.php#L1) |
| CharacterSkillSet | L17: `return $this->belongsTo(Character::class);` | [app/Models/CharacterSkillSet.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterSkillSet.php#L1) |
| CharacterTalent | L12: `protected $table = 'character_talents';`<br>L22: `return $this->belongsTo(Character::class);` | [app/Models/CharacterTalent.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CharacterTalent.php#L1) |
| ChatMessage | L9: `protected $table = 'chat_messages';` | [app/Models/ChatMessage.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ChatMessage.php#L1) |
| Clan | L21: `return $this->hasMany(ClanMember::class);` | [app/Models/Clan.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Clan.php#L1) |
| ClanAuthToken | Framework table inference; no relationship matched | [app/Models/ClanAuthToken.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ClanAuthToken.php#L1) |
| ClanBattle | L21: `return $this->belongsTo(Clan::class, 'attacker_clan_id');`<br>L26: `return $this->belongsTo(Clan::class, 'defender_clan_id');` | [app/Models/ClanBattle.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ClanBattle.php#L1) |
| ClanMember | L21: `return $this->belongsTo(Character::class);`<br>L26: `return $this->belongsTo(Clan::class);` | [app/Models/ClanMember.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ClanMember.php#L1) |
| ClanRequest | L14: `return $this->belongsTo(Character::class);`<br>L19: `return $this->belongsTo(Clan::class);` | [app/Models/ClanRequest.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ClanRequest.php#L1) |
| ClanSeason | Framework table inference; no relationship matched | [app/Models/ClanSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ClanSeason.php#L1) |
| Crew | L21: `return $this->hasMany(CrewMember::class);` | [app/Models/Crew.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Crew.php#L1) |
| CrewAuthToken | Framework table inference; no relationship matched | [app/Models/CrewAuthToken.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewAuthToken.php#L1) |
| CrewBattle | Framework table inference; no relationship matched | [app/Models/CrewBattle.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewBattle.php#L1) |
| CrewCastle | L21: `return $this->belongsTo(CrewSeason::class, 'season_id');`<br>L26: `return $this->belongsTo(Crew::class, 'owner_crew_id');` | [app/Models/CrewCastle.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewCastle.php#L1) |
| CrewMember | L22: `return $this->belongsTo(Character::class);`<br>L27: `return $this->belongsTo(Crew::class);` | [app/Models/CrewMember.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewMember.php#L1) |
| CrewRequest | L14: `return $this->belongsTo(Character::class);`<br>L19: `return $this->belongsTo(Crew::class);` | [app/Models/CrewRequest.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewRequest.php#L1) |
| CrewSeason | L20: `return $this->hasMany(CrewCastle::class, 'season_id');` | [app/Models/CrewSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/CrewSeason.php#L1) |
| Enemy | Framework table inference; no relationship matched | [app/Models/Enemy.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Enemy.php#L1) |
| FriendRequest | Framework table inference; no relationship matched | [app/Models/FriendRequest.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/FriendRequest.php#L1) |
| GameConfig | Framework table inference; no relationship matched | [app/Models/GameConfig.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/GameConfig.php#L1) |
| GameEvent | Framework table inference; no relationship matched | [app/Models/GameEvent.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/GameEvent.php#L1) |
| Giveaway | L28: `return $this->hasMany(GiveawayParticipant::class);` | [app/Models/Giveaway.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Giveaway.php#L1) |
| GiveawayParticipant | L17: `return $this->belongsTo(Giveaway::class);`<br>L22: `return $this->belongsTo(Character::class);` | [app/Models/GiveawayParticipant.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/GiveawayParticipant.php#L1) |
| Item | Framework table inference; no relationship matched | [app/Models/Item.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Item.php#L1) |
| ListMysteriousMarkets | Framework table inference; no relationship matched | [app/Models/ListMysteriousMarkets.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ListMysteriousMarkets.php#L1) |
| Mission | Framework table inference; no relationship matched | [app/Models/Mission.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Mission.php#L1) |
| MysteriousMarket | L33: `return $this->hasMany(CharacterMysteriousMarket::class, 'market_id');` | [app/Models/MysteriousMarket.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/MysteriousMarket.php#L1) |
| Npc | Framework table inference; no relationship matched | [app/Models/Npc.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Npc.php#L1) |
| Pet | Framework table inference; no relationship matched | [app/Models/Pet.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Pet.php#L1) |
| PetVillaSlot | L23: `return $this->belongsTo(Character::class);`<br>L28: `return $this->belongsTo(CharacterPet::class, 'pet_instance_id');` | [app/Models/PetVillaSlot.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/PetVillaSlot.php#L1) |
| PvpBattle | Framework table inference; no relationship matched | [app/Models/PvpBattle.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/PvpBattle.php#L1) |
| PvpBugReport | Framework table inference; no relationship matched | [app/Models/PvpBugReport.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/PvpBugReport.php#L1) |
| Senjutsu | Framework table inference; no relationship matched | [app/Models/Senjutsu.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Senjutsu.php#L1) |
| ShadowWarBattle | Framework table inference; no relationship matched | [app/Models/ShadowWarBattle.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ShadowWarBattle.php#L1) |
| ShadowWarSeason | Framework table inference; no relationship matched | [app/Models/ShadowWarSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ShadowWarSeason.php#L1) |
| ShadowWarSquad | Framework table inference; no relationship matched | [app/Models/ShadowWarSquad.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/ShadowWarSquad.php#L1) |
| Skill | Framework table inference; no relationship matched | [app/Models/Skill.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Skill.php#L1) |
| Talent | L12: `protected $table = 'talents';` | [app/Models/Talent.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/Talent.php#L1) |
| User | Framework table inference; no relationship matched | [app/Models/User.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/User.php#L1) |
| XP | L9: `protected $table = 'x_p_s';` | [app/Models/XP.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Models/XP.php#L1) |

## Seeders

DatabaseSeeder calls AdminUser, EudemonGarden, GameData, XP, Talent, GameEvent, SpecialDeals in that order. AdminUserSeeder contains development account provisioning; do not reuse its credential defaults.

- **AdminUserSeeder** — models: none imported; JSON catalogs: none via loadJson. [database/seeders/AdminUserSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/AdminUserSeeder.php#L1)
- **DatabaseSeeder** — models: User; JSON catalogs: none via loadJson. [database/seeders/DatabaseSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/DatabaseSeeder.php#L1)
- **EudemonGardenSeeder** — models: GameConfig; JSON catalogs: none via loadJson. [database/seeders/EudemonGardenSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/EudemonGardenSeeder.php#L1)
- **GameDataSeeder** — models: Item, Skill, Pet, Mission, Npc, Enemy, Talent, Senjutsu, GameConfig, XP; JSON catalogs: library, skills, pet, mission, npc, enemy, talents, senjutsu, gamedata. [database/seeders/GameDataSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/GameDataSeeder.php#L1)
- **GameEventSeeder** — models: GameEvent; JSON catalogs: none via loadJson. [database/seeders/GameEventSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/GameEventSeeder.php#L1)
- **SpecialDealsSeeder** — models: GameConfig; JSON catalogs: none via loadJson. [database/seeders/SpecialDealsSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/SpecialDealsSeeder.php#L1)
- **TalentSeeder** — models: GameConfig, Talent; JSON catalogs: none via loadJson. [database/seeders/TalentSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/TalentSeeder.php#L1)
- **XPSeeder** — models: Item, Skill, Pet, Mission, XP; JSON catalogs: none via loadJson. [database/seeders/XPSeeder.php:1](../../reference/upstream/Database/Laragon/ninjasage/database/seeders/XPSeeder.php#L1)
