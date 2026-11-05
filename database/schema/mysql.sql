-- Source: ecf9012d885d1c89fd5f42791eed4f37f90f2edd; MySQL 8.4.7 SHOW CREATE TABLE.
-- Baseline migration workaround: session strict mode relaxed for event data inserts only.
CREATE TABLE IF NOT EXISTS `admin_commands` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `command_type` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `params` json DEFAULT NULL,
  `active` tinyint(1) NOT NULL DEFAULT '1',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `cache` (
  `key` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `value` mediumtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `expiration` int NOT NULL,
  PRIMARY KEY (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `cache_locks` (
  `key` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `owner` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `expiration` int NOT NULL,
  PRIMARY KEY (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_event_data` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `event_key` varchar(100) COLLATE utf8mb4_unicode_ci NOT NULL,
  `energy` smallint unsigned NOT NULL DEFAULT '0',
  `battles` int unsigned NOT NULL DEFAULT '0',
  `milestones_claimed` json DEFAULT NULL,
  `bought` tinyint(1) NOT NULL DEFAULT '0',
  `extra` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `ced_char_event_unique` (`character_id`,`event_key`),
  CONSTRAINT `character_event_data_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_friends` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `friend_id` bigint unsigned NOT NULL,
  `is_favorite` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `character_friends_character_id_friend_id_unique` (`character_id`,`friend_id`),
  KEY `character_friends_friend_id_foreign` (`friend_id`),
  CONSTRAINT `character_friends_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `character_friends_friend_id_foreign` FOREIGN KEY (`friend_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_items` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `item_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `quantity` int NOT NULL DEFAULT '1',
  `category` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_items_character_id_foreign` (`character_id`),
  CONSTRAINT `character_items_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_mails` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `title` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `sender` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'System',
  `body` text COLLATE utf8mb4_unicode_ci,
  `type` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'system',
  `rewards` json DEFAULT NULL,
  `claimed` tinyint(1) NOT NULL DEFAULT '0',
  `viewed` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_mails_character_id_foreign` (`character_id`),
  CONSTRAINT `character_mails_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_mysterious_markets` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `market_id` bigint unsigned NOT NULL,
  `refresh_count` int unsigned NOT NULL DEFAULT '0',
  `custom_items` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `character_mysterious_markets_character_id_market_id_unique` (`character_id`,`market_id`),
  KEY `character_mysterious_markets_market_id_foreign` (`market_id`),
  CONSTRAINT `character_mysterious_markets_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `character_mysterious_markets_market_id_foreign` FOREIGN KEY (`market_id`) REFERENCES `mysterious_markets` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_pets` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `pet_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL DEFAULT '1',
  `xp` bigint NOT NULL DEFAULT '0',
  `maturity_points` int unsigned NOT NULL DEFAULT '0',
  `name` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_pets_character_id_foreign` (`character_id`),
  CONSTRAINT `character_pets_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_recruits` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `recruit_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_skill_sets` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `preset_index` int NOT NULL,
  `skills` text COLLATE utf8mb4_unicode_ci,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_skill_sets_character_id_foreign` (`character_id`),
  CONSTRAINT `character_skill_sets_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_skills` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `skill_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_skills_character_id_foreign` (`character_id`),
  CONSTRAINT `character_skills_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `character_talents` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `skill_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL DEFAULT '1',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `character_talents_character_id_foreign` (`character_id`),
  CONSTRAINT `character_talents_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `characters` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `user_id` bigint unsigned NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL DEFAULT '1',
  `xp` bigint NOT NULL DEFAULT '0',
  `gender` tinyint NOT NULL DEFAULT '0',
  `rank` int NOT NULL DEFAULT '1',
  `class` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gold` int NOT NULL DEFAULT '1000',
  `tp` int NOT NULL DEFAULT '0',
  `prestige` int NOT NULL DEFAULT '0',
  `ss` int NOT NULL DEFAULT '0',
  `element_1` int NOT NULL DEFAULT '0',
  `element_2` int NOT NULL DEFAULT '0',
  `element_3` int NOT NULL DEFAULT '0',
  `talent_1` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `talent_2` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `talent_3` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `hair_style` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `hair_color` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `skin_color` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `equipment_weapon` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `equipment_back` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `equipment_clothing` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `equipment_accessory` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `equipment_skills` text COLLATE utf8mb4_unicode_ci,
  `equipment_pet` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `claimed_welcome_rewards` text COLLATE utf8mb4_unicode_ci,
  `point_wind` int NOT NULL DEFAULT '0',
  `point_fire` int NOT NULL DEFAULT '0',
  `point_lightning` int NOT NULL DEFAULT '0',
  `point_water` int NOT NULL DEFAULT '0',
  `point_earth` int NOT NULL DEFAULT '0',
  `point_free` int NOT NULL DEFAULT '0',
  `chunin_exam_progress` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '1,0,0,0,0',
  `chunin_claimed` tinyint(1) NOT NULL DEFAULT '0',
  `jounin_exam_progress` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '1,0,0,0,0',
  `jounin_claimed` tinyint(1) NOT NULL DEFAULT '0',
  `special_jounin_exam_progress` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '1,0,0,0,0,0,0,0,0,0,0,0,0',
  `special_jounin_claimed` tinyint(1) NOT NULL DEFAULT '0',
  `ninja_tutor_exam_progress` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '1,0,0,0,0,0',
  `ninja_tutor_claimed` tinyint(1) NOT NULL DEFAULT '0',
  `daily_token_claimed_at` timestamp NULL DEFAULT NULL,
  `daily_xp_claimed_at` timestamp NULL DEFAULT NULL,
  `daily_scroll_claimed_at` timestamp NULL DEFAULT NULL,
  `double_xp_expire_at` timestamp NULL DEFAULT NULL,
  `xp_bonus_rate` int NOT NULL DEFAULT '0',
  `daily_scratch_date` date DEFAULT NULL,
  `daily_scratch_consecutive` int NOT NULL DEFAULT '1',
  `daily_scratch_count` int NOT NULL DEFAULT '0',
  `scratch_grand_progress` int unsigned NOT NULL DEFAULT '0',
  `scratch_rare_progress` int unsigned NOT NULL DEFAULT '0',
  `daily_roulette_date` date DEFAULT NULL,
  `daily_roulette_consecutive` int NOT NULL DEFAULT '1',
  `daily_roulette_count` int NOT NULL DEFAULT '0',
  `attendance_days` json DEFAULT NULL,
  `attendance_rewards` json DEFAULT NULL,
  `attendance_last_reset` date DEFAULT NULL,
  `talent_skills` text COLLATE utf8mb4_unicode_ci,
  `senjutsu_skills` text COLLATE utf8mb4_unicode_ci,
  `senjutsu_type` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `senjutsu_equipped_skills` text COLLATE utf8mb4_unicode_ci,
  `hunting_house_tries` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `hunting_house_date` date DEFAULT NULL,
  `eudemon_garden_tries` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `eudemon_garden_date` date DEFAULT NULL,
  `recruitable` tinyint(1) NOT NULL DEFAULT '1',
  `pvp_played` int NOT NULL DEFAULT '0',
  `pvp_won` int NOT NULL DEFAULT '0',
  `pvp_lost` int NOT NULL DEFAULT '0',
  `pvp_points` int NOT NULL DEFAULT '0',
  `pvp_trophy` int NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `characters_user_id_foreign` (`user_id`),
  CONSTRAINT `characters_user_id_foreign` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clan_auth_tokens` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `user_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `token` varchar(64) COLLATE utf8mb4_unicode_ci NOT NULL,
  `expires_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `clan_auth_tokens_token_unique` (`token`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clan_battles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `attacker_clan_id` bigint unsigned NOT NULL,
  `defender_clan_id` bigint unsigned NOT NULL,
  `season_id` bigint unsigned DEFAULT NULL,
  `attacker_won` tinyint(1) NOT NULL DEFAULT '0',
  `battle_data` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clan_members` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `clan_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `role` enum('master','elder','member') COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'member',
  `stamina` int unsigned NOT NULL DEFAULT '5',
  `max_stamina` int unsigned NOT NULL DEFAULT '5',
  `donated_golds` bigint unsigned NOT NULL DEFAULT '0',
  `donated_tokens` bigint unsigned NOT NULL DEFAULT '0',
  `prestige_boost_expires_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `clan_members_clan_id_character_id_unique` (`clan_id`,`character_id`),
  UNIQUE KEY `clan_members_character_id_unique` (`character_id`),
  CONSTRAINT `clan_members_clan_id_foreign` FOREIGN KEY (`clan_id`) REFERENCES `clans` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clan_requests` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `clan_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `status` enum('pending','accepted','rejected') COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'pending',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `clan_requests_clan_id_foreign` (`clan_id`),
  CONSTRAINT `clan_requests_clan_id_foreign` FOREIGN KEY (`clan_id`) REFERENCES `clans` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clan_seasons` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `number` int unsigned NOT NULL DEFAULT '1',
  `active` tinyint(1) NOT NULL DEFAULT '0',
  `started_at` timestamp NULL DEFAULT NULL,
  `ended_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `clans` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `season_id` bigint unsigned DEFAULT NULL,
  `name` varchar(50) COLLATE utf8mb4_unicode_ci NOT NULL,
  `master_id` bigint unsigned NOT NULL,
  `prestige` int unsigned NOT NULL DEFAULT '0',
  `max_members` int unsigned NOT NULL DEFAULT '10',
  `gold` bigint unsigned NOT NULL DEFAULT '0',
  `tokens` bigint unsigned NOT NULL DEFAULT '0',
  `announcement_draft` text COLLATE utf8mb4_unicode_ci,
  `announcement_published` text COLLATE utf8mb4_unicode_ci,
  `buildings` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `clans_name_unique` (`name`),
  KEY `clans_season_id_foreign` (`season_id`),
  CONSTRAINT `clans_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `clan_seasons` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_auth_tokens` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `user_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `token` varchar(64) COLLATE utf8mb4_unicode_ci NOT NULL,
  `expires_at` timestamp NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `crew_auth_tokens_token_unique` (`token`),
  KEY `crew_auth_tokens_user_id_foreign` (`user_id`),
  KEY `crew_auth_tokens_character_id_foreign` (`character_id`),
  CONSTRAINT `crew_auth_tokens_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `crew_auth_tokens_user_id_foreign` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_battles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `season_id` bigint unsigned NOT NULL,
  `castle_id` bigint unsigned DEFAULT NULL,
  `attacker_crew_id` bigint unsigned NOT NULL,
  `defender_crew_id` bigint unsigned DEFAULT NULL,
  `attacker_won` tinyint(1) NOT NULL DEFAULT '0',
  `battle_data` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `crew_battles_season_id_foreign` (`season_id`),
  KEY `crew_battles_attacker_crew_id_foreign` (`attacker_crew_id`),
  CONSTRAINT `crew_battles_attacker_crew_id_foreign` FOREIGN KEY (`attacker_crew_id`) REFERENCES `crews` (`id`) ON DELETE CASCADE,
  CONSTRAINT `crew_battles_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `crew_seasons` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_castles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `season_id` bigint unsigned NOT NULL,
  `castle_index` tinyint unsigned NOT NULL,
  `name` varchar(100) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '',
  `owner_crew_id` bigint unsigned DEFAULT NULL,
  `wall_hp` int unsigned NOT NULL DEFAULT '100',
  `defender_hp` int unsigned NOT NULL DEFAULT '100',
  `last_recovery_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `crew_castles_season_id_castle_index_unique` (`season_id`,`castle_index`),
  KEY `crew_castles_owner_crew_id_index` (`owner_crew_id`),
  CONSTRAINT `crew_castles_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `crew_seasons` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_members` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `crew_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `role` varchar(20) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'member',
  `battle_role` tinyint unsigned NOT NULL DEFAULT '1',
  `role_limit_at` varchar(50) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '',
  `stamina` int NOT NULL DEFAULT '50',
  `max_stamina` int NOT NULL DEFAULT '50',
  `donated_golds` int NOT NULL DEFAULT '0',
  `donated_tokens` int NOT NULL DEFAULT '0',
  `minigame_energy` int NOT NULL DEFAULT '3',
  `prestige_boost_expires_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `crew_members_character_id_unique` (`character_id`),
  KEY `crew_members_crew_id_foreign` (`crew_id`),
  CONSTRAINT `crew_members_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `crew_members_crew_id_foreign` FOREIGN KEY (`crew_id`) REFERENCES `crews` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_requests` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `crew_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `status` varchar(20) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'pending',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `crew_requests_crew_id_character_id_unique` (`crew_id`,`character_id`),
  KEY `crew_requests_character_id_foreign` (`character_id`),
  CONSTRAINT `crew_requests_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `crew_requests_crew_id_foreign` FOREIGN KEY (`crew_id`) REFERENCES `crews` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crew_seasons` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `number` int unsigned NOT NULL DEFAULT '1',
  `active` tinyint(1) NOT NULL DEFAULT '0',
  `phase` tinyint unsigned NOT NULL DEFAULT '1',
  `started_at` timestamp NULL DEFAULT NULL,
  `ended_at` timestamp NULL DEFAULT NULL,
  `phase_ends_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `crew_seasons_active_index` (`active`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `crews` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `season_id` bigint unsigned NOT NULL,
  `name` varchar(50) COLLATE utf8mb4_unicode_ci NOT NULL,
  `master_id` bigint unsigned NOT NULL,
  `prestige` int NOT NULL DEFAULT '0',
  `max_members` int NOT NULL DEFAULT '20',
  `gold` int NOT NULL DEFAULT '0',
  `tokens` int NOT NULL DEFAULT '0',
  `buildings` json DEFAULT NULL,
  `announcement_published` text COLLATE utf8mb4_unicode_ci,
  `announcement_draft` text COLLATE utf8mb4_unicode_ci,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `crews_name_unique` (`name`),
  KEY `crews_season_id_foreign` (`season_id`),
  KEY `crews_master_id_index` (`master_id`),
  CONSTRAINT `crews_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `crew_seasons` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `enemies` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `enemy_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL,
  `hp` int NOT NULL,
  `cp` int NOT NULL,
  `agility` int NOT NULL,
  `attacks` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `enemies_enemy_id_unique` (`enemy_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `failed_jobs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `uuid` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `connection` text COLLATE utf8mb4_unicode_ci NOT NULL,
  `queue` text COLLATE utf8mb4_unicode_ci NOT NULL,
  `payload` longtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `exception` longtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `failed_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `failed_jobs_uuid_unique` (`uuid`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `friend_requests` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `requester_id` bigint unsigned NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `friend_requests_character_id_requester_id_unique` (`character_id`,`requester_id`),
  KEY `friend_requests_requester_id_foreign` (`requester_id`),
  CONSTRAINT `friend_requests_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `friend_requests_requester_id_foreign` FOREIGN KEY (`requester_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `game_configs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `key` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `value` json NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `game_configs_key_unique` (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `game_events` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `title` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `type` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'seasonal',
  `panel` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `date` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `icon` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `inside` tinyint(1) NOT NULL DEFAULT '0',
  `sort_order` smallint unsigned NOT NULL DEFAULT '0',
  `starts_at` timestamp NULL DEFAULT NULL,
  `ends_at` timestamp NULL DEFAULT NULL,
  `image_url` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `active` tinyint(1) NOT NULL DEFAULT '1',
  `data` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `giveaway_participants` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `giveaway_id` bigint unsigned NOT NULL,
  `character_id` bigint unsigned NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `giveaway_participants_giveaway_id_character_id_unique` (`giveaway_id`,`character_id`),
  KEY `giveaway_participants_character_id_index` (`character_id`),
  CONSTRAINT `giveaway_participants_giveaway_id_foreign` FOREIGN KEY (`giveaway_id`) REFERENCES `giveaways` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `giveaways` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `title` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `prizes` json DEFAULT NULL,
  `requirements` json DEFAULT NULL,
  `ends_at` timestamp NOT NULL,
  `winners` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `items` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `item_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL DEFAULT '1',
  `price_gold` int NOT NULL DEFAULT '0',
  `price_tokens` int NOT NULL DEFAULT '0',
  `category` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `premium` tinyint(1) NOT NULL DEFAULT '0',
  `icon` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `items_item_id_unique` (`item_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `job_batches` (
  `id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `total_jobs` int NOT NULL,
  `pending_jobs` int NOT NULL,
  `failed_jobs` int NOT NULL,
  `failed_job_ids` longtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `options` mediumtext COLLATE utf8mb4_unicode_ci,
  `cancelled_at` int DEFAULT NULL,
  `created_at` int NOT NULL,
  `finished_at` int DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `jobs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `queue` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `payload` longtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `attempts` tinyint unsigned NOT NULL,
  `reserved_at` int unsigned DEFAULT NULL,
  `available_at` int unsigned NOT NULL,
  `created_at` int unsigned NOT NULL,
  PRIMARY KEY (`id`),
  KEY `jobs_queue_index` (`queue`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `migrations` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `migration` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `batch` int NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `missions` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `mission_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `req_lvl` int NOT NULL DEFAULT '1',
  `xp` int NOT NULL DEFAULT '0',
  `gold` int NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `missions_mission_id_unique` (`mission_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `mysterious_markets` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `active` tinyint(1) NOT NULL DEFAULT '0',
  `ends_at` timestamp NOT NULL,
  `discount` varchar(10) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '0',
  `refresh_cost` int unsigned NOT NULL DEFAULT '50',
  `refresh_max` int unsigned NOT NULL DEFAULT '3',
  `items` json DEFAULT NULL,
  `all_packages` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `mysterious_markets_active_index` (`active`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `npcs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `npc_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL,
  `rank` int NOT NULL,
  `hp` int NOT NULL,
  `cp` int NOT NULL,
  `agility` int NOT NULL,
  `dodge` int NOT NULL,
  `critical` int NOT NULL,
  `accuracy` int NOT NULL,
  `purify` int NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `attacks` json DEFAULT NULL,
  `price_gold` int NOT NULL DEFAULT '0',
  `price_tokens` int NOT NULL DEFAULT '0',
  `premium` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `npcs_npc_id_unique` (`npc_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `password_reset_tokens` (
  `email` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `token` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `pet_villa_slots` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `slot_index` tinyint NOT NULL,
  `status` tinyint NOT NULL DEFAULT '1',
  `pet_instance_id` bigint unsigned DEFAULT NULL,
  `training_ends_at` int unsigned DEFAULT NULL,
  `gold_spent` int unsigned NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `pet_villa_slots_character_id_slot_index_unique` (`character_id`,`slot_index`),
  KEY `pet_villa_slots_pet_instance_id_foreign` (`pet_instance_id`),
  CONSTRAINT `pet_villa_slots_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `pet_villa_slots_pet_instance_id_foreign` FOREIGN KEY (`pet_instance_id`) REFERENCES `character_pets` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `pets` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `pet_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `swf` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `price_gold` int NOT NULL DEFAULT '0',
  `price_tokens` int NOT NULL DEFAULT '0',
  `premium` tinyint(1) NOT NULL DEFAULT '0',
  `icon` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `skills` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `pets_pet_id_unique` (`pet_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `pvp_battles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `host_id` bigint unsigned NOT NULL,
  `enemy_id` bigint unsigned NOT NULL,
  `mode` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `host_won` tinyint(1) NOT NULL DEFAULT '0',
  `trophy_delta` int NOT NULL DEFAULT '0',
  `host_trophy_before` int NOT NULL DEFAULT '0',
  `host_trophy_after` int NOT NULL DEFAULT '0',
  `enemy_trophy_before` int NOT NULL DEFAULT '0',
  `enemy_trophy_after` int NOT NULL DEFAULT '0',
  `host_level` int NOT NULL DEFAULT '1',
  `enemy_level` int NOT NULL DEFAULT '1',
  `host_rank` int NOT NULL DEFAULT '1',
  `enemy_rank` int NOT NULL DEFAULT '1',
  `host_snapshot` json DEFAULT NULL,
  `enemy_snapshot` json DEFAULT NULL,
  `battle_data` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `pvp_battles_host_id_index` (`host_id`),
  KEY `pvp_battles_enemy_id_index` (`enemy_id`),
  KEY `pvp_battles_created_at_index` (`created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `pvp_bug_reports` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `title` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci NOT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `pvp_bug_reports_character_id_index` (`character_id`),
  KEY `pvp_bug_reports_created_at_index` (`created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `senjutsus` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `senjutsu_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `effects` json DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `senjutsus_senjutsu_id_unique` (`senjutsu_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `sessions` (
  `id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `user_id` bigint unsigned DEFAULT NULL,
  `ip_address` varchar(45) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `user_agent` text COLLATE utf8mb4_unicode_ci,
  `payload` longtext COLLATE utf8mb4_unicode_ci NOT NULL,
  `last_activity` int NOT NULL,
  PRIMARY KEY (`id`),
  KEY `sessions_user_id_index` (`user_id`),
  KEY `sessions_last_activity_index` (`last_activity`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `shadow_war_battles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `season_id` bigint unsigned NOT NULL,
  `battle_code` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `character_id` bigint unsigned NOT NULL,
  `enemy_id` bigint unsigned DEFAULT NULL,
  `character_squad` tinyint unsigned DEFAULT NULL,
  `enemy_squad` tinyint unsigned DEFAULT NULL,
  `character_level` int NOT NULL DEFAULT '0',
  `enemy_level` int NOT NULL DEFAULT '0',
  `character_rank` int NOT NULL DEFAULT '0',
  `enemy_rank` int NOT NULL DEFAULT '0',
  `character_trophy_before` int NOT NULL DEFAULT '0',
  `enemy_trophy_before` int NOT NULL DEFAULT '0',
  `character_trophy_after` int NOT NULL DEFAULT '0',
  `enemy_trophy_after` int NOT NULL DEFAULT '0',
  `trophy_delta` int NOT NULL DEFAULT '0',
  `total_damage` int NOT NULL DEFAULT '0',
  `won` tinyint(1) NOT NULL DEFAULT '0',
  `battle_data` json DEFAULT NULL,
  `energy_cost` int NOT NULL DEFAULT '0',
  `energy_before` int NOT NULL DEFAULT '0',
  `energy_after` int NOT NULL DEFAULT '0',
  `refills_used_today` int NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `shadow_war_battles_season_id_foreign` (`season_id`),
  KEY `shadow_war_battles_character_id_foreign` (`character_id`),
  KEY `shadow_war_battles_battle_code_index` (`battle_code`),
  CONSTRAINT `shadow_war_battles_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `shadow_war_battles_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `shadow_war_seasons` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `shadow_war_seasons` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `num` int NOT NULL,
  `start_at` datetime NOT NULL,
  `end_at` datetime NOT NULL,
  `active` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `shadow_war_seasons_num_index` (`num`),
  KEY `shadow_war_seasons_active_index` (`active`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `shadow_war_squads` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `character_id` bigint unsigned NOT NULL,
  `season_id` bigint unsigned NOT NULL,
  `squad` tinyint unsigned NOT NULL,
  `rank` int NOT NULL DEFAULT '0',
  `trophy` int NOT NULL DEFAULT '0',
  `energy` int NOT NULL DEFAULT '100',
  `energy_last_reset` date DEFAULT NULL,
  `energy_refills_today` int NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `shadow_war_squads_character_id_season_id_unique` (`character_id`,`season_id`),
  KEY `shadow_war_squads_season_id_foreign` (`season_id`),
  CONSTRAINT `shadow_war_squads_character_id_foreign` FOREIGN KEY (`character_id`) REFERENCES `characters` (`id`) ON DELETE CASCADE,
  CONSTRAINT `shadow_war_squads_season_id_foreign` FOREIGN KEY (`season_id`) REFERENCES `shadow_war_seasons` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `skills` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `skill_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `level` int NOT NULL DEFAULT '1',
  `element` tinyint NOT NULL DEFAULT '0',
  `price_gold` int NOT NULL DEFAULT '0',
  `price_tokens` int NOT NULL DEFAULT '0',
  `premium` tinyint(1) NOT NULL DEFAULT '0',
  `icon` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `swf` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `skills_skill_id_unique` (`skill_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `talents` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `talent_id` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `skills` json DEFAULT NULL,
  `price_gold` int NOT NULL DEFAULT '0',
  `price_tokens` int NOT NULL DEFAULT '0',
  `is_emblem` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `talents_talent_id_unique` (`talent_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `users` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `username` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `email` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `account_type` tinyint NOT NULL DEFAULT '0',
  `tokens` int NOT NULL DEFAULT '0',
  `email_verified_at` timestamp NULL DEFAULT NULL,
  `password` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `session_key` varchar(64) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `remember_token` varchar(100) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `users_username_unique` (`username`),
  UNIQUE KEY `users_email_unique` (`email`),
  KEY `users_session_key_index` (`session_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `x_p_s` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `level` int NOT NULL,
  `character_xp` int NOT NULL,
  `pet_xp` int NOT NULL DEFAULT '999999999',
  `created_at` timestamp NULL DEFAULT NULL,
  `updated_at` timestamp NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `x_p_s_level_unique` (`level`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
