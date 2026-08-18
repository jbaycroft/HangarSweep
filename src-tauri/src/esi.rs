use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio::task::JoinSet;

use crate::db;

// ─── Shared helpers ───────────────────────────────────────────────────────────

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ─── Market prices ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EsiMarketPrice {
    type_id: i64,
    average_price: Option<f64>,
}

pub async fn sync_market_prices(pool: &SqlitePool) -> Result<()> {
    let client = Client::new();
    let prices: Vec<EsiMarketPrice> = client
        .get("https://esi.evetech.net/latest/markets/prices/?datasource=tranquility")
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let now = now_unix();
    let mut tx = pool.begin().await?;

    for p in &prices {
        let avg = p.average_price.unwrap_or(0.0);
        sqlx::query(
            "INSERT INTO market_prices (type_id, average_price, last_updated) VALUES (?, ?, ?) \
             ON CONFLICT(type_id) DO UPDATE SET \
             average_price=excluded.average_price, last_updated=excluded.last_updated",
        )
        .bind(p.type_id)
        .bind(avg)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn market_prices_stale(pool: &SqlitePool) -> bool {
    let row = sqlx::query("SELECT MAX(last_updated) as lu FROM market_prices")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    match row {
        Some(r) => {
            let lu: i64 = r.try_get("lu").unwrap_or(0);
            now_unix() - lu > 86_400
        }
        None => true,
    }
}

// ─── Asset sync ───────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct EsiAsset {
    item_id: i64,
    type_id: i64,
    location_id: i64,
    location_flag: String,
    quantity: i64,
    is_singleton: bool,
}

pub async fn sync_assets(
    character_id: i64,
    access_token: &str,
    pool: &SqlitePool,
    app: &tauri::AppHandle,
) -> Result<()> {
    let client = Client::new();
    let base_url = format!(
        "https://esi.evetech.net/latest/characters/{}/assets/?datasource=tranquility&page=",
        character_id
    );

    // ── Page 1: discover total page count ────────────────────────────────────
    let resp = client
        .get(format!("{}1", base_url))
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?;

    let x_pages: u32 = resp
        .headers()
        .get("x-pages")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let mut all_assets: Vec<EsiAsset> = resp.json().await?;

    // ── Pages 2..x_pages: concurrent fetch ───────────────────────────────────
    if x_pages > 1 {
        let mut join_set: JoinSet<Result<Vec<EsiAsset>>> = JoinSet::new();

        for page in 2..=x_pages {
            let c = client.clone();
            let url = format!("{}{}", base_url, page);
            let token = access_token.to_string();

            join_set.spawn(async move {
                let resp = c
                    .get(&url)
                    .bearer_auth(&token)
                    .header("Accept", "application/json")
                    .send()
                    .await?;

                if let Some(remain) = resp
                    .headers()
                    .get("x-esi-error-limit-remain")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    if remain < 10 {
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    }
                }

                let assets: Vec<EsiAsset> = resp.error_for_status()?.json().await?;
                Ok(assets)
            });
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(page_assets)) => all_assets.extend(page_assets),
                Ok(Err(e)) => eprintln!("[esi] page fetch error: {}", e),
                Err(e) => eprintln!("[esi] join error: {}", e),
            }
        }
    }

    let _ = app.emit(
        "sync-progress",
        serde_json::json!({
            "step": "assets",
            "status": "running",
            "message": format!("Storing {} items...", all_assets.len())
        }),
    );

    // ── Transactional replace ──────────────────────────────────────────────────
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM assets WHERE character_id = ?")
        .bind(character_id)
        .execute(&mut *tx)
        .await?;

    for a in &all_assets {
        let singleton: i64 = if a.is_singleton { 1 } else { 0 };
        sqlx::query(
            "INSERT OR IGNORE INTO assets \
             (item_id, character_id, type_id, location_id, location_flag, quantity, is_singleton) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(a.item_id)
        .bind(character_id)
        .bind(a.type_id)
        .bind(a.location_id)
        .bind(&a.location_flag)
        .bind(a.quantity)
        .bind(singleton)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

// ─── Bulk name resolution (NPC stations + item types) ────────────────────────
//
// After asset sync we know every type_id and location_id in the DB.
// For NPC stations (id < 1_000_000_000_000) and type names we hit
// POST /universe/names/ which accepts up to 1 000 ids per call and
// returns [{id, name, category}, …] — no auth required.
// Results land in sde_stations / sde_types so location_name and
// type_name columns in all queries resolve properly.

#[derive(Deserialize, Debug)]
struct UniverseNameEntry {
    id: i64,
    name: String,
    category: String,
}

pub async fn resolve_names(
    character_id: i64,
    pool: &SqlitePool,
    app: &tauri::AppHandle,
) -> Result<()> {
    // ── Collect unresolved type_ids ────────────────────────────────────────────
    let unknown_types: Vec<i64> = sqlx::query(
        "SELECT DISTINCT a.type_id FROM assets a
         LEFT JOIN sde_types t ON t.type_id = a.type_id
         WHERE a.character_id = ? AND t.type_id IS NULL",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
    .filter(|id| *id > 0)
    .collect();

    // ── Collect unresolved NPC station location_ids (<1T) ─────────────────────
    let unknown_stations: Vec<i64> = sqlx::query(
        "SELECT DISTINCT a.location_id FROM assets a
         LEFT JOIN sde_stations s ON s.station_id = a.location_id
         WHERE a.character_id = ?
           AND a.location_id < 1000000000000
           AND s.station_id IS NULL",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
    .filter(|id| *id > 0)
    .collect();

    // Combine into one deduplicated list, resolve in batches of 1000
    let mut all_ids: Vec<i64> = unknown_types.iter().chain(unknown_stations.iter()).copied().collect();
    all_ids.sort_unstable();
    all_ids.dedup();

    if all_ids.is_empty() {
        return Ok(());
    }

    let _ = app.emit(
        "sync-progress",
        serde_json::json!({
            "step": "names",
            "status": "running",
            "message": format!("Resolving {} type/station names…", all_ids.len())
        }),
    );

    let client = Client::new();

    for chunk in all_ids.chunks(1000) {
        let body = serde_json::to_string(chunk)?;
        let resp = client
            .post("https://esi.evetech.net/latest/universe/names/?datasource=tranquility")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => { eprintln!("[esi] universe/names request error: {}", e); continue; }
        };

        if !resp.status().is_success() {
            eprintln!("[esi] universe/names returned {}", resp.status());
            continue;
        }

        let entries: Vec<UniverseNameEntry> = match resp.json().await {
            Ok(v) => v,
            Err(e) => { eprintln!("[esi] universe/names parse error: {}", e); continue; }
        };

        let mut tx = pool.begin().await?;
        for entry in &entries {
            match entry.category.as_str() {
                "inventory_type" => {
                    sqlx::query(
                        "INSERT OR IGNORE INTO sde_types (type_id, type_name) VALUES (?, ?)",
                    )
                    .bind(entry.id)
                    .bind(&entry.name)
                    .execute(&mut *tx)
                    .await?;
                }
                "station" => {
                    sqlx::query(
                        "INSERT OR IGNORE INTO sde_stations (station_id, name) VALUES (?, ?)",
                    )
                    .bind(entry.id)
                    .bind(&entry.name)
                    .execute(&mut *tx)
                    .await?;
                }
                // solar_system ids can appear as container location_ids; store as station fallback
                "solar_system" | "constellation" | "region" => {
                    sqlx::query(
                        "INSERT OR IGNORE INTO sde_stations (station_id, name) VALUES (?, ?)",
                    )
                    .bind(entry.id)
                    .bind(&entry.name)
                    .execute(&mut *tx)
                    .await?;
                }
                _ => {}
            }
        }
        tx.commit().await?;
    }

    let _ = app.emit(
        "sync-progress",
        serde_json::json!({ "step": "names", "status": "complete" }),
    );

    Ok(())
}

// ─── Structure name resolution (player citadels, requires auth) ───────────────

#[derive(Deserialize)]
struct UniverseName {
    id: i64,
    name: String,
}

pub async fn resolve_structures(
    character_id: i64,
    access_token: &str,
    pool: &SqlitePool,
) -> Result<()> {
    let ids = db::uncached_structure_ids(pool, character_id).await?;
    if ids.is_empty() {
        return Ok(());
    }

    let client = Client::new();

    for chunk in ids.chunks(1000) {
        let body = serde_json::to_string(chunk)?;
        let resp = client
            .post("https://esi.evetech.net/latest/universe/names/?datasource=tranquility")
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            eprintln!("[esi] universe/names returned {}", resp.status());
            continue;
        }

        let names: Vec<UniverseName> = resp.json().await?;
        for entry in &names {
            db::cache_structure(pool, entry.id, &entry.name).await?;
        }
    }

    Ok(())
}
