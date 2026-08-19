# Changelog

All notable changes to HangarSweep are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Version numbers follow [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned
- Multiple character aggregate view
- Per-location ISK history
- Jita buy-order price comparison
- macOS / Linux builds

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

[Unreleased]: https://github.com/jbaycroft/HangarSweep/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jbaycroft/HangarSweep/releases/tag/v0.1.0
