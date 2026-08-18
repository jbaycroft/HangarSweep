<p align="center">
  <img src="docs/banner.png" alt="HangarSweep" width="520" />
</p>

<h1 align="center">HangarSweep</h1>

<p align="center">
  <strong>Dead-capital finder for EVE Online pilots.</strong><br/>
  See exactly where your ISK is sitting idle — and get it to market in two clicks.
</p>

<p align="center">
  <img alt="Tauri 2.0" src="https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.78+-orange?logo=rust" />
  <img alt="React 18" src="https://img.shields.io/badge/React-18-61dafb?logo=react" />
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-local-green?logo=sqlite" />
  <img alt="License MIT" src="https://img.shields.io/badge/License-MIT-lightgrey" />
</p>

---

## What is HangarSweep?

HangarSweep is a **native desktop application** built with [Tauri 2.0](https://tauri.app) that connects to the [EVE Online ESI API](https://esi.evetech.net) to give you a consolidated view of every tradeable asset scattered across your characters' hangars.

It highlights **dead capital** — stacks of items sitting in stations and citadels worth more than 500M ISK that aren't fitted to a ship or trained as a skill — so you can decide what to consolidate or sell. Once you find a location, one click formats your entire inventory into EVE's **Multibuy** format and copies it to your clipboard, ready to paste straight into the game.

---

## Features

| Feature | Detail |
|---|---|
| 🔐 **EVE SSO login** | OAuth 2.0 PKCE flow — no passwords stored, no secrets in the app |
| 👥 **Multi-character** | Add as many characters as you like; switch between them instantly |
| 📦 **Full asset sync** | Fetches all pages concurrently, respects ESI error limits |
| 💰 **Market prices** | Pulled from ESI once per 24 hours, cached locally |
| 🏰 **Citadel resolution** | Player structure names resolved via `POST /universe/names/` |
| 🔄 **Auto token refresh** | Access tokens silently refreshed before any ESI call |
| 📋 **Multibuy export** | Formats `TypeName Qty` lines → clipboard in one click |
| 🌑 **Dark EVE theme** | Gold accents, amber ISK values, per-session state |
| 💾 **100% local** | All data stays in a SQLite database on your machine |

---

## Screenshots

> *(Screenshots will be added after first successful build.)*

---

## Tech Stack

| Layer | Technology |
|---|---|
| App shell | [Tauri 2.0](https://tauri.app) |
| Backend logic | Rust (`sqlx`, `reqwest`, `tokio`, `sha2`, `base64`, `rand`) |
| Frontend | React 18 + TypeScript + Vite |
| Database | SQLite via `sqlx` with embedded migrations |
| Auth | EVE SSO v2 — OAuth 2.0 Authorization Code + PKCE |

---

## Prerequisites

### Windows

| Requirement | Version | Notes |
|---|---|---|
| **Rust** | 1.78+ | Install via [rustup.rs](https://rustup.rs) |
| **MSVC Build Tools** | Latest | ["Desktop development with C++"](https://visualstudio.microsoft.com/visual-cpp-build-tools/) workload |
| **Node.js** | 18+ | [nodejs.org](https://nodejs.org) |
| **WebView2** | Any | Pre-installed on Windows 10/11; otherwise [download here](https://developer.microsoft.com/microsoft-edge/webview2/) |

### macOS / Linux

Rust and Node.js are sufficient. Follow the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for your distro.

---

## Getting Started

```bash
# 1. Clone
git clone git@github.com:jbaycroft/HangarSweep.git
cd HangarSweep

# 2. Install frontend dependencies
npm install

# 3. Launch in dev mode (first run compiles Rust — allow 3-5 minutes)
npm run tauri dev
```

The app window opens automatically. Click **+ Add EVE Character** to begin the login flow.

---

## EVE Developer Application

HangarSweep is registered as a native application on the EVE Developer Portal. The credentials embedded in the source are for the public build. If you fork and publish your own version you **must** register a new application at https://developers.eveonline.com/applications/ with:

| Field | Value |
|---|---|
| Connection type | Authentication & API Access |
| Callback URL | `http://localhost` |
| Scopes | `publicData` `esi-assets.read_assets.v1` `esi-universe.read_structures.v1` `esi-ui.open_window.v1` `esi-ui.write_waypoint.v1` |

Then update `src-tauri/src/auth.rs`:

```rust
pub const CLIENT_ID: &str     = "YOUR_CLIENT_ID";
pub const CLIENT_SECRET: &str = "YOUR_CLIENT_SECRET";
```

> **Note on the callback port:** HangarSweep listens on `http://localhost:57423/callback`. Per [RFC 8252 §7.3](https://www.rfc-editor.org/rfc/rfc8252#section-7.3), EVE SSO matches the scheme and host for loopback URIs and ignores the port, so registering `http://localhost` is sufficient.

---

## SQLite Database

The database is stored at:

| OS | Path |
|---|---|
| Windows | `%APPDATA%\com.hangarsweep.app\hangarsweep.db` |
| macOS | `~/Library/Application Support/com.hangarsweep.app/hangarsweep.db` |
| Linux | `~/.local/share/com.hangarsweep.app/hangarsweep.db` |

Migrations run automatically on launch via `sqlx::migrate!`.

### Schema Overview

```
characters      — EVE character auth tokens
assets          — Full asset ledger, replaced on each sync
market_prices   — ESI market averages (refreshed every 24h)
structure_cache — Resolved citadel/upwell names
sde_types       — Item type names (from EVE SDE)
sde_stations    — NPC station names (from EVE SDE)
```

### Loading the SDE (Optional but Recommended)

Without the SDE, type and station names display as numeric IDs. To populate them:

1. Download the [Fuzzwork SDE SQLite mirror](https://www.fuzzwork.co.uk/dump/latest/eve.db.bz2) (`eve.db`)
2. Open your HangarSweep database with any SQLite client and run:

```sql
ATTACH 'path/to/eve.db' AS sde;

INSERT OR IGNORE INTO sde_types (type_id, type_name)
    SELECT typeID, typeName FROM sde.invTypes WHERE published = 1;

INSERT OR IGNORE INTO sde_stations (station_id, name)
    SELECT stationID, stationName FROM sde.staStations;

DETACH sde;
```

---

## Project Layout

```
HangarSweep/
├── src/                          # React / TypeScript frontend
│   ├── App.tsx                   # Root component — state, layout, SSO event listeners
│   ├── types.ts                  # Shared interfaces + ISK formatter
│   ├── main.tsx                  # ReactDOM entry point
│   ├── components/
│   │   ├── CharacterHeader.tsx   # Portrait chip, character switcher dropdown
│   │   ├── LocationList.tsx      # Dead-capital location table
│   │   └── AssetDetail.tsx       # Per-location asset breakdown + Copy Multibuy
│   └── styles/app.css            # Full dark EVE theme
│
└── src-tauri/                    # Rust backend
    ├── src/
    │   ├── main.rs               # Binary entry point
    │   ├── lib.rs                # App bootstrap, Tauri state management
    │   ├── auth.rs               # PKCE generation, callback listener, token exchange
    │   ├── esi.rs                # ESI HTTP calls (market, assets, universe/names)
    │   ├── db.rs                 # All SQLite queries and structs
    │   └── commands.rs           # #[tauri::command] IPC handlers
    ├── migrations/
    │   └── 0001_init.sql         # Embedded schema (auto-applied on launch)
    ├── capabilities/
    │   └── default.json          # Tauri 2.0 permission grants
    ├── Cargo.toml
    ├── build.rs
    └── tauri.conf.json
```

---

## Tauri IPC Commands

These are the Rust commands callable from the frontend via `invoke()`:

| Command | Args | Returns | Description |
|---|---|---|---|
| `login` | — | `void` | Opens EVE SSO in browser, starts callback listener |
| `get_characters` | — | `Character[]` | Lists all stored characters |
| `delete_character` | `character_id` | `void` | Removes character + their assets |
| `sync_all` | `character_id` | `void` | Market prices → assets → structure names |
| `get_liquidity_summary` | `character_id` | `LiquidityRow[]` | Locations with >500M ISK |
| `get_assets_at_location` | `location_id`, `character_id` | `AssetRow[]` | Items at a location |
| `export_multibuy` | `location_id`, `character_id` | `string` | `TypeName Qty` newline-separated |

### Frontend Events (Rust → React)

| Event | Payload | Fired when |
|---|---|---|
| `auth-complete` | `{ character_id, character_name }` | SSO callback succeeded |
| `auth-error` | `{ message }` | Any auth step fails |
| `sync-progress` | `{ step, status, message? }` | Each sync stage starts/completes |

---

## Building for Distribution

```bash
npm run tauri build
```

Outputs:
- **Windows:** `src-tauri/target/release/bundle/msi/` and `/nsis/`
- **macOS:** `src-tauri/target/release/bundle/dmg/`
- **Linux:** `src-tauri/target/release/bundle/appimage/` and `/deb/`

---

## Liquidity Query

The query that drives the location list, applied on the local DB:

```sql
SELECT
    a.location_id,
    COALESCE(sc.name, s.name, CAST(a.location_id AS TEXT)) AS location_name,
    SUM(a.quantity * COALESCE(m.average_price, 0)) AS total_isk_value,
    COUNT(DISTINCT a.item_id) AS stack_count
FROM assets a
LEFT JOIN market_prices  m  ON a.type_id     = m.type_id
LEFT JOIN sde_stations   s  ON a.location_id = s.station_id
LEFT JOIN structure_cache sc ON a.location_id = sc.id
WHERE a.character_id = :char_id
  AND a.location_flag NOT IN (
      'Fitted','RigSlot0','RigSlot1','RigSlot2','RigSlot3',
      'RigSlot4','RigSlot5','RigSlot6','RigSlot7','Implant','Skill'
  )
  AND a.is_singleton = 0
GROUP BY a.location_id
HAVING total_isk_value > 500000000
ORDER BY total_isk_value DESC;
```

---

## Contributing

Pull requests are welcome! If you find a bug or have a feature idea, open an issue first so we can discuss the approach.

1. Fork → branch (`feature/my-feature`) → commit → PR.
2. Keep Rust code `cargo clippy`-clean.
3. Format frontend with `prettier` (config TBD).

---

## License

MIT © 2026 jbaycroft

---

> *HangarSweep is not affiliated with CCP Games. EVE Online is a registered trademark of CCP hf.*
