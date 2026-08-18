use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use tauri::Manager;

// ─── Data transfer types ──────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow, Clone)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct LiquidityRow {
    pub location_id: i64,
    pub location_name: String,
    pub total_isk_value: f64,
    pub stack_count: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct AssetRow {
    pub item_id: i64,
    pub type_id: i64,
    pub type_name: String,
    pub quantity: i64,
    pub estimated_value: f64,
    pub location_flag: String,
}

// ─── Init ─────────────────────────────────────────────────────────────────────

pub async fn init_db(app: &tauri::AppHandle) -> Result<SqlitePool> {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("Could not resolve app data dir");
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("hangarsweep.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// ─── Queries ──────────────────────────────────────────────────────────────────

pub async fn get_characters(pool: &SqlitePool) -> Result<Vec<Character>> {
    let rows = sqlx::query_as::<_, Character>(
        "SELECT id, name, access_token, refresh_token, token_expiry FROM characters ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_character(pool: &SqlitePool, id: i64) -> Result<Character> {
    let row = sqlx::query_as::<_, Character>(
        "SELECT id, name, access_token, refresh_token, token_expiry FROM characters WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_character(pool: &SqlitePool, char: &Character) -> Result<()> {
    sqlx::query(
        "INSERT INTO characters (id, name, access_token, refresh_token, token_expiry) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
         name=excluded.name, access_token=excluded.access_token, \
         refresh_token=excluded.refresh_token, token_expiry=excluded.token_expiry",
    )
    .bind(char.id)
    .bind(&char.name)
    .bind(&char.access_token)
    .bind(&char.refresh_token)
    .bind(char.token_expiry)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_tokens(
    pool: &SqlitePool,
    character_id: i64,
    access_token: &str,
    refresh_token: &str,
    expiry: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE characters SET access_token = ?, refresh_token = ?, token_expiry = ? WHERE id = ?",
    )
    .bind(access_token)
    .bind(refresh_token)
    .bind(expiry)
    .bind(character_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_character(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM assets WHERE character_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_liquidity_summary(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<LiquidityRow>> {
    let rows = sqlx::query_as::<_, LiquidityRow>(
        r"
        SELECT
            a.location_id,
            COALESCE(sc.name, s.name, CAST(a.location_id AS TEXT)) AS location_name,
            CAST(SUM(a.quantity * COALESCE(m.average_price, 0.0)) AS REAL) AS total_isk_value,
            CAST(COUNT(DISTINCT a.item_id) AS INTEGER) AS stack_count
        FROM assets a
        LEFT JOIN market_prices   m  ON a.type_id     = m.type_id
        LEFT JOIN sde_stations    s  ON a.location_id = s.station_id
        LEFT JOIN structure_cache sc ON a.location_id = sc.id
        WHERE a.character_id = ?
          AND a.location_flag NOT IN (
              'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
              'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
          )
          AND a.is_singleton = 0
        GROUP BY a.location_id
        HAVING total_isk_value > 500000000
        ORDER BY total_isk_value DESC
        ",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_assets_at_location(
    pool: &SqlitePool,
    location_id: i64,
    character_id: i64,
) -> Result<Vec<AssetRow>> {
    let rows = sqlx::query_as::<_, AssetRow>(
        r"
        SELECT
            a.item_id,
            a.type_id,
            COALESCE(t.type_name, CAST(a.type_id AS TEXT)) AS type_name,
            CAST(SUM(a.quantity) AS INTEGER) AS quantity,
            CAST(SUM(a.quantity * COALESCE(m.average_price, 0.0)) AS REAL) AS estimated_value,
            a.location_flag
        FROM assets a
        LEFT JOIN sde_types     t ON a.type_id = t.type_id
        LEFT JOIN market_prices m ON a.type_id = m.type_id
        WHERE a.character_id = ? AND a.location_id = ?
          AND a.location_flag NOT IN (
              'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
              'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
          )
          AND a.is_singleton = 0
        GROUP BY a.type_id
        ORDER BY estimated_value DESC
        ",
    )
    .bind(character_id)
    .bind(location_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_multibuy_lines(
    pool: &SqlitePool,
    location_id: i64,
    character_id: i64,
) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query(
        r"
        SELECT
            COALESCE(t.type_name, CAST(a.type_id AS TEXT)) AS type_name,
            CAST(SUM(a.quantity) AS INTEGER) AS total_qty
        FROM assets a
        LEFT JOIN sde_types t ON a.type_id = t.type_id
        WHERE a.character_id = ? AND a.location_id = ?
          AND a.location_flag NOT IN (
              'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
              'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
          )
          AND a.is_singleton = 0
        GROUP BY a.type_id
        ORDER BY type_name
        ",
    )
    .bind(character_id)
    .bind(location_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let name: String = r.get("type_name");
            let qty: i64 = r.get("total_qty");
            (name, qty)
        })
        .collect())
}

pub async fn cache_structure(pool: &SqlitePool, id: i64, name: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO structure_cache (id, name) VALUES (?, ?) \
         ON CONFLICT(id) DO UPDATE SET name=excluded.name",
    )
    .bind(id)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn uncached_structure_ids(pool: &SqlitePool, character_id: i64) -> Result<Vec<i64>> {
    let rows = sqlx::query(
        "SELECT DISTINCT a.location_id FROM assets a \
         LEFT JOIN structure_cache sc ON a.location_id = sc.id \
         WHERE a.character_id = ? AND a.location_id >= 1000000000000 AND sc.id IS NULL",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.get::<i64, _>("location_id")).collect())
}
