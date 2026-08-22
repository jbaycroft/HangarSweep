<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="HangarSweep" width="96" />
</p>

<h1 align="center">HangarSweep</h1>

<p align="center">
  <strong>Dead-capital finder for EVE Online pilots.</strong><br/>
  See exactly where your ISK is sitting idle — and get it to market in two clicks.
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-gold" />
  <img alt="CI" src="https://github.com/jbaycroft/HangarSweep/actions/workflows/ci.yml/badge.svg" />
  <img alt="Tauri 2.0" src="https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.78+-orange?logo=rust" />
  <img alt="React 18" src="https://img.shields.io/badge/React-18-61dafb?logo=react" />
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-local-green?logo=sqlite" />
  <img alt="Tests" src="https://img.shields.io/badge/tests-59%20passing-brightgreen" />
  <img alt="License MIT" src="https://img.shields.io/badge/License-MIT-lightgrey" />
</p>

---

## What is HangarSweep?

HangarSweep is a **native Windows desktop application** built with [Tauri 2.0](https://tauri.app) that connects to the [EVE Online ESI API](https://esi.evetech.net) to give you a consolidated view of every tradeable asset scattered across your character's hangars.

It surfaces **dead capital** — items sitting in stations and citadels that aren't fitted, skilled, or implanted — sorted by estimated ISK value. Use the **Min Value slider** to filter by threshold. Once you find a location worth hitting, one click formats your entire inventory into EVE's **Multibuy** format and copies it to your clipboard, ready to paste straight into the game.

---

## Features

| Feature | Detail |
|---|---|
| 🔐 **EVE SSO login** | OAuth 2.0 PKCE flow — no passwords stored, no client secret required |
| 👥 **Multi-character** | Add as many characters as you like; switch between them instantly |
| 📦 **Full asset sync** | Fetches all pages concurrently, respects ESI error limits |
| 💰 **Market prices** | Pulled from ESI once per 24 hours, cached locally |
| 📊 **Jita price comparison** | Streams The Forge order book; shows **Jita Sell** (undercut price) and **Jita Buy** (instant ISK) per item with a ±% delta vs global average |
| 🏷️ **Auto name resolution** | Item type names + station names resolved automatically via ESI on first sync |
| 🏰 **Citadel resolution** | Player structure names resolved via authenticated ESI call |
| 🔄 **Auto token refresh** | Access tokens silently refreshed before any ESI call |
| 🎚️ **Threshold slider** | Filter locations by ISK value: Show all → 10M → 50M → … → 10B |
| 📋 **Multibuy export** | Formats `TypeName Qty` lines → clipboard in one click |
| 🌑 **Dark EVE theme** | Navy/gold palette, monospaced ISK values, glowing location dots |
| 💾 **100% local** | All data stays in a SQLite database on your machine. Nothing phoned home. |

---

## Quick Start (Windows)

1. **Download** `HangarSweep.exe` from the repo root (or [build it yourself](#building))
2. Double-click to run — no installation needed. WebView2 is pre-installed on Windows 10/11.
3. Click **+ Add Character**, log in via the EVE SSO browser window
4. Click **⟳ Sync** — assets, prices, and names are fetched automatically
5. Browse locations, click one to see assets, hit **Copy Multibuy** to copy to clipboard

> On first sync, name resolution fetches item and station names from ESI. This takes a few extra seconds once, then names are cached permanently.

---

## Tech Stack

| Layer | Technology |
|---|---|
| App shell | [Tauri 2.0](https://tauri.app) |
| Backend | Rust — `sqlx`, `reqwest`, `tokio`, `sha2`, `rand`, `serde` |
| Frontend | React 18 + TypeScript + Vite |
| Database | SQLite via `sqlx` with embedded migrations |
| Auth | EVE SSO v2 — OAuth 2.0 Authorization Code + PKCE |

---

## Building

### Prerequisites (Windows)

| Requirement | Notes |
|---|---|
| **Rust** (stable-x86_64-pc-windows-msvc) | [rustup.rs](https://rustup.rs) |
| **VS Build Tools 2022** | "Desktop development with C++" workload |
| **Node.js** 18+ | [nodejs.org](https://nodejs.org) |
| **WebView2 Runtime** | Pre-installed on Windows 10 22H2+ and Windows 11 |

### Build the exe

```powershell
git clone git@github.com:jbaycroft/HangarSweep.git
cd HangarSweep
npm install
npm run build:exe          # builds Rust + bundles React → HangarSweep.exe in project root
```

The linker path is pinned in `.cargo/config.toml` — no PATH setup needed.

### Dev mode

```powershell
npm run tauri dev          # hot-reloads frontend, recompiles Rust on change
```

---

## EVE Developer Application

HangarSweep is registered as a native application. The credentials in `src/auth.rs` are for the public build. If you fork and publish your own version, register a new app at [developers.eveonline.com](https://developers.eveonline.com/applications/) with:

| Field | Value |
|---|---|
| Connection type | Authentication & API Access |
| Callback URL | `http://localhost:57423/callback` |
| Scopes | `publicData` `esi-assets.read_assets.v1` `esi-universe.read_structures.v1` `esi-ui.open_window.v1` `esi-ui.write_waypoint.v1` |

Then update `src-tauri/src/auth.rs`:

```rust
pub const CLIENT_ID: &str     = "YOUR_CLIENT_ID";
pub const CLIENT_SECRET: &str = "YOUR_CLIENT_SECRET";
pub const REDIRECT_URI: &str  = "http://localhost:57423/callback";
```

---

## Data & Privacy

- **All data is local.** The SQLite database lives at `%APPDATA%\com.hangarsweep.desktop\hangarsweep.db`
- No analytics, no telemetry, no external servers beyond ESI
- EVE access tokens are stored encrypted-at-rest in the local DB and refreshed automatically
- Uninstalling the app does not delete the database — delete the folder manually if desired

### Database Schema

```
characters      — EVE character auth tokens (id, name, access_token, refresh_token, expiry)
assets          — Full asset ledger (replaced wholesale on each sync)
market_prices   — ESI market averages (refreshed every 24h)
jita_prices     — The Forge order-book aggregates: sell_min + buy_max per type (refreshed every 24h)
structure_cache — Resolved citadel/upwell names (authenticated ESI)
sde_types       — Item type names (auto-populated from ESI /universe/names/ on first sync)
sde_stations    — NPC station names (auto-populated from ESI /universe/names/ on first sync)
```

---

## Project Layout

```
HangarSweep/
├── HangarSweep.exe               # ← Ready-to-run binary (Windows x64)
│
├── src/                          # React / TypeScript frontend
│   ├── App.tsx                   # Root — state, layout, threshold slider, SSO events
│   ├── types.ts                  # Shared interfaces + ISK formatter (T/B/M/K)
│   ├── main.tsx                  # ReactDOM entry
│   ├── components/
│   │   ├── CharacterHeader.tsx   # Portrait chip, character switcher dropdown
│   │   ├── LocationList.tsx      # Location table with scrollable body
│   │   └── AssetDetail.tsx       # Per-location asset table + Copy Multibuy
│   └── styles/app.css            # Full dark EVE theme (CSS custom properties)
│
└── src-tauri/                    # Rust backend
    ├── src/
    │   ├── main.rs               # Binary entry point
    │   ├── lib.rs                # Tauri setup, AppState, plugin registration
    │   ├── auth.rs               # PKCE generation, TCP callback listener, token exchange
    │   ├── esi.rs                # ESI HTTP: market prices, assets, universe/names
    │   ├── db.rs                 # All SQLite queries and row structs
    │   └── commands.rs           # #[tauri::command] IPC handlers
    ├── migrations/
    │   └── 0001_init.sql         # Schema — auto-applied on launch via sqlx::migrate!
    ├── capabilities/
    │   └── default.json          # Tauri 2.0 permission grants
    ├── .cargo/config.toml        # Pins VS 2022 link.exe (works from any shell)
    ├── Cargo.toml
    └── tauri.conf.json
```

---

## Sync Pipeline

Each **⟳ Sync** runs these steps in order:

| Step | What happens |
|---|---|
| 1. Market prices | `GET /markets/prices/` — fetched once per 24h, skipped if fresh |
| 1b. Jita prices | `GET /markets/10000002/orders/` — all pages fetched concurrently, aggregated to sell-min & buy-max per type, skipped if fresh |
| 2. Token refresh | Access token silently refreshed if expiry < 60s |
| 3. Asset fetch | `GET /characters/{id}/assets/` — all pages fetched concurrently |
| 4. Structure names | `POST /universe/names/` with citadel IDs — requires auth |
| 5. Type & station names | `POST /universe/names/` with type IDs + NPC station IDs — no auth, cached permanently |

---

## IPC Commands

| Command | Args | Returns | Description |
|---|---|---|---|
| `login` | — | `void` | Opens EVE SSO in browser, starts TCP callback listener |
| `get_characters` | — | `Character[]` | Lists all stored characters |
| `delete_character` | `characterId` | `void` | Removes character + their assets |
| `sync_all` | `characterId` | `void` | Runs full 5-step sync pipeline |
| `get_liquidity_summary` | `characterId` | `LiquidityRow[]` | All locations sorted by ISK value desc |
| `get_assets_at_location` | `locationId`, `characterId` | `AssetRow[]` | Items at a location, sorted by value |
| `export_multibuy` | `locationId`, `characterId` | `string` | `TypeName Qty` newline-separated |

### Frontend Events (Rust → React via `app.emit`)

| Event | Payload | Fired when |
|---|---|---|
| `auth-complete` | `{ character_id, character_name }` | SSO callback succeeded |
| `auth-error` | `{ message }` | Any auth step fails |
| `sync-progress` | `{ step, status, message? }` | Each sync stage starts/completes |

---

## Roadmap

- [ ] Multiple character support in single sync run
- [ ] Per-location profit/loss history
- [x] ~~Jita price comparison (buy vs sell)~~ — shipped in v0.2.0
- [ ] Structure access error handling (triage inaccessible citadels)
- [ ] macOS / Linux builds

---

## Contributing

Pull requests are welcome. Open an issue first for anything non-trivial.

1. Fork → branch (`feature/my-feature`) → commit → PR
2. Rust: keep `cargo clippy` clean
3. Frontend: TypeScript strict mode, no `any`

---

## License

MIT © 2026 jbaycroft

---

> *HangarSweep is not affiliated with CCP Games. EVE Online is a registered trademark of CCP hf.*
