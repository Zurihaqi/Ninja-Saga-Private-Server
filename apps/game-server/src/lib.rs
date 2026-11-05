//! Legacy AMF gateway, persistent gameplay services and local client asset delivery.
use amf_codec::{Body, Envelope, Limits, Value};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
mod academy;
mod account;
mod admin;
mod animations;
mod billing;
mod characters;
mod clan_http;
mod consumables;
mod crafting;
mod crew_http;
mod daily;
mod economy;
mod events;
mod exams;
mod friends;
mod gear_presets;
mod hunting;
mod inventory;
mod mail;
mod markets;
mod missions;
mod pet_actions;
mod pets;
mod recruits;
mod resets;
mod rewards;
mod seasonal;
mod senjutsu;
mod shadow_war;
mod skillsets;
mod talents;
mod wisdom;
use persistence::MySqlPool;

type SharedMap<K, V> = std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<K, V>>>;

#[derive(Clone, Default)]
pub struct GameState {
    pub database: Option<MySqlPool>,
    battles: SharedMap<i64, missions::Battle>,
    hunting_battles: SharedMap<i64, hunting::HuntingBattle>,
    eudemon_battles: SharedMap<i64, hunting::HuntingBattle>,
    pet_boosts: SharedMap<i64, i64>,
    shadow_war_battles: SharedMap<i64, shadow_war::ShadowWarBattle>,
    shadow_war_presets: SharedMap<i64, (i64, Vec<serde_json::Value>)>,
    shadow_war_enemies: SharedMap<i64, Vec<serde_json::Value>>,
    event_battles: SharedMap<(i64, String), seasonal::EventBattle>,
    gacha_histories: SharedMap<String, Vec<serde_json::Value>>,
}
impl GameState {
    pub fn with_database(database: Option<MySqlPool>) -> Self {
        Self {
            database,
            ..Self::default()
        }
    }
}

pub fn router() -> Router {
    configured_router(GameState::default())
}

pub fn configured_router(state: GameState) -> Router {
    let mut router = Router::new()
        .route("/amf", any(gateway))
        .route("/gateway.php", any(gateway))
        .route("/api/id-custom-color", axum::routing::get(|| async {axum::Json(serde_json::json!({"data":{}}))}))
        .route("/crossdomain.xml", axum::routing::get(|| async { ([(header::CONTENT_TYPE,"application/xml")],"<cross-domain-policy><allow-access-from domain=\"127.0.0.1\" secure=\"false\"/><allow-access-from domain=\"localhost\" secure=\"false\"/></cross-domain-policy>") }))
        .route("/admin", axum::routing::get(|| async { axum::response::Redirect::temporary("/app/#admin") }))
        .merge(admin::router())
        .merge(clan_http::router())
        .merge(crew_http::router())
        .layer(DefaultBodyLimit::max(Limits::default().max_bytes))
        .with_state(state);
    if let Ok(root) = std::env::var("GAME_DATA_ROOT") {
        router = router.nest_service("/game_data", tower_http::services::ServeDir::new(root));
    }
    // Self-hosted web client (React + Ruffle WASM build). Served same-origin
    // alongside the SWF gateway so the movie needs no CORS or spoofing.
    if let Ok(root) = std::env::var("WEB_CLIENT_ROOT") {
        router = router.nest_service("/app", tower_http::services::ServeDir::new(root));
    }
    if let Ok(root) = std::env::var("CLIENT_ASSET_ROOT") {
        router = router
            .nest_service("/client", tower_http::services::ServeDir::new(&root))
            .fallback_service(tower_http::services::ServeDir::new(root));
    }
    router
}

async fn gateway(State(state): State<GameState>, bytes: Bytes) -> Response {
    if bytes.is_empty() {
        return "AMF Gateway Ready".into_response();
    }
    let request = match amf_codec::decode(&bytes, Limits::default()) {
        Ok(request) => request,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid AMF Data: {error}"),
            )
                .into_response();
        }
    };
    let wrap = request.version == 3;
    let mut bodies = Vec::new();
    for body in request.bodies {
        let data = if body.target.ends_with(".getVersion") {
            dispatch(&body.target, wrap)
        } else {
            let args = account::arguments(&body.data);
            let result = account::dispatch(&state, &body.target, &args).await;
            account::to_amf(&result, wrap)
        };
        bodies.push(Body {
            target: format!("{}/onResult", body.response),
            response: String::new(),
            data: if wrap {
                Value::Amf3(Box::new(data))
            } else {
                data
            },
        });
    }
    match amf_codec::encode(
        &Envelope {
            version: 0,
            headers: vec![],
            bodies,
        },
        Limits::default(),
    ) {
        Ok(output) => ([(header::CONTENT_TYPE, "application/x-amf")], output).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AMF response encoding failed",
        )
            .into_response(),
    }
}

fn dispatch(target: &str, amf3: bool) -> Value {
    // Source: app/Services/Amf/GameService/VersionService.php, pinned reference commit.
    let pairs = if matches!(
        target,
        "Game.getVersion" | "GameService.getVersion" | "game.getVersion" | "gameService.getVersion"
    ) {
        vec![
            ("version".into(), Value::String("1.0.0".into())),
            (
                "server".into(),
                Value::String("Laravel Ninja Saga Revived".into()),
            ),
            ("status".into(), Value::String("active".into())),
        ]
    } else {
        // Explicit spike limitation; not a fabricated gameplay implementation.
        vec![
            (
                "status".into(),
                if amf3 {
                    Value::Integer(500)
                } else {
                    Value::Number(500.0)
                },
            ),
            (
                "error".into(),
                Value::String(format!(
                    "{target} failed: method not implemented in Rust compatibility spike"
                )),
            ),
        ]
    };
    if amf3 {
        Value::Object(pairs)
    } else {
        Value::MixedArray(pairs)
    }
}

#[cfg(test)]
mod tests {
    mod client_panels;
    mod extension;
    mod free_products;
    mod pet_villa_repair;
    mod player_actions;
    mod skills_repair;
    mod social_repair;
    use super::*;
    use axum::{
        body::{Body as HttpBody, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    async fn amf_call(app: &Router, target: &str, args: serde_json::Value) -> serde_json::Value {
        let request = Envelope {
            version: 3,
            headers: vec![],
            bodies: vec![amf_codec::Body {
                target: target.into(),
                response: "/1".into(),
                data: Value::Amf3(Box::new(account::to_amf(&args, true))),
            }],
        };
        let bytes = amf_codec::encode(&request, Limits::default()).unwrap();
        let response = app
            .clone()
            .oneshot(Request::post("/amf").body(HttpBody::from(bytes)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let message = amf_codec::decode(&bytes, Limits::default()).unwrap();
        account::arguments(&Value::Array(vec![message.bodies[0].data.clone()])).remove(0)
    }

    #[tokio::test]
    #[ignore = "requires isolated migrated MySQL DATABASE_URL"]
    async fn persistent_account_character_and_ownership_flow() {
        use persistence::sqlx;
        use serde_json::json;
        let pool = persistence::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let database: String = sqlx::query_scalar("SELECT DATABASE()")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            matches!(
                database.as_str(),
                "ninja_ci_test" | "ninja_ci_reference_test"
            ),
            "Refusing gameplay regression outside isolated test schemas"
        );
        if std::env::var("BOOTSTRAP_GAMEPLAY_TEST_DB").as_deref() == Ok("1") {
            let mut connection = pool.acquire().await.unwrap();
            sqlx::query("SET FOREIGN_KEY_CHECKS=0")
                .execute(&mut *connection)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!("../../../database/schema/mysql.sql"))
                .execute(&mut *connection)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!(
                "../../../database/migrations/001_pet_skills.sql"
            ))
            .execute(&mut *connection)
            .await
            .unwrap();
            sqlx::raw_sql(include_str!(
                "../../../database/migrations/002_gear_presets.sql"
            ))
            .execute(&mut *connection)
            .await
            .unwrap();
            sqlx::raw_sql(include_str!(
                "../../../database/migrations/003_free_products.sql"
            ))
            .execute(&mut *connection)
            .await
            .unwrap();
            sqlx::query("SET FOREIGN_KEY_CHECKS=1")
                .execute(&mut *connection)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!("../../../database/fixtures/gameplay.sql"))
                .execute(&mut *connection)
                .await
                .unwrap();
        }
        let app = configured_router(GameState {
            database: Some(pool.clone()),
            ..GameState::default()
        });
        let username = format!("test_{}", authentication::random_hex().unwrap());
        let registered = amf_call(
            &app,
            "SystemLogin.registerUser",
            json!([
                username,
                format!("{username}@example.invalid"),
                "NinjaTest123!",
                ""
            ]),
        )
        .await;
        assert_eq!(registered["status"], 1, "{registered}");
        assert_eq!(
            amf_call(
                &app,
                "SystemLogin.registerUser",
                json!([username, "other@example.invalid", "NinjaTest123!", ""])
            )
            .await["status"],
            2
        );
        let login = amf_call(
            &app,
            "SystemLogin.loginUser",
            json!([
                username,
                "jo+n7pAgPZwhDMCN9reD8w==",
                "123456",
                0,
                0,
                "0123456789abcdef",
                0,
                0,
                12
            ]),
        )
        .await;
        assert_eq!(login["status"], 1, "{login}");
        let uid = login["uid"].as_u64().unwrap();
        let key = login["sessionkey"].as_str().unwrap();
        assert_eq!(
            amf_call(
                &app,
                "SystemLogin.loginUser",
                json!([
                    username,
                    "invalid",
                    "123456",
                    0,
                    0,
                    "0123456789abcdef",
                    0,
                    0,
                    12
                ])
            )
            .await["status"],
            2
        );
        // Actual AIR CharacterCreate wraps its params, and NetConnection wraps again.
        let created = amf_call(
            &app,
            "CharacterService.characterRegister",
            json!([[[
                uid,
                authentication::registration_hash(key),
                username,
                0,
                1,
                "0|0",
                1
            ]]]),
        )
        .await;
        assert_eq!(created["status"], 1, "{created}");
        let list = amf_call(
            &app,
            "SystemLogin.getAllCharacters",
            json!([uid, login["hash"]]),
        )
        .await;
        assert_eq!(list["total_characters"], 1, "{list}");
        let id = list["account_data"][0]["char_id"].as_u64().unwrap();
        let loaded = amf_call(&app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(loaded["status"], 1, "{loaded}");
        assert_eq!(loaded["character_inventory"]["char_skills"], "skill_13");
        assert_eq!(loaded["character_data"]["character_gold"], 1000);
        let info = amf_call(
            &app,
            "CharacterService.getInfo",
            serde_json::json!([id, key, id, "char"]),
        )
        .await;
        assert_eq!(info["status"], 1, "{info}");
        assert_eq!(info["character_sets"]["skills"], "skill_13");
        assert_eq!(
            amf_call(&app, "SystemLogin.getCharacterData", json!([id, "wrong"])).await["error"],
            "Invalid session key"
        );
        let equip = amf_call(
            &app,
            "CharacterService.equipSet",
            json!([
                id,
                key,
                "wpn_01",
                "back_01",
                "set_01_0",
                "accessory_01",
                "hair_01_0",
                "1|2",
                "3|4"
            ]),
        )
        .await;
        assert_eq!(equip["status"], 1, "{equip}");
        assert_eq!(
            amf_call(
                &app,
                "CharacterService.equipSet",
                json!([
                    id,
                    key,
                    "wpn_NOT_OWNED",
                    "back_01",
                    "set_01_0",
                    "accessory_01",
                    "hair_01_0",
                    "1|2",
                    "3|4"
                ])
            )
            .await["status"],
            2
        );
        let reloaded = amf_call(&app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(reloaded["character_sets"]["hair_color"], "1|2");
        let allocated = amf_call(
            &app,
            "CharacterService.setPoints",
            json!([[id, key, 1, 0, 0, 0, 0, 0]]),
        )
        .await;
        assert_eq!(allocated, json!({"status":1,"error":0}));
        for invalid in [
            json!([id, key, 2, 0, 0, 0, 0, 0]),
            json!([id, key, -1, 0, 0, 0, 0, 2]),
            json!([id, key, 0.5, 0, 0, 0, 0, 0.5]),
            json!([id, key, 1, 0, 0, 0, 0]),
            json!([id, key, null, 0, 0, 0, 0, 1]),
        ] {
            assert_eq!(
                amf_call(&app, "CharacterService.setPoints", invalid).await["status"],
                2
            );
        }
        assert_eq!(
            amf_call(
                &app,
                "CharacterService.setPoints",
                json!([id, "wrong", 0, 1, 0, 0, 0, 0])
            )
            .await["status"],
            0
        );
        let allocated = amf_call(&app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(allocated["character_points"]["atrrib_wind"], 1);
        assert_eq!(allocated["character_points"]["atrrib_free"], 0);
        let enemy = sqlx::query("SELECT hp,agility FROM enemies WHERE enemy_id='ene_01'")
            .fetch_one(&pool)
            .await
            .unwrap();
        use persistence::Row;
        let stats = format!(
            "id:ene_01|hp:{}|agility:{}",
            enemy.get::<i32, _>("hp"),
            enemy.get::<i32, _>("agility")
        );
        let bad_start = amf_call(
            &app,
            "BattleSystem.startMission",
            json!([id, "msn_1", "ene_fake", stats, "", "", key]),
        )
        .await;
        assert_eq!(bad_start["error"], "Invalid enemies");
        let token = amf_call(
            &app,
            "BattleSystem.startMission",
            json!([id, "msn_1", "ene_01", stats, "", "", key]),
        )
        .await;
        assert!(token.is_string(), "{token}");
        assert_eq!(
            token.as_str().unwrap().len(),
            10,
            "AIR requires a ten-character battle token"
        );
        let finish_args = json!([id, "msn_1", token, "", 0, key, [], null]);
        let (first, duplicate) = tokio::join!(
            amf_call(&app, "BattleSystem.finishMission", finish_args.clone()),
            amf_call(&app, "BattleSystem.finishMission", finish_args)
        );
        assert!(
            first["status"] == 1 || duplicate["status"] == 1,
            "{first} / {duplicate}"
        );
        assert!(
            first["status"] == 0 || duplicate["status"] == 0,
            "Duplicate rewards accepted"
        );
        let after_mission = amf_call(&app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(after_mission["character_data"]["character_gold"], 1020);
        assert_eq!(after_mission["character_data"]["character_xp"], 20);
        assert_eq!(after_mission["character_data"]["character_level"], 2);
        assert_eq!(
            amf_call(
                &app,
                "CharacterService.setPoints",
                json!([id, key, 1, 0, 0, 0, 0, 1])
            )
            .await["status"],
            1
        );
        // The real profile permits reset/reallocation; repeat saves must stay idempotent.
        for _ in 0..2 {
            assert_eq!(
                amf_call(
                    &app,
                    "CharacterService.setPoints",
                    json!([id, key, 0, 2, 0, 0, 0, 0])
                )
                .await["status"],
                1
            );
        }
        // A new login rotates the persistent key; stale sessions cannot load the character.
        let login2 = amf_call(
            &app,
            "SystemLogin.loginUser",
            json!([
                username,
                "jo+n7pAgPZwhDMCN9reD8w==",
                "123456",
                0,
                0,
                "0123456789abcdef",
                0,
                0,
                12
            ]),
        )
        .await;
        assert_eq!(login2["status"], 1);
        assert_eq!(
            amf_call(&app, "SystemLogin.getCharacterData", json!([id, key])).await["status"],
            0
        );
        assert_eq!(
            amf_call(
                &app,
                "SystemLogin.getCharacterData",
                json!([id, login2["sessionkey_b64"]])
            )
            .await["status"],
            1
        );
        let fresh_app = configured_router(GameState::with_database(Some(pool.clone())));
        let persisted = amf_call(
            &fresh_app,
            "SystemLogin.getCharacterData",
            json!([id, login2["sessionkey"]]),
        )
        .await;
        assert_eq!(persisted["character_data"]["character_gold"], 1020);
        assert_eq!(persisted["character_points"]["atrrib_fire"], 2);
        assert_eq!(persisted["character_points"]["atrrib_wind"], 0);
        assert_eq!(persisted["character_points"]["atrrib_free"], 0);
        assert_eq!(
            amf_call(
                &fresh_app,
                "BattleSystem.finishMission",
                json!([id, "msn_1", token, "", 0, login2["sessionkey"], [], null])
            )
            .await["status"],
            0
        );
        // Complete ordinary shop -> equip -> sell and academy -> equip -> preset flows.
        let key = login2["sessionkey"].as_str().unwrap();
        let item=sqlx::query("SELECT item_id,price_gold FROM items WHERE category='weapon' AND item_id<>'wpn_01' AND level<=2 AND premium=0 AND price_tokens=0 AND price_gold>0 AND price_gold<=500 ORDER BY price_gold,item_id LIMIT 1").fetch_one(&pool).await.unwrap();
        let item_id: String = item.get("item_id");
        let price = i64::from(item.get::<i32, _>("price_gold"));
        let bought = amf_call(
            &fresh_app,
            "CharacterService.buyItem",
            json!([[id, key, item_id, 2]]),
        )
        .await;
        assert_eq!(bought["status"], 1, "{bought}");
        assert_eq!(bought["data"]["character_gold"], 1020 - price * 2);
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.buyItem",
                json!([id, key, item_id, 1000000])
            )
            .await["status"],
            3
        );
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.equipSet",
                json!([
                    id,
                    key,
                    item_id,
                    "back_01",
                    "set_01_0",
                    "accessory_01",
                    "hair_01_0",
                    "1|2",
                    "3|4"
                ])
            )
            .await["status"],
            1
        );
        let equipped = amf_call(&fresh_app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(equipped["character_sets"]["weapon"], item_id);
        assert!(
            equipped["character_inventory"]["char_weapons"]
                .as_str()
                .unwrap()
                .contains(&format!("{item_id}:2"))
        );
        let sold = amf_call(
            &fresh_app,
            "CharacterService.sellItem",
            json!([[id, key, item_id, 1]]),
        )
        .await;
        let after_sale = 1020 - price * 2 + price / 2;
        assert_eq!(sold["status"], 1, "{sold}");
        assert_eq!(sold["data"]["character_gold"], after_sale);
        let skill=sqlx::query("SELECT skill_id,price_gold FROM skills WHERE element=2 AND level<=2 AND premium=0 AND price_tokens=0 AND price_gold<=100 ORDER BY price_gold,skill_id LIMIT 1").fetch_one(&pool).await.unwrap();
        let skill_id: String = skill.get("skill_id");
        let skill_price = i64::from(skill.get::<i32, _>("price_gold"));
        let learned = amf_call(
            &fresh_app,
            "CharacterService.buySkill",
            json!([[key, id, skill_id]]),
        )
        .await;
        assert_eq!(learned["status"], 1, "{learned}");
        assert_eq!(learned["data"]["character_gold"], after_sale - skill_price);
        assert_eq!(learned["data"]["character_element_2"], 2);
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.buySkill",
                json!([key, id, skill_id])
            )
            .await["status"],
            2
        );
        let skills = format!("skill_13,{skill_id}");
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.equipSkillSet",
                json!([id, key, skills])
            )
            .await["status"],
            1
        );
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.getSkillSets",
                json!([id, key])
            )
            .await["skillsets"],
            json!([])
        );
        assert_eq!(
            amf_call(
                &fresh_app,
                "CharacterService.createSkillSet",
                json!([id, key])
            )
            .await["status"],
            1
        );
        sqlx::query("UPDATE users SET account_type=1,tokens=GREATEST(tokens,200) WHERE id=?")
            .bind(uid)
            .execute(&pool)
            .await
            .unwrap();
        let preset = amf_call(
            &fresh_app,
            "CharacterService.createSkillSet",
            json!([id, key]),
        )
        .await;
        assert_eq!(preset["status"], 1, "{preset}");
        let preset_id = preset["skillsets"][0]["id"].as_u64().unwrap();
        let saved = amf_call(
            &fresh_app,
            "CharacterService.saveSkillSet",
            json!([id, key, preset_id, skill_id]),
        )
        .await;
        assert_eq!(saved["status"], 1, "{saved}");
        assert_eq!(saved["skillsets"][0]["skills"], skill_id);
        let final_app = configured_router(GameState::with_database(Some(pool.clone())));
        let final_state =
            amf_call(&final_app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(final_state["character_sets"]["skills"], skill_id);
        assert_eq!(
            final_state["character_data"]["character_gold"],
            after_sale - skill_price
        );
        assert!(
            final_state["character_inventory"]["char_skills"]
                .as_str()
                .unwrap()
                .split(',')
                .any(|s| s == skill_id)
        );
        extension::progression_and_companions(&final_app, &pool, id, key).await;
        social_repair::run(&final_app, &pool, id, key).await;
        free_products::run(&final_app, &pool, id, key).await;
        sqlx::query("DELETE FROM users WHERE id=?")
            .bind(uid)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn gateway_matches_reference_php_bytes() {
        let cases: &[(&[u8], &[u8])] = &[
            (
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf0-request.amf"
                ),
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf0-response.amf"
                ),
            ),
            (
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf3-request.amf"
                ),
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf3-response.amf"
                ),
            ),
        ];
        for path in ["/amf", "/gateway.php"] {
            for (request, expected) in cases {
                let response = router()
                    .oneshot(
                        Request::post(path)
                            .body(HttpBody::from(request.to_vec()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(
                    response.headers()[header::CONTENT_TYPE],
                    "application/x-amf"
                );
                assert_eq!(
                    to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
                    *expected
                );
            }
        }
    }

    #[tokio::test]
    async fn both_routes_preserve_amf3_wrapper_and_multiple_response_targets() {
        for path in ["/amf", "/gateway.php"] {
            for version in [0, 3] {
                let input = Envelope {
                    version,
                    headers: vec![],
                    bodies: vec![
                        Body {
                            target: "Game.getVersion".into(),
                            response: "/1".into(),
                            data: Value::Array(vec![]),
                        },
                        Body {
                            target: "Unknown.method".into(),
                            response: "/2".into(),
                            data: Value::Array(vec![]),
                        },
                    ],
                };
                let bytes = amf_codec::encode(&input, Limits::default()).unwrap();
                let response = router()
                    .oneshot(Request::post(path).body(HttpBody::from(bytes)).unwrap())
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(
                    response.headers()[header::CONTENT_TYPE],
                    "application/x-amf"
                );
                let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
                let output = amf_codec::decode(&bytes, Limits::default()).unwrap();
                assert_eq!(output.version, 0);
                assert_eq!(output.bodies.len(), 2);
                assert_eq!(output.bodies[0].target, "/1/onResult");
                assert_eq!(output.bodies[1].target, "/2/onResult");
                assert_eq!(
                    output.bodies[0].data,
                    if version == 3 {
                        Value::Amf3(Box::new(dispatch("Game.getVersion", true)))
                    } else {
                        dispatch("Game.getVersion", false)
                    }
                );
            }
        }
    }

    #[tokio::test]
    async fn malformed_and_oversized_requests_are_rejected() {
        for (data, status) in [
            (vec![0, 3, 0], StatusCode::INTERNAL_SERVER_ERROR),
            (vec![0; 1024 * 1024 + 1], StatusCode::PAYLOAD_TOO_LARGE),
        ] {
            let response = router()
                .oneshot(Request::post("/amf").body(HttpBody::from(data)).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
        }
    }
}
