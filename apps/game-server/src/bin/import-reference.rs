//! Explicit local/admin import command, never run automatically by the server.
use persistence::sqlx;
use serde_json::Value;
#[tokio::main]
async fn main() -> shared::ServerResult {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| "Set DATABASE_URL for a fresh target database")?;
    let path = std::env::args()
        .nth(1)
        .ok_or("Pass the local reference catalog JSON export path")?;
    let catalogs: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let pool = persistence::connect(&url)
        .await
        .map_err(|_| "Unable to connect to target database")?;
    let mut connection = pool.acquire().await?;
    sqlx::query("SET FOREIGN_KEY_CHECKS=0")
        .execute(&mut *connection)
        .await?;
    let imported=async {
        sqlx::raw_sql(include_str!("../../../../database/schema/mysql.sql")).execute(&mut *connection).await?;
        sqlx::raw_sql(include_str!("../../../../database/migrations/001_pet_skills.sql")).execute(&mut *connection).await?;
        let mut count=0;
        for table in ["items","skills","pets","missions","npcs","enemies","talents","senjutsus","game_configs","x_p_s"] {
            let rows=catalogs[table].as_array().ok_or_else(||sqlx::Error::Protocol(format!("Missing catalog {table}")))?;
            let mut tx=sqlx::Connection::begin(&mut *connection).await?;
            for row in rows {
                let object=row.as_object().ok_or_else(||sqlx::Error::Protocol("Invalid catalog row".into()))?;
                if object.keys().any(|k|!k.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_')) {return Err(sqlx::Error::Protocol("Invalid column name".into()));}
                let columns=object.keys().map(|k|format!("`{k}`")).collect::<Vec<_>>().join(",");
                let placeholders=vec!["?";object.len()].join(",");
                let update=object.keys().filter(|k|k.as_str()!="id").map(|k|format!("`{k}`=VALUES(`{k}`)")).collect::<Vec<_>>().join(",");
                let statement=format!("INSERT INTO `{table}` ({columns}) VALUES ({placeholders}) ON DUPLICATE KEY UPDATE {update}");
                let mut query=sqlx::query(&statement);
                for value in object.values() {query=query.bind(if value.is_null(){None}else{Some(value.as_str().map(str::to_owned).unwrap_or_else(||value.to_string()))});}
                query.execute(&mut *tx).await?; count+=1;
            }
            tx.commit().await?;
        }
        Ok::<_,sqlx::Error>(count)
    }.await;
    sqlx::query("SET FOREIGN_KEY_CHECKS=1")
        .execute(&mut *connection)
        .await?;
    println!("Imported {} catalog rows into target schema", imported?);
    Ok(())
}
