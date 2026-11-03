-- Minimal synthetic catalog for headless protocol/persistence checks only.
INSERT IGNORE INTO enemies (enemy_id,name,level,hp,cp,agility) VALUES ('ene_01','Regression enemy',1,100,100,5);
INSERT IGNORE INTO missions (mission_id,req_lvl,xp,gold) VALUES ('msn_1',1,20,20);
INSERT IGNORE INTO x_p_s (level,character_xp,pet_xp) VALUES (1,15,999999999),(2,100,999999999);
INSERT IGNORE INTO items (item_id,name,level,price_gold,price_tokens,category,premium) VALUES ('wpn_regression','Regression weapon',1,50,0,'weapon',0);
INSERT IGNORE INTO skills (skill_id,name,level,element,price_gold,price_tokens,premium) VALUES ('skill_regression','Regression skill',1,2,25,0,0);
INSERT IGNORE INTO pets (pet_id,name,swf,price_gold,price_tokens,premium) VALUES ('pet_regression','Regression pet','pet_01',50,0,0);
INSERT IGNORE INTO npcs (npc_id,name,level,`rank`,hp,cp,agility,dodge,critical,accuracy,purify,price_gold) VALUES ('npc_regression','Regression teammate',1,1,100,100,5,0,0,0,0,25);
INSERT IGNORE INTO game_configs (`key`,value) VALUES
('talent_shop','{"extreme":[{"id":"de"}],"secret":[]}'),
('talent_info','{"de":{"is_emblem":false,"price_gold":500000,"price_token":0}}'),
('talent_description','{"talent_de_skill_1":{"talent_skill_id":"skill_1006"}}'),
('academy_chains','{"fire":{"rage":["skill_152","skill_232"]}}');
INSERT IGNORE INTO skills (skill_id,name,level,element,price_gold,price_tokens,premium) VALUES ('skill_152','Regression base skill',40,2,10,0,0),('skill_232','Regression upgraded skill',40,2,25,0,0);
