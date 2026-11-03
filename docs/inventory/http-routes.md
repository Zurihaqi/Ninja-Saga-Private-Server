# HTTP route inventory

Source: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Evidence: static source inspection only; no migrated database or runtime route list has been verified.

Regenerate with `python scripts/schema_inventory.py`. Paths and line numbers refer to the pinned reference. The extractor is lexical: conditional branches, framework defaults and dynamic names require runtime verification.

## Registration and middleware

`bootstrap/app.php` registers `routes/web.php`, `routes/console.php`, and health `/up`. It does not register `routes/clan.php`; that file is an alternate/unwired definition, not an additional active route set. Filament dynamically registers `/admin` routes (see admin inventory).

CSRF exceptions cover `amf`, `gateway.php`, `clan/*`, `crew/*`. The root-level crew aliases are registered through the web middleware but are not exempted: POST requests without CSRF tokens may fail there even when their `/crew/*` equivalents succeed. This is source-derived and needs a runtime regression fixture.

Clan protected routes use `ClanAuthMiddleware`; crew protected routes use `CrewAuthMiddleware`. Public login/season routes still inherit web middleware. Every crew row below also has a root alias obtained by removing `/crew`. No hostname constraint appears on those aliases.

| Method | Registered path | Handler | Auth | Source |
|---|---|---|---|---|
| GET | `/` | closure | public | [routes/web.php:9](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L9) |
| ANY | `/amf` | AmfController::handle | public | [routes/web.php:14](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L14) |
| ANY | `/gateway.php` | AmfController::handle | public | [routes/web.php:15](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L15) |
| GET | `/enemy/{filename}` | closure | public | [routes/web.php:19](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L19) |
| GET | `/skills/{filename}` | closure | public | [routes/web.php:38](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L38) |
| POST | `/clan/auth/login` | ClanApiController::login | public | [routes/web.php:64](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L64) |
| POST | `/clan/season` | ClanApiController::season | public | [routes/web.php:65](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L65) |
| POST | `/clan/player/clan` | ClanApiController::playerClan | clan.auth | [routes/web.php:69](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L69) |
| POST | `/clan/player/stamina` | ClanApiController::playerStamina | clan.auth | [routes/web.php:70](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L70) |
| POST | `/clan/player/request/{clanId}` | ClanApiController::sendRequest | clan.auth | [routes/web.php:71](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L71) |
| POST | `/clan/player/clan-members` | ClanApiController::clanMembers | clan.auth | [routes/web.php:72](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L72) |
| POST | `/clan/player/quit` | ClanApiController::quitClan | clan.auth | [routes/web.php:73](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L73) |
| POST | `/clan/player/donate/{amount}/golds` | ClanApiController::donateGolds | clan.auth | [routes/web.php:74](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L74) |
| POST | `/clan/player/donate/{amount}/tokens` | ClanApiController::donateTokens | clan.auth | [routes/web.php:75](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L75) |
| POST | `/clan/player/stamina/upgrade-max` | ClanApiController::upgradeMaxStamina | clan.auth | [routes/web.php:76](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L76) |
| POST | `/clan/player/stamina/refill` | ClanApiController::refillStamina | clan.auth | [routes/web.php:77](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L77) |
| POST | `/clan/player/boost-prestige` | ClanApiController::boostPrestige | clan.auth | [routes/web.php:78](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L78) |
| POST | `/clan/player/buy-onigiri/{id}` | ClanApiController::buyOnigiriPackage | clan.auth | [routes/web.php:79](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L79) |
| POST | `/clan/history` | ClanApiController::history | clan.auth | [routes/web.php:81](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L81) |
| POST | `/clan/create` | ClanApiController::createClan | clan.auth | [routes/web.php:82](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L82) |
| POST | `/clan/rename` | ClanApiController::renameClan | clan.auth | [routes/web.php:83](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L83) |
| POST | `/clan/swap-master` | ClanApiController::swapMaster | clan.auth | [routes/web.php:84](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L84) |
| POST | `/clan/invite-character/{id}` | ClanApiController::inviteCharacter | clan.auth | [routes/web.php:85](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L85) |
| POST | `/clan/season-histories` | ClanApiController::seasonHistories | clan.auth | [routes/web.php:86](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L86) |
| POST | `/clan/upgrade-building/{id}` | ClanApiController::upgradeBuilding | clan.auth | [routes/web.php:87](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L87) |
| POST | `/clan/announcement/save` | ClanApiController::saveAnnouncement | clan.auth | [routes/web.php:88](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L88) |
| POST | `/clan/announcement/publish` | ClanApiController::publishAnnouncement | clan.auth | [routes/web.php:89](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L89) |
| POST | `/clan/battle/opponents` | ClanApiController::battleOpponents | clan.auth | [routes/web.php:91](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L91) |
| POST | `/clan/battle/opponents/{id}` | ClanApiController::searchBattleOpponent | clan.auth | [routes/web.php:92](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L92) |
| POST | `/clan/battle/defenders` | ClanApiController::battleDefenders | clan.auth | [routes/web.php:93](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L93) |
| POST | `/clan/battle/quick/{clanId}` | ClanApiController::quickAttack | clan.auth | [routes/web.php:94](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L94) |
| POST | `/clan/battle/manual/start/{clanId}` | ClanApiController::startManualAttack | clan.auth | [routes/web.php:95](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L95) |
| POST | `/clan/battle/manual/finish` | ClanApiController::finishManualAttack | clan.auth | [routes/web.php:96](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L96) |
| POST | `/clan/request/all/reject` | ClanApiController::rejectAllRequests | clan.auth | [routes/web.php:99](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L99) |
| POST | `/clan/request/all` | ClanApiController::memberRequests | clan.auth | [routes/web.php:100](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L100) |
| POST | `/clan/request/available` | ClanApiController::availableClans | clan.auth | [routes/web.php:101](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L101) |
| POST | `/clan/request/available/{id}` | ClanApiController::searchAvailableClan | clan.auth | [routes/web.php:102](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L102) |
| POST | `/clan/request/{id}/reject` | ClanApiController::rejectRequest | clan.auth | [routes/web.php:103](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L103) |
| POST | `/clan/request/{id}/accept` | ClanApiController::acceptRequest | clan.auth | [routes/web.php:104](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L104) |
| POST | `/clan/member/increase-max-members` | ClanApiController::increaseMaxMembers | clan.auth | [routes/web.php:106](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L106) |
| POST | `/clan/member/{id}/kick` | ClanApiController::kickMember | clan.auth | [routes/web.php:107](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L107) |
| POST | `/clan/member/{id}/promote-elder` | ClanApiController::promoteElder | clan.auth | [routes/web.php:108](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L108) |
| POST | `/clan/member/{id}/onigiri/limit` | ClanApiController::getOnigiriInfo | clan.auth | [routes/web.php:109](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L109) |
| POST | `/clan/member/{id}/onigiri/gift/{amount}` | ClanApiController::giveOnigiri | clan.auth | [routes/web.php:110](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L110) |
| POST | `/crew/season` | CrewApiController::season | public | [routes/web.php:119](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L119) |
| POST | `/crew/auth/login` | CrewApiController::login | public | [routes/web.php:120](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L120) |
| POST | `/crew/season/pool` | CrewApiController::tokenPool | crew.auth | [routes/web.php:124](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L124) |
| POST | `/crew/season/previous` | CrewApiController::previousSeason | crew.auth | [routes/web.php:125](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L125) |
| POST | `/crew/season-histories` | CrewApiController::seasonHistories | crew.auth | [routes/web.php:126](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L126) |
| POST | `/crew/player/crew` | CrewApiController::playerCrew | crew.auth | [routes/web.php:128](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L128) |
| POST | `/crew/player/crew/members` | CrewApiController::crewMembers | crew.auth | [routes/web.php:129](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L129) |
| POST | `/crew/player/stamina` | CrewApiController::playerStamina | crew.auth | [routes/web.php:130](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L130) |
| POST | `/crew/player/stamina/upgrade-max` | CrewApiController::upgradeMaxStamina | crew.auth | [routes/web.php:131](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L131) |
| POST | `/crew/player/stamina/refill` | CrewApiController::refillStamina | crew.auth | [routes/web.php:132](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L132) |
| POST | `/crew/player/boost-prestige` | CrewApiController::boostPrestige | crew.auth | [routes/web.php:133](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L133) |
| POST | `/crew/player/buy-onigiri/{id}` | CrewApiController::buyOnigiriPackage | crew.auth | [routes/web.php:134](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L134) |
| POST | `/crew/player/quit` | CrewApiController::quitCrew | crew.auth | [routes/web.php:135](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L135) |
| POST | `/crew/player/kick/{id}` | CrewApiController::kickMember | crew.auth | [routes/web.php:136](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L136) |
| POST | `/crew/player/promote-elder/{id}` | CrewApiController::promoteElder | crew.auth | [routes/web.php:137](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L137) |
| POST | `/crew/player/donate/{amount}/golds` | CrewApiController::donateGolds | crew.auth | [routes/web.php:138](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L138) |
| POST | `/crew/player/donate/{amount}/tokens` | CrewApiController::donateTokens | crew.auth | [routes/web.php:139](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L139) |
| POST | `/crew/player/switch-master/{id}` | CrewApiController::switchMaster | crew.auth | [routes/web.php:140](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L140) |
| POST | `/crew/player/minigame` | CrewApiController::miniGame | crew.auth | [routes/web.php:141](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L141) |
| POST | `/crew/player/minigame/start` | CrewApiController::startMiniGame | crew.auth | [routes/web.php:142](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L142) |
| POST | `/crew/player/minigame/finish` | CrewApiController::finishMiniGame | crew.auth | [routes/web.php:143](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L143) |
| POST | `/crew/player/minigame/buy/{type}` | CrewApiController::buyMiniGame | crew.auth | [routes/web.php:144](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L144) |
| POST | `/crew/player/request/{crewId}` | CrewApiController::sendRequest | crew.auth | [routes/web.php:145](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L145) |
| POST | `/crew/history` | CrewApiController::history | crew.auth | [routes/web.php:147](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L147) |
| POST | `/crew/create` | CrewApiController::createCrew | crew.auth | [routes/web.php:148](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L148) |
| POST | `/crew/rename` | CrewApiController::renameCrew | crew.auth | [routes/web.php:149](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L149) |
| POST | `/crew/upgrade/building/{id}` | CrewApiController::upgradeBuilding | crew.auth | [routes/web.php:151](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L151) |
| POST | `/crew/upgrade/max-members` | CrewApiController::increaseMaxMembers | crew.auth | [routes/web.php:152](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L152) |
| POST | `/crew/announcements` | CrewApiController::saveAnnouncement | crew.auth | [routes/web.php:154](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L154) |
| POST | `/crew/announcement/publish` | CrewApiController::publishAnnouncement | crew.auth | [routes/web.php:155](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L155) |
| POST | `/crew/request/all/reject` | CrewApiController::rejectAllRequests | crew.auth | [routes/web.php:158](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L158) |
| POST | `/crew/request/all` | CrewApiController::memberRequests | crew.auth | [routes/web.php:159](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L159) |
| POST | `/crew/request/available` | CrewApiController::availableCrews | crew.auth | [routes/web.php:160](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L160) |
| POST | `/crew/request/available/{id}` | CrewApiController::searchAvailableCrew | crew.auth | [routes/web.php:161](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L161) |
| POST | `/crew/request/{id}/reject` | CrewApiController::rejectRequest | crew.auth | [routes/web.php:162](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L162) |
| POST | `/crew/request/{id}/accept` | CrewApiController::acceptRequest | crew.auth | [routes/web.php:163](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L163) |
| POST | `/crew/request/{id}/invite` | CrewApiController::inviteCharacter | crew.auth | [routes/web.php:164](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L164) |
| POST | `/crew/member/{id}/onigiri/limit` | CrewApiController::getOnigiriInfo | crew.auth | [routes/web.php:166](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L166) |
| POST | `/crew/member/{id}/onigiri/gift/{amount}` | CrewApiController::giveOnigiri | crew.auth | [routes/web.php:167](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L167) |
| POST | `/crew/battle/castles/{id}/ranks` | CrewApiController::castleRanks | crew.auth | [routes/web.php:170](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L170) |
| POST | `/crew/battle/castles/{id}/recovery` | CrewApiController::castleRecovery | crew.auth | [routes/web.php:171](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L171) |
| POST | `/crew/battle/castles/{id}/recover` | CrewApiController::castleRecover | crew.auth | [routes/web.php:172](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L172) |
| POST | `/crew/battle/castles/{id}/defenders` | CrewApiController::castleDefenders | crew.auth | [routes/web.php:173](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L173) |
| POST | `/crew/battle/castles/{id}` | CrewApiController::getCastle | crew.auth | [routes/web.php:174](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L174) |
| POST | `/crew/battle/castles` | CrewApiController::getCastles | crew.auth | [routes/web.php:175](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L175) |
| POST | `/crew/battle/role/switch/{id}` | CrewApiController::switchRole | crew.auth | [routes/web.php:176](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L176) |
| POST | `/crew/battle/attackers` | CrewApiController::battleAttackers | crew.auth | [routes/web.php:177](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L177) |
| POST | `/crew/battle/opponents` | CrewApiController::battleOpponents | crew.auth | [routes/web.php:178](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L178) |
| POST | `/crew/battle/opponents/{id}` | CrewApiController::searchBattleOpponent | crew.auth | [routes/web.php:179](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L179) |
| POST | `/crew/battle/phase1/start` | CrewApiController::startPhaseOneBattle | crew.auth | [routes/web.php:180](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L180) |
| POST | `/crew/battle/phase1/finish` | CrewApiController::finishPhaseOneBattle | crew.auth | [routes/web.php:181](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L181) |
| POST | `/crew/battle/phase2/start` | CrewApiController::startPhaseTwoBattle | crew.auth | [routes/web.php:182](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L182) |
| POST | `/crew/battle/phase2/finish` | CrewApiController::finishPhaseTwoBattle | crew.auth | [routes/web.php:183](../../reference/upstream/Database/Laragon/ninjasage/routes/web.php#L183) |
| GET | `/up` | Laravel health | public | [bootstrap/app.php:11](../../reference/upstream/Database/Laragon/ninjasage/bootstrap/app.php#L11) |

## Static asset behavior

`/enemy/{filename}` accepts an alphanumeric/underscore/hyphen `.swf` filename and returns 404 for missing base directories/files. `/skills/{filename}` first checks the exact file, then a database `skills.swf` alias or `SKILL_DEFAULT_SWF` (default `skill_01.swf`), then 404. Both return `application/x-shockwave-flash`. Base directories are controlled by `ENEMY_SWF_PATH` / `SKILL_SWF_PATH`. Static web-server/public assets are not exhaustively represented by application routes.
