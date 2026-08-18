use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
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

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub item_id: i64,
    pub character_id: i64,
    pub type_id: i64,
    pub location_id: i64,
    pub location_flag: String,
    pub quantity: i64,
    pub is_singleton: i64,
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

    // Run embedded migrations from src-tauri/migrations/
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// ─── Queries ──────────────────────────────────────────────────────────────────

pub async fn get_characters(pool: &SqlitePool) -> Result<Vec<Character>> {
    let rows = sqlx::query_as!(Character, "SELECT id, name, access_token, refresh_token, token_expiry FROM characters ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get_character(pool: &SqlitePool, id: i64) -> Result<Character> {
    let row = sqlx::query_as!(Character, "SELECT id, name, access_token, refresh_token, token_expiry FROM characters WHERE id = ?", id)
        .fetch_one(pool)
        .await?;
    Ok(row)
}

pub async fn upsert_character(pool: &SqlitePool, char: &Character) -> Result<()> {
    sqlx::query!(
        "INSERT INTO characters (id, name, access_token, refresh_token, token_expiry) VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, access_token=excluded.access_token, \
         refresh_token=excluded.refresh_token, token_expiry=excluded.token_expiry",
        char.id,
        char.name,
        char.access_token,
        char.refresh_token,
        char.token_expiry
    )
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
    sqlx::query!(
        "UPDATE characters SET access_token = ?, refresh_token = ?, token_expiry = ? WHERE id = ?",
        access_token,
        refresh_token,
        expiry,
        character_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_character(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query!("DELETE FROM assets WHERE character_id = ?", id)
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM characters WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_liquidity_summary(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<LiquidityRow>> {
    let rows = sqlx::query_as!(
        LiquidityRow,
        r#"
        SELECT
            a.location_id,
            COALESCE(sc.name, s.name, CAST(a.location_id AS TEXT)) AS "location_name!: String",
            SUM(a.quantity * COALESCE(m.average_price, 0.0)) AS "total_isk_value!: f64",
            COUNT(DISTINCT a.item_id) AS "stack_count!: i64"
        FROM assets a
        LEFT JOIN market_prices m   ON a.type_id     = m.type_id
        LEFT JOIN sde_stations  s   ON a.location_id = s.station_id
        LEFT JOIN structure_cache sc ON a.location_id = sc.id
        WHERE a.character_id = ?
          AND a.location_flag NOT IN (
              'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
              'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
          )
          AND a.is_singleton = 0
        GROUP BY a.location_id
        HAVING "total_isk_value!: f64" > 500000000
        ORDER BY "total_isk_value!: f64" DESC
        "#,
        character_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_assets_at_location(
    pool: &SqlitePool,
    location_id: i64,
    character_id: i64,
) -> Result<Vec<AssetRow>> {
    let rows = sqlx::query_as!(
        AssetRow,
        r#"
        SELECT
            a.item_id,
            a.type_id,
            COALESCE(t.type_name, CAST(a.type_id AS TEXT)) AS "type_name!: String",
            SUM(a.quantity) AS "quantity!: i64",
            SUM(a.quantity * COALESCE(m.average_price, 0.0)) AS "estimated_value!: f64",
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
        ORDER BY "estimated_value!: f64" DESC
        "#,
        character_id,
        location_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_multibuy_lines(
    pool: &SqlitePool,
    location_id: i64,
    character_id: i64,
) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            COALESCE(t.type_name, CAST(a.type_id AS TEXT)) AS "type_name!: String",
            SUM(a.quantity) AS "total_qty!: i64"
        FROM assets a
        LEFT JOIN sde_types t ON a.type_id = t.type_id
        WHERE a.character_id = ? AND a.location_id = ?
          AND a.location_flag NOT IN (
              'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
              'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
          )
          AND a.is_singleton = 0
        GROUP BY a.type_id
        ORDER BY "type_name!: String"
        "#,
        character_id,
        location_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.type_name, r.total_qty)).collect())
}

pub async fn cache_structure(pool: &SqlitePool, id: i64, name: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO structure_cache (id, name) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name",
        id,
        name
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn uncached_structure_ids(pool: &SqlitePool, character_id: i64) -> Result<Vec<i64>> {
    let rows = sqlx::query!(
        "SELECT DISTINCT a.location_id FROM assets a \
         LEFT JOIN structure_cache sc ON a.location_id = sc.id \
         WHERE a.character_id = ? AND a.location_id >= 1000000000000 AND sc.id IS NULL",
        character_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.location_id).collect())
}
