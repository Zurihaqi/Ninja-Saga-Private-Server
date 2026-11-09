//! Shared MySQL pool. Schema is imported from the audited reference, never auto-mutated at startup.
use sqlx::mysql::MySqlPoolOptions;
pub use sqlx::{self, MySqlPool, Row};
use sqlx::{Column, TypeInfo, ValueRef};
use std::time::Duration;

pub fn row_json(row: &sqlx::mysql::MySqlRow) -> Result<serde_json::Value, sqlx::Error> {
    let mut object = serde_json::Map::new();
    for column in row.columns() {
        let name = column.name();
        let value = if row.try_get_raw(name)?.is_null() {
            serde_json::Value::Null
        } else {
            match column.type_info().name() {
                "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" | "BIGINT" => {
                    serde_json::json!(row.try_get::<i64, _>(name)?)
                }
                "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED"
                | "MEDIUMINT UNSIGNED" | "BIGINT UNSIGNED" => {
                    serde_json::json!(row.try_get::<u64, _>(name)?)
                }
                "JSON" => row.try_get::<serde_json::Value, _>(name)?,
                "BOOLEAN" => serde_json::json!(row.try_get::<bool, _>(name)?),
                "FLOAT" | "DOUBLE" => serde_json::json!(row.try_get::<f64, _>(name)?),
                "DATE" | "DATETIME" | "TIMESTAMP" => serde_json::Value::Null,
                _ => serde_json::json!(row.try_get::<String, _>(name)?),
            }
        };
        object.insert(name.to_owned(), value);
    }
    Ok(serde_json::Value::Object(object))
}

pub async fn connect(url: &str) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
}
