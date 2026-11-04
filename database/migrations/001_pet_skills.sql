-- Additive player-state extension. Existing character/pet rows are left unchanged.
CREATE TABLE IF NOT EXISTS character_pet_skills (
  pet_instance_id bigint unsigned NOT NULL,
  skill_slot tinyint unsigned NOT NULL,
  learned_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (pet_instance_id, skill_slot),
  CONSTRAINT character_pet_skills_pet_foreign FOREIGN KEY (pet_instance_id)
    REFERENCES character_pets (id) ON DELETE CASCADE,
  CONSTRAINT character_pet_skills_valid_slot CHECK (skill_slot BETWEEN 1 AND 6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
