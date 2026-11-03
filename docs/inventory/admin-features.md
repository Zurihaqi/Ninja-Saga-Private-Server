# Administration inventory

Source: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Evidence: static source inspection only; no migrated database or runtime route list has been verified.

Regenerate with `python scripts/schema_inventory.py`. Paths and line numbers refer to the pinned reference. The extractor is lexical: conditional branches, framework defaults and dynamic names require runtime verification.

## Access boundary

`AdminPanelProvider` configures `/admin`, login, session/CSRF middleware and Filament Authenticate. `User::canAccessPanel` allows account type 99 **or** email `admin@ninjasage.test`. This email bypass must be recorded as an upstream security defect, not silently treated as role-based authorization. No app Policies directory or explicit resource canCreate/canEdit/canDelete overrides were observed. Framework authorization defaults are not verified.

Sources: [app/Models/User.php:58](../../reference/upstream/Database/Laragon/ninjasage/app/Models/User.php#L58), [app/Providers/Filament/AdminPanelProvider.php:25](../../reference/upstream/Database/Laragon/ninjasage/app/Providers/Filament/AdminPanelProvider.php#L25).

16 resource classes across 15 resource directories; 20 standalone page classes.

## Resource classes

- **AdminCommandResource** — model: `AdminCommand`. [app/Filament/Resources/AdminCommands/AdminCommandResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/AdminCommands/AdminCommandResource.php#L1)
  - `'index'  => ListAdminCommands::route('/')` (L49)
  - `'create' => CreateAdminCommand::route('/create')` (L50)
  - `'edit'   => EditAdminCommand::route('/{record}/edit')` (L51)
- **CharacterResource** — model: `Character`. [app/Filament/Resources/Characters/CharacterResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/CharacterResource.php#L1)
  - `'index' => ListCharacters::route('/')` (L53)
  - `'create' => CreateCharacter::route('/create')` (L54)
  - `'edit' => EditCharacter::route('/{record}/edit')` (L55)
- **ClanSeasonResource** — model: `ClanSeason`. [app/Filament/Resources/ClanSeasons/ClanSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ClanSeasons/ClanSeasonResource.php#L1)
  - `'index'  => Pages\ListClanSeasons::route('/')` (L161)
  - `'create' => Pages\CreateClanSeason::route('/create')` (L162)
  - `'edit'   => Pages\EditClanSeason::route('/{record}/edit')` (L163)
- **CrewSeasonResource** — model: `CrewSeason`. [app/Filament/Resources/CrewSeasons/CrewSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/CrewSeasons/CrewSeasonResource.php#L1)
  - `'index'  => Pages\ListCrewSeasons::route('/')` (L189)
  - `'create' => Pages\CreateCrewSeason::route('/create')` (L190)
  - `'edit'   => Pages\EditCrewSeason::route('/{record}/edit')` (L191)
- **EventAnalyticsResource** — model: `CharacterEventData`. [app/Filament/Resources/GameEvents/EventAnalyticsResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/EventAnalyticsResource.php#L1)
  - `'index'   => Pages\ListEventAnalytics::route('/')` (L119)
  - `'players' => Pages\ListEventPlayers::route('/players')` (L120)
- **GameEventResource** — model: `GameEvent`. [app/Filament/Resources/GameEvents/GameEventResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/GameEventResource.php#L1)
  - `'index'  => Pages\ListGameEvents::route('/')` (L569)
  - `'create' => Pages\CreateGameEvent::route('/create')` (L570)
  - `'edit'   => Pages\EditGameEvent::route('/{record}/edit')` (L571)
- **GiveawayResource** — model: `Giveaway`. [app/Filament/Resources/Giveaways/GiveawayResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Giveaways/GiveawayResource.php#L1)
  - `'index'  => ListGiveaways::route('/')` (L47)
  - `'create' => CreateGiveaway::route('/create')` (L48)
  - `'edit'   => EditGiveaway::route('/{record}/edit')` (L49)
- **ItemResource** — model: `Item`. [app/Filament/Resources/Items/ItemResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Items/ItemResource.php#L1)
  - `'index' => ListItems::route('/')` (L43)
  - `'create' => CreateItem::route('/create')` (L44)
  - `'edit' => EditItem::route('/{record}/edit')` (L45)
- **MailResource** — model: `CharacterMail`. [app/Filament/Resources/Mails/MailResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Mails/MailResource.php#L1)
  - `'index'  => ListMails::route('/')` (L46)
  - `'create' => CreateMail::route('/create')` (L47)
- **MissionResource** — model: `Mission`. [app/Filament/Resources/Missions/MissionResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Missions/MissionResource.php#L1)
  - `'index' => ListMissions::route('/')` (L43)
  - `'create' => CreateMission::route('/create')` (L44)
  - `'edit' => EditMission::route('/{record}/edit')` (L45)
- **MysteriousMarketResource** — model: `MysteriousMarket`. [app/Filament/Resources/MysteriousMarkets/MysteriousMarketResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/MysteriousMarkets/MysteriousMarketResource.php#L1)
  - `'index'  => Pages\ListMysteriousMarkets::route('/')` (L194)
  - `'create' => Pages\CreateMysteriousMarket::route('/create')` (L195)
  - `'edit'   => Pages\EditMysteriousMarket::route('/{record}/edit')` (L196)
- **PetResource** — model: `Pet`. [app/Filament/Resources/Pets/PetResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Pets/PetResource.php#L1)
  - `'index' => ListPets::route('/')` (L43)
  - `'create' => CreatePet::route('/create')` (L44)
  - `'edit' => EditPet::route('/{record}/edit')` (L45)
- **ShadowWarSeasonResource** — model: `ShadowWarSeason`. [app/Filament/Resources/ShadowWarSeasons/ShadowWarSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ShadowWarSeasons/ShadowWarSeasonResource.php#L1)
  - `'index'  => Pages\ListShadowWarSeasons::route('/')` (L160)
  - `'create' => Pages\CreateShadowWarSeason::route('/create')` (L161)
  - `'edit'   => Pages\EditShadowWarSeason::route('/{record}/edit')` (L162)
- **SkillResource** — model: `Skill`. [app/Filament/Resources/Skills/SkillResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Skills/SkillResource.php#L1)
  - `'index' => ListSkills::route('/')` (L43)
  - `'create' => CreateSkill::route('/create')` (L44)
  - `'edit' => EditSkill::route('/{record}/edit')` (L45)
- **UserResource** — model: `User`. [app/Filament/Resources/Users/UserResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Users/UserResource.php#L1)
  - `'index' => ListUsers::route('/')` (L43)
  - `'create' => CreateUser::route('/create')` (L44)
  - `'edit' => EditUser::route('/{record}/edit')` (L45)
- **XPLevelResource** — model: `XP`. [app/Filament/Resources/XPLevels/XPLevelResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/XPLevels/XPLevelResource.php#L1)
  - `'index' => ListXPLevels::route('/')` (L48)
  - `'edit'  => EditXPLevel::route('/{record}/edit')` (L49)

## Standalone pages and callable methods

| Page | Public methods | Source |
|---|---|---|
| AcademySettings | mount (L37), form (L61), save (L175) | [app/Filament/Pages/AcademySettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/AcademySettings.php#L1) |
| AnnouncementSettings | mount (L30), form (L38), save (L72) | [app/Filament/Pages/AnnouncementSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/AnnouncementSettings.php#L1) |
| ClanSettings | mount (L38), form (L51), save (L126) | [app/Filament/Pages/ClanSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ClanSettings.php#L1) |
| DailyAttendanceSettings | mount (L31), form (L42), save (L101) | [app/Filament/Pages/DailyAttendanceSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyAttendanceSettings.php#L1) |
| DailyRouletteSettings | mount (L41), form (L59), save (L121) | [app/Filament/Pages/DailyRouletteSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyRouletteSettings.php#L1) |
| DailyScratchSettings | mount (L49), form (L69), save (L134) | [app/Filament/Pages/DailyScratchSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyScratchSettings.php#L1) |
| EudemonGardenSettings | mount (L44), form (L101), save (L300) | [app/Filament/Pages/EudemonGardenSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/EudemonGardenSettings.php#L1) |
| ExoticPackageSettings | mount (L45), form (L59), save (L123) | [app/Filament/Pages/ExoticPackageSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ExoticPackageSettings.php#L1) |
| GameSettings | mount (L30), form (L37), save (L90) | [app/Filament/Pages/GameSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/GameSettings.php#L1) |
| HuntingHouseSettings | mount (L73), form (L112), save (L233) | [app/Filament/Pages/HuntingHouseSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/HuntingHouseSettings.php#L1) |
| HuntingMarketManager | mount (L46), form (L77), save (L163) | [app/Filament/Pages/HuntingMarketManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/HuntingMarketManager.php#L1) |
| ItemCreator | mount (L50), updatedItemCategory (L80), loadItem (L89), resetToNew (L142), form (L165), save (L426) | [app/Filament/Pages/ItemCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ItemCreator.php#L1) |
| MaterialMarketManager | mount (L50), form (L82), save (L173) | [app/Filament/Pages/MaterialMarketManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/MaterialMarketManager.php#L1) |
| PrestigeSettings | mount (L45), form (L64), save (L172) | [app/Filament/Pages/PrestigeSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/PrestigeSettings.php#L1) |
| ShopManager | mount (L53), form (L83), save (L194) | [app/Filament/Pages/ShopManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ShopManager.php#L1) |
| SkillCreator | mount (L54), loadSkill (L73), resetToNew (L177), form (L205), save (L536) | [app/Filament/Pages/SkillCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/SkillCreator.php#L1) |
| SpecialDealsSettings | mount (L30), form (L46), save (L113) | [app/Filament/Pages/SpecialDealsSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/SpecialDealsSettings.php#L1) |
| WeaponCreator | mount (L50), loadWeapon (L69), resetToNew (L121), form (L147), save (L431) | [app/Filament/Pages/WeaponCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WeaponCreator.php#L1) |
| WelcomeLoginSettings | mount (L50), form (L71), save (L167), resetDefaults (L199) | [app/Filament/Pages/WelcomeLoginSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WelcomeLoginSettings.php#L1) |
| WorldChat | availableChannels (L38), selectChannel (L48), table (L56) | [app/Filament/Pages/WorldChat.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WorldChat.php#L1) |

## Actions, forms and relationship managers

This index includes declared action and field identifiers, not implicit framework actions or rendered UI verification. Validate permission, validation and database effects per operation before replacing it.

### Pages/AcademySettings.php

[app/Filament/Pages/AcademySettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/AcademySettings.php#L1)

`Section(Wind (元素: 風))` L65, `Section(Fire (元素: 火))` L70, `Section(Thunder (元素: 雷))` L76, `Section(Earth (元素: 土))` L82, `Section(Water (元素: 水))` L88, `Section(Taijutsu (体術))` L94, `Section(Genjutsu (幻術))` L100, `Select(skill_id)` L114, `TextInput(price_gold)` L134, `TextInput(price_tokens)` L142, `Action(save)` L166.

### Pages/AnnouncementSettings.php

[app/Filament/Pages/AnnouncementSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/AnnouncementSettings.php#L1)

`Section(News Panel Announcement)` L42, `Toggle(enabled)` L48, `Textarea(text)` L53, `Action(save)` L65.

### Pages/ClanSettings.php

[app/Filament/Pages/ClanSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ClanSettings.php#L1)

`Section(Clan Stamina)` L55, `TextInput(initial_stamina)` L59, `TextInput(max_stamina_cap)` L66, `TextInput(stamina_upgrade_cost)` L73, `TextInput(stamina_upgrade_step)` L80, `Section(Clan Max Members)` L88, `TextInput(default_max_members)` L92, `TextInput(max_members_cap)` L99, `TextInput(increase_members_cost)` L106, `Action(save)` L119.

### Pages/DailyAttendanceSettings.php

[app/Filament/Pages/DailyAttendanceSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyAttendanceSettings.php#L1)

`Section(Reward Slots)` L45, `Repeater(rewards)` L53, `TextInput(id)` L57, `TextInput(price)` L63, `TextInput(item)` L71, `Action(save)` L94.

### Pages/DailyRouletteSettings.php

[app/Filament/Pages/DailyRouletteSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyRouletteSettings.php#L1)

`Section(Reward Pool)` L62, `Repeater(rewards)` L69, `Select(type)` L73, `TextInput(amount)` L84, `Action(save)` L111.

### Pages/DailyScratchSettings.php

[app/Filament/Pages/DailyScratchSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/DailyScratchSettings.php#L1)

`Section(Regular Reward Pool)` L73, `Repeater(rewards)` L81, `TextInput(reward_id)` L84, `Section(Grand Prize Pool)` L96, `Repeater(grand_prize)` L102, `TextInput(reward_id)` L105, `Action(save)` L124.

### Pages/EudemonGardenSettings.php

[app/Filament/Pages/EudemonGardenSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/EudemonGardenSettings.php#L1)

`Section(Global Settings)` L105, `TextInput(default_tries)` L106, `Section(Bosses)` L116, `Repeater(bosses)` L119, `TextInput(name)` L125, `TextInput(lvl)` L130, `Select(rank)` L137, `TextInput(bg)` L146, `TextInput(gold)` L155, `TextInput(xp)` L159, `Textarea(desc)` L165, `Section(Enemy IDs)` L171, `Repeater(enemy_ids)` L175, `TextInput(enemy_id)` L178, `Section(Display Rewards (client icons))` L192, `Repeater(display_rewards)` L196, `TextInput(item_id)` L199, `Section(Server Rewards (actual drop table))` L212, `Repeater(server_rewards)` L216, `TextInput(item_id)` L220, `TextInput(rate)` L227, `TextInput(min)` L237, `TextInput(max)` L241, `Toggle(unique)` L248, `TextInput(owned_rate)` L254, `Action(save)` L290.

### Pages/ExoticPackageSettings.php

[app/Filament/Pages/ExoticPackageSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ExoticPackageSettings.php#L1)

`Section(Exotic Package Configuration)` L62, `Repeater(packages)` L70, `Select(key)` L74, `TextInput(name)` L80, `TextInput(price)` L86, `Textarea(items)` L93, `Action(save)` L116.

### Pages/GameSettings.php

[app/Filament/Pages/GameSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/GameSettings.php#L1)

`Section(Side Panel Icons)` L41, `Toggle(special_deals_visible)` L47, `Placeholder(limited_store_note)` L55, `Placeholder(event_menu_note)` L63, `Placeholder(design_contest_note)` L70, `Action(save)` L83.

### Pages/HuntingHouseSettings.php

[app/Filament/Pages/HuntingHouseSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/HuntingHouseSettings.php#L1)

`Section(General Settings)` L117, `TextInput(daily_claim_free)` L121, `TextInput(daily_claim_premium)` L126, `TextInput(material_price)` L131, `Section(Zone Rewards)` L139, `Section(Forge Recipes)` L155, `Repeater(recipes)` L162, `TextInput(output_item)` L166, `Repeater(materials)` L172, `TextInput(mat_id)` L176, `TextInput(qty)` L183, `TextInput(item_id)` L207, `Action(save)` L223.

### Pages/HuntingMarketManager.php

[app/Filament/Pages/HuntingMarketManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/HuntingMarketManager.php#L1)

`TextInput(item_id)` L90, `Section(Craft Requirements)` L98, `Repeater(requirements)` L102, `TextInput(material_id)` L106, `TextInput(qty)` L114, `Tabs(categories)` L144, `Action(save)` L153.

### Pages/ItemCreator.php

[app/Filament/Pages/ItemCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ItemCreator.php#L1)

`Section(Edit Existing Item)` L169, `Select(loadItemSelect)` L173, `Section(Item Category)` L194, `Select(item_category)` L197, `Section(Basic Information)` L214, `TextInput(item_id)` L218, `TextInput(item_name)` L225, `Textarea(item_description)` L231, `Section(Pricing & Availability)` L238, `TextInput(item_level)` L242, `TextInput(item_sell_price)` L250, `TextInput(item_price_gold)` L259, `TextInput(item_price_tokens)` L266, `TextInput(item_price_pvp)` L273, `TextInput(item_price_prestige)` L282, `TextInput(item_price_merit)` L289, `Toggle(item_buyable)` L298, `Toggle(item_buyable_clan)` L303, `Toggle(item_premium)` L308, `Toggle(item_sellable)` L313, `Section(Passive Effects)` L320, `Repeater(item_effects)` L324, `Select(target)` L328, `Select(type)` L338, `Select(effect)` L350, `TextInput(effect_name)` L357, `Select(calc_type)` L365, `TextInput(amount)` L376, `TextInput(chance)` L382, `Action(save)` L409, `Action(resetToNew)` L415.

### Pages/MaterialMarketManager.php

[app/Filament/Pages/MaterialMarketManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/MaterialMarketManager.php#L1)

`TextInput(item_id)` L95, `Toggle(available)` L103, `Section(Craft Requirements)` L110, `Repeater(requirements)` L114, `TextInput(material_id)` L118, `TextInput(qty)` L126, `Tabs(categories)` L154, `Action(save)` L163.

### Pages/PrestigeSettings.php

[app/Filament/Pages/PrestigeSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/PrestigeSettings.php#L1)

`Section(Clan Battle Prestige)` L68, `TextInput(clan_win_char_prestige)` L72, `TextInput(clan_lose_char_prestige)` L79, `TextInput(clan_win_clan_reputation)` L86, `TextInput(clan_lose_clan_reputation)` L93, `Section(Crew Battle Prestige)` L101, `TextInput(crew_win_prestige)` L105, `TextInput(crew_lose_prestige)` L112, `Section(Live PvP Battle Rewards)` L120, `TextInput(pvp_points_win)` L124, `TextInput(pvp_points_lose)` L131, `TextInput(pvp_prestige_win)` L138, `TextInput(trophy_k_factor)` L145, `TextInput(trophy_floor)` L152, `Action(save)` L165.

### Pages/ShopManager.php

[app/Filament/Pages/ShopManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/ShopManager.php#L1)

`TextInput(item_id)` L103, `TextInput(name)` L110, `TextInput(price_gold)` L119, `TextInput(price_tokens)` L127, `TextInput(price_pvp)` L135, `TextInput(price_prestige)` L143, `TextInput(price_merit)` L151, `Tabs(shopTypes)` L175, `Action(save)` L184.

### Pages/SkillCreator.php

[app/Filament/Pages/SkillCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/SkillCreator.php#L1)

`Section(Edit Existing Skill)` L210, `Select(loadSkillSelect)` L214, `Section(Basic Information)` L241, `TextInput(skill_id)` L245, `TextInput(skill_name)` L252, `Textarea(skill_description)` L259, `FileUpload(skill_icon)` L265, `Section(Combat Stats)` L276, `Select(skill_type)` L280, `TextInput(skill_level)` L298, `Select(skill_target)` L306, `TextInput(skill_damage)` L319, `TextInput(skill_cp_cost)` L326, `TextInput(skill_cooldown)` L333, `TextInput(skill_hit_chance)` L342, `Select(skill_attack_hit_position)` L348, `Toggle(skill_multi_hit)` L364, `Section(Animation)` L371, `TextInput(skill_anims_hit)` L374, `TextInput(skill_swf)` L380, `Section(Shop & Availability)` L387, `Toggle(skill_buyable)` L391, `Toggle(skill_premium)` L396, `TextInput(skill_price_gold)` L403, `TextInput(skill_price_tokens)` L410, `Toggle(skill_na)` L418, `Section(Skill Effects)` L424, `Repeater(skill_effects)` L427, `Select(target)` L431, `Select(type)` L441, `Select(effect)` L453, `TextInput(effect_name)` L460, `TextInput(duration)` L468, `Select(calc_type)` L475, `TextInput(amount)` L486, `TextInput(chance)` L492, `Action(save)` L519, `Action(resetToNew)` L525.

### Pages/SpecialDealsSettings.php

[app/Filament/Pages/SpecialDealsSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/SpecialDealsSettings.php#L1)

`Section(Special Deal Packages)` L50, `Repeater(deals)` L58, `TextInput(id)` L61, `TextInput(name)` L68, `TextInput(end)` L74, `TextInput(price)` L80, `Textarea(items)` L87, `Action(save)` L106.

### Pages/WeaponCreator.php

[app/Filament/Pages/WeaponCreator.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WeaponCreator.php#L1)

`Section(Edit Existing Weapon / Item)` L151, `Select(loadWeaponSelect)` L155, `Section(Basic Information)` L175, `TextInput(wpn_id)` L179, `TextInput(wpn_name)` L186, `Textarea(wpn_description)` L193, `Select(wpn_item_type)` L200, `Select(wpn_attack_type)` L211, `Section(Combat Stats)` L233, `TextInput(wpn_level)` L237, `TextInput(wpn_damage)` L245, `Section(Pricing & Availability)` L254, `TextInput(wpn_price_gold)` L258, `TextInput(wpn_price_tokens)` L265, `TextInput(wpn_price_pvp)` L272, `TextInput(wpn_price_prestige)` L281, `TextInput(wpn_price_merit)` L288, `TextInput(wpn_sell_price)` L295, `Toggle(wpn_buyable)` L304, `Toggle(wpn_buyable_clan)` L309, `Toggle(wpn_premium)` L314, `Toggle(wpn_sellable)` L319, `Section(Passive Weapon Effects)` L326, `Repeater(wpn_effects)` L329, `Select(target)` L333, `Select(type)` L343, `Select(effect)` L355, `TextInput(effect_name)` L362, `Select(calc_type)` L370, `TextInput(amount)` L381, `TextInput(chance)` L387, `Action(save)` L414, `Action(resetToNew)` L420.

### Pages/WelcomeLoginSettings.php

[app/Filament/Pages/WelcomeLoginSettings.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WelcomeLoginSettings.php#L1)

`Section(7-Day Reward Slots)` L82, `TextInput(day1)` L90, `TextInput(day2)` L97, `TextInput(day3)` L105, `TextInput(day4)` L111, `TextInput(day5)` L119, `TextInput(day6)` L125, `TextInput(day7)` L133, `Action(save)` L149, `Action(resetDefaults)` L154.

### Pages/WorldChat.php

[app/Filament/Pages/WorldChat.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Pages/WorldChat.php#L1)

`TextColumn(character_name)` L66, `TextColumn(character_level)` L71, `TextColumn(message)` L75, `TextColumn(created_at)` L80, `DeleteAction()` L86, `DeleteBulkAction()` L90, `Action(announce)` L101, `Select(channel)` L108, `Textarea(text)` L117.

### Resources/AdminCommands/Pages/EditAdminCommand.php

[app/Filament/Resources/AdminCommands/Pages/EditAdminCommand.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/AdminCommands/Pages/EditAdminCommand.php#L1)

`DeleteAction()` L16.

### Resources/AdminCommands/Pages/ListAdminCommands.php

[app/Filament/Resources/AdminCommands/Pages/ListAdminCommands.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/AdminCommands/Pages/ListAdminCommands.php#L1)

`CreateAction()` L16.

### Resources/AdminCommands/Schemas/AdminCommandForm.php

[app/Filament/Resources/AdminCommands/Schemas/AdminCommandForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/AdminCommands/Schemas/AdminCommandForm.php#L1)

`TextInput(name)` L18, `Textarea(description)` L22, `Select(command_type)` L26, `KeyValue(params)` L32, `Toggle(active)` L39.

### Resources/AdminCommands/Tables/AdminCommandsTable.php

[app/Filament/Resources/AdminCommands/Tables/AdminCommandsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/AdminCommands/Tables/AdminCommandsTable.php#L1)

`TextColumn(name)` L25, `TextColumn(description)` L28, `TextColumn(command_type)` L31, `TextColumn(params)` L64, `IconColumn(active)` L70, `TextColumn(updated_at)` L73, `Action(run)` L80, `Select(character_id)` L85, `EditAction()` L105, `DeleteAction()` L106, `DeleteBulkAction()` L111.

### Resources/Characters/Pages/EditCharacter.php

[app/Filament/Resources/Characters/Pages/EditCharacter.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/Pages/EditCharacter.php#L1)

`DeleteAction()` L21.

### Resources/Characters/Pages/ListCharacters.php

[app/Filament/Resources/Characters/Pages/ListCharacters.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/Pages/ListCharacters.php#L1)

`CreateAction()` L16.

### Resources/Characters/RelationManagers/AccessoriesRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/AccessoriesRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/AccessoriesRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L47, `TextColumn(item_id)` L56, `TextColumn(item.name)` L57, `TextColumn(quantity)` L58, `CreateAction()` L60, `EditAction()` L61, `DeleteAction()` L61, `DeleteBulkAction()` L62.

### Resources/Characters/RelationManagers/BackItemsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/BackItemsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/BackItemsRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L47, `TextColumn(item_id)` L56, `TextColumn(item.name)` L57, `TextColumn(quantity)` L58, `CreateAction()` L60, `EditAction()` L61, `DeleteAction()` L61, `DeleteBulkAction()` L62.

### Resources/Characters/RelationManagers/ClothingRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/ClothingRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/ClothingRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L47, `TextColumn(item_id)` L56, `TextColumn(item.name)` L57, `TextColumn(quantity)` L58, `CreateAction()` L60, `EditAction()` L61, `DeleteAction()` L61, `DeleteBulkAction()` L62.

### Resources/Characters/RelationManagers/ConsumablesRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/ConsumablesRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/ConsumablesRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L36, `TextColumn(item_id)` L45, `TextColumn(item.name)` L46, `TextColumn(quantity)` L47, `CreateAction()` L49, `EditAction()` L50, `DeleteAction()` L50, `DeleteBulkAction()` L51.

### Resources/Characters/RelationManagers/EssentialsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/EssentialsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/EssentialsRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L36, `TextColumn(item_id)` L45, `TextColumn(item.name)` L46, `TextColumn(quantity)` L47, `CreateAction()` L49, `EditAction()` L50, `DeleteAction()` L50, `DeleteBulkAction()` L51.

### Resources/Characters/RelationManagers/HairstylesRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/HairstylesRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/HairstylesRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L47, `TextColumn(item_id)` L56, `TextColumn(item.name)` L57, `TextColumn(quantity)` L58, `CreateAction()` L60, `EditAction()` L61, `DeleteAction()` L61, `DeleteBulkAction()` L62.

### Resources/Characters/RelationManagers/MaterialsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/MaterialsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/MaterialsRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L36, `TextColumn(item_id)` L45, `TextColumn(item.name)` L46, `TextColumn(quantity)` L47, `CreateAction()` L49, `EditAction()` L50, `DeleteAction()` L50, `DeleteBulkAction()` L51.

### Resources/Characters/RelationManagers/PetsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/PetsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/PetsRelationManager.php#L1)

`Select(pet_id)` L26, `TextInput(name)` L31, `TextInput(level)` L34, `TextInput(xp)` L38, `TextColumn(pet_id)` L49, `TextColumn(pet.name)` L52, `TextColumn(name)` L55, `TextColumn(level)` L58, `TextColumn(xp)` L61, `CreateAction()` L69, `EditAction()` L72, `DeleteAction()` L73, `DeleteBulkAction()` L77.

### Resources/Characters/RelationManagers/SkillsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/SkillsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/SkillsRelationManager.php#L1)

`Select(skill_id)` L26, `TextColumn(skill_id)` L38, `TextColumn(skill.name)` L41, `CreateAction()` L49, `EditAction()` L52, `DeleteAction()` L53, `DeleteBulkAction()` L57.

### Resources/Characters/RelationManagers/WeaponsRelationManager.php

[app/Filament/Resources/Characters/RelationManagers/WeaponsRelationManager.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/RelationManagers/WeaponsRelationManager.php#L1)

`Hidden(category)` L30, `Select(item_id)` L31, `TextInput(quantity)` L47, `TextColumn(item_id)` L56, `TextColumn(item.name)` L57, `TextColumn(quantity)` L58, `CreateAction()` L60, `EditAction()` L61, `DeleteAction()` L61, `DeleteBulkAction()` L62.

### Resources/Characters/Schemas/CharacterForm.php

[app/Filament/Resources/Characters/Schemas/CharacterForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/Schemas/CharacterForm.php#L1)

`SchemaTabs(Character Management)` L28, `Tab(Profile)` L30, `Section(Basic Information)` L33, `Select(user_id)` L35, `TextInput(name)` L39, `Select(gender)` L41, `Select(rank)` L45, `TextInput(level)` L56, `TextInput(xp)` L57, `TextInput(gold)` L58, `TextInput(prestige)` L59, `Tab(Attributes & Talents)` L63, `Section(Elements)` L66, `Select(element_1)` L68, `Select(element_2)` L69, `Select(element_3)` L70, `Section(Stats)` L72, `TextInput(tp)` L74, `TextInput(ss)` L75, `TextInput(talent_1)` L76, `TextInput(talent_2)` L77, `TextInput(talent_3)` L78, `TextInput(point_wind)` L79, `TextInput(point_fire)` L80, `TextInput(point_lightning)` L81, `TextInput(point_water)` L82, `TextInput(point_earth)` L83, `TextInput(point_free)` L84, `Tab(Style & Equipment)` L88, `TextInput(hair_style)` L91, `TextInput(hair_color)` L92, `TextInput(skin_color)` L93, `TextInput(equipment_weapon)` L94, `TextInput(equipment_back)` L95, `TextInput(equipment_clothing)` L96, `TextInput(equipment_accessory)` L97, `TextInput(equipment_pet)` L98, `Textarea(equipment_skills)` L99, `Tab(Game Progress)` L102, `TextInput(chunin_exam_progress)` L105, `Toggle(chunin_claimed)` L106, `TextInput(jounin_exam_progress)` L107, `Toggle(jounin_claimed)` L108, `TextInput(special_jounin_exam_progress)` L109, `Toggle(special_jounin_claimed)` L110, `TextInput(ninja_tutor_exam_progress)` L111, `Toggle(ninja_tutor_claimed)` L112, `TextInput(hunting_house_tries)` L113, `TextInput(hunting_house_date)` L114, `TextInput(eudemon_garden_tries)` L115, `TextInput(eudemon_garden_date)` L116, `Textarea(claimed_welcome_rewards)` L117.

### Resources/Characters/Tables/CharactersTable.php

[app/Filament/Resources/Characters/Tables/CharactersTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Characters/Tables/CharactersTable.php#L1)

`TextColumn(id)` L20, `TextColumn(user.username)` L21, `TextColumn(name)` L25, `TextColumn(level)` L28, `TextColumn(rank)` L31, `TextColumn(gold)` L42, `TextColumn(created_at)` L45, `EditAction()` L54, `Action(inventory)` L55, `Action(skills)` L60, `Action(pets)` L65, `DeleteBulkAction()` L76.

### Resources/ClanSeasons/ClanSeasonResource.php

[app/Filament/Resources/ClanSeasons/ClanSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ClanSeasons/ClanSeasonResource.php#L1)

`Section(Season Info)` L33, `TextInput(number)` L36, `Toggle(active)` L42, `Section(Duration)` L48, `DateTimePicker(started_at)` L51, `DateTimePicker(ended_at)` L56, `TextColumn(number)` L70, `IconColumn(active)` L74, `TextColumn(started_at)` L78, `TextColumn(ended_at)` L83, `TextColumn(duration)` L89, `TextColumn(time_left)` L99, `EditAction()` L118, `Action(end_and_advance)` L119, `TextInput(duration_days)` L127.

### Resources/ClanSeasons/Pages/EditClanSeason.php

[app/Filament/Resources/ClanSeasons/Pages/EditClanSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ClanSeasons/Pages/EditClanSeason.php#L1)

`DeleteAction()` L16.

### Resources/ClanSeasons/Pages/ListClanSeasons.php

[app/Filament/Resources/ClanSeasons/Pages/ListClanSeasons.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ClanSeasons/Pages/ListClanSeasons.php#L1)

`Action(start_season)` L20, `TextInput(duration_days)` L27, `CreateAction()` L57.

### Resources/CrewSeasons/CrewSeasonResource.php

[app/Filament/Resources/CrewSeasons/CrewSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/CrewSeasons/CrewSeasonResource.php#L1)

`Section(Season Info)` L33, `TextInput(number)` L36, `Toggle(active)` L42, `Select(phase)` L47, `Section(Duration)` L57, `DateTimePicker(started_at)` L60, `DateTimePicker(ended_at)` L65, `TextColumn(number)` L79, `IconColumn(active)` L83, `TextColumn(phase)` L87, `TextColumn(started_at)` L97, `TextColumn(ended_at)` L102, `TextColumn(time_left)` L108, `EditAction()` L127, `Action(advance_phase)` L129, `Action(end_and_advance)` L146, `TextInput(duration_days)` L154.

### Resources/CrewSeasons/Pages/EditCrewSeason.php

[app/Filament/Resources/CrewSeasons/Pages/EditCrewSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/CrewSeasons/Pages/EditCrewSeason.php#L1)

`DeleteAction()` L16.

### Resources/CrewSeasons/Pages/ListCrewSeasons.php

[app/Filament/Resources/CrewSeasons/Pages/ListCrewSeasons.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/CrewSeasons/Pages/ListCrewSeasons.php#L1)

`Action(start_season)` L20, `TextInput(duration_days)` L27, `CreateAction()` L57.

### Resources/GameEvents/EventAnalyticsResource.php

[app/Filament/Resources/GameEvents/EventAnalyticsResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/EventAnalyticsResource.php#L1)

`TextColumn(event_key)` L57, `TextColumn(player_count)` L63, `TextColumn(total_battles)` L69, `TextColumn(avg_battles)` L75, `TextColumn(total_milestones_claimed)` L80, `TextColumn(bought_count)` L86, `TextColumn(total_energy_remaining)` L92, `Action(drill_down)` L101.

### Resources/GameEvents/GameEventResource.php

[app/Filament/Resources/GameEvents/GameEventResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/GameEventResource.php#L1)

`Section(Basic Info)` L31, `TextInput(title)` L34, `Select(type)` L39, `Select(panel)` L49, `TextInput(icon)` L57, `TextInput(date)` L63, `TextInput(sort_order)` L69, `Toggle(active)` L75, `DateTimePicker(starts_at)` L80, `DateTimePicker(ends_at)` L87, `Toggle(inside)` L95, `Section(Seasonal Event Details)` L101, `TextInput(image_url)` L104, `Textarea(description)` L109, `Section(Monster Hunter Settings)` L114, `TextInput(mh_boss_id)` L117, `Section(Gacha Pool Editor)` L124, `Section(Pool Weights)` L128, `TextInput(gacha_weight_top)` L132, `TextInput(gacha_weight_mid)` L140, `TextInput(gacha_weight_common)` L148, `Repeater(gacha_draws)` L157, `TextInput(draw_key)` L164, `TextInput(qty)` L169, `TextInput(coin_cost)` L174, `TextInput(token_cost)` L179, `Section(Pool Items)` L185, `Textarea(gacha_pool_top)` L189, `Textarea(gacha_pool_mid)` L194, `Textarea(gacha_pool_common)` L199, `Section(Event Configuration (JSON))` L206, `Textarea(data)` L210, `TextColumn(sort_order)` L249, `TextColumn(title)` L253, `TextColumn(type)` L256, `TextColumn(panel)` L266, `IconColumn(active)` L270, `TextColumn(starts_at)` L273, `TextColumn(ends_at)` L280, `TextColumn(updated_at)` L287, `SelectFilter(type)` L294, `TernaryFilter(active)` L301, `Action(toggleActive)` L307, `EditAction()` L313, `DeleteAction()` L314, `DeleteBulkAction()` L318.

### Resources/GameEvents/Pages/EditGameEvent.php

[app/Filament/Resources/GameEvents/Pages/EditGameEvent.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/Pages/EditGameEvent.php#L1)

`DeleteAction()` L16.

### Resources/GameEvents/Pages/ListEventPlayers.php

[app/Filament/Resources/GameEvents/Pages/ListEventPlayers.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/Pages/ListEventPlayers.php#L1)

`Action(back)` L37, `TextColumn(character_id)` L52, `TextColumn(event_key)` L57, `TextColumn(battles)` L62, `TextColumn(energy)` L68, `IconColumn(bought)` L74, `TextColumn(milestones_claimed)` L78, `TextColumn(updated_at)` L83, `SelectFilter(event_key)` L89.

### Resources/GameEvents/Pages/ListGameEvents.php

[app/Filament/Resources/GameEvents/Pages/ListGameEvents.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/Pages/ListGameEvents.php#L1)

`CreateAction()` L16.

### Resources/GameEvents/Tables/GameEventsTable.php

[app/Filament/Resources/GameEvents/Tables/GameEventsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/GameEvents/Tables/GameEventsTable.php#L1)

`EditAction()` L22, `DeleteBulkAction()` L26.

### Resources/Giveaways/Pages/EditGiveaway.php

[app/Filament/Resources/Giveaways/Pages/EditGiveaway.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Giveaways/Pages/EditGiveaway.php#L1)

`DeleteAction()` L16.

### Resources/Giveaways/Pages/ListGiveaways.php

[app/Filament/Resources/Giveaways/Pages/ListGiveaways.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Giveaways/Pages/ListGiveaways.php#L1)

`CreateAction()` L16.

### Resources/Giveaways/Schemas/GiveawayForm.php

[app/Filament/Resources/Giveaways/Schemas/GiveawayForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Giveaways/Schemas/GiveawayForm.php#L1)

`Section(Giveaway Details)` L21, `TextInput(title)` L24, `Textarea(description)` L29, `DateTimePicker(ends_at)` L33, `Section(Prizes)` L39, `TagsInput(prizes)` L42, `Section(Entry Requirements)` L48, `Repeater(requirements)` L51, `TextInput(name)` L55, `Select(type)` L60, `TextInput(value)` L70, `Section(Winners)` L82, `Repeater(winners)` L85, `TextInput(id)` L89, `TextInput(name)` L94.

### Resources/Giveaways/Tables/GiveawaysTable.php

[app/Filament/Resources/Giveaways/Tables/GiveawaysTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Giveaways/Tables/GiveawaysTable.php#L1)

`TextColumn(id)` L20, `TextColumn(title)` L24, `TextColumn(ends_at)` L28, `TextColumn(participants_count)` L33, `IconColumn(is_active)` L38, `IconColumn(has_winners)` L43, `EditAction()` L50, `DeleteAction()` L51, `DeleteBulkAction()` L55.

### Resources/Items/Pages/EditItem.php

[app/Filament/Resources/Items/Pages/EditItem.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Items/Pages/EditItem.php#L1)

`DeleteAction()` L16.

### Resources/Items/Pages/ListItems.php

[app/Filament/Resources/Items/Pages/ListItems.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Items/Pages/ListItems.php#L1)

`CreateAction()` L16.

### Resources/Items/Schemas/ItemForm.php

[app/Filament/Resources/Items/Schemas/ItemForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Items/Schemas/ItemForm.php#L1)

`TextInput(item_id)` L17, `TextInput(name)` L19, `FileUpload(icon)` L21, `TextInput(level)` L24, `TextInput(price_gold)` L28, `TextInput(price_tokens)` L32, `Toggle(premium)` L36, `Select(category)` L39.

### Resources/Items/Tables/ItemsTable.php

[app/Filament/Resources/Items/Tables/ItemsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Items/Tables/ItemsTable.php#L1)

`TextColumn(item_id)` L21, `ImageColumn(icon)` L23, `TextColumn(name)` L24, `TextColumn(level)` L26, `TextColumn(price_gold)` L29, `TextColumn(price_tokens)` L32, `IconColumn(premium)` L35, `TextColumn(category)` L38, `TextColumn(created_at)` L52, `TextColumn(updated_at)` L56, `SelectFilter(category)` L62, `EditAction()` L75, `DeleteBulkAction()` L79.

### Resources/Mails/Pages/ListMails.php

[app/Filament/Resources/Mails/Pages/ListMails.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Mails/Pages/ListMails.php#L1)

`CreateAction()` L16.

### Resources/Mails/Schemas/MailForm.php

[app/Filament/Resources/Mails/Schemas/MailForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Mails/Schemas/MailForm.php#L1)

`Select(character_id)` L18, `TextInput(title)` L31, `TextInput(sender)` L36, `Textarea(body)` L40, `TagsInput(rewards)` L44, `Hidden(type)` L50, `Hidden(viewed)` L51, `Hidden(claimed)` L52.

### Resources/Mails/Tables/MailsTable.php

[app/Filament/Resources/Mails/Tables/MailsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Mails/Tables/MailsTable.php#L1)

`TextColumn(character.name)` L23, `TextColumn(title)` L27, `TextColumn(sender)` L30, `TextColumn(rewards)` L32, `IconColumn(viewed)` L47, `IconColumn(claimed)` L50, `TextColumn(created_at)` L53, `Action(claimRewards)` L59, `Action(resend)` L89, `DeleteAction()` L113, `DeleteBulkAction()` L118.

### Resources/Missions/Pages/EditMission.php

[app/Filament/Resources/Missions/Pages/EditMission.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Missions/Pages/EditMission.php#L1)

`DeleteAction()` L16.

### Resources/Missions/Pages/ListMissions.php

[app/Filament/Resources/Missions/Pages/ListMissions.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Missions/Pages/ListMissions.php#L1)

`CreateAction()` L16.

### Resources/Missions/Schemas/MissionForm.php

[app/Filament/Resources/Missions/Schemas/MissionForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Missions/Schemas/MissionForm.php#L1)

`TextInput(mission_id)` L14, `TextInput(req_lvl)` L16, `TextInput(xp)` L20, `TextInput(gold)` L24.

### Resources/Missions/Tables/MissionsTable.php

[app/Filament/Resources/Missions/Tables/MissionsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Missions/Tables/MissionsTable.php#L1)

`TextColumn(mission_id)` L17, `TextColumn(req_lvl)` L19, `TextColumn(xp)` L22, `TextColumn(gold)` L25, `TextColumn(created_at)` L28, `TextColumn(updated_at)` L32, `EditAction()` L41, `DeleteBulkAction()` L45.

### Resources/MysteriousMarkets/MysteriousMarketResource.php

[app/Filament/Resources/MysteriousMarkets/MysteriousMarketResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/MysteriousMarkets/MysteriousMarketResource.php#L1)

`Section(Market Settings)` L33, `Toggle(active)` L36, `DateTimePicker(ends_at)` L42, `TextInput(discount)` L47, `TextInput(refresh_cost)` L53, `TextInput(refresh_max)` L59, `Section(Featured Items (up to 4))` L67, `Repeater(items)` L70, `TextInput(code)` L73, `TextInput(prices.0)` L79, `TextInput(prices.1)` L85, `Section(All Packages (Full List & Refresh Pool))` L99, `Repeater(all_packages)` L102, `TextInput(code)` L105, `TextInput(prices.0)` L110, `TextInput(prices.1)` L116, `IconColumn(active)` L134, `TextColumn(ends_at)` L139, `TextColumn(discount)` L144, `TextColumn(refresh_cost)` L148, `TextColumn(refresh_max)` L152, `TextColumn(items_count)` L155, `TextColumn(all_packages_count)` L159, `TextColumn(updated_at)` L163, `TernaryFilter(active)` L170, `EditAction()` L176, `DeleteAction()` L177, `DeleteBulkAction()` L181.

### Resources/MysteriousMarkets/Pages/EditMysteriousMarket.php

[app/Filament/Resources/MysteriousMarkets/Pages/EditMysteriousMarket.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/MysteriousMarkets/Pages/EditMysteriousMarket.php#L1)

`DeleteAction()` L16.

### Resources/MysteriousMarkets/Pages/ListMysteriousMarkets.php

[app/Filament/Resources/MysteriousMarkets/Pages/ListMysteriousMarkets.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/MysteriousMarkets/Pages/ListMysteriousMarkets.php#L1)

`CreateAction()` L16.

### Resources/Pets/Pages/EditPet.php

[app/Filament/Resources/Pets/Pages/EditPet.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Pets/Pages/EditPet.php#L1)

`DeleteAction()` L16.

### Resources/Pets/Pages/ListPets.php

[app/Filament/Resources/Pets/Pages/ListPets.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Pets/Pages/ListPets.php#L1)

`CreateAction()` L16.

### Resources/Pets/Schemas/PetForm.php

[app/Filament/Resources/Pets/Schemas/PetForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Pets/Schemas/PetForm.php#L1)

`TextInput(pet_id)` L16, `TextInput(name)` L18, `FileUpload(icon)` L20, `TextInput(swf)` L23, `TextInput(price_gold)` L25, `TextInput(price_tokens)` L29, `Toggle(premium)` L33.

### Resources/Pets/Tables/PetsTable.php

[app/Filament/Resources/Pets/Tables/PetsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Pets/Tables/PetsTable.php#L1)

`TextColumn(pet_id)` L19, `ImageColumn(icon)` L21, `TextColumn(name)` L22, `TextColumn(swf)` L24, `TextColumn(price_gold)` L26, `TextColumn(price_tokens)` L29, `IconColumn(premium)` L32, `TextColumn(created_at)` L35, `TextColumn(updated_at)` L39, `EditAction()` L48, `DeleteBulkAction()` L52.

### Resources/ShadowWarSeasons/Pages/EditShadowWarSeason.php

[app/Filament/Resources/ShadowWarSeasons/Pages/EditShadowWarSeason.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ShadowWarSeasons/Pages/EditShadowWarSeason.php#L1)

`DeleteAction()` L16.

### Resources/ShadowWarSeasons/Pages/ListShadowWarSeasons.php

[app/Filament/Resources/ShadowWarSeasons/Pages/ListShadowWarSeasons.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ShadowWarSeasons/Pages/ListShadowWarSeasons.php#L1)

`Action(start_season)` L20, `TextInput(duration_days)` L27, `CreateAction()` L56.

### Resources/ShadowWarSeasons/ShadowWarSeasonResource.php

[app/Filament/Resources/ShadowWarSeasons/ShadowWarSeasonResource.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/ShadowWarSeasons/ShadowWarSeasonResource.php#L1)

`Section(Season Info)` L33, `TextInput(num)` L36, `Toggle(active)` L42, `Section(Duration)` L48, `DateTimePicker(start_at)` L51, `DateTimePicker(end_at)` L56, `TextColumn(num)` L70, `IconColumn(active)` L74, `TextColumn(start_at)` L78, `TextColumn(end_at)` L83, `TextColumn(duration)` L89, `TextColumn(time_left)` L98, `EditAction()` L117, `Action(end_and_advance)` L118, `TextInput(duration_days)` L126.

### Resources/Skills/Pages/EditSkill.php

[app/Filament/Resources/Skills/Pages/EditSkill.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Skills/Pages/EditSkill.php#L1)

`DeleteAction()` L16.

### Resources/Skills/Pages/ListSkills.php

[app/Filament/Resources/Skills/Pages/ListSkills.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Skills/Pages/ListSkills.php#L1)

`CreateAction()` L16.

### Resources/Skills/Schemas/SkillForm.php

[app/Filament/Resources/Skills/Schemas/SkillForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Skills/Schemas/SkillForm.php#L1)

`TextInput(skill_id)` L17, `TextInput(name)` L19, `FileUpload(icon)` L21, `TextInput(level)` L24, `Select(element)` L28, `TextInput(swf)` L39, `Toggle(premium)` L43, `TextInput(price_gold)` L45, `TextInput(price_tokens)` L49.

### Resources/Skills/Tables/SkillsTable.php

[app/Filament/Resources/Skills/Tables/SkillsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Skills/Tables/SkillsTable.php#L1)

`TextColumn(skill_id)` L19, `ImageColumn(icon)` L21, `TextColumn(name)` L22, `TextColumn(level)` L24, `TextColumn(element)` L27, `IconColumn(premium)` L46, `TextColumn(price_gold)` L48, `TextColumn(price_tokens)` L51, `TextColumn(created_at)` L54, `TextColumn(updated_at)` L58, `EditAction()` L67, `DeleteBulkAction()` L71.

### Resources/Users/Pages/EditUser.php

[app/Filament/Resources/Users/Pages/EditUser.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Users/Pages/EditUser.php#L1)

`DeleteAction()` L16.

### Resources/Users/Pages/ListUsers.php

[app/Filament/Resources/Users/Pages/ListUsers.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Users/Pages/ListUsers.php#L1)

`CreateAction()` L16.

### Resources/Users/Schemas/UserForm.php

[app/Filament/Resources/Users/Schemas/UserForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Users/Schemas/UserForm.php#L1)

`TextInput(username)` L16, `TextInput(name)` L18, `TextInput(email)` L20, `Select(account_type)` L24, `TextInput(tokens)` L31, `DateTimePicker(email_verified_at)` L35, `TextInput(password)` L36.

### Resources/Users/Tables/UsersTable.php

[app/Filament/Resources/Users/Tables/UsersTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/Users/Tables/UsersTable.php#L1)

`TextColumn(username)` L17, `TextColumn(name)` L19, `TextColumn(email)` L21, `TextColumn(account_type)` L24, `TextColumn(tokens)` L35, `TextColumn(email_verified_at)` L38, `TextColumn(created_at)` L41, `TextColumn(updated_at)` L45, `EditAction()` L54, `DeleteBulkAction()` L58.

### Resources/XPLevels/Schemas/XPLevelForm.php

[app/Filament/Resources/XPLevels/Schemas/XPLevelForm.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/XPLevels/Schemas/XPLevelForm.php#L1)

`TextInput(level)` L14, `TextInput(character_xp)` L18, `TextInput(pet_xp)` L24.

### Resources/XPLevels/Tables/XPLevelsTable.php

[app/Filament/Resources/XPLevels/Tables/XPLevelsTable.php:1](../../reference/upstream/Database/Laragon/ninjasage/app/Filament/Resources/XPLevels/Tables/XPLevelsTable.php#L1)

`TextColumn(level)` L15, `TextColumn(character_xp)` L19, `TextColumn(pet_xp)` L23, `TextColumn(updated_at)` L27, `EditAction()` L36.
