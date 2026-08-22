# Changelog

All notable changes to HangarSweep are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Version numbers follow [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned
- Multiple character aggregate view
- Per-location ISK history
- macOS / Linux builds

---

## [0.2.0] — 2026-08-22

### Added
- **Jita price comparison** — streams The Forge (`10000002`) full order book on every sync (cached 24h); surfaces per-item **Jita Sell** (lowest active sell order) and **Jita Buy** (highest active buy order) in the asset detail panel
- **Price mode toggle** — three-button switcher in the asset detail header: `ESI Avg` / `Jita Sell` / `Jita Buy`; all totals, values, and unit prices respond instantly to the selection
- **±% delta column** — when a Jita mode is active, a green/red delta column shows how much higher or lower Jita prices are vs the ESI global average
- **Contextual hint bar** — brief explanatory text beneath the header describing what each price mode means ("list here to undercut" vs "instant ISK via buy order")
- **`jita_prices` table** — new SQLite table with `sell_min`, `buy_max`, and `last_updated`; migration `0002_jita_prices.sql` applied automatically on first launch
- **`esi.rs` — `sync_jita_prices` / `jita_prices_stale`** — concurrent pagination of The Forge orders, O(n) HashMap aggregation, transactional upsert
- **4 new Rust tests** — `jita_prices` table insert/upsert and join coverage (total: 47 passing)
- **14 new TypeScript tests** — price mode helper functions, delta computation, no-data sentinel behaviour (total: 34 passing)

### Changed
- `sync_all` pipeline: step 1b added (Jita price fetch, skipped if fresh)
- `get_assets_at_location` SQL: LEFT JOIN `jita_prices` on `type_id`
- `AssetRow` (Rust + TypeScript): new `jita_sell: f64` and `jita_buy: f64` fields
- Asset detail column layout: adapts between 3 columns (avg mode) and 5 columns (Jita mode)

---

## [0.1.0] — 2026-08-18

### Added
- **EVE SSO login** via OAuth 2.0 Authorization Code + PKCE (RFC 7636)
- **Multi-character support** — add, switch, and remove characters from a dropdown
- **Full asset sync** — all ESI asset pages fetched concurrently, full replace per sync
- **Market price cache** — `GET /markets/prices/` refreshed once per 24 hours
- **Auto name resolution** — item type names and NPC station names resolved via `POST /universe/names/` after first sync; cached permanently
- **Player citadel names** — resolved via authenticated ESI, cached in `structure_cache`
- **Token auto-refresh** — access tokens silently refreshed 60 seconds before expiry
- **Min value threshold slider** — filter locations by ISK value (Show all / 10M / … / 10B)
- **Asset summary bar** — always-visible total across all locations
- **Per-location asset detail** — click any location to see item breakdown sorted by value
- **Copy Multibuy** — formats `TypeName Qty` clipboard text for the EVE in-game market
- **Dark EVE theme** — navy/gold palette, monospaced ISK values, fixed table layout

### Architecture
- Tauri 2.0 desktop app (Rust backend + React 18 + TypeScript frontend)
- SQLite database with embedded migrations (`sqlx::migrate!`)
- PKCE verifier/challenge generated fresh per login session
- ESI batch ID filtering: NPC IDs only (`< 100,000,000`) to prevent ESI 404 batch poisoning
- Standalone `HangarSweep.exe` — no installer required

### Security
- Client secret removed from source; injected at build time via `EVE_CLIENT_SECRET` env var
- All data stored locally; no analytics or telemetry

---

[Unreleased]: https://github.com/jbaycroft/HangarSweep/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jbaycroft/HangarSweep/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jbaycroft/HangarSweep/releases/tag/v0.1.0
