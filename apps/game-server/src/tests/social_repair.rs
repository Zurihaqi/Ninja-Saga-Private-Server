//! REST requests and fields used by LinkMenu, Clan and Crew in the shipped client.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use persistence::{MySqlPool, sqlx};
use serde_json::{Value as Json, json};
use tower::ServiceExt;

async fn post(app: &Router, path: &str, token: Option<&str>, body: Json) -> (StatusCode, Json) {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn run(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let uid: u64 = sqlx::query_scalar("SELECT user_id FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let name = format!("social_{}", authentication::random_hex().unwrap());
    let friend=sqlx::query("INSERT INTO characters(user_id,name,level,`rank`,gold,created_at,updated_at) VALUES(?,?,85,9,10000,NOW(),NOW())").bind(uid).bind(&name).execute(pool).await.unwrap().last_insert_id();
    // Both characters belong to the dedicated synthetic regression account.
    for kind in ["clan", "crew"] {
        let (status, login) = post(
            app,
            &format!("/{kind}/auth/login"),
            None,
            json!({"char_id":id,"session_key":key}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{login}");
        let token = login["access_token"].as_str().unwrap();
        assert_eq!(token.len(), 64);
        let (status, _) = post(
            app,
            &format!("/{kind}/auth/login"),
            None,
            json!({"char_id":id,"session_key":"wrong"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let group_name = format!("{}{}", kind, &authentication::random_hex().unwrap()[..8]);
        let (_, created) = post(
            app,
            &format!("/{kind}/create"),
            Some(token),
            json!({"name":group_name}),
        )
        .await;
        assert!(created == "ok" || created["status"] == "ok", "{created}");
        let (_, data) = post(
            app,
            &format!("/{kind}/player/{kind}"),
            Some(token),
            json!({}),
        )
        .await;
        assert_eq!(data[kind]["name"], group_name, "{data}");
        assert_eq!(
            data["char"]["char_id"], id,
            "LinkMenu requires char: {data}"
        );
        assert!(data["char"]["stamina"].as_u64().unwrap() > 0);
        let group = data[kind]["id"].as_u64().unwrap();
        let (_, stamina) = post(
            app,
            &format!("/{kind}/player/stamina"),
            Some(token),
            json!({}),
        )
        .await;
        assert_eq!(stamina["char"]["char_id"], id, "{stamina}");
        let (_, friend_login) = post(
            app,
            &format!("/{kind}/auth/login"),
            None,
            json!({"char_id":friend,"session_key":key}),
        )
        .await;
        let friend_token = friend_login["access_token"].as_str().unwrap();
        let (_, empty) = post(
            app,
            &format!("/{kind}/player/{kind}"),
            Some(friend_token),
            json!({}),
        )
        .await;
        if kind == "crew" {
            assert_eq!(empty["code"], 404, "{empty}");
        }
        let (_, available) = post(
            app,
            &format!("/{kind}/request/available"),
            Some(friend_token),
            json!({}),
        )
        .await;
        let plural = format!("{kind}s");
        assert!(
            available[&plural]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["id"] == group),
            "{available}"
        );
        let (_, sent) = post(
            app,
            &format!("/{kind}/player/request/{group}"),
            Some(friend_token),
            json!({}),
        )
        .await;
        assert!(sent.get("errorMessage").is_none(), "{sent}");
        let (_, requests) =
            post(app, &format!("/{kind}/request/all"), Some(token), json!({})).await;
        assert_eq!(requests["requests"][0]["char_id"], friend, "{requests}");
        let (_, accepted) = post(
            app,
            &format!("/{kind}/request/{friend}/accept"),
            Some(token),
            json!({}),
        )
        .await;
        assert!(accepted.get("errorMessage").is_none(), "{accepted}");
        let member_path = if kind == "clan" {
            "/clan/player/clan-members"
        } else {
            "/crew/player/crew/members"
        };
        let (_, members) = post(app, member_path, Some(token), json!({})).await;
        assert_eq!(members["members"].as_array().unwrap().len(), 2, "{members}");
        let (_, joined) = post(
            app,
            &format!("/{kind}/player/{kind}"),
            Some(friend_token),
            json!({}),
        )
        .await;
        assert_eq!(joined[kind]["id"], group);
        assert_eq!(
            joined["char"]["role"],
            if kind == "clan" {
                json!("member")
            } else {
                json!(1)
            }
        );
        if kind == "crew" {
            crew_battle_flow(app, pool, id, token, friend, friend_token, group).await;
        } else {
            clan_prestige_flow(app, pool, id, key, token, friend, group).await;
        }
        let (_, donated) = post(
            app,
            &format!("/{kind}/player/donate/100/golds"),
            Some(token),
            json!({}),
        )
        .await;
        assert!(donated.get("errorMessage").is_none(), "{donated}");
        let gold: i64 = sqlx::query_scalar(&format!(
            "SELECT CAST(gold AS SIGNED) FROM {kind}s WHERE id=?"
        ))
        .bind(group)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(gold, 100);
        let (_, left) = post(
            app,
            &format!("/{kind}/player/quit"),
            Some(friend_token),
            json!({}),
        )
        .await;
        assert!(left.get("errorMessage").is_none(), "{left}");
        let (_, members) = post(app, member_path, Some(token), json!({})).await;
        assert_eq!(members["members"].as_array().unwrap().len(), 1);
        sqlx::query(&format!("DELETE FROM {kind}s WHERE id=?"))
            .bind(group)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(&format!(
            "DELETE FROM {kind}_auth_tokens WHERE character_id IN (?,?)"
        ))
        .bind(id)
        .bind(friend)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("DELETE FROM characters WHERE id=?")
        .bind(friend)
        .execute(pool)
        .await
        .unwrap();
}

async fn crew_battle_flow(
    app: &Router,
    pool: &MySqlPool,
    id: u64,
    token: &str,
    friend: u64,
    friend_token: &str,
    crew: u64,
) {
    // Minimal CI fixtures omit this boss; original catalogs already contain it.
    sqlx::query("INSERT IGNORE INTO enemies(enemy_id,name,level,hp,cp,agility) VALUES('ene_471','Crew regression Wyvern',1,100,100,5)").execute(pool).await.unwrap();
    let season: u64 = sqlx::query_scalar("SELECT season_id FROM crews WHERE id=?")
        .bind(crew)
        .fetch_one(pool)
        .await
        .unwrap();
    let old_phase: u8 = sqlx::query_scalar("SELECT phase FROM crew_seasons WHERE id=?")
        .bind(season)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE crew_seasons SET phase=1 WHERE id=?")
        .bind(season)
        .execute(pool)
        .await
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let nonce = authentication::random_hex().unwrap();
    let time = now * 1000;
    let hash = authentication::hashes(id, &format!("|{nonce}|{time}|1|ene_471")).1;
    let request = json!({"b":nonce,"t":time,"c":1,"e":"ene_471","f":[friend],"h":hash});
    let (_, start) = post(
        app,
        "/crew/battle/phase1/start",
        Some(token),
        request.clone(),
    )
    .await;
    assert!(start["b"].as_str().is_some(), "{start}");
    assert_eq!(start["f"], json!([friend]));
    let (_, again) = post(app, "/crew/battle/phase1/start", Some(token), request).await;
    assert_eq!(start, again);
    let stamina: i32 = sqlx::query_scalar("SELECT stamina FROM crew_members WHERE character_id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(stamina, 40);
    let code = start["b"].as_str().unwrap();
    let time = start["t"].as_i64().unwrap();
    let stats = "e30=";
    let hash = authentication::hashes(id, &format!("|{code}|{time}|1|{stats}")).1;
    let finish = json!({"b":code,"c":1,"t":time,"s":stats,"h":hash,"f":[friend]});
    let (_, wrong_owner) = post(
        app,
        "/crew/battle/phase1/finish",
        Some(friend_token),
        finish.clone(),
    )
    .await;
    assert!(wrong_owner.get("errorMessage").is_some());
    let (_, result) = post(
        app,
        "/crew/battle/phase1/finish",
        Some(token),
        finish.clone(),
    )
    .await;
    assert!(result["data"]["m"].as_i64().unwrap() > 0, "{result}");
    let (_, replay) = post(app, "/crew/battle/phase1/finish", Some(token), finish).await;
    assert_eq!(result, replay);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM crew_battles WHERE attacker_crew_id=? AND battle_data IS NOT NULL"
        )
        .bind(crew)
        .fetch_one(pool)
        .await
        .unwrap(),
        1
    );
    let (_, roster) = post(app, "/crew/player/crew/members", Some(token), json!({})).await;
    let member = roster["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["char_id"] == id)
        .unwrap();
    assert_eq!(member["damage"], result["data"]["d"]);
    assert_eq!(member["boss_kill"], 1);
    sqlx::query("UPDATE crew_seasons SET phase=2 WHERE id=?")
        .bind(season)
        .execute(pool)
        .await
        .unwrap();
    let castle: u64 = sqlx::query_scalar(
        "SELECT id FROM crew_castles WHERE season_id=? ORDER BY castle_index LIMIT 1",
    )
    .bind(season)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE crew_castles SET wall_hp=1,owner_crew_id=NULL WHERE id=?")
        .bind(castle)
        .execute(pool)
        .await
        .unwrap();
    let nonce = authentication::random_hex().unwrap();
    let hash = authentication::hashes(id, &format!("|{nonce}|{now}|{castle}")).1;
    let request = json!({"b":nonce,"t":now,"c":castle,"h":hash});
    let (_, capture) = post(
        app,
        "/crew/battle/phase2/start",
        Some(token),
        request.clone(),
    )
    .await;
    assert_eq!(capture["b"], true, "{capture}");
    assert_eq!(capture["s"], 30);
    let (_, replay) = post(
        app,
        "/crew/battle/phase2/start",
        Some(token),
        request.clone(),
    )
    .await;
    assert_eq!(capture, replay);
    let (_, finished) = post(app, "/crew/battle/phase2/finish", Some(token), request).await;
    assert_eq!(capture, finished);
    assert_eq!(
        sqlx::query_scalar::<_, Option<u64>>("SELECT owner_crew_id FROM crew_castles WHERE id=?")
            .bind(castle)
            .fetch_one(pool)
            .await
            .unwrap(),
        Some(crew)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT stamina FROM crew_members WHERE character_id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        30
    );
    // The disposable regression must not alter the season phase for later flows.
    sqlx::query(
        "UPDATE crew_castles SET owner_crew_id=NULL,wall_hp=100,defender_hp=100 WHERE id=?",
    )
    .bind(castle)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE crew_seasons SET phase=? WHERE id=?")
        .bind(old_phase)
        .bind(season)
        .execute(pool)
        .await
        .unwrap();
}

async fn clan_prestige_flow(
    app: &Router,
    pool: &MySqlPool,
    id: u64,
    key: &str,
    token: &str,
    friend: u64,
    clan: u64,
) {
    let initial: i32 = sqlx::query_scalar("SELECT prestige FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let season: Option<u64> = sqlx::query_scalar("SELECT season_id FROM clans WHERE id=?")
        .bind(clan)
        .fetch_one(pool)
        .await
        .unwrap();
    let opponent=sqlx::query("INSERT INTO clans(season_id,name,master_id,prestige,max_members,buildings,created_at,updated_at) VALUES(?,?,?,100,20,'{}',NOW(),NOW())").bind(season).bind(format!("opponent_{}",authentication::random_hex().unwrap())).bind(friend).execute(pool).await.unwrap().last_insert_id();
    sqlx::query("UPDATE clan_members SET stamina=40,max_stamina=50 WHERE character_id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let (_, lost) = post(
        app,
        &format!("/clan/battle/quick/{opponent}"),
        Some(token),
        json!({"code":"regression-loss"}),
    )
    .await;
    assert_eq!(lost["prestige"], 1, "{lost}");
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT prestige FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        initial + 1
    );
    sqlx::query("UPDATE clans SET prestige=0 WHERE id=?")
        .bind(opponent)
        .execute(pool)
        .await
        .unwrap();
    let (_, won) = post(
        app,
        &format!("/clan/battle/quick/{opponent}"),
        Some(token),
        json!({"code":"regression-win"}),
    )
    .await;
    assert_eq!(won["prestige"], 5, "{won}");
    let (_, started) = post(
        app,
        &format!("/clan/battle/manual/start/{opponent}"),
        Some(token),
        json!({"recruits":[]}),
    )
    .await;
    let code = started["battle_id"].as_str().unwrap();
    let hash = authentication::hashes(id, &format!("|{key}|{opponent}|{code}")).1;
    let (_, manual) = post(
        app,
        "/clan/battle/manual/finish",
        Some(token),
        json!({"id":opponent,"stats":"e30=","code":code,"hash":hash}),
    )
    .await;
    assert_eq!(manual["prestige"], 5, "{manual}");
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT prestige FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        initial + 11
    );
    // Both LinkMenu and periodic stamina refresh must expose the saved total.
    let (_, loaded) = post(app, "/clan/player/clan", Some(token), json!({})).await;
    assert_eq!(loaded["char"]["prestige"], initial + 11);
    let (_, refreshed) = post(app, "/clan/player/stamina", Some(token), json!({})).await;
    assert_eq!(refreshed["char"]["prestige"], initial + 11);
    sqlx::query("DELETE FROM clan_battles WHERE attacker_clan_id=? AND defender_clan_id=?")
        .bind(clan)
        .bind(opponent)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM clans WHERE id=?")
        .bind(opponent)
        .execute(pool)
        .await
        .unwrap();
}
