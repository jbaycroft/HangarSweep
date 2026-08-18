-- HangarSweep Database Schema
-- Migration: 0001_init

-- ─────────────────────────────────────────
-- Core character / auth storage
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS characters (
    id              INTEGER PRIMARY KEY,
    name            TEXT    NOT NULL,
    access_token    TEXT    NOT NULL,
    refresh_token   TEXT    NOT NULL,
    token_expiry    INTEGER NOT NULL
);

-- ─────────────────────────────────────────
-- Asset ledger (replaced wholesale on each sync)
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS assets (
    item_id         INTEGER PRIMARY KEY,
    character_id    INTEGER NOT NULL,
    type_id         INTEGER NOT NULL,
    location_id     INTEGER NOT NULL,
    location_flag   TEXT    NOT NULL,
    quantity        INTEGER NOT NULL,
    is_singleton    INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(character_id) REFERENCES characters(id)
);

CREATE INDEX IF NOT EXISTS idx_assets_location    ON assets(location_id);
CREATE INDEX IF NOT EXISTS idx_assets_type        ON assets(type_id);
CREATE INDEX IF NOT EXISTS idx_assets_character   ON assets(character_id);

-- ─────────────────────────────────────────
-- Market prices (unauthenticated ESI, refreshed every 24h)
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS market_prices (
    type_id         INTEGER PRIMARY KEY,
    average_price   REAL    NOT NULL,
    last_updated    INTEGER NOT NULL
);

-- ─────────────────────────────────────────
-- Player structure name cache (citadels resolved via ESI)
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS structure_cache (
    id              INTEGER PRIMARY KEY,
    name            TEXT    NOT NULL
);

-- ─────────────────────────────────────────
-- Static Data Export stubs (pre-populate from SDE)
-- Rows are inserted by running:
--   SELECT type_id, typeName FROM invTypes WHERE published = 1
--   SELECT stationID, stationName FROM staStations
-- against the SDE sqlite dump.
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS sde_types (
    type_id         INTEGER PRIMARY KEY,
    type_name       TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS sde_stations (
    station_id      INTEGER PRIMARY KEY,
    name            TEXT    NOT NULL
);
