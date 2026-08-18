use tauri::Manager;
use tauri_plugin_shell::ShellExt;

use crate::{
    auth,
    db::{self, AssetRow, Character, LiquidityRow},
    esi, AppState,
};

// ─── Auth ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn login(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let pkce = auth::generate_pkce();
    let auth_url = auth::build_auth_url(&pkce);

    // Store PKCE params for callback validation
    {
        let mut pending = state.pending_auth.lock().unwrap();
        *pending = Some(crate::PendingAuth {
            verifier: pkce.verifier.clone(),
            state: pkce.state.clone(),
        });
    }

    // Open system browser to EVE SSO
    app.shell()
        .open(&auth_url, None)
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    // Spawn background listener
    let verifier = pkce.verifier;
    let expected_state = pkce.state;
    let pool = state.db.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        auth::run_callback_listener(verifier, expected_state, pool, app_clone).await;
    });

    Ok(())
}

// ─── Characters ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_characters(state: tauri::State<'_, AppState>) -> Result<Vec<Character>, String> {
    db::get_characters(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_character(
    state: tauri::State<'_, AppState>,
    character_id: i64,
) -> Result<(), String> {
    db::delete_character(&state.db, character_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Sync ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn sync_all(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: i64,
) -> Result<(), String> {
    let emit = |step: &str, status: &str, msg: Option<&str>| {
        let _ = app.emit(
            "sync-progress",
            serde_json::json!({ "step": step, "status": status, "message": msg }),
        );
    };

    // 1. Market prices (only if stale)
    emit("market_prices", "running", Some("Fetching market prices..."));
    if esi::market_prices_stale(&state.db).await {
        esi::sync_market_prices(&state.db)
            .await
            .map_err(|e| format!("Market price sync failed: {}", e))?;
    }
    emit("market_prices", "complete", None);

    // 2. Ensure valid token
    let access_token = auth::ensure_valid_token(character_id, &state.db)
        .await
        .map_err(|e| format!("Token refresh failed: {}", e))?;

    // 3. Asset sync
    emit("assets", "running", Some("Fetching assets from ESI..."));
    esi::sync_assets(character_id, &access_token, &state.db, &app)
        .await
        .map_err(|e| format!("Asset sync failed: {}", e))?;
    emit("assets", "complete", None);

    // 4. Resolve structure names
    emit("structures", "running", Some("Resolving structure names..."));
    esi::resolve_structures(character_id, &access_token, &state.db)
        .await
        .map_err(|e| format!("Structure resolution failed: {}", e))?;
    emit("structures", "complete", None);

    Ok(())
}

// ─── Queries ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_liquidity_summary(
    state: tauri::State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<LiquidityRow>, String> {
    db::get_liquidity_summary(&state.db, character_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_assets_at_location(
    state: tauri::State<'_, AppState>,
    location_id: i64,
    character_id: i64,
) -> Result<Vec<AssetRow>, String> {
    db::get_assets_at_location(&state.db, location_id, character_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_multibuy(
    state: tauri::State<'_, AppState>,
    location_id: i64,
    character_id: i64,
) -> Result<String, String> {
    let lines = db::get_multibuy_lines(&state.db, location_id, character_id)
        .await
        .map_err(|e| e.to_string())?;

    let multibuy = lines
        .into_iter()
        .map(|(name, qty)| format!("{} {}", name, qty))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(multibuy)
}
