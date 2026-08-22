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
    /// Jita minimum sell-order price per unit (0.0 = no Jita data)
    pub jita_sell: f64,
    /// Jita maximum buy-order price per unit (0.0 = no Jita data)
    pub jita_buy: f64,
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
            a.location_flag,
            COALESCE(j.sell_min, 0.0) AS jita_sell,
            COALESCE(j.buy_max,  0.0) AS jita_buy
        FROM assets a
        LEFT JOIN sde_types     t ON a.type_id = t.type_id
        LEFT JOIN market_prices m ON a.type_id = m.type_id
        LEFT JOIN jita_prices   j ON a.type_id = j.type_id
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Create an isolated in-memory SQLite pool with migrations applied.
    /// Each test gets its own pool so they never interfere.
    async fn setup() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite failed");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations failed");
        pool
    }

    fn test_char(id: i64) -> Character {
        Character {
            id,
            name: format!("Pilot {id}"),
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            token_expiry: 9_999_999_999,
        }
    }

    // ── Character CRUD ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_upsert_and_get_character() {
        let pool = setup().await;
        let c = test_char(1234);
        upsert_character(&pool, &c).await.unwrap();

        let fetched = get_character(&pool, 1234).await.unwrap();
        assert_eq!(fetched.id, 1234);
        assert_eq!(fetched.name, "Pilot 1234");
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_character() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(99)).await.unwrap();

        let updated = Character {
            id: 99,
            name: "Updated Name".into(),
            access_token: "new_access".into(),
            refresh_token: "new_refresh".into(),
            token_expiry: 1_000,
        };
        upsert_character(&pool, &updated).await.unwrap();

        let fetched = get_character(&pool, 99).await.unwrap();
        assert_eq!(fetched.name, "Updated Name");
        assert_eq!(fetched.access_token, "new_access");
    }

    #[tokio::test]
    async fn test_get_characters_returns_all_sorted() {
        let pool = setup().await;
        let mut c1 = test_char(1); c1.name = "Zebra".into();
        let mut c2 = test_char(2); c2.name = "Alpha".into();
        upsert_character(&pool, &c1).await.unwrap();
        upsert_character(&pool, &c2).await.unwrap();

        let chars = get_characters(&pool).await.unwrap();
        assert_eq!(chars.len(), 2);
        assert_eq!(chars[0].name, "Alpha");
        assert_eq!(chars[1].name, "Zebra");
    }

    #[tokio::test]
    async fn test_get_character_not_found() {
        let pool = setup().await;
        let result = get_character(&pool, 99999).await;
        assert!(result.is_err(), "expected error for missing character");
    }

    #[tokio::test]
    async fn test_delete_character_removes_character_and_assets() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(42)).await.unwrap();

        // Insert an asset for this character
        sqlx::query(
            "INSERT INTO assets (item_id, character_id, type_id, location_id, location_flag, quantity, is_singleton)
             VALUES (1, 42, 34, 60001099, 'Hangar', 100, 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        delete_character(&pool, 42).await.unwrap();

        // Character should be gone
        assert!(get_character(&pool, 42).await.is_err());

        // Assets should be cascaded
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE character_id = 42")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "assets should be deleted with character");
    }

    #[tokio::test]
    async fn test_update_tokens() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(7)).await.unwrap();
        update_tokens(&pool, 7, "tok_new", "ref_new", 12345).await.unwrap();

        let c = get_character(&pool, 7).await.unwrap();
        assert_eq!(c.access_token, "tok_new");
        assert_eq!(c.refresh_token, "ref_new");
        assert_eq!(c.token_expiry, 12345);
    }

    // ── Liquidity summary ─────────────────────────────────────────────────────

    /// Insert a market price for testing ISK estimates.
    async fn insert_price(pool: &SqlitePool, type_id: i64, price: f64) {
        sqlx::query(
            "INSERT INTO market_prices (type_id, average_price, last_updated) VALUES (?, ?, 0)
             ON CONFLICT(type_id) DO UPDATE SET average_price = excluded.average_price",
        )
        .bind(type_id)
        .bind(price)
        .execute(pool)
        .await
        .unwrap();
    }

    /// Insert a test asset.
    async fn insert_asset(
        pool: &SqlitePool,
        item_id: i64,
        character_id: i64,
        type_id: i64,
        location_id: i64,
        flag: &str,
        qty: i64,
        singleton: bool,
    ) {
        sqlx::query(
            "INSERT INTO assets (item_id, character_id, type_id, location_id, location_flag, quantity, is_singleton)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item_id)
        .bind(character_id)
        .bind(type_id)
        .bind(location_id)
        .bind(flag)
        .bind(qty)
        .bind(singleton as i64)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_liquidity_summary_returns_locations_with_value() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 10.0).await; // Tritanium = 10 ISK
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1000, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].location_id, 60001099);
        // 1000 × 10.0 = 10,000 ISK
        assert!((rows[0].total_isk_value - 10_000.0).abs() < 0.01);
        assert_eq!(rows[0].stack_count, 1);
    }

    #[tokio::test]
    async fn test_liquidity_summary_excludes_fitted_items() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 10.0).await;
        // Fitted items should be excluded
        for (item_id, flag) in [
            (10, "Fitted"), (11, "RigSlot0"), (12, "Implant"), (13, "Skill"),
        ] {
            insert_asset(&pool, item_id, 1, 34, 60001099, flag, 100, false).await;
        }
        // One valid Hangar item
        insert_asset(&pool, 20, 1, 34, 60001099, "Hangar", 500, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].total_isk_value - 5_000.0).abs() < 0.01,
            "only Hangar item should count; got {}", rows[0].total_isk_value);
    }

    #[tokio::test]
    async fn test_liquidity_summary_excludes_singletons() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 10.0).await;
        // Singleton (assembled ship) should be excluded
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1, true).await;
        // Normal stack should be included
        insert_asset(&pool, 2, 1, 35, 60001099, "Hangar", 100, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        // Only the non-singleton item contributes; type_id 35 has no price → 0 ISK
        // location still appears but with 0 value (type_id 35 not in market_prices)
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].stack_count, 1);
    }

    #[tokio::test]
    async fn test_liquidity_summary_sorted_descending() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 1.0).await;
        insert_price(&pool, 35, 100.0).await;
        // Location A: 10 × 1 ISK = 10
        insert_asset(&pool, 1, 1, 34, 60000001, "Hangar", 10, false).await;
        // Location B: 10 × 100 ISK = 1000
        insert_asset(&pool, 2, 1, 35, 60000002, "Hangar", 10, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].total_isk_value > rows[1].total_isk_value,
            "rows should be descending by ISK value");
    }

    #[tokio::test]
    async fn test_liquidity_summary_resolves_station_name() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 1.0).await;
        sqlx::query("INSERT INTO sde_stations (station_id, name) VALUES (60001099, 'Jita IV - Moon 4')")
            .execute(&pool).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        assert_eq!(rows[0].location_name, "Jita IV - Moon 4");
    }

    #[tokio::test]
    async fn test_liquidity_summary_falls_back_to_id_string() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 1.0).await;
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1, false).await;

        let rows = get_liquidity_summary(&pool, 1).await.unwrap();
        assert_eq!(rows[0].location_name, "60001099",
            "unresolved station should fall back to ID string");
    }

    // ── Assets at location ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_assets_at_location_groups_by_type() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        // Two stacks of the same type at the same location
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 500, false).await;
        insert_asset(&pool, 2, 1, 34, 60001099, "Hangar", 200, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows.len(), 1, "same type should be grouped into one row");
        assert_eq!(rows[0].quantity, 700);
    }

    #[tokio::test]
    async fn test_assets_at_location_resolves_type_name() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        sqlx::query("INSERT INTO sde_types (type_id, type_name) VALUES (34, 'Tritanium')")
            .execute(&pool).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows[0].type_name, "Tritanium");
    }

    #[tokio::test]
    async fn test_assets_at_location_falls_back_type_id() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows[0].type_name, "34", "unresolved type should fall back to ID");
    }

    #[tokio::test]
    async fn test_assets_at_location_only_returns_requested_location() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;
        insert_asset(&pool, 2, 1, 35, 60002000, "Hangar", 200, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].type_id, 34);
    }

    #[tokio::test]
    async fn test_assets_at_location_empty_for_wrong_character() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        upsert_character(&pool, &test_char(2)).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 2).await.unwrap();
        assert!(rows.is_empty(), "character isolation failed");
    }

    // ── Multibuy ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_multibuy_lines_sorted_alphabetically() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        sqlx::query("INSERT INTO sde_types (type_id, type_name) VALUES (34,'Tritanium'),(35,'Pyerite')")
            .execute(&pool).await.unwrap();
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1000, false).await;
        insert_asset(&pool, 2, 1, 35, 60001099, "Hangar", 500, false).await;

        let lines = get_multibuy_lines(&pool, 60001099, 1).await.unwrap();
        assert_eq!(lines.len(), 2);
        // Alphabetical: Pyerite before Tritanium
        assert_eq!(lines[0].0, "Pyerite");
        assert_eq!(lines[0].1, 500);
        assert_eq!(lines[1].0, "Tritanium");
        assert_eq!(lines[1].1, 1000);
    }

    // ── Structure cache ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cache_structure_insert_and_upsert() {
        let pool = setup().await;
        cache_structure(&pool, 1_000_000_000_001, "Perimeter - TTT").await.unwrap();
        cache_structure(&pool, 1_000_000_000_001, "Perimeter - Updated").await.unwrap();

        let name: String = sqlx::query_scalar("SELECT name FROM structure_cache WHERE id = ?")
            .bind(1_000_000_000_001i64)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Perimeter - Updated", "upsert should overwrite name");
    }

    #[tokio::test]
    async fn test_uncached_structure_ids_returns_only_missing() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        // Two structure IDs — one cached, one not
        insert_asset(&pool, 1, 1, 34, 1_000_000_000_001, "Hangar", 1, false).await;
        insert_asset(&pool, 2, 1, 34, 1_000_000_000_002, "Hangar", 1, false).await;
        cache_structure(&pool, 1_000_000_000_001, "Known Citadel").await.unwrap();

        let uncached = uncached_structure_ids(&pool, 1).await.unwrap();
        assert_eq!(uncached.len(), 1);
        assert_eq!(uncached[0], 1_000_000_000_002i64);
    }

    #[tokio::test]
    async fn test_uncached_structure_ids_excludes_npc_stations() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        // NPC station — should NOT appear in uncached list (id < 1T)
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1, false).await;

        let uncached = uncached_structure_ids(&pool, 1).await.unwrap();
        assert!(uncached.is_empty(), "NPC stations should not appear in structure cache query");
    }

    // ── Jita price comparison ─────────────────────────────────────────────────

    /// Insert a Jita price row for testing.
    async fn insert_jita_price(pool: &SqlitePool, type_id: i64, sell_min: f64, buy_max: f64) {
        sqlx::query(
            "INSERT INTO jita_prices (type_id, sell_min, buy_max, last_updated) \
             VALUES (?, ?, ?, 0) \
             ON CONFLICT(type_id) DO UPDATE SET sell_min=excluded.sell_min, buy_max=excluded.buy_max",
        )
        .bind(type_id)
        .bind(sell_min)
        .bind(buy_max)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_assets_at_location_includes_jita_prices() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        sqlx::query("INSERT INTO sde_types (type_id, type_name) VALUES (34, 'Tritanium')")
            .execute(&pool).await.unwrap();
        insert_price(&pool, 34, 5.0).await;
        insert_jita_price(&pool, 34, 6.50, 5.80).await;
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 1000, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].jita_sell - 6.50).abs() < 0.001, "jita_sell should be 6.50");
        assert!((rows[0].jita_buy  - 5.80).abs() < 0.001, "jita_buy should be 5.80");
        // estimated_value still uses market average
        assert!((rows[0].estimated_value - 5000.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_assets_at_location_jita_zero_when_no_jita_data() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        insert_price(&pool, 34, 10.0).await;
        // Intentionally do NOT insert a jita_prices row
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].jita_sell, 0.0, "jita_sell should fall back to 0.0");
        assert_eq!(rows[0].jita_buy,  0.0, "jita_buy should fall back to 0.0");
    }

    #[tokio::test]
    async fn test_jita_prices_upsert() {
        let pool = setup().await;
        insert_jita_price(&pool, 34, 10.0, 9.0).await;
        // Upsert with new prices
        insert_jita_price(&pool, 34, 11.5, 9.5).await;

        let (sell, buy): (f64, f64) =
            sqlx::query_as("SELECT sell_min, buy_max FROM jita_prices WHERE type_id = 34")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!((sell - 11.5).abs() < 0.001, "upsert should update sell_min");
        assert!((buy  -  9.5).abs() < 0.001, "upsert should update buy_max");
    }

    #[tokio::test]
    async fn test_assets_at_location_jita_on_multiple_types() {
        let pool = setup().await;
        upsert_character(&pool, &test_char(1)).await.unwrap();
        sqlx::query("INSERT INTO sde_types (type_id, type_name) VALUES (34,'Tritanium'),(35,'Pyerite')")
            .execute(&pool).await.unwrap();
        insert_price(&pool, 34, 5.0).await;
        insert_price(&pool, 35, 8.0).await;
        insert_jita_price(&pool, 34, 5.5, 4.9).await;
        // type 35 has no Jita data — should default to 0.0
        insert_asset(&pool, 1, 1, 34, 60001099, "Hangar", 100, false).await;
        insert_asset(&pool, 2, 1, 35, 60001099, "Hangar", 200, false).await;

        let rows = get_assets_at_location(&pool, 60001099, 1).await.unwrap();
        assert_eq!(rows.len(), 2);

        let trit = rows.iter().find(|r| r.type_id == 34).unwrap();
        let pye  = rows.iter().find(|r| r.type_id == 35).unwrap();

        assert!((trit.jita_sell - 5.5).abs() < 0.001);
        assert!((trit.jita_buy  - 4.9).abs() < 0.001);
        assert_eq!(pye.jita_sell, 0.0);
        assert_eq!(pye.jita_buy,  0.0);
    }
}

