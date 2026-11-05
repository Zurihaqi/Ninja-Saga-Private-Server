-- Persistent client gear presets; no existing player data is rewritten.
CREATE TABLE IF NOT EXISTS character_gear_presets (
  id bigint unsigned NOT NULL AUTO_INCREMENT PRIMARY KEY,
  character_id bigint unsigned NOT NULL,
  name varchar(32) NOT NULL,
  weapon varchar(255) NOT NULL,
  back_item varchar(255) NOT NULL,
  clothing varchar(255) NOT NULL,
  accessory varchar(255) NOT NULL,
  hair varchar(255) NOT NULL,
  hair_color varchar(255) NOT NULL,
  CONSTRAINT character_gear_presets_character_foreign FOREIGN KEY (character_id)
    REFERENCES characters(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
