-- HangarSweep Database Schema
-- Migration: 0002_jita_prices
--
-- Adds Jita (The Forge) real-time order-book prices, refreshed every 24h alongside
-- the existing ESI market averages.  Stores the minimum sell-order price (what you
-- would list at to undercut) and the maximum buy-order price (what you would get
-- immediately from a buy order) for every type seen in Jita.

CREATE TABLE IF NOT EXISTS jita_prices (
    type_id     INTEGER PRIMARY KEY,
    sell_min    REAL    NOT NULL DEFAULT 0.0,   -- lowest active sell order (ISK)
    buy_max     REAL    NOT NULL DEFAULT 0.0,   -- highest active buy  order (ISK)
    last_updated INTEGER NOT NULL DEFAULT 0
);
