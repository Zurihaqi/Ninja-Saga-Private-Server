//! Ninja Saga realtime PvP Server (`/pvp` Socket.IO namespace, matchmaking queue,
//! custom rooms, turn-based combat engine, spectator mode, and MySQL persistence).

use axum::{
    Router,
    extract::{DefaultBodyLimit, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use persistence::{MySqlPool, Row, sqlx};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use socket_protocol::{
    EngineKind, EnginePacket, Limits, SocketPacket, decode_payload, decode_payload_v4,
    decode_socket_lenient, encode_payload, encode_payload_v4, encode_socket,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Notify;

const MAX_BODY: usize = 64 * 1024;
const MAX_PACKET: usize = 32 * 1024;
const MAX_QUEUE: usize = 128;
const MAX_SESSIONS: usize = 1024;

pub const CROSS_DOMAIN_POLICY: &str = concat!(
    "<?xml version=\"1.0\"?>",
    "<!DOCTYPE cross-domain-policy SYSTEM \"http://www.macromedia.com/xml/dtds/cross-domain-policy.dtd\">",
    "<cross-domain-policy>",
    "<site-control permitted-cross-domain-policies=\"all\"/>",
    "<allow-access-from domain=\"*\" to-ports=\"*\" secure=\"false\"/>",
    "<allow-http-request-headers-from domain=\"*\" headers=\"*\"/>",
    "</cross-domain-policy>"
);

#[derive(Clone, Debug)]
pub struct PvpConfig {
    pub turn_seconds: u64,
    pub hp_per_level: i64,
    pub hp_base: i64,
    pub cp_per_level: i64,
    pub cp_base: i64,
    pub dodge_base: f64,
    pub dodge_cap: f64,
    pub purify_base: f64,
    pub crit_base: f64,
    pub accuracy_base: f64,
    pub agility_base: i64,
    pub trophy_k_factor: i32,
    pub points_on_win: i32,
    pub points_on_loss: i32,
    pub prestige_on_win: i32,
    pub weapon_damage_base: i64,
    pub weapon_damage_per_lvl: i64,
    pub map_ids: Vec<String>,
    pub max_lobby_messages: usize,
    pub session_ttl: Duration,
    pub poll_wait: Duration,
    pub capacity: usize,
}

impl Default for PvpConfig {
    fn default() -> Self {
        Self {
            turn_seconds: 20,
            hp_per_level: 40,
            hp_base: 60,
            cp_per_level: 40,
            cp_base: 60,
            dodge_base: 5.0,
            dodge_cap: 60.0,
            purify_base: 5.0,
            crit_base: 5.0,
            accuracy_base: 0.0,
            agility_base: 10,
            trophy_k_factor: 32,
            points_on_win: 100,
            points_on_loss: 25,
            prestige_on_win: 10,
            weapon_damage_base: 30,
            weapon_damage_per_lvl: 2,
            map_ids: vec![
                "map_01".to_owned(),
                "map_02".to_owned(),
                "map_03".to_owned(),
                "map_04".to_owned(),
                "map_05".to_owned(),
            ],
            max_lobby_messages: 50,
            session_ttl: Duration::from_secs(60),
            poll_wait: Duration::from_secs(20),
            capacity: MAX_SESSIONS,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub damage: i64,
    pub cp_cost: i64,
    pub cooldown: u32,
    pub level: i32,
    pub effects: Vec<SkillEffectDef>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillEffectDef {
    pub target: String,
    pub r#type: String,
    pub effect: String,
    pub effect_name: String,
    pub duration: i32,
    pub calc_type: String,
    pub amount: f64,
    pub chance: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatStats {
    pub max_hp: i64,
    pub hp: i64,
    pub max_cp: i64,
    pub cp: i64,
    pub agility: i64,
    pub dodge: f64,
    pub critical: f64,
    pub accuracy: f64,
    pub purify: f64,
    pub weapon_damage: i64,
    pub fire_bonus: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub effect: String,
    pub effect_name: String,
    pub r#type: String,
    pub calc_type: String,
    pub amount: f64,
    pub remaining: i32,
}

#[derive(Clone, Debug)]
pub struct PvpCharacter {
    pub id: u64,
    pub user_id: u64,
    pub name: String,
    pub level: i32,
    pub rank: i32,
    pub gender: i8,
    pub xp: i64,
    pub gold: i32,
    pub prestige: i32,
    pub pvp_played: i32,
    pub pvp_won: i32,
    pub pvp_lost: i32,
    pub pvp_points: i32,
    pub pvp_trophy: i32,
    pub point_wind: i64,
    pub point_fire: i64,
    pub point_lightning: i64,
    pub point_water: i64,
    pub point_earth: i64,
    pub point_free: i64,
    pub skills: Vec<String>,
    pub sets: JsonValue,
    pub elements: Vec<i32>,
    pub senjutsu_type: Option<String>,
}

impl PvpCharacter {
    pub fn to_info_json(&self) -> JsonValue {
        json!({
            "id": self.id,
            "user_id": self.user_id,
            "name": self.name,
            "level": self.level,
            "rank": self.rank,
            "gender": self.gender,
            "xp": self.xp,
            "gold": self.gold,
            "prestige": self.prestige,
            "pvp": {
                "played": self.pvp_played,
                "won": self.pvp_won,
                "lost": self.pvp_lost,
                "points": self.pvp_points,
                "trophy": self.pvp_trophy,
            },
            "points": {
                "wind": self.point_wind,
                "fire": self.point_fire,
                "lightning": self.point_lightning,
                "water": self.point_water,
                "earth": self.point_earth,
                "free": self.point_free,
            },
            "sets": self.sets,
            "skills": self.skills,
            "elements": self.elements,
            "senjutsu_type": self.senjutsu_type,
        })
    }

    pub fn build_stats(&self, cfg: &PvpConfig) -> CombatStats {
        let lvl = i64::from(self.level.max(1));
        let wind = self.point_wind.max(0);
        let fire = self.point_fire.max(0);
        let lightning = self.point_lightning.max(0);
        let water = self.point_water.max(0);
        let earth = self.point_earth.max(0);

        let max_hp = cfg.hp_base + lvl * cfg.hp_per_level + earth * 30;
        let max_cp = cfg.cp_base + lvl * cfg.cp_per_level + water * 30;
        let agility = cfg.agility_base + lvl + wind;
        let dodge = (cfg.dodge_base + (wind as f64) * 0.4).min(cfg.dodge_cap);
        let critical = cfg.crit_base + (lightning as f64) * 0.4;
        let accuracy = cfg.accuracy_base + (lightning as f64) * 0.2;
        let purify = cfg.purify_base + (water as f64) * 0.3;
        let fire_bonus = 1.0 + (fire as f64) * 0.004;
        let weapon_damage = (((cfg.weapon_damage_base + lvl * cfg.weapon_damage_per_lvl) as f64)
            * fire_bonus)
            .round() as i64;

        CombatStats {
            max_hp,
            hp: max_hp,
            max_cp,
            cp: max_cp,
            agility,
            dodge,
            critical,
            accuracy,
            purify,
            weapon_damage,
            fire_bonus,
        }
    }
}

pub fn calc_trophy_delta(winner_trophies: i32, loser_trophies: i32, k_factor: i32) -> (i32, i32) {
    let expected = 1.0 / (1.0 + 10.0_f64.powf(f64::from(loser_trophies - winner_trophies) / 400.0));
    let gain = ((f64::from(k_factor)) * (1.0 - expected)).round().max(1.0) as i32;
    let loss = ((f64::from(gain)) * 0.8).round().max(1.0) as i32;
    (gain, loss)
}

pub fn apply_trophies(current: i32, delta: i32) -> i32 {
    (current + delta).max(0)
}

fn rand_f64_100() -> f64 {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    let val = u32::from_le_bytes(b);
    (f64::from(val) / f64::from(u32::MAX)) * 100.0
}

#[derive(Clone, Debug)]
struct FighterState {
    sid: String,
    character: PvpCharacter,
    stats: CombatStats,
    dodging: bool,
    cooldowns: HashMap<String, u32>,
    effects: Vec<ActiveEffect>,
}

impl FighterState {
    fn snapshot(&self) -> JsonValue {
        json!({
            "id": self.character.id,
            "name": self.character.name,
            "level": self.character.level,
            "rank": self.character.rank,
            "sets": self.character.sets,
            "skills": self.character.skills,
            "hp": self.stats.hp,
            "max_hp": self.stats.max_hp,
            "cp": self.stats.cp,
            "max_cp": self.stats.max_cp,
            "agility": self.stats.agility,
            "trophies": self.character.pvp_trophy,
            "effects": self.effects,
        })
    }
}

#[derive(Clone, Debug)]
struct RoomState {
    id: u64,
    host_sid: String,
    host: PvpCharacter,
    challenger: Option<(String, PvpCharacter)>,
    challenger_ready: bool,
    password: Option<String>,
    mode: String,
    map_id: String,
    allow_preset: bool,
}

impl RoomState {
    fn public_info(&self) -> JsonValue {
        json!({
            "room_id": self.id,
            "host_id": self.host.id,
            "host_name": self.host.name,
            "host_level": self.host.level,
            "host_rank": self.host.rank,
            "host_trophy": self.host.pvp_trophy,
            "mode": self.mode,
            "map_id": self.map_id,
            "allow_preset": self.allow_preset,
            "locked": self.password.is_some(),
            "full": self.challenger.is_some(),
            "challenger": self.challenger.as_ref().map(|(_, c)| json!({
                "id": c.id,
                "name": c.name,
                "level": c.level,
                "rank": c.rank,
            })),
        })
    }
}

#[derive(Clone, Debug)]
struct BattleState {
    id: String,
    mode: String,
    map_id: String,
    host: FighterState,
    enemy: FighterState,
    spectators: HashSet<String>,
    turn: String, // "host" or "enemy"
    round: u32,
    action_log: Vec<JsonValue>,
}

struct Session {
    eio: u8,
    last_ping: Instant,
    polling: bool,
    queue: VecDeque<EnginePacket>,
    notify: Arc<Notify>,
    pvp_connected: bool,
    character: Option<PvpCharacter>,
    room_id: Option<u64>,
    battle_id: Option<String>,
    spectating_id: Option<String>,
}

#[derive(Default)]
struct PvpState {
    sessions: HashMap<String, Session>,
    lobby_messages: VecDeque<JsonValue>,
    matchmaking_queue: VecDeque<(String, String)>, // (sid, mode)
    rooms: HashMap<u64, RoomState>,
    next_room_id: u64,
    battles: HashMap<String, BattleState>,
    next_battle_seq: u64,
    mock_characters: HashMap<u64, (String, PvpCharacter)>,
}

#[derive(Clone)]
pub struct PvpServer {
    inner: Arc<PvpInner>,
}

struct PvpInner {
    pool: Option<MySqlPool>,
    config: PvpConfig,
    skills: HashMap<String, SkillDef>,
    started_at: Instant,
    state: Mutex<PvpState>,
}

#[derive(Deserialize)]
struct PollParams {
    #[serde(rename = "EIO")]
    eio: Option<String>,
    transport: Option<String>,
    sid: Option<String>,
}

fn limits() -> Limits {
    Limits {
        max_payload_bytes: MAX_BODY,
        max_packet_bytes: MAX_PACKET,
        max_packets: MAX_QUEUE,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn engine_packet(kind: EngineKind, data: impl Into<String>) -> EnginePacket {
    EnginePacket {
        kind,
        data: data.into(),
    }
}

fn pvp_event_packet(event: &str, payload: JsonValue) -> Option<EnginePacket> {
    let pkt = SocketPacket::Event {
        namespace: "/pvp".to_owned(),
        name: event.to_owned(),
        args: vec![payload],
    };
    encode_socket(&pkt, limits())
        .ok()
        .map(|encoded| engine_packet(EngineKind::Message, encoded))
}

fn cors_text(status: StatusCode, content_type: &'static str, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS"),
            (
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                "Content-Type, Authorization",
            ),
        ],
        body,
    )
        .into_response()
}

fn cors_json(status: StatusCode, body: JsonValue) -> Response {
    cors_text(status, "application/json", body.to_string())
}

fn encode_polling(eio: u8, packets: &[EnginePacket]) -> Response {
    let encoded = if eio == 4 {
        encode_payload_v4(packets, limits())
    } else {
        encode_payload(packets, limits())
    };
    match encoded {
        Ok(text) => cors_text(StatusCode::OK, "text/plain; charset=UTF-8", text),
        Err(_) => cors_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "response framing failed"}),
        ),
    }
}

fn decode_polling(eio: u8, body: &str) -> Result<Vec<EnginePacket>, socket_protocol::Error> {
    if eio == 4 {
        decode_payload_v4(body, limits())
    } else {
        decode_payload(body, limits())
    }
}

pub fn load_skill_catalog(game_data_dir: Option<&Path>) -> HashMap<String, SkillDef> {
    let mut map = HashMap::new();
    let candidate_dirs = [
        game_data_dir.map(Path::to_path_buf),
        std::env::var("GAME_DATA_ROOT").ok().map(Into::into),
        Some(
            Path::new("reference/upstream/Database/Laragon/ninjasage/public/game_data")
                .to_path_buf(),
        ),
    ];
    let mut chosen_dir = None;
    for dir in candidate_dirs.into_iter().flatten() {
        if dir.join("skills.json").is_file() {
            chosen_dir = Some(dir);
            break;
        }
    }
    if let Some(dir) = chosen_dir {
        if let Ok(raw_skills) = std::fs::read_to_string(dir.join("skills.json"))
            && let Ok(arr) = serde_json::from_str::<Vec<JsonValue>>(&raw_skills)
        {
            for entry in arr {
                let Some(id) = entry.get("id").and_then(JsonValue::as_str) else {
                    continue;
                };
                map.insert(
                    id.to_owned(),
                    SkillDef {
                        id: id.to_owned(),
                        name: entry
                            .get("name")
                            .and_then(JsonValue::as_str)
                            .unwrap_or(id)
                            .to_owned(),
                        damage: entry.get("damage").and_then(JsonValue::as_i64).unwrap_or(0),
                        cp_cost: entry
                            .get("cp_cost")
                            .and_then(JsonValue::as_i64)
                            .unwrap_or(0),
                        cooldown: entry
                            .get("cooldown")
                            .and_then(JsonValue::as_u64)
                            .unwrap_or(0) as u32,
                        level: entry.get("level").and_then(JsonValue::as_i64).unwrap_or(1) as i32,
                        effects: Vec::new(),
                    },
                );
            }
        }
        if let Ok(raw_effects) = std::fs::read_to_string(dir.join("skill-effect.json"))
            && let Ok(arr) = serde_json::from_str::<Vec<JsonValue>>(&raw_effects)
        {
            for entry in arr {
                let Some(skill_id) = entry.get("skill_id").and_then(JsonValue::as_str) else {
                    continue;
                };
                let Some(skill) = map.get_mut(skill_id) else {
                    continue;
                };
                if let Some(effects) = entry.get("skill_effect").and_then(JsonValue::as_array) {
                    for e in effects {
                        skill.effects.push(SkillEffectDef {
                            target: e
                                .get("target")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("enemy")
                                .to_owned(),
                            r#type: e
                                .get("type")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("")
                                .to_owned(),
                            effect: e
                                .get("effect")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("")
                                .to_owned(),
                            effect_name: e
                                .get("effect_name")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("")
                                .to_owned(),
                            duration: e.get("duration").and_then(JsonValue::as_i64).unwrap_or(0)
                                as i32,
                            calc_type: e
                                .get("calc_type")
                                .and_then(JsonValue::as_str)
                                .unwrap_or("")
                                .to_owned(),
                            amount: e.get("amount").and_then(JsonValue::as_f64).unwrap_or(0.0),
                            chance: e.get("chance").and_then(JsonValue::as_f64).unwrap_or(100.0),
                        });
                    }
                }
            }
        }
    }
    // Always ensure baseline test/starter skills exist
    map.entry("skill_01".to_owned())
        .or_insert_with(|| SkillDef {
            id: "skill_01".to_owned(),
            name: "Lightning Edge".to_owned(),
            damage: 60,
            cp_cost: 30,
            cooldown: 2,
            level: 1,
            effects: Vec::new(),
        });
    map.entry("skill_02".to_owned())
        .or_insert_with(|| SkillDef {
            id: "skill_02".to_owned(),
            name: "Monkey Transformation".to_owned(),
            damage: 0,
            cp_cost: 45,
            cooldown: 4,
            level: 1,
            effects: vec![SkillEffectDef {
                target: "enemy".to_owned(),
                r#type: "Debuff".to_owned(),
                effect: "stun".to_owned(),
                effect_name: "Stun".to_owned(),
                duration: 2,
                calc_type: String::new(),
                amount: 0.0,
                chance: 100.0,
            }],
        });
    map
}

impl PvpServer {
    pub fn new(pool: Option<MySqlPool>, config: PvpConfig) -> Self {
        let skills = load_skill_catalog(None);
        Self {
            inner: Arc::new(PvpInner {
                pool,
                config,
                skills,
                started_at: Instant::now(),
                state: Mutex::new(PvpState {
                    next_room_id: 1,
                    next_battle_seq: 1,
                    ..PvpState::default()
                }),
            }),
        }
    }

    #[cfg(test)]
    pub fn insert_mock_character(&self, session_key: &str, character: PvpCharacter) {
        let mut state = self.inner.state.lock().expect("pvp state lock poisoned");
        state
            .mock_characters
            .insert(character.id, (session_key.to_owned(), character));
    }

    fn push_to_session(session: &mut Session, pkt: EnginePacket) {
        if session.queue.len() < MAX_QUEUE {
            session.queue.push_back(pkt);
            session.notify.notify_waiters();
        }
    }

    fn emit_to_sid(state: &mut PvpState, sid: &str, event: &str, payload: JsonValue) {
        let Some(pkt) = pvp_event_packet(event, payload) else {
            return;
        };
        if let Some(session) = state.sessions.get_mut(sid) {
            Self::push_to_session(session, pkt);
        }
    }

    fn broadcast_pvp(state: &mut PvpState, event: &str, payload: JsonValue) {
        let Some(pkt) = pvp_event_packet(event, payload) else {
            return;
        };
        for session in state.sessions.values_mut() {
            if session.pvp_connected && session.character.is_some() {
                Self::push_to_session(session, pkt.clone());
            }
        }
    }

    fn active_players_count(state: &PvpState) -> usize {
        state
            .sessions
            .values()
            .filter(|s| s.pvp_connected && s.character.is_some())
            .count()
    }

    fn public_rooms_list(state: &PvpState) -> Vec<JsonValue> {
        state.rooms.values().map(RoomState::public_info).collect()
    }

    fn cleanup_disconnected_session(&self, state: &mut PvpState, sid: &str) {
        state
            .matchmaking_queue
            .retain(|(queued_sid, _)| queued_sid != sid);

        // Leave room if in one
        let room_id = state.sessions.get(sid).and_then(|s| s.room_id);
        if let Some(rid) = room_id {
            Self::exit_room_locked(state, sid, rid);
        }

        // Remove from spectators
        let spec_id = state
            .sessions
            .get(sid)
            .and_then(|s| s.spectating_id.clone());
        if let Some(bid) = spec_id
            && let Some(battle) = state.battles.get_mut(&bid)
        {
            battle.spectators.remove(sid);
        }

        // If in active battle, opponent wins by disconnect
        let battle_id = state.sessions.get(sid).and_then(|s| s.battle_id.clone());
        if let Some(bid) = battle_id
            && let Some(battle) = state.battles.remove(&bid)
        {
            let winner_role = if battle.host.sid == sid {
                "enemy"
            } else {
                "host"
            };
            self.finish_battle_locked(state, battle, winner_role, "disconnect");
        }
    }

    fn prune_locked(&self, state: &mut PvpState) {
        let ttl = self.inner.config.session_ttl;
        let expired: Vec<String> = state
            .sessions
            .iter()
            .filter(|(_, s)| s.last_ping.elapsed() >= ttl)
            .map(|(sid, _)| sid.clone())
            .collect();
        if expired.is_empty() {
            return;
        }
        for sid in expired {
            self.cleanup_disconnected_session(state, &sid);
            if let Some(s) = state.sessions.remove(&sid) {
                s.notify.notify_waiters();
            }
        }
        let count = Self::active_players_count(state);
        Self::broadcast_pvp(state, "System.activePlayers", json!(count));
    }

    async fn verify_character(
        &self,
        character_id: u64,
        session_key: &str,
    ) -> Result<PvpCharacter, &'static str> {
        if character_id == 0 || session_key.trim().is_empty() {
            return Err("Invalid credentials.");
        }
        {
            let state = self.inner.state.lock().expect("pvp state lock poisoned");
            if let Some((stored_key, mock_char)) = state.mock_characters.get(&character_id) {
                if authentication::matches_session(mock_char.user_id, stored_key, session_key)
                    || stored_key == session_key
                {
                    return Ok(mock_char.clone());
                }
                return Err("Authentication failed.");
            }
        }
        let Some(pool) = &self.inner.pool else {
            return Err("Authentication failed.");
        };
        let row = sqlx::query(
            "SELECT c.id, c.user_id, c.name, c.level, c.`rank`, c.gender, c.xp, c.gold, c.prestige,
                    c.pvp_played, c.pvp_won, c.pvp_lost, c.pvp_points, c.pvp_trophy,
                    c.point_wind, c.point_fire, c.point_lightning, c.point_water, c.point_earth, c.point_free,
                    c.hair_style, c.hair_color, c.skin_color,
                    c.equipment_weapon, c.equipment_back, c.equipment_clothing, c.equipment_accessory,
                    c.equipment_pet, c.equipment_skills,
                    c.element_1, c.element_2, c.element_3, c.senjutsu_type,
                    u.session_key
             FROM characters c
             JOIN users u ON u.id = c.user_id
             WHERE c.id = ?
             LIMIT 1",
        )
        .bind(character_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| "Internal server error.")?
        .ok_or("Authentication failed.")?;

        let user_id: u64 = row.try_get("user_id").unwrap_or(0);
        let stored_key: String = row
            .try_get::<Option<String>, _>("session_key")
            .ok()
            .flatten()
            .unwrap_or_default();
        if !authentication::matches_session(user_id, &stored_key, session_key) {
            return Err("Authentication failed.");
        }

        let gender: i8 = row.try_get("gender").unwrap_or(0);
        let suffix = if gender == 0 { "m" } else { "f" };
        let raw_hair: Option<String> = row.try_get("hair_style").ok().flatten();
        let hair_style = match raw_hair.as_deref() {
            None | Some("") => format!("hair_01_{suffix}"),
            Some(val) => {
                if let Ok(num) = val.parse::<u32>() {
                    format!("hair_{num:02}_{suffix}")
                } else {
                    val.to_owned()
                }
            }
        };
        let skills_str = row
            .try_get::<Option<String>, _>("equipment_skills")
            .ok()
            .flatten()
            .unwrap_or_default();
        let skills: Vec<String> = skills_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let elements: Vec<i32> = [
            row.try_get::<i32, _>("element_1").unwrap_or(0),
            row.try_get::<i32, _>("element_2").unwrap_or(0),
            row.try_get::<i32, _>("element_3").unwrap_or(0),
        ]
        .into_iter()
        .filter(|&e| e > 0)
        .collect();
        let rank = row
            .try_get::<i32, _>("rank")
            .or_else(|_| row.try_get::<i8, _>("rank").map(i32::from))
            .unwrap_or(1);

        let sets = json!({
            "hair_style": hair_style,
            "hair_color": row.try_get::<Option<String>, _>("hair_color").ok().flatten().unwrap_or_else(|| "0|0".to_owned()),
            "skin_color": row.try_get::<Option<String>, _>("skin_color").ok().flatten().unwrap_or_else(|| "null".to_owned()),
            "face": format!("face_01_{suffix}"),
            "weapon": row.try_get::<Option<String>, _>("equipment_weapon").ok().flatten(),
            "back_item": row.try_get::<Option<String>, _>("equipment_back").ok().flatten(),
            "clothing": row.try_get::<Option<String>, _>("equipment_clothing").ok().flatten(),
            "accessory": row.try_get::<Option<String>, _>("equipment_accessory").ok().flatten(),
            "pet": row.try_get::<Option<String>, _>("equipment_pet").ok().flatten(),
        });

        Ok(PvpCharacter {
            id: character_id,
            user_id,
            name: row.try_get::<String, _>("name").unwrap_or_default(),
            level: row.try_get::<i32, _>("level").unwrap_or(1),
            rank,
            gender,
            xp: row.try_get::<i64, _>("xp").unwrap_or(0),
            gold: row.try_get::<i32, _>("gold").unwrap_or(0),
            prestige: row.try_get::<i32, _>("prestige").unwrap_or(0),
            pvp_played: row.try_get::<i32, _>("pvp_played").unwrap_or(0),
            pvp_won: row.try_get::<i32, _>("pvp_won").unwrap_or(0),
            pvp_lost: row.try_get::<i32, _>("pvp_lost").unwrap_or(0),
            pvp_points: row.try_get::<i32, _>("pvp_points").unwrap_or(0),
            pvp_trophy: row.try_get::<i32, _>("pvp_trophy").unwrap_or(0),
            point_wind: i64::from(row.try_get::<i32, _>("point_wind").unwrap_or(0)),
            point_fire: i64::from(row.try_get::<i32, _>("point_fire").unwrap_or(0)),
            point_lightning: i64::from(row.try_get::<i32, _>("point_lightning").unwrap_or(0)),
            point_water: i64::from(row.try_get::<i32, _>("point_water").unwrap_or(0)),
            point_earth: i64::from(row.try_get::<i32, _>("point_earth").unwrap_or(0)),
            point_free: i64::from(row.try_get::<i32, _>("point_free").unwrap_or(0)),
            skills,
            sets,
            elements,
            senjutsu_type: row
                .try_get::<Option<String>, _>("senjutsu_type")
                .ok()
                .flatten(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn start_battle_locked(
        &self,
        state: &mut PvpState,
        host_sid: String,
        host_char: PvpCharacter,
        enemy_sid: String,
        enemy_char: PvpCharacter,
        mode: String,
        map_id: String,
    ) {
        let seq = state.next_battle_seq;
        state.next_battle_seq += 1;
        let battle_id = format!("battle_{}_{}", now_ms(), seq);

        let host_stats = host_char.build_stats(&self.inner.config);
        let enemy_stats = enemy_char.build_stats(&self.inner.config);

        let battle = BattleState {
            id: battle_id.clone(),
            mode: mode.clone(),
            map_id: map_id.clone(),
            host: FighterState {
                sid: host_sid.clone(),
                character: host_char,
                stats: host_stats,
                dodging: false,
                cooldowns: HashMap::new(),
                effects: Vec::new(),
            },
            enemy: FighterState {
                sid: enemy_sid.clone(),
                character: enemy_char,
                stats: enemy_stats,
                dodging: false,
                cooldowns: HashMap::new(),
                effects: Vec::new(),
            },
            spectators: HashSet::new(),
            turn: "host".to_owned(),
            round: 1,
            action_log: Vec::new(),
        };

        if let Some(hs) = state.sessions.get_mut(&host_sid) {
            hs.battle_id = Some(battle_id.clone());
            hs.room_id = None;
        }
        if let Some(es) = state.sessions.get_mut(&enemy_sid) {
            es.battle_id = Some(battle_id.clone());
            es.room_id = None;
        }

        let started_payload = json!({
            "battle_id": battle_id,
            "map_id": map_id,
            "mode": mode,
            "host": battle.host.snapshot(),
            "enemy": battle.enemy.snapshot(),
            "turn": "host",
            "turn_time": self.inner.config.turn_seconds,
        });
        let turn_payload = json!({
            "turn": "host",
            "active_id": battle.host.character.id,
            "round": 1,
            "cooldowns": {},
        });

        state.battles.insert(battle_id, battle);

        Self::emit_to_sid(state, &host_sid, "Battle.started", started_payload.clone());
        Self::emit_to_sid(state, &enemy_sid, "Battle.started", started_payload);
        Self::emit_to_sid(state, &host_sid, "Battle.turn", turn_payload.clone());
        Self::emit_to_sid(state, &enemy_sid, "Battle.turn", turn_payload);
    }

    fn exit_room_locked(state: &mut PvpState, sid: &str, room_id: u64) {
        if let Some(s) = state.sessions.get_mut(sid) {
            s.room_id = None;
        }
        let Some(mut room) = state.rooms.remove(&room_id) else {
            return;
        };
        if room.host_sid == sid {
            if let Some((chal_sid, _)) = room.challenger {
                if let Some(cs) = state.sessions.get_mut(&chal_sid) {
                    cs.room_id = None;
                }
                Self::emit_to_sid(
                    state,
                    &chal_sid,
                    "Room.kicked",
                    json!({ "reason": "Host closed the room." }),
                );
            }
        } else if room
            .challenger
            .as_ref()
            .is_some_and(|(chal_sid, _)| chal_sid == sid)
        {
            let departed_id = room.challenger.as_ref().map(|(_, c)| c.id).unwrap_or(0);
            room.challenger = None;
            room.challenger_ready = false;
            let host_sid = room.host_sid.clone();
            state.rooms.insert(room_id, room);
            Self::emit_to_sid(
                state,
                &host_sid,
                "Room.playerLeft",
                json!({ "character_id": departed_id }),
            );
        } else {
            state.rooms.insert(room_id, room);
        }
        let list = Self::public_rooms_list(state);
        Self::broadcast_pvp(state, "Room.list", json!(list));
    }

    fn finish_battle_locked(
        &self,
        state: &mut PvpState,
        battle: BattleState,
        winner_role: &str,
        reason: &str,
    ) {
        let host_won = winner_role == "host";
        let (winner_trophies, loser_trophies) = if host_won {
            (
                battle.host.character.pvp_trophy,
                battle.enemy.character.pvp_trophy,
            )
        } else {
            (
                battle.enemy.character.pvp_trophy,
                battle.host.character.pvp_trophy,
            )
        };
        let (gain, loss) = calc_trophy_delta(
            winner_trophies,
            loser_trophies,
            self.inner.config.trophy_k_factor,
        );
        let host_delta = if host_won { gain } else { -loss };
        let enemy_delta = if host_won { -loss } else { gain };

        let host_before = battle.host.character.pvp_trophy;
        let enemy_before = battle.enemy.character.pvp_trophy;
        let host_after = apply_trophies(host_before, host_delta);
        let enemy_after = apply_trophies(enemy_before, enemy_delta);

        let points_win = self.inner.config.points_on_win;
        let points_loss = self.inner.config.points_on_loss;
        let prestige_win = self.inner.config.prestige_on_win;

        // Update in-memory character stats in sessions
        if let Some(hs) = state.sessions.get_mut(&battle.host.sid) {
            hs.battle_id = None;
            if let Some(ch) = &mut hs.character {
                ch.pvp_played += 1;
                if host_won {
                    ch.pvp_won += 1;
                    ch.pvp_points += points_win;
                    ch.prestige += prestige_win;
                } else {
                    ch.pvp_lost += 1;
                    ch.pvp_points += points_loss;
                }
                ch.pvp_trophy = host_after;
            }
        }
        if let Some(es) = state.sessions.get_mut(&battle.enemy.sid) {
            es.battle_id = None;
            if let Some(ch) = &mut es.character {
                ch.pvp_played += 1;
                if !host_won {
                    ch.pvp_won += 1;
                    ch.pvp_points += points_win;
                    ch.prestige += prestige_win;
                } else {
                    ch.pvp_lost += 1;
                    ch.pvp_points += points_loss;
                }
                ch.pvp_trophy = enemy_after;
            }
        }
        for spec_sid in &battle.spectators {
            if let Some(ss) = state.sessions.get_mut(spec_sid) {
                ss.spectating_id = None;
            }
        }

        let (winner_id, loser_id) = if host_won {
            (battle.host.character.id, battle.enemy.character.id)
        } else {
            (battle.enemy.character.id, battle.host.character.id)
        };

        let ended_payload = json!({
            "winner_id": winner_id,
            "loser_id": loser_id,
            "reason": reason,
            "trophy_delta": gain,
            "host_trophies": host_after,
            "enemy_trophies": enemy_after,
            "points_won": points_win,
            "points_lost": points_loss,
        });

        Self::emit_to_sid(
            state,
            &battle.host.sid,
            "Battle.ended",
            ended_payload.clone(),
        );
        Self::emit_to_sid(
            state,
            &battle.enemy.sid,
            "Battle.ended",
            ended_payload.clone(),
        );
        for spec_sid in &battle.spectators {
            Self::emit_to_sid(state, spec_sid, "Battle.ended", ended_payload.clone());
        }

        // Persist to MySQL if connected
        if let Some(pool) = self.inner.pool.clone() {
            let host_id = battle.host.character.id;
            let enemy_id = battle.enemy.character.id;
            let mode = battle.mode.clone();
            let host_level = battle.host.character.level;
            let enemy_level = battle.enemy.character.level;
            let host_rank = battle.host.character.rank;
            let enemy_rank = battle.enemy.character.rank;
            let host_snap = battle.host.snapshot().to_string();
            let enemy_snap = battle.enemy.snapshot().to_string();
            let battle_data = json!({
                "rounds": battle.round,
                "reason": reason,
                "actions": battle.action_log,
            })
            .to_string();
            let host_pts = if host_won { points_win } else { points_loss };
            let enemy_pts = if host_won { points_loss } else { points_win };
            let host_pres = if host_won { prestige_win } else { 0 };
            let enemy_pres = if host_won { 0 } else { prestige_win };

            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE characters SET
                        pvp_played = pvp_played + 1,
                        pvp_won    = pvp_won + ?,
                        pvp_lost   = pvp_lost + ?,
                        pvp_points = GREATEST(0, pvp_points + ?),
                        pvp_trophy = ?,
                        prestige   = prestige + ?
                     WHERE id = ?",
                )
                .bind(i32::from(host_won))
                .bind(i32::from(!host_won))
                .bind(host_pts)
                .bind(host_after)
                .bind(host_pres)
                .bind(host_id)
                .execute(&pool)
                .await;

                let _ = sqlx::query(
                    "UPDATE characters SET
                        pvp_played = pvp_played + 1,
                        pvp_won    = pvp_won + ?,
                        pvp_lost   = pvp_lost + ?,
                        pvp_points = GREATEST(0, pvp_points + ?),
                        pvp_trophy = ?,
                        prestige   = prestige + ?
                     WHERE id = ?",
                )
                .bind(i32::from(!host_won))
                .bind(i32::from(host_won))
                .bind(enemy_pts)
                .bind(enemy_after)
                .bind(enemy_pres)
                .bind(enemy_id)
                .execute(&pool)
                .await;

                let _ = sqlx::query(
                    "INSERT INTO pvp_battles
                        (host_id, enemy_id, mode, host_won, trophy_delta,
                         host_trophy_before, host_trophy_after,
                         enemy_trophy_before, enemy_trophy_after,
                         host_level, enemy_level, host_rank, enemy_rank,
                         host_snapshot, enemy_snapshot, battle_data, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())",
                )
                .bind(host_id)
                .bind(enemy_id)
                .bind(mode)
                .bind(i32::from(host_won))
                .bind(gain)
                .bind(host_before)
                .bind(host_after)
                .bind(enemy_before)
                .bind(enemy_after)
                .bind(host_level)
                .bind(enemy_level)
                .bind(host_rank)
                .bind(enemy_rank)
                .bind(host_snap)
                .bind(enemy_snap)
                .bind(battle_data)
                .execute(&pool)
                .await;
            });
        }
    }

    fn execute_battle_action_locked(
        &self,
        state: &mut PvpState,
        sid: &str,
        action_type: &str,
        action_data: &JsonValue,
    ) {
        let Some(battle_id) = state.sessions.get(sid).and_then(|s| s.battle_id.clone()) else {
            return;
        };
        let Some(mut battle) = state.battles.remove(&battle_id) else {
            return;
        };

        let actor_role = if battle.host.sid == sid {
            "host"
        } else if battle.enemy.sid == sid {
            "enemy"
        } else {
            state.battles.insert(battle_id, battle);
            return;
        };

        if action_type == "run" {
            let winner_role = if actor_role == "host" {
                "enemy"
            } else {
                "host"
            };
            let result_payload = json!({
                "action": "run",
                "actor": actor_role,
                "actor_id": if actor_role == "host" { battle.host.character.id } else { battle.enemy.character.id },
                "host_hp": battle.host.stats.hp,
                "host_cp": battle.host.stats.cp,
                "enemy_hp": battle.enemy.stats.hp,
                "enemy_cp": battle.enemy.stats.cp,
                "round": battle.round,
            });
            Self::emit_to_sid(
                state,
                &battle.host.sid,
                "Battle.actionResult",
                result_payload.clone(),
            );
            Self::emit_to_sid(
                state,
                &battle.enemy.sid,
                "Battle.actionResult",
                result_payload.clone(),
            );
            for spec_sid in &battle.spectators {
                Self::emit_to_sid(
                    state,
                    spec_sid,
                    "Battle.actionResult",
                    result_payload.clone(),
                );
            }
            self.finish_battle_locked(state, battle, winner_role, "run");
            return;
        }

        if battle.turn != actor_role {
            state.battles.insert(battle_id, battle);
            return;
        }

        let (mut attacker, mut defender) = if actor_role == "host" {
            (battle.host.clone(), battle.enemy.clone())
        } else {
            (battle.enemy.clone(), battle.host.clone())
        };

        attacker.dodging = false;

        let mut damage = 0_i64;
        let mut critical = false;
        let mut dodged = false;
        let mut cp_used = 0_i64;
        let mut cp_gained = 0_i64;
        let mut skill_id_out: Option<String> = None;
        let mut applied_effects = Vec::new();
        let mut resolved_action = action_type.to_owned();

        let stunned = attacker
            .effects
            .iter()
            .any(|e| matches!(e.effect.as_str(), "stun" | "sleep" | "freeze"));

        if stunned {
            resolved_action = "stunned".to_owned();
        } else {
            match action_type {
                "weapon" => {
                    let base_dodge = if defender.dodging {
                        (defender.stats.dodge * 2.5).min(85.0)
                    } else {
                        defender.stats.dodge
                    };
                    let eff_dodge = (base_dodge - attacker.stats.accuracy).clamp(0.0, 90.0);
                    if rand_f64_100() < eff_dodge {
                        dodged = true;
                    } else {
                        critical = rand_f64_100() < attacker.stats.critical;
                        let raw = if critical {
                            ((attacker.stats.weapon_damage as f64) * 1.5).round() as i64
                        } else {
                            attacker.stats.weapon_damage
                        };
                        damage = raw.max(1);
                        defender.stats.hp = (defender.stats.hp - damage).max(0);
                    }
                    defender.dodging = false;
                }
                "skill" => {
                    let req_skill = action_data
                        .get("skill_id")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("skill_01")
                        .to_owned();
                    let skill_def =
                        self.inner
                            .skills
                            .get(&req_skill)
                            .cloned()
                            .unwrap_or_else(|| SkillDef {
                                id: req_skill.clone(),
                                name: req_skill.clone(),
                                damage: 40,
                                cp_cost: 20,
                                cooldown: 2,
                                level: 1,
                                effects: Vec::new(),
                            });
                    if attacker.Cooldown(&req_skill) == 0 && attacker.stats.cp >= skill_def.cp_cost
                    {
                        cp_used = skill_def.cp_cost;
                        attacker.stats.cp = (attacker.stats.cp - cp_used).max(0);
                        if skill_def.cooldown > 0 {
                            attacker
                                .cooldowns
                                .insert(req_skill.clone(), skill_def.cooldown);
                        }
                        skill_id_out = Some(req_skill);
                        if skill_def.damage > 0 {
                            let base_dodge = if defender.dodging {
                                (defender.stats.dodge * 2.5).min(85.0)
                            } else {
                                defender.stats.dodge
                            };
                            let eff_dodge = (base_dodge - attacker.stats.accuracy).clamp(0.0, 90.0);
                            if rand_f64_100() < eff_dodge {
                                dodged = true;
                            } else {
                                critical = rand_f64_100() < attacker.stats.critical;
                                let mut scaled = (skill_def.damage as f64)
                                    * attacker.stats.fire_bonus
                                    * (1.0 + f64::from(attacker.character.level) * 0.02);
                                if critical {
                                    scaled *= 1.5;
                                }
                                damage = (scaled.round() as i64).max(1);
                                defender.stats.hp = (defender.stats.hp - damage).max(0);
                            }
                        }
                        if !dodged {
                            for eff in &skill_def.effects {
                                if eff.duration > 0 {
                                    let active = ActiveEffect {
                                        effect: eff.effect.clone(),
                                        effect_name: eff.effect_name.clone(),
                                        r#type: eff.r#type.clone(),
                                        calc_type: eff.calc_type.clone(),
                                        amount: eff.amount,
                                        remaining: eff.duration,
                                    };
                                    if eff.target == "self" {
                                        attacker.effects.push(active.clone());
                                    } else {
                                        defender.effects.push(active.clone());
                                    }
                                    applied_effects.push(json!({
                                        "target": eff.target,
                                        "effect": eff.effect,
                                        "effect_name": eff.effect_name,
                                        "duration": eff.duration,
                                        "amount": eff.amount,
                                    }));
                                }
                            }
                        }
                        defender.dodging = false;
                    } else {
                        // Fallback to weapon attack if skill unavailable
                        resolved_action = "weapon".to_owned();
                        damage = attacker.stats.weapon_damage.max(1);
                        defender.stats.hp = (defender.stats.hp - damage).max(0);
                        defender.dodging = false;
                    }
                }
                "dodge" => {
                    attacker.dodging = true;
                }
                "charge" => {
                    cp_gained = (((attacker.stats.max_cp as f64) * 0.25).floor() as i64).max(1);
                    attacker.stats.cp = (attacker.stats.cp + cp_gained).min(attacker.stats.max_cp);
                }
                "scroll" => {
                    let item_id = action_data
                        .get("item_id")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("");
                    if item_id.contains("cp") {
                        cp_gained = ((attacker.stats.max_cp as f64) * 0.30).floor() as i64;
                        attacker.stats.cp =
                            (attacker.stats.cp + cp_gained).min(attacker.stats.max_cp);
                    } else {
                        let heal = ((attacker.stats.max_hp as f64) * 0.25).floor() as i64;
                        attacker.stats.hp = (attacker.stats.hp + heal).min(attacker.stats.max_hp);
                    }
                }
                _ => {}
            }
        }

        // Tick down cooldowns on attacker and effect durations on defender
        attacker.cooldowns.retain(|_, v| {
            *v = v.saturating_sub(1);
            *v > 0
        });
        defender.effects.retain_mut(|e| {
            e.remaining -= 1;
            e.remaining > 0
        });

        if actor_role == "host" {
            battle.host = attacker.clone();
            battle.enemy = defender.clone();
        } else {
            battle.enemy = attacker.clone();
            battle.host = defender.clone();
        }

        let result_payload = json!({
            "action": resolved_action,
            "actor": actor_role,
            "actor_id": attacker.character.id,
            "skill_id": skill_id_out,
            "damage": damage,
            "critical": critical,
            "dodged": dodged,
            "cp_used": cp_used,
            "cp_gained": cp_gained,
            "dot_damage": 0,
            "applied_effects": applied_effects,
            "host_hp": battle.host.stats.hp,
            "host_cp": battle.host.stats.cp,
            "enemy_hp": battle.enemy.stats.hp,
            "enemy_cp": battle.enemy.stats.cp,
            "host_effects": battle.host.effects,
            "enemy_effects": battle.enemy.effects,
            "round": battle.round,
        });
        battle.action_log.push(result_payload.clone());

        Self::emit_to_sid(
            state,
            &battle.host.sid,
            "Battle.actionResult",
            result_payload.clone(),
        );
        Self::emit_to_sid(
            state,
            &battle.enemy.sid,
            "Battle.actionResult",
            result_payload.clone(),
        );
        for spec_sid in &battle.spectators {
            Self::emit_to_sid(
                state,
                spec_sid,
                "Battle.actionResult",
                result_payload.clone(),
            );
        }

        if battle.host.stats.hp <= 0 || battle.enemy.stats.hp <= 0 {
            let winner_role = if battle.enemy.stats.hp <= 0 {
                "host"
            } else {
                "enemy"
            };
            self.finish_battle_locked(state, battle, winner_role, "ko");
            return;
        }

        battle.turn = if actor_role == "host" {
            "enemy".to_owned()
        } else {
            "host".to_owned()
        };
        battle.round += 1;
        let next_fighter = if battle.turn == "host" {
            &battle.host
        } else {
            &battle.enemy
        };
        let turn_payload = json!({
            "turn": battle.turn,
            "active_id": next_fighter.character.id,
            "round": battle.round,
            "cooldowns": next_fighter.cooldowns,
        });
        Self::emit_to_sid(state, &battle.host.sid, "Battle.turn", turn_payload.clone());
        Self::emit_to_sid(
            state,
            &battle.enemy.sid,
            "Battle.turn",
            turn_payload.clone(),
        );
        for spec_sid in &battle.spectators {
            Self::emit_to_sid(state, spec_sid, "Battle.turn", turn_payload.clone());
        }

        state.battles.insert(battle_id, battle);
    }
}

impl FighterState {
    #[allow(non_snake_case)]
    fn Cooldown(&self, skill_id: &str) -> u32 {
        self.cooldowns.get(skill_id).copied().unwrap_or(0)
    }
}

pub fn router(server: PvpServer) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/crossdomain.xml", get(crossdomain_handler))
        .route(
            "/socket.io/",
            get(socket_poll_handler)
                .post(socket_post_handler)
                .options(options_handler),
        )
        .route(
            "/socket.io",
            get(socket_poll_handler)
                .post(socket_post_handler)
                .options(options_handler),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(server)
}

async fn options_handler() -> Response {
    cors_text(StatusCode::NO_CONTENT, "text/plain", String::new())
}

async fn health_handler(State(server): State<PvpServer>) -> Response {
    let uptime = server.inner.started_at.elapsed().as_secs_f64();
    cors_json(StatusCode::OK, json!({ "status": "ok", "uptime": uptime }))
}

async fn crossdomain_handler() -> Response {
    cors_text(
        StatusCode::OK,
        "text/x-cross-domain-policy",
        CROSS_DOMAIN_POLICY.to_owned(),
    )
}

fn parse_eio(query: &PollParams) -> Result<u8, &'static str> {
    let transport = query.transport.as_deref().unwrap_or("polling");
    if transport != "polling" {
        return Err("only transport=polling is supported");
    }
    match query.eio.as_deref().unwrap_or("3") {
        "3" => Ok(3),
        "4" => Ok(4),
        _ => Err("only EIO=3 and EIO=4 are supported"),
    }
}

struct PollGuard {
    server: PvpServer,
    sid: String,
}

impl Drop for PollGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.server.inner.state.lock()
            && let Some(session) = state.sessions.get_mut(&self.sid)
        {
            session.polling = false;
        }
    }
}

async fn socket_poll_handler(
    State(server): State<PvpServer>,
    Query(query): Query<PollParams>,
) -> Response {
    let eio = match parse_eio(&query) {
        Ok(v) => v,
        Err(msg) => return cors_json(StatusCode::BAD_REQUEST, json!({ "error": msg })),
    };

    let Some(sid) = query.sid else {
        let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
        server.prune_locked(&mut state);
        if state.sessions.len() >= server.inner.config.capacity {
            return cors_json(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "error": "session capacity reached" }),
            );
        }
        let sid = loop {
            let mut bytes = [0u8; 16];
            if getrandom::fill(&mut bytes).is_err() {
                return cors_json(
                    StatusCode::SERVICE_UNAVAILABLE,
                    json!({ "error": "session entropy unavailable" }),
                );
            }
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            if !state.sessions.contains_key(&id) {
                break id;
            }
        };
        let mut initial_queue = VecDeque::new();
        if eio == 4 {
            initial_queue.push_back(engine_packet(EngineKind::Message, "0"));
        }
        state.sessions.insert(
            sid.clone(),
            Session {
                eio,
                last_ping: Instant::now(),
                polling: false,
                queue: initial_queue,
                notify: Arc::new(Notify::new()),
                pvp_connected: false,
                character: None,
                room_id: None,
                battle_id: None,
                spectating_id: None,
            },
        );
        return encode_polling(
            eio,
            &[engine_packet(
                EngineKind::Open,
                json!({
                    "sid": sid,
                    "upgrades": [],
                    "pingInterval": 25000,
                    "pingTimeout": 20000,
                    "maxPayload": MAX_BODY
                })
                .to_string(),
            )],
        );
    };

    let (notify, session_eio) = {
        let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
        server.prune_locked(&mut state);
        let Some(session) = state.sessions.get_mut(&sid) else {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "unknown or expired session" }),
            );
        };
        session.last_ping = Instant::now();
        if !session.queue.is_empty() {
            let packets: Vec<_> = session.queue.drain(..).collect();
            return encode_polling(session.eio, &packets);
        }
        if session.polling {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "concurrent polling is unsupported" }),
            );
        }
        session.polling = true;
        (session.notify.clone(), session.eio)
    };

    let _guard = PollGuard {
        server: server.clone(),
        sid: sid.clone(),
    };
    let deadline = tokio::time::Instant::now() + server.inner.config.poll_wait;
    loop {
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        {
            let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
            server.prune_locked(&mut state);
            let Some(session) = state.sessions.get_mut(&sid) else {
                return cors_json(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": "unknown or expired session" }),
                );
            };
            if !session.queue.is_empty() {
                let packets: Vec<_> = session.queue.drain(..).collect();
                return encode_polling(session.eio, &packets);
            }
        }
        if tokio::time::timeout_at(deadline, notified).await.is_err() {
            return encode_polling(session_eio, &[engine_packet(EngineKind::Noop, "")]);
        }
    }
}

async fn socket_post_handler(
    State(server): State<PvpServer>,
    Query(query): Query<PollParams>,
    body: String,
) -> Response {
    let eio = match parse_eio(&query) {
        Ok(v) => v,
        Err(msg) => return cors_json(StatusCode::BAD_REQUEST, json!({ "error": msg })),
    };
    let Some(sid) = query.sid else {
        return cors_json(
            StatusCode::BAD_REQUEST,
            json!({ "error": "POST requires a session ID" }),
        );
    };
    let session_eio = {
        let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
        server.prune_locked(&mut state);
        let Some(session) = state.sessions.get_mut(&sid) else {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "unknown or expired session" }),
            );
        };
        session.last_ping = Instant::now();
        session.eio
    };
    let packets = match decode_polling(session_eio.max(eio), &body)
        .or_else(|_| decode_polling(3, &body))
        .or_else(|_| decode_polling(4, &body))
    {
        Ok(pkts) if !pkts.is_empty() => pkts,
        _ => {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "malformed polling payload" }),
            );
        }
    };

    for incoming in packets {
        match incoming.kind {
            EngineKind::Ping => {
                let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
                if let Some(session) = state.sessions.get_mut(&sid) {
                    session.last_ping = Instant::now();
                    PvpServer::push_to_session(
                        session,
                        engine_packet(EngineKind::Pong, incoming.data),
                    );
                }
            }
            EngineKind::Pong => {
                let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
                if let Some(session) = state.sessions.get_mut(&sid) {
                    session.last_ping = Instant::now();
                }
            }
            EngineKind::Close => {
                let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
                server.cleanup_disconnected_session(&mut state, &sid);
                if let Some(session) = state.sessions.remove(&sid) {
                    session.notify.notify_waiters();
                }
                let count = PvpServer::active_players_count(&state);
                PvpServer::broadcast_pvp(&mut state, "System.activePlayers", json!(count));
                return cors_text(StatusCode::OK, "text/plain; charset=UTF-8", "ok".to_owned());
            }
            EngineKind::Message => {
                let Ok(socket_pkt) = decode_socket_lenient(&incoming.data, limits()) else {
                    return cors_json(
                        StatusCode::BAD_REQUEST,
                        json!({ "error": "unsupported Socket.IO frame" }),
                    );
                };
                handle_socket_packet(&server, &sid, socket_pkt).await;
            }
            _ => {}
        }
    }

    cors_text(StatusCode::OK, "text/plain; charset=UTF-8", "ok".to_owned())
}

async fn handle_socket_packet(server: &PvpServer, sid: &str, packet: SocketPacket) {
    match packet {
        SocketPacket::Connect { namespace } => {
            if !matches!(namespace.as_str(), "/" | "/pvp") {
                return;
            }
            let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
            let Some(session) = state.sessions.get_mut(sid) else {
                return;
            };
            if namespace == "/pvp" {
                session.pvp_connected = true;
            }
            if let Ok(encoded) = encode_socket(&SocketPacket::Connect { namespace }, limits()) {
                PvpServer::push_to_session(session, engine_packet(EngineKind::Message, encoded));
            }
        }
        SocketPacket::Disconnect { namespace } => {
            if namespace == "/pvp" {
                let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
                server.cleanup_disconnected_session(&mut state, sid);
                if let Some(session) = state.sessions.get_mut(sid) {
                    session.pvp_connected = false;
                    session.character = None;
                }
                let count = PvpServer::active_players_count(&state);
                PvpServer::broadcast_pvp(&mut state, "System.activePlayers", json!(count));
            }
        }
        SocketPacket::Event {
            namespace,
            name,
            args,
        } => {
            if namespace != "/pvp" {
                return;
            }
            let data = args.first().cloned().unwrap_or_else(|| json!({}));
            if name == "System.auth" {
                handle_system_auth(server, sid, &data).await;
                return;
            }
            handle_pvp_event(server, sid, &name, &data);
        }
    }
}

async fn handle_system_auth(server: &PvpServer, sid: &str, data: &JsonValue) {
    let char_id = data
        .get("id")
        .or_else(|| data.get("character_id"))
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .unwrap_or(0);
    let session_key = data
        .get("session_key")
        .and_then(JsonValue::as_str)
        .unwrap_or("");

    let character = match server.verify_character(char_id, session_key).await {
        Ok(c) => c,
        Err(msg) => {
            let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
            PvpServer::emit_to_sid(&mut state, sid, "System.error", json!({ "message": msg }));
            return;
        }
    };

    let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
    // Evict any existing session with the same character_id
    let duplicate_sids: Vec<String> = state
        .sessions
        .iter()
        .filter(|(other_sid, s)| {
            *other_sid != sid && s.character.as_ref().is_some_and(|c| c.id == char_id)
        })
        .map(|(other_sid, _)| other_sid.clone())
        .collect();
    for dup_sid in duplicate_sids {
        server.cleanup_disconnected_session(&mut state, &dup_sid);
        if let Some(s) = state.sessions.get_mut(&dup_sid) {
            s.character = None;
        }
    }

    let info_json = character.to_info_json();
    if let Some(session) = state.sessions.get_mut(sid) {
        session.pvp_connected = true;
        session.character = Some(character);
    }

    let map_ids = server.inner.config.map_ids.clone();
    let active_count = PvpServer::active_players_count(&state);
    let history: Vec<JsonValue> = state.lobby_messages.iter().cloned().collect();

    PvpServer::emit_to_sid(&mut state, sid, "Client.characterInfo", info_json);
    PvpServer::emit_to_sid(&mut state, sid, "System.mapIDs", json!(map_ids));
    PvpServer::emit_to_sid(&mut state, sid, "System.activePlayers", json!(active_count));
    PvpServer::emit_to_sid(
        &mut state,
        sid,
        "Conversation.lobby.messageHistory",
        json!(history),
    );
    PvpServer::broadcast_pvp(&mut state, "System.activePlayers", json!(active_count));
}

fn handle_pvp_event(server: &PvpServer, sid: &str, name: &str, data: &JsonValue) {
    let mut state = server.inner.state.lock().expect("pvp state lock poisoned");
    let Some(character) = state.sessions.get(sid).and_then(|s| s.character.clone()) else {
        PvpServer::emit_to_sid(
            &mut state,
            sid,
            "System.error",
            json!({ "message": "Not authenticated." }),
        );
        return;
    };

    match name {
        "Conversation.lobby.sendMessage" => {
            let text: String = data
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim()
                .chars()
                .take(200)
                .collect();
            if text.is_empty() {
                return;
            }
            let entry = json!({
                "name": character.name,
                "message": text,
            });
            state.lobby_messages.push_back(entry.clone());
            while state.lobby_messages.len() > server.inner.config.max_lobby_messages {
                state.lobby_messages.pop_front();
            }
            PvpServer::broadcast_pvp(&mut state, "Conversation.lobby.newMessage", entry);
        }
        "Conversation.battle.sendMessage" => {
            let text: String = data
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim()
                .chars()
                .take(200)
                .collect();
            if text.is_empty() {
                return;
            }
            let battle_id = state
                .sessions
                .get(sid)
                .and_then(|s| s.battle_id.clone().or_else(|| s.spectating_id.clone()));
            if let Some(bid) = battle_id
                && let Some(battle) = state.battles.get(&bid).cloned()
            {
                let payload = json!({
                    "name": character.name,
                    "message": text,
                });
                PvpServer::emit_to_sid(
                    &mut state,
                    &battle.host.sid,
                    "Conversation.battle.newMessage",
                    payload.clone(),
                );
                PvpServer::emit_to_sid(
                    &mut state,
                    &battle.enemy.sid,
                    "Conversation.battle.newMessage",
                    payload.clone(),
                );
                for spec_sid in &battle.spectators {
                    PvpServer::emit_to_sid(
                        &mut state,
                        spec_sid,
                        "Conversation.battle.newMessage",
                        payload.clone(),
                    );
                }
            }
        }
        "System.listLiveMatches" => {
            let matches: Vec<JsonValue> = state
                .battles
                .values()
                .map(|b| {
                    json!({
                        "battle_id": b.id,
                        "host_name": b.host.character.name,
                        "enemy_name": b.enemy.character.name,
                        "turn": b.round,
                    })
                })
                .collect();
            PvpServer::emit_to_sid(&mut state, sid, "System.liveMatches", json!(matches));
        }
        "Battle.getPlayerInfo" => {
            let target_id = data
                .get("character_id")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                })
                .unwrap_or(0);
            let found = state
                .sessions
                .values()
                .find_map(|s| s.character.as_ref().filter(|c| c.id == target_id))
                .map(PvpCharacter::to_info_json);
            if let Some(info) = found {
                PvpServer::emit_to_sid(&mut state, sid, "Battle.playerInfo", info);
            }
        }
        "Battle.startMatchMaking" => {
            let is_busy = state
                .sessions
                .get(sid)
                .is_some_and(|s| s.battle_id.is_some() || s.room_id.is_some());
            if is_busy {
                return;
            }
            let mode = data
                .get("mode")
                .and_then(JsonValue::as_str)
                .unwrap_or("quick")
                .to_owned();
            if !state.matchmaking_queue.iter().any(|(s, _)| s == sid) {
                state.matchmaking_queue.push_back((sid.to_owned(), mode));
            }
            while state.matchmaking_queue.len() >= 2 {
                let (sid_a, mode_a) = state.matchmaking_queue.pop_front().unwrap();
                let (sid_b, _) = state.matchmaking_queue.pop_front().unwrap();
                let char_a = state.sessions.get(&sid_a).and_then(|s| s.character.clone());
                let char_b = state.sessions.get(&sid_b).and_then(|s| s.character.clone());
                if let (Some(ca), Some(cb)) = (char_a, char_b) {
                    let map_id = server
                        .inner
                        .config
                        .map_ids
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "map_01".to_owned());
                    server.start_battle_locked(&mut state, sid_a, ca, sid_b, cb, mode_a, map_id);
                }
            }
        }
        "Battle.stopMatchMaking" => {
            state.matchmaking_queue.retain(|(s, _)| s != sid);
        }
        "Room.create" => {
            let in_room = state
                .sessions
                .get(sid)
                .is_some_and(|s| s.room_id.is_some() || s.battle_id.is_some());
            if in_room {
                return;
            }
            let room_id = state.next_room_id;
            state.next_room_id += 1;
            let password = data
                .get("password")
                .and_then(JsonValue::as_str)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let mode = data
                .get("mode")
                .and_then(JsonValue::as_str)
                .unwrap_or("private")
                .to_owned();
            let map_id = data
                .get("map_id")
                .and_then(JsonValue::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| {
                    server
                        .inner
                        .config
                        .map_ids
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "map_01".to_owned())
                });
            let allow_preset = data
                .get("allow_Preset")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true);

            let room = RoomState {
                id: room_id,
                host_sid: sid.to_owned(),
                host: character.clone(),
                challenger: None,
                challenger_ready: false,
                password: password.clone(),
                mode: mode.clone(),
                map_id: map_id.clone(),
                allow_preset,
            };
            state.rooms.insert(room_id, room);
            if let Some(s) = state.sessions.get_mut(sid) {
                s.room_id = Some(room_id);
            }
            PvpServer::emit_to_sid(
                &mut state,
                sid,
                "Room.created",
                json!({
                    "room_id": room_id,
                    "host": character.to_info_json(),
                    "map_id": map_id,
                    "mode": mode,
                    "locked": password.is_some(),
                }),
            );
            let list = PvpServer::public_rooms_list(&state);
            PvpServer::broadcast_pvp(&mut state, "Room.list", json!(list));
        }
        "Room.join" => {
            let room_id = data
                .get("room_id")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                })
                .unwrap_or(0);
            let password = data.get("password").and_then(JsonValue::as_str);
            let Some(room) = state.rooms.get_mut(&room_id) else {
                PvpServer::emit_to_sid(
                    &mut state,
                    sid,
                    "Room.joinFailed",
                    json!({ "reason": "Room not found." }),
                );
                return;
            };
            if room.challenger.is_some() {
                PvpServer::emit_to_sid(
                    &mut state,
                    sid,
                    "Room.joinFailed",
                    json!({ "reason": "Room is full." }),
                );
                return;
            }
            if let Some(expected_pw) = &room.password
                && password != Some(expected_pw.as_str())
            {
                PvpServer::emit_to_sid(
                    &mut state,
                    sid,
                    "Room.joinFailed",
                    json!({ "reason": "Wrong password." }),
                );
                return;
            }
            room.challenger = Some((sid.to_owned(), character.clone()));
            room.challenger_ready = false;
            let host_sid = room.host_sid.clone();
            let joined_payload = json!({
                "room_id": room_id,
                "host": room.host.to_info_json(),
                "challenger": character.to_info_json(),
                "map_id": room.map_id,
                "mode": room.mode,
            });
            if let Some(s) = state.sessions.get_mut(sid) {
                s.room_id = Some(room_id);
            }
            PvpServer::emit_to_sid(&mut state, &host_sid, "Room.joined", joined_payload.clone());
            PvpServer::emit_to_sid(&mut state, sid, "Room.joined", joined_payload);
            let list = PvpServer::public_rooms_list(&state);
            PvpServer::broadcast_pvp(&mut state, "Room.list", json!(list));
        }
        "Room.exit" => {
            if let Some(room_id) = state.sessions.get(sid).and_then(|s| s.room_id) {
                PvpServer::exit_room_locked(&mut state, sid, room_id);
            }
        }
        "Room.ready" => {
            let ready = data
                .get("ready")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            let Some(room_id) = state.sessions.get(sid).and_then(|s| s.room_id) else {
                return;
            };
            let Some(room) = state.rooms.get_mut(&room_id) else {
                return;
            };
            if room
                .challenger
                .as_ref()
                .is_some_and(|(csid, _)| csid == sid)
            {
                room.challenger_ready = ready;
                let host_sid = room.host_sid.clone();
                let payload = json!({
                    "character_id": character.id,
                    "ready": ready,
                });
                PvpServer::emit_to_sid(&mut state, &host_sid, "Room.readyChanged", payload.clone());
                PvpServer::emit_to_sid(&mut state, sid, "Room.readyChanged", payload);
            }
        }
        "Room.kick" => {
            let Some(room_id) = state.sessions.get(sid).and_then(|s| s.room_id) else {
                return;
            };
            let Some(room) = state.rooms.get_mut(&room_id) else {
                return;
            };
            if room.host_sid == sid
                && let Some((chal_sid, chal_char)) = room.challenger.take()
            {
                room.challenger_ready = false;
                if let Some(cs) = state.sessions.get_mut(&chal_sid) {
                    cs.room_id = None;
                }
                PvpServer::emit_to_sid(
                    &mut state,
                    &chal_sid,
                    "Room.kicked",
                    json!({ "reason": "Kicked by host." }),
                );
                PvpServer::emit_to_sid(
                    &mut state,
                    sid,
                    "Room.playerLeft",
                    json!({ "character_id": chal_char.id }),
                );
                let list = PvpServer::public_rooms_list(&state);
                PvpServer::broadcast_pvp(&mut state, "Room.list", json!(list));
            }
        }
        "Room.skills.list" => {
            PvpServer::emit_to_sid(
                &mut state,
                sid,
                "Room.skills",
                json!({ "skills": character.skills }),
            );
        }
        "Room.skills.set" => {
            if let Some(arr) = data.get("skills").and_then(JsonValue::as_array) {
                let updated: Vec<String> = arr
                    .iter()
                    .filter_map(JsonValue::as_str)
                    .take(8)
                    .map(ToOwned::to_owned)
                    .collect();
                if let Some(s) = state.sessions.get_mut(sid)
                    && let Some(ch) = &mut s.character
                {
                    ch.skills = updated.clone();
                }
                PvpServer::emit_to_sid(
                    &mut state,
                    sid,
                    "Room.skillsUpdated",
                    json!({ "skills": updated }),
                );
            }
        }
        "Room.countdown.start" | "Battle.start" => {
            let Some(room_id) = state.sessions.get(sid).and_then(|s| s.room_id) else {
                return;
            };
            let can_start = state
                .rooms
                .get(&room_id)
                .is_some_and(|r| r.host_sid == sid && r.challenger.is_some() && r.challenger_ready);
            if !can_start {
                return;
            }
            let room = state.rooms.remove(&room_id).unwrap();
            let (chal_sid, chal_char) = room.challenger.unwrap();
            let list = PvpServer::public_rooms_list(&state);
            PvpServer::broadcast_pvp(&mut state, "Room.list", json!(list));
            server.start_battle_locked(
                &mut state,
                room.host_sid,
                room.host,
                chal_sid,
                chal_char,
                room.mode,
                room.map_id,
            );
        }
        "Battle.action.weapon" => {
            server.execute_battle_action_locked(&mut state, sid, "weapon", data);
        }
        "Battle.action.skill" => {
            server.execute_battle_action_locked(&mut state, sid, "skill", data);
        }
        "Battle.action.dodge" => {
            server.execute_battle_action_locked(&mut state, sid, "dodge", data);
        }
        "Battle.action.charge" => {
            server.execute_battle_action_locked(&mut state, sid, "charge", data);
        }
        "Battle.action.scroll" => {
            server.execute_battle_action_locked(&mut state, sid, "scroll", data);
        }
        "Battle.action.run" => {
            server.execute_battle_action_locked(&mut state, sid, "run", data);
        }
        "Battle.action.timeout" => {
            server.execute_battle_action_locked(&mut state, sid, "dodge", data);
        }
        "Battle.action.finished" => {}
        "Battle.spectator.join" => {
            let battle_id = data
                .get("battle_id")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_owned();
            if let Some(battle) = state.battles.get_mut(&battle_id) {
                battle.spectators.insert(sid.to_owned());
                let payload = json!({
                    "battle_id": battle.id,
                    "map_id": battle.map_id,
                    "host": battle.host.snapshot(),
                    "enemy": battle.enemy.snapshot(),
                    "turn": battle.turn,
                    "round": battle.round,
                });
                if let Some(s) = state.sessions.get_mut(sid) {
                    s.spectating_id = Some(battle_id);
                }
                PvpServer::emit_to_sid(&mut state, sid, "Battle.spectatorJoined", payload);
            }
        }
        "Battle.spectator.leave" => {
            if let Some(bid) = state
                .sessions
                .get_mut(sid)
                .and_then(|s| s.spectating_id.take())
                && let Some(battle) = state.battles.get_mut(&bid)
            {
                battle.spectators.remove(sid);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    fn mock_char(id: u64, name: &str, trophy: i32) -> PvpCharacter {
        PvpCharacter {
            id,
            user_id: id,
            name: name.to_owned(),
            level: 40,
            rank: 3,
            gender: 0,
            xp: 10000,
            gold: 50000,
            prestige: 100,
            pvp_played: 10,
            pvp_won: 5,
            pvp_lost: 5,
            pvp_points: 500,
            pvp_trophy: trophy,
            point_wind: 10,
            point_fire: 20,
            point_lightning: 10,
            point_water: 10,
            point_earth: 10,
            point_free: 0,
            skills: vec!["skill_01".to_owned(), "skill_02".to_owned()],
            sets: json!({ "hair_style": "hair_01_m" }),
            elements: vec![1, 2],
            senjutsu_type: None,
        }
    }

    fn test_server() -> PvpServer {
        let cfg = PvpConfig {
            poll_wait: Duration::from_millis(10),
            ..PvpConfig::default()
        };
        let srv = PvpServer::new(None, cfg);
        srv.insert_mock_character("sess_1", mock_char(1, "Naruto", 1000));
        srv.insert_mock_character("sess_2", mock_char(2, "Sasuke", 1000));
        srv
    }

    async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), MAX_BODY).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    async fn open_session(app: &Router) -> String {
        let (status, body) = call(app, "GET", "/socket.io/?EIO=3&transport=polling", "").await;
        assert_eq!(status, StatusCode::OK);
        let pkts = decode_payload(&body, limits()).unwrap();
        let info: JsonValue = serde_json::from_str(&pkts[0].data).unwrap();
        info["sid"].as_str().unwrap().to_owned()
    }

    async fn post_frames(app: &Router, sid: &str, frames: &[EnginePacket]) {
        let uri = format!("/socket.io/?EIO=3&transport=polling&sid={sid}");
        let payload = encode_payload(frames, limits()).unwrap();
        let (status, _) = call(app, "POST", &uri, &payload).await;
        assert_eq!(status, StatusCode::OK);
    }

    async fn poll_events(app: &Router, sid: &str) -> Vec<SocketPacket> {
        let uri = format!("/socket.io/?EIO=3&transport=polling&sid={sid}");
        let (status, body) = call(app, "GET", &uri, "").await;
        assert_eq!(status, StatusCode::OK);
        let pkts = decode_payload(&body, limits()).unwrap();
        pkts.into_iter()
            .filter(|p| p.kind == EngineKind::Message)
            .filter_map(|p| decode_socket_lenient(&p.data, limits()).ok())
            .collect()
    }

    #[tokio::test]
    async fn pvp_auth_matchmaking_combat_and_room_flow() {
        let srv = test_server();
        let app = shared::health_router().merge(router(srv));
        assert_eq!(
            call(&app, "GET", "/health/live", "").await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", "/health/ready", "").await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );

        let sid1 = open_session(&app).await;
        let sid2 = open_session(&app).await;

        post_frames(
            &app,
            &sid1,
            &[
                engine_packet(EngineKind::Message, "0/pvp,"),
                engine_packet(
                    EngineKind::Message,
                    r#"2/pvp,["System.auth",{"id":1,"session_key":"sess_1"}]"#,
                ),
            ],
        )
        .await;
        let ev1 = poll_events(&app, &sid1).await;
        assert!(ev1.iter().any(
            |e| matches!(e, SocketPacket::Event { name, .. } if name == "Client.characterInfo")
        ));

        post_frames(
            &app,
            &sid2,
            &[
                engine_packet(EngineKind::Message, "0/pvp,"),
                engine_packet(
                    EngineKind::Message,
                    r#"2/pvp,["System.auth",{"id":2,"session_key":"sess_2"}]"#,
                ),
            ],
        )
        .await;
        let _ = poll_events(&app, &sid2).await;
        let _ = poll_events(&app, &sid1).await;

        // Matchmaking pairs player 1 and player 2 into a live battle
        post_frames(
            &app,
            &sid1,
            &[engine_packet(
                EngineKind::Message,
                r#"2/pvp,["Battle.startMatchMaking",{"mode":"quick"}]"#,
            )],
        )
        .await;
        post_frames(
            &app,
            &sid2,
            &[engine_packet(
                EngineKind::Message,
                r#"2/pvp,["Battle.startMatchMaking",{"mode":"quick"}]"#,
            )],
        )
        .await;

        let ev1 = poll_events(&app, &sid1).await;
        assert!(
            ev1.iter()
                .any(|e| matches!(e, SocketPacket::Event { name, .. } if name == "Battle.started"))
        );
        let _ = poll_events(&app, &sid2).await;

        // Host (sid1) uses skill_01
        post_frames(
            &app,
            &sid1,
            &[engine_packet(
                EngineKind::Message,
                r#"2/pvp,["Battle.action.skill",{"skill_id":"skill_01"}]"#,
            )],
        )
        .await;
        let ev1 = poll_events(&app, &sid1).await;
        assert!(ev1.iter().any(
            |e| matches!(e, SocketPacket::Event { name, .. } if name == "Battle.actionResult")
        ));
        let _ = poll_events(&app, &sid2).await;

        // Enemy (sid2) runs -> Host wins and Battle.ended is emitted
        post_frames(
            &app,
            &sid2,
            &[engine_packet(
                EngineKind::Message,
                r#"2/pvp,["Battle.action.run",{}]"#,
            )],
        )
        .await;
        let ev1 = poll_events(&app, &sid1).await;
        assert!(ev1.iter().any(|e| matches!(
            e,
            SocketPacket::Event { name, args, .. }
            if name == "Battle.ended" && args[0]["winner_id"] == 1
        )));
    }
}
