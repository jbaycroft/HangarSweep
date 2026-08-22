# HangarSweep — Project Roadmap

> **Vision:** The Bloomberg Terminal for EVE Online — a unified wealth intelligence platform that tells a capsuleer what they made, what to do next, and what opportunities they'll miss if they don't act today.

---

## How to orient yourself after a break

1. **Read this file top to bottom** — current status is always here
2. **Check git log** — `git log --oneline -10` shows recent work
3. **Run tests** — `cargo test` (Rust, 47+ passing) + `npm test` (TS, 34+ passing)
4. **Check the branch** — you should be on `main` or the next feature branch
5. **Read the next PR's section** — full description, acceptance criteria, branch name

---

## What's shipped

| Version | Date | Key Feature |
|---|---|---|
| `v0.1.0` | 2026-08-18 | Core app: EVE SSO, asset sync, location list, Multibuy export, dark theme |
| `v0.2.0` | 2026-08-22 | **Jita price comparison** — sell/buy order prices, price mode toggle, ±% delta |

**Current branch:** `main` (feature/jita-price-comparison has been submitted as PR)  
**Test count:** 47 Rust / 34 TypeScript  
**Active ESI scopes:** `publicData esi-assets.read_assets.v1 esi-universe.read_structures.v1 esi-ui.open_window.v1 esi-ui.write_waypoint.v1`

---

## PR Stack — Master Tracking Table

Legend: `⬜ Not started` · `🟦 In progress` · `✅ Complete` · `🔴 Blocked`

| # | Status | Branch | Title | Depends On |
|---|---|---|---|---|
| 01 | ⬜ | `feat/sync-daemon` | Background Sync Daemon | — |
| 02 | ⬜ | `feat/sde-integration` | SDE Integration | 01 |
| 03 | ⬜ | `feat/skills-margin` | Character Skills + Real Margin Calculator | 01 |
| 04 | ⬜ | `feat/wallet-fetch` | Wallet Transactions + Journal Fetch | 03 |
| 05 | ⬜ | `feat/pnl-engine` | Realized P&L Engine | 04 |
| 06 | ⬜ | `feat/pnl-ui` | Wallet P&L UI | 05 |
| 07 | ⬜ | `feat/active-orders` | Active Orders + Book Position Tracking | 03 |
| 08 | ⬜ | `feat/order-monitor-ui` | Active Order Monitor UI | 07 |
| 09 | ⬜ | `feat/all-regions` | All-Region Order Books + Arbitrage Routes | 01 |
| 10 | ⬜ | `feat/market-history` | Market History + Trend Analysis | 09 |
| 11 | ⬜ | `feat/arbitrage-ui` | Arbitrage Finder UI + Sparklines | 10 |
| 12 | ⬜ | `feat/manufacturing-calc` | Blueprint Fetch + Manufacturing Cost Calculator | 02, 03, 09 |
| 13 | ⬜ | `feat/manufacturing-ui` | Manufacturing Analyzer UI | 12 |
| 14 | ⬜ | `feat/killmail-feed` | zKillboard Killmail Feed | 01 |
| 15 | ⬜ | `feat/demand-signals` | Demand Signal Engine | 14, 10 |
| 16 | ⬜ | `feat/net-worth` | Net Worth Snapshot Tracker | 04, 07 |
| 17 | ⬜ | `feat/opportunity-scorer` | Opportunity Scorer (Intelligence Engine) | 05, 07, 10, 15, 16 |
| 18 | ⬜ | `feat/daily-playbook` | Daily Playbook UI | 17 |
| 19 | ⬜ | `feat/corp-endpoints` | Corporation Endpoints + Data | 01 |
| 20 | ⬜ | `feat/corp-dashboard` | Corporation Dashboard UI | 19 |
| 21 | ⬜ | `feat/multi-char-aggregate` | Multi-Character Aggregate View + Sync | 18 |

---

## PR Detail Sheets

Each section below is the full spec for one PR. When starting a PR, read its sheet. When finishing, tick the acceptance criteria and update the status above.

---

### PR 01 — Background Sync Daemon
**Branch:** `feat/sync-daemon`  
**Status:** ⬜ Not started  
**New ESI scopes:** none  
**Estimated size:** ~400 lines Rust, ~150 lines TypeScript  

**Why:** Everything in phases 2-9 requires data refreshed on its own schedule. The current manual sync button is a blocking UX dead-end. This PR replaces it with a daemon.

**What to build:**

*Rust (`src-tauri/src/sync_scheduler.rs` — new file):*
- `SyncScheduler` struct holding a `HashMap<&'static str, SyncSource>`
- Each `SyncSource` has: `interval_secs`, `last_run: Arc<Mutex<i64>>`, `run_fn: Box<dyn Fn() -> Future>`
- `scheduler.start()` spawns one `tokio::task` per source in a loop: check if `now - last_run > interval`, run if yes, sleep 30s, repeat
- Sources registered at startup: `market_prices` (86400s), `jita_prices` (300s), `assets` (86400s), `character_orders` (stub, 300s), `wallet_transactions` (stub, 300s)

*DB (`sync_state` table — new migration `0003_sync_state.sql`):*
```sql
CREATE TABLE IF NOT EXISTS sync_state (
    source      TEXT    PRIMARY KEY,
    last_run    INTEGER NOT NULL DEFAULT 0,
    last_status TEXT    NOT NULL DEFAULT 'never',
    last_error  TEXT
);
```

*New IPC command:* `get_sync_state() -> Vec<SyncStateRow>` — returns per-source last-run + status  
*Frontend:* Replace sync button with a sync status panel showing each source's last-run time and status dot (green/amber/red). Manual "Force Sync" button still works for each source individually.

**Acceptance criteria:**
- [ ] All 5 sync sources run on their configured intervals without intervention
- [ ] A crashed individual source does not stop other sources
- [ ] `sync_state` table updates correctly on each run
- [ ] UI shows accurate last-synced time per source
- [ ] Manual force-sync still works
- [ ] `cargo test` passes (add tests for scheduler interval logic)
- [ ] `npm test` passes

**Note:** Keep the existing `sync_all` command working for backwards compatibility — the scheduler calls the same underlying functions.

---

### PR 02 — SDE Integration
**Branch:** `feat/sde-integration`  
**Status:** ⬜ Not started  
**Depends on:** PR 01  
**Estimated size:** ~300 lines Rust, ~200MB data asset (downloaded, never committed)

**Why:** The Static Data Export is the blueprint/material database for the entire game. Without it, manufacturing margin calculation is impossible.

**What to build:**

*SDE source:*
```
https://eve-static-data-export.s3-eu-west-1.amazonaws.com/tranquility/sde.zip
```
Contains YAML files; CCP also publishes a pre-converted SQLite via Fuzzwork:
```
https://www.fuzzwork.co.uk/dump/latest/eve.db.bz2
```
Use the Fuzzwork SQLite — it's ready to query without YAML parsing.

*Rust (`src-tauri/src/sde.rs` — new file):*
- `sde_needs_update(pool)` — checks `sde_version` table vs published date (check monthly)
- `download_and_import_sde(pool, app)` — downloads bz2, decompresses, reads relevant tables, inserts into local DB
- Tables to import from SDE: `industryActivity`, `industryActivityMaterials`, `invTypes` (supplement existing `sde_types`), `planetSchematics`, `invBlueprintTypes`
- Emit progress events during import (can take 10-30 seconds)

*New migrations (`0004_sde_extended.sql`):*
```sql
CREATE TABLE IF NOT EXISTS sde_blueprints (
    bp_type_id      INTEGER NOT NULL,
    product_type_id INTEGER NOT NULL,
    activity        INTEGER NOT NULL,  -- 1=manufacturing, 8=invention
    time_seconds    INTEGER NOT NULL,
    PRIMARY KEY (bp_type_id, activity)
);
CREATE TABLE IF NOT EXISTS sde_activity_materials (
    bp_type_id      INTEGER NOT NULL,
    activity        INTEGER NOT NULL,
    material_type_id INTEGER NOT NULL,
    quantity        INTEGER NOT NULL,
    PRIMARY KEY (bp_type_id, activity, material_type_id)
);
CREATE TABLE IF NOT EXISTS sde_planet_schematics (
    schematic_id    INTEGER PRIMARY KEY,
    cycle_time      INTEGER NOT NULL,
    type_id         INTEGER NOT NULL,  -- product
    quantity        INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS sde_schematic_inputs (
    schematic_id    INTEGER NOT NULL,
    type_id         INTEGER NOT NULL,
    quantity        INTEGER NOT NULL,
    PRIMARY KEY (schematic_id, type_id)
);
CREATE TABLE IF NOT EXISTS sde_version (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    version_date    TEXT NOT NULL,
    imported_at     INTEGER NOT NULL
);
```

**Acceptance criteria:**
- [ ] SDE downloads and imports on first launch without user action
- [ ] Re-import triggers when CCP publishes new version (monthly check)
- [ ] `sde_activity_materials` has correct row count for known items (e.g., Hobgoblin I requires Tritanium)
- [ ] Import is non-blocking (runs in background, emits progress)
- [ ] `cargo test` passes (add test: known BOM query returns expected materials)

---

### PR 03 — Character Skills + Real Margin Calculator
**Branch:** `feat/skills-margin`  
**Status:** ⬜ Not started  
**Depends on:** PR 01  
**New ESI scope:** `esi-skills.read_skills.v1`  
**Estimated size:** ~250 lines Rust, ~100 lines TypeScript

**Why:** Every margin calculation in the app is currently wrong because it ignores broker fees and transaction taxes. This PR makes them accurate.

**What to build:**

*New migration (`0005_character_skills.sql`):*
```sql
CREATE TABLE IF NOT EXISTS character_skills (
    character_id    INTEGER NOT NULL,
    skill_type_id   INTEGER NOT NULL,
    trained_level   INTEGER NOT NULL,
    last_updated    INTEGER NOT NULL,
    PRIMARY KEY (character_id, skill_type_id)
);
```

*Skills to fetch and store (only what we use):*
```
Accounting          (type_id: 16622) — transaction tax reduction
Broker Relations    (type_id: 3446)  — broker fee reduction
Industry            (type_id: 3380)  — manufacturing time
Production Efficiency (type_id: 3387) — material efficiency
Advanced Industry   (type_id: 3388)  — manufacturing time
Mass Production     (type_id: 3387)  — parallel jobs
Supply Chain Management (type_id: 24268) — remote job management
Daytrading          (type_id: 16595) — remote order modification
Marketing           (type_id: 16594) — buy order range
```

*`MarginCalculator` struct (`src-tauri/src/margin.rs` — new file):*
```rust
pub struct TradeCosts {
    pub broker_fee_pct: f64,      // buy-side broker fee
    pub transaction_tax_pct: f64, // sell-side transaction tax
}

impl TradeCosts {
    pub fn net_margin(&self, gross_spread: f64) -> f64 {
        gross_spread - self.broker_fee_pct * 2.0 - self.transaction_tax_pct
    }
    pub fn net_profit(&self, buy_price: f64, sell_price: f64, qty: f64) -> f64 {
        let gross = (sell_price - buy_price) * qty;
        let fees = buy_price * qty * self.broker_fee_pct
                 + sell_price * qty * self.broker_fee_pct
                 + sell_price * qty * self.transaction_tax_pct;
        gross - fees
    }
}

// CCP formulas:
// transaction_tax = 0.08 * (1 - 0.11 * accounting_level)
// broker_fee = 0.03 * (1 - 0.03 * broker_relations_level) * (standing_modifier)
```

*New IPC command:* `get_trade_costs(character_id) -> TradeCosts`  
*Update Jita price comparison in AssetDetail:* Show net margin (after fees) not gross spread

**Acceptance criteria:**
- [ ] Skills sync runs on 24h schedule via daemon
- [ ] `MarginCalculator` produces correct values at all skill levels (unit tested)
- [ ] Jita price panel shows net margin %, not gross
- [ ] `get_trade_costs` IPC returns accurate values
- [ ] `cargo test` passes (add margin formula tests at levels 0, 3, 5)

---

### PR 04 — Wallet Transactions + Journal Fetch
**Branch:** `feat/wallet-fetch`  
**Status:** ⬜ Not started  
**Depends on:** PR 03  
**New ESI scope:** `esi-wallet.read_character_wallet.v1`  
**Estimated size:** ~350 lines Rust, ~50 lines TypeScript

**What to build:**

*New migration (`0006_wallet.sql`):*
```sql
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id              INTEGER PRIMARY KEY,
    character_id    INTEGER NOT NULL,
    date            INTEGER NOT NULL,  -- unix timestamp
    type_id         INTEGER NOT NULL,
    location_id     INTEGER NOT NULL,
    unit_price      REAL    NOT NULL,
    quantity        INTEGER NOT NULL,
    is_buy          INTEGER NOT NULL,  -- 0/1
    client_id       INTEGER NOT NULL,
    tax_paid        REAL    NOT NULL DEFAULT 0.0,
    journal_ref_id  INTEGER
);
CREATE TABLE IF NOT EXISTS wallet_journal (
    id              INTEGER PRIMARY KEY,
    character_id    INTEGER NOT NULL,
    date            INTEGER NOT NULL,
    ref_type        TEXT    NOT NULL,
    amount          REAL    NOT NULL,
    running_balance REAL    NOT NULL,
    first_party_id  INTEGER,
    second_party_id INTEGER,
    context_id      INTEGER,
    description     TEXT
);
CREATE INDEX IF NOT EXISTS idx_wt_char_date  ON wallet_transactions(character_id, date);
CREATE INDEX IF NOT EXISTS idx_wt_char_type  ON wallet_transactions(character_id, type_id);
CREATE INDEX IF NOT EXISTS idx_wj_char_date  ON wallet_journal(character_id, date);
CREATE INDEX IF NOT EXISTS idx_wj_ref_type   ON wallet_journal(character_id, ref_type);
```

*ESI endpoints:*
- `GET /characters/{id}/wallet/transactions/?datasource=tranquility` (paginated)
- `GET /characters/{id}/wallet/journal/?datasource=tranquility` (paginated)
- Both cache at 5 minutes; fetch all pages back to last stored ID

*Sync strategy:* Store the most recent transaction ID per character. On sync, only fetch pages until we reach a known ID (avoid re-fetching all history every 5 minutes).

*New IPC:* `get_wallet_balance(character_id) -> f64`

**Acceptance criteria:**
- [ ] Transactions stored without duplicates across multiple syncs
- [ ] Pagination correctly fetches only new records
- [ ] Journal ref_types all captured correctly
- [ ] 5-minute sync registered with daemon
- [ ] `cargo test` passes

---

### PR 05 — Realized P&L Engine
**Branch:** `feat/pnl-engine`  
**Status:** ⬜ Not started  
**Depends on:** PR 04  
**Estimated size:** ~400 lines Rust

**What to build:**

*FIFO trade matching algorithm:*
For each sell transaction (`is_buy = 0`), find prior buy transactions for the same `type_id` at or near the same `location_id` (same station first, then same region), ordered by `date ASC` (FIFO). Consume buy quantities to cover the sell quantity, computing weighted average cost basis.

*New migration (`0007_pnl.sql`):*
```sql
CREATE TABLE IF NOT EXISTS realized_trades (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id    INTEGER NOT NULL,
    type_id         INTEGER NOT NULL,
    closed_date     INTEGER NOT NULL,
    quantity        INTEGER NOT NULL,
    cost_basis_per_unit REAL NOT NULL,
    proceeds_per_unit   REAL NOT NULL,
    gross_profit    REAL NOT NULL,
    tax_paid        REAL NOT NULL,
    broker_fees     REAL NOT NULL,
    net_profit      REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS daily_pnl (
    character_id        INTEGER NOT NULL,
    date                TEXT    NOT NULL,  -- YYYY-MM-DD
    trading_profit      REAL    NOT NULL DEFAULT 0.0,
    bounty_income       REAL    NOT NULL DEFAULT 0.0,
    manufacturing_income REAL   NOT NULL DEFAULT 0.0,
    contract_income     REAL    NOT NULL DEFAULT 0.0,
    other_income        REAL    NOT NULL DEFAULT 0.0,
    total_expenses      REAL    NOT NULL DEFAULT 0.0,
    net                 REAL    NOT NULL DEFAULT 0.0,
    PRIMARY KEY (character_id, date)
);
```

*Journal income attribution — `ref_type` classification:*
```
bounty_prizes, ess_escrow_transfer → bounty_income
industry_job_tax → manufacturing_income (subtract)
contract_price, contract_reward → contract_income
broker_fee, transaction_tax → total_expenses
player_trading, market_transaction → trading (computed via realized_trades)
```

*New IPC commands:*
- `get_pnl_summary(character_id, period: "today"|"7d"|"30d"|"all") -> PnlSummary`
- `get_income_breakdown(character_id, period) -> Vec<IncomeCategory>`
- `get_top_profitable_items(character_id, period, limit) -> Vec<ItemPnl>`

**Acceptance criteria:**
- [ ] FIFO matching produces correct results for known multi-trade sequences (unit tested)
- [ ] Daily P&L table populated for all historical data on first run
- [ ] Income attribution covers all known `ref_type` values
- [ ] `cargo test` passes (add P&L matching tests)

---

### PR 06 — Wallet P&L UI
**Branch:** `feat/pnl-ui`  
**Status:** ⬜ Not started  
**Depends on:** PR 05  
**Frontend dependency:** `recharts` (`npm install recharts`)  
**Estimated size:** ~500 lines TypeScript/React, ~200 lines CSS

**New component:** `src/components/Ledger.tsx`  
**New route/panel:** "Ledger" tab in the main navigation

**What to build:**
- Period selector: Today / 7d / 30d / All time
- **P&L summary strip**: Net ISK / Gross trading / Fees paid / Bounties / Manufacturing
- **Daily bar chart** (Recharts `BarChart`): 30 bars, green above zero / red below
- **Income breakdown donut** (Recharts `PieChart`): Trading vs Bounty vs Manufacturing vs Other
- **Top earning items table**: type_name / total_qty_sold / avg_margin / total_net_profit
- **Recent trades list**: last 50 with per-trade realized profit (green/red)

**Acceptance criteria:**
- [ ] All charts render with no data, single day, and full 30-day data
- [ ] Period selector changes all views simultaneously
- [ ] Negative P&L days shown in red
- [ ] `npm test` passes (add component tests)

---

### PR 07 — Active Orders + Book Position Tracking
**Branch:** `feat/active-orders`  
**Status:** ⬜ Not started  
**Depends on:** PR 03 (margin calculator), PR 09 (book data — integrate after PR 09 merges)  
**New ESI scope:** `esi-markets.read_character_orders.v1`  
**Estimated size:** ~400 lines Rust, ~100 lines TypeScript

**What to build:**

*New migration (`0008_active_orders.sql`):*
```sql
CREATE TABLE IF NOT EXISTS active_orders (
    order_id            INTEGER PRIMARY KEY,
    character_id        INTEGER NOT NULL,
    type_id             INTEGER NOT NULL,
    location_id         INTEGER NOT NULL,
    region_id           INTEGER NOT NULL,
    is_buy              INTEGER NOT NULL,
    price               REAL    NOT NULL,
    volume_remain       INTEGER NOT NULL,
    volume_total        INTEGER NOT NULL,
    issued              INTEGER NOT NULL,
    duration_days       INTEGER NOT NULL,
    min_volume          INTEGER NOT NULL DEFAULT 1,
    status              TEXT    NOT NULL DEFAULT 'active',
    book_position       INTEGER,        -- 1 = best price in book
    best_competing_price REAL,
    fills_24h           INTEGER NOT NULL DEFAULT 0,
    last_updated        INTEGER NOT NULL
);
```

*Book position calculation (runs after order fetch):*
```
For each sell order:
  query regional_prices for the same type_id + region
  if my price <= sell_min → LEADING (I am the book)
  elif my price <= sell_min * 1.01 → CLOSE (within 1%)
  else → UNDERCUT (someone is cheaper)

For each buy order:
  if my price >= buy_max → LEADING
  elif my price >= buy_max * 0.99 → CLOSE
  else → UNDERCUT
```

*Order status classification:*
- `LEADING` — best price in book
- `UNDERCUT` — not best price
- `STALE` — no fills in 7 days AND volume_remain = volume_total
- `FILLING` — volume_remain < volume_total AND changed in last 24h

*New IPC commands:*
- `get_active_orders(character_id) -> Vec<ActiveOrderRow>`
- `get_undercut_orders(character_id) -> Vec<ActiveOrderRow>` (UNDERCUT status only, sorted by ISK at stake)

**Acceptance criteria:**
- [ ] Orders sync every 5 minutes via daemon
- [ ] Book position computed correctly (unit tested)
- [ ] Status classification tested at each case
- [ ] `cargo test` passes

---

### PR 08 — Active Order Monitor UI
**Branch:** `feat/order-monitor-ui`  
**Status:** ⬜ Not started  
**Depends on:** PR 07  
**Estimated size:** ~450 lines TypeScript/React, ~150 lines CSS

**New component:** `src/components/OrderMonitor.tsx`

**What to build:**
- New "Orders" panel/tab
- Order table with columns: Type / Buy-Sell / My Price / Competition / Book Position / Status / Volume / Age / ISK at stake
- Status badge: 🔴 UNDERCUT / 🟢 LEADING / 🟡 STALE / 🔵 FILLING
- Default sort: UNDERCUT first, then by ISK-at-stake descending
- **Relist price helper**: for undercut orders, show `(best_competing_price - 0.01)` with one-click copy
- Aggregate bar: "47 orders / 12 undercut / 3B ISK in orders / 120M/day fill rate"
- Filter buttons: All / Undercut only / Selling / Buying

**Acceptance criteria:**
- [ ] Status badges correct for all states
- [ ] Relist price math correct (0.01 ISK below competition)
- [ ] Copy to clipboard works
- [ ] `npm test` passes

---

### PR 09 — All-Region Order Books + Arbitrage Routes
**Branch:** `feat/all-regions`  
**Status:** ⬜ Not started  
**Depends on:** PR 01  
**Estimated size:** ~300 lines Rust, ~100 lines TypeScript

**Why:** The Jita price comparison (PR v0.2.0) fetches only The Forge. Extending to all 64 regions with the same concurrent pattern opens the full arbitrage map.

**What to build:**

*New migration (`0009_regional_prices.sql`):*
```sql
CREATE TABLE IF NOT EXISTS regional_prices (
    type_id         INTEGER NOT NULL,
    region_id       INTEGER NOT NULL,
    sell_min        REAL    NOT NULL DEFAULT 0.0,
    buy_max         REAL    NOT NULL DEFAULT 0.0,
    sell_volume     INTEGER NOT NULL DEFAULT 0,
    buy_volume      INTEGER NOT NULL DEFAULT 0,
    last_updated    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (type_id, region_id)
);
CREATE TABLE IF NOT EXISTS arbitrage_routes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    type_id         INTEGER NOT NULL,
    buy_region_id   INTEGER NOT NULL,
    sell_region_id  INTEGER NOT NULL,
    net_per_unit    REAL    NOT NULL,
    volume_available INTEGER NOT NULL,
    total_profit    REAL    NOT NULL,
    m3_required     REAL    NOT NULL,
    isk_per_m3      REAL    NOT NULL,
    scored_at       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_arb_isk ON arbitrage_routes(isk_per_m3 DESC);
```

*Regions to fetch (the 8 major market hubs cover 90% of trade):*
```
10000002 The Forge      (Jita)     — already fetched
10000043 Domain         (Amarr)
10000032 Sinq Laison    (Dodixie)
10000030 Heimatar       (Rens)
10000042 Metropolis     (Hek)
10000016 Lonetrek       (Caldari high-sec adjacent)
10000033 The Citadel    (Perimeter structures)
10000038 The Bleak Lands (null-adjacent)
```
(Extendable to all 64 later — same pattern, just more concurrent fetches)

*Arbitrage scoring (runs after regional fetch):*
```
for each type_id where Forge_sell_min > 0 and Target_buy_max > 0:
  gross = target_buy_max - forge_sell_min
  net   = gross - (forge_sell_min * broker_fee) - (target_buy_max * broker_fee) - (target_buy_max * tx_tax)
  if net > 0:
    profit_per_m3 = net / packaged_volume (from sde_types)
    record in arbitrage_routes
```

**Acceptance criteria:**
- [ ] All 8 regions fetch concurrently without hitting ESI rate limits
- [ ] Arbitrage scoring math tested against known route (Jita→Amarr)
- [ ] `jita_prices` table updated to use `regional_prices` as source of truth for The Forge
- [ ] `cargo test` passes

---

### PR 10 — Market History + Trend Analysis
**Branch:** `feat/market-history`  
**Status:** ⬜ Not started  
**Depends on:** PR 09  
**Estimated size:** ~400 lines Rust, ~50 lines TypeScript

**What to build:**

*New migration (`0010_market_history.sql`):*
```sql
CREATE TABLE IF NOT EXISTS market_history (
    type_id         INTEGER NOT NULL,
    region_id       INTEGER NOT NULL,
    date            TEXT    NOT NULL,  -- YYYY-MM-DD
    average         REAL    NOT NULL,
    highest         REAL    NOT NULL,
    lowest          REAL    NOT NULL,
    order_count     INTEGER NOT NULL,
    volume          INTEGER NOT NULL,
    PRIMARY KEY (type_id, region_id, date)
);
CREATE TABLE IF NOT EXISTS market_signals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    type_id         INTEGER NOT NULL,
    region_id       INTEGER NOT NULL,
    signal_type     TEXT    NOT NULL,  -- RISING, FALLING, VOLATILE, ANOMALY, SPREAD_WIDENING
    signal_value    REAL,
    severity        TEXT    NOT NULL,  -- INFO, WARN, ALERT
    detected_at     INTEGER NOT NULL,
    expires_at      INTEGER,
    detail          TEXT
);
CREATE INDEX IF NOT EXISTS idx_mh_type_region ON market_history(type_id, region_id, date);
CREATE INDEX IF NOT EXISTS idx_signal_type    ON market_signals(type_id, region_id, detected_at);
```

*Fetch strategy:* Only fetch history for type_ids in `assets`, `active_orders`, or `arbitrage_routes` — not all 10,000 types. Cache indefinitely (append-only), only fetch days not yet in DB.

*Trend classifier (nightly task):*
```
slope_7d  = linear regression of avg price over last 7 days
slope_30d = linear regression of avg price over last 30 days
stddev_90d = standard deviation of avg price over last 90 days

RISING:   slope_7d > 0 AND slope_30d > 0 AND volume expanding
FALLING:  slope_7d < 0 AND slope_30d < 0
VOLATILE: stddev_90d / mean > 0.25
ANOMALY:  today_avg > (mean_90d + 2.0 * stddev_90d)  ← price spike
SPREAD_WIDENING: (sell_min - buy_max) / sell_min growing vs 30d avg
```

**Acceptance criteria:**
- [ ] History fetches only for tracked items
- [ ] Append-only: re-sync never duplicates existing dates
- [ ] Trend classifier unit tested at known sequences
- [ ] Anomaly threshold catches real EVE price spikes (backtest against Tritanium 2023-2024 data)

---

### PR 11 — Arbitrage Finder UI + Market Sparklines
**Branch:** `feat/arbitrage-ui`  
**Status:** ⬜ Not started  
**Depends on:** PR 10  
**Estimated size:** ~500 lines TypeScript/React, ~200 lines CSS

**New component:** `src/components/ArbitrageFinder.tsx`

**What to build:**
- New "Arbitrage" panel/tab
- Route table: Item / Buy Region / Sell Region / Net margin % / Volume / Total ISK / m³
- **Freight rate slider** (ISK/m³): re-scores all routes in real time
- Filters: max m³ / min total ISK / specific region pair
- Trend badge (↑↓→🔥) next to each item name, sourced from `market_signals`
- 30-day **sparkline** (inline Recharts `LineChart`) in each row
- **Add to watchlist** per item (persisted to new `watchlist` table)
- Inline sparklines also added to the main location list asset rows

**Acceptance criteria:**
- [ ] Freight re-scoring is instant (client-side math, not a new IPC call)
- [ ] Routes sorted by ISK/m³ descending by default
- [ ] Sparklines render correctly with 30-day data
- [ ] `npm test` passes

---

### PR 12 — Blueprint Fetch + Manufacturing Cost Calculator
**Branch:** `feat/manufacturing-calc`  
**Status:** ⬜ Not started  
**Depends on:** PR 02 (SDE), PR 03 (skills), PR 09 (material prices)  
**New ESI scopes:** `esi-characters.read_blueprints.v1`, `esi-industry.read_character_jobs.v1`  
**Estimated size:** ~500 lines Rust, ~100 lines TypeScript

**What to build:**

*New migration (`0011_manufacturing.sql`):*
```sql
CREATE TABLE IF NOT EXISTS character_blueprints (
    item_id         INTEGER PRIMARY KEY,
    character_id    INTEGER NOT NULL,
    type_id         INTEGER NOT NULL,
    location_id     INTEGER NOT NULL,
    quantity        INTEGER NOT NULL,  -- -1 = BPO (unlimited), >0 = BPC runs
    material_efficiency INTEGER NOT NULL,
    time_efficiency INTEGER NOT NULL,
    last_updated    INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS industry_jobs (
    job_id          INTEGER PRIMARY KEY,
    character_id    INTEGER NOT NULL,
    blueprint_type_id INTEGER NOT NULL,
    product_type_id INTEGER NOT NULL,
    facility_id     INTEGER NOT NULL,
    start_date      INTEGER NOT NULL,
    end_date        INTEGER NOT NULL,
    runs            INTEGER NOT NULL,
    status          TEXT    NOT NULL,
    cost            REAL    NOT NULL
);
CREATE TABLE IF NOT EXISTS industry_indices (
    solar_system_id INTEGER NOT NULL,
    activity        INTEGER NOT NULL,
    cost_index      REAL    NOT NULL,
    last_updated    INTEGER NOT NULL,
    PRIMARY KEY (solar_system_id, activity)
);
CREATE TABLE IF NOT EXISTS manufacturing_costs (
    type_id         INTEGER NOT NULL,
    character_id    INTEGER NOT NULL,
    facility_id     INTEGER NOT NULL DEFAULT 0,
    material_cost   REAL    NOT NULL,
    index_cost      REAL    NOT NULL,
    total_cost      REAL    NOT NULL,
    jita_sell_price REAL    NOT NULL,
    margin_pct      REAL    NOT NULL,
    computed_at     INTEGER NOT NULL,
    PRIMARY KEY (type_id, character_id, facility_id)
);
```

*Manufacturing cost formula:*
```
adjusted_qty     = ceil(base_qty * (1 - ME_level * 0.01) * (1 - facility_ME_bonus))
material_cost    = SUM(adjusted_qty[i] * jita_buy_max[material_type_id[i]])
estimated_value  = product_jita_sell_min * runs
index_cost       = estimated_value * system_cost_index * activity_modifier
total_cost       = material_cost + index_cost
margin_pct       = (jita_sell_min - total_cost/runs) / jita_sell_min
```

**Acceptance criteria:**
- [ ] BOM calculation unit tested against known items (Hobgoblin I, Rifter, Tengu)
- [ ] ME adjustment formula correct at 0, 5, 10
- [ ] `make_vs_buy` analysis for intermediates correct
- [ ] `cargo test` passes

---

### PR 13 — Manufacturing Analyzer UI
**Branch:** `feat/manufacturing-ui`  
**Status:** ⬜ Not started  
**Depends on:** PR 12  
**Estimated size:** ~600 lines TypeScript/React, ~200 lines CSS

**New component:** `src/components/IndustryPanel.tsx`

**What to build:**
- New "Industry" panel with two sub-views:
  - **Blueprint Library**: sortable table of all BPOs/BPCs with margin, make/skip badge
  - **Active Jobs**: countdown timers, projected profit, "queue next" button
- **Waterfall chart** (stacked bar): Materials / Index / Fees / Margin per item
- **Job queue optimizer**: given current blueprints and available capital, suggest next N runs
- System cost index comparison: show cheapest build location for selected blueprint
- Notification when job completes (via daemon polling `industry_jobs`)

**Acceptance criteria:**
- [ ] Blueprint library loads and sorts by margin correctly
- [ ] Countdown timers accurate
- [ ] Waterfall chart renders for known items
- [ ] `npm test` passes

---

### PR 14 — zKillboard Killmail Feed
**Branch:** `feat/killmail-feed`  
**Status:** ⬜ Not started  
**Depends on:** PR 01  
**External API:** `https://zkillboard.com/api/` (no auth required)  
**Estimated size:** ~250 lines Rust

**Rate limit:** zKillboard enforces 1 request/second and requires a User-Agent header identifying the app. Use a `tokio::Semaphore` with 1 permit.

**What to build:**

*New migration (`0012_killmails.sql`):*
```sql
CREATE TABLE IF NOT EXISTS killmail_stats (
    type_id         INTEGER PRIMARY KEY,
    kills_24h       INTEGER NOT NULL DEFAULT 0,
    kills_7d        INTEGER NOT NULL DEFAULT 0,
    kills_30d       INTEGER NOT NULL DEFAULT 0,
    avg_ship_value  REAL    NOT NULL DEFAULT 0.0,
    last_updated    INTEGER NOT NULL DEFAULT 0
);
```

*Fetch strategy:*
- Maintain a list of ~300 most-traded ship types (sourced from `active_orders` + `arbitrage_routes` + a hardcoded top-200 list)
- For each: `GET https://zkillboard.com/api/stats/shipTypeID/{type_id}/` returns 7d/30d kill counts
- Run on 1-hour schedule, rate-limited to 1 req/sec
- Also: `GET https://zkillboard.com/api/kills/shipTypeID/{id}/` for recent kill stream

**Acceptance criteria:**
- [ ] Rate limiter enforced (1 req/sec, tested with a mock)
- [ ] Kill stats stored and updated without duplication
- [ ] Graceful degradation when zKillboard is unavailable (keep stale data, don't crash)
- [ ] `cargo test` passes

---

### PR 15 — Demand Signal Engine
**Branch:** `feat/demand-signals`  
**Status:** ⬜ Not started  
**Depends on:** PR 14, PR 10  
**Estimated size:** ~350 lines Rust

**What to build:**

*Supply pressure model:*
```
consumption_rate = kills_7d / 7.0   (kills per day)
replacement_demand_isk = consumption_rate * avg_ship_value
jita_supply_depth = total_sell_volume in Jita for that type_id (from market_history)
days_of_supply = jita_supply_depth / (consumption_rate * fill_rate_factor)

if days_of_supply < 7.0:
    signal = DEMAND_SHOCK (severity: ALERT)
elif days_of_supply < 14.0:
    signal = DEMAND_PRESSURE (severity: WARN)
```

Write to `market_signals` with `signal_type = 'DEMAND_SHOCK'` or `'DEMAND_PRESSURE'`.

Also generate signals for: unusual kill spike (kills_24h > 3× daily average from kills_30d)

**Acceptance criteria:**
- [ ] Supply pressure formula tested against known wartime data
- [ ] Signals written to `market_signals` with correct severity
- [ ] Deduplication: don't create new signal if unexpired signal exists for same type_id + region

---

### PR 16 — Net Worth Snapshot Tracker
**Branch:** `feat/net-worth`  
**Status:** ⬜ Not started  
**Depends on:** PR 04 (wallet), PR 07 (orders)  
**Estimated size:** ~200 lines Rust, ~300 lines TypeScript/React

**What to build:**

*New migration (`0013_net_worth.sql`):*
```sql
CREATE TABLE IF NOT EXISTS net_worth_snapshots (
    character_id    INTEGER NOT NULL,
    snapshot_date   TEXT    NOT NULL,  -- YYYY-MM-DD
    liquid_isk      REAL    NOT NULL,
    orders_value    REAL    NOT NULL,  -- active orders at listed price
    assets_value_sell REAL NOT NULL,  -- assets at Jita sell
    assets_value_buy  REAL NOT NULL,  -- assets at Jita buy
    wip_value       REAL    NOT NULL DEFAULT 0.0,  -- in-progress industry jobs
    total_sell      REAL    NOT NULL,
    total_buy       REAL    NOT NULL,
    PRIMARY KEY (character_id, snapshot_date)
);
```

One snapshot written daily (on first sync of each day). Never overwrite existing dates.

*Frontend:*
- Net worth line chart: 90 days, two lines (sell value / buy value)
- Wealth velocity: "+X ISK/day" (slope of last 7 days)
- Breakdown today: Liquid / Orders / Assets / WIP (stacked area or donut)

**Acceptance criteria:**
- [ ] Snapshot written once per calendar day
- [ ] Wealth velocity calculation unit tested
- [ ] Chart renders with 1, 7, and 90 data points correctly
- [ ] `npm test` passes

---

### PR 17 — Opportunity Scorer (Intelligence Engine)
**Branch:** `feat/opportunity-scorer`  
**Status:** ⬜ Not started  
**Depends on:** PRs 05, 07, 10, 15, 16  
**Estimated size:** ~600 lines Rust (pure analytics, no UI changes)

**This is the integration PR.** It reads from every data source and produces scored, ranked, actionable opportunities.

*New migration (`0014_opportunities.sql`):*
```sql
CREATE TABLE IF NOT EXISTS opportunities (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id    INTEGER NOT NULL,
    category        TEXT    NOT NULL,
    -- UNDERCUT_ORDER, STATION_TRADE, ARBITRAGE, DEMAND_SHOCK,
    -- MANUFACTURING, STALE_ORDER, JOB_COMPLETE, FALLING_MARKET
    type_id         INTEGER,
    isk_potential   REAL    NOT NULL,
    capital_required REAL   NOT NULL DEFAULT 0.0,
    risk_score      REAL    NOT NULL DEFAULT 0.5,  -- 0.0 (low) to 1.0 (high)
    urgency         INTEGER NOT NULL DEFAULT 5,     -- 1 (now) to 10 (someday)
    action_text     TEXT    NOT NULL,
    detail_json     TEXT,
    generated_at    INTEGER NOT NULL,
    expires_at      INTEGER,
    is_dismissed    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_opp_char_urgency ON opportunities(character_id, urgency, isk_potential DESC);
```

*Opportunity generators (each is a Rust function returning `Vec<Opportunity>`):*

| Generator | Source data | What it creates |
|---|---|---|
| `gen_undercut_alerts` | active_orders | One per undercut order, urgency=1, ISK = (position_value × 0.15/day) |
| `gen_station_trades` | regional_prices + skills | Top 20 items by net ISK/day not currently in orders |
| `gen_arbitrage` | arbitrage_routes | Top 10 routes by total ISK, urgency scales with route expiry |
| `gen_demand_shock_positions` | market_signals + active_orders | Items with DEMAND_SHOCK where player has no sell orders |
| `gen_manufacturing` | manufacturing_costs | Blueprints where margin > 15% and player isn't currently running jobs |
| `gen_falling_market` | market_signals (FALLING) + assets | Assets in player's hangars that are in falling markets |
| `gen_job_complete` | industry_jobs | Jobs completed with no follow-up job queued |

*Scoring runs every 15 minutes.* Old opportunities (> 1h) replaced by fresh ones. Dismissed opportunities excluded by their `expires_at`.

**Acceptance criteria:**
- [ ] Each generator unit tested with mock data
- [ ] Urgency + ISK scoring produces correct ranking order
- [ ] Dismissed items not regenerated until they expire
- [ ] 15-minute run completes in < 500ms on typical data size
- [ ] `cargo test` passes

---

### PR 18 — Daily Playbook UI
**Branch:** `feat/daily-playbook`  
**Status:** ⬜ Not started  
**Depends on:** PR 17  
**Estimated size:** ~700 lines TypeScript/React, ~300 lines CSS

**This is the north star feature.** New default home screen.

**What to build:**

*Morning Briefing header (always visible):*
```
Yesterday: +4.7B ISK  |  7-day avg: +3.2B/day  |  Net worth: 847B  |  Velocity: ↑
```

*Three sections:*
1. **🔴 Urgent** (urgency 1-3): Do these now. Undercut orders, completed jobs, expiring routes.
2. **📈 Opportunities** (urgency 4-7): Act today. Station trades, arbitrage, demand shocks.
3. **👁 Watch** (urgency 8-10): No action needed. Monitor these.

*Per-opportunity card:*
- Category icon + type name (if applicable)
- Action text (e.g., "Relist Tengu — undercut by 2.4M ISK")
- ISK potential + capital required
- One-click action button (copy price / open in Jita / dismiss)
- Dismiss with expiry choice (1h / 24h / 7d / forever)

*System tray notification:* Tauri `tauri-plugin-notification` — push alert when urgent items appear while app is minimized.

**Acceptance criteria:**
- [ ] All three sections render correctly with 0, 1, and many opportunities
- [ ] Dismiss persists and resets at correct expiry
- [ ] System tray notification fires when urgency-1 opportunity generated
- [ ] `npm test` passes

---

### PR 19 — Corporation Endpoints + Data
**Branch:** `feat/corp-endpoints`  
**Status:** ⬜ Not started  
**Depends on:** PR 01  
**New ESI scopes:** `esi-corporations.read_wallets.v1`, `esi-corporations.read_structures.v1`, `esi-corporations.read_assets.v1`  
**Note:** These scopes require the character to have Accountant/Director roles in their corp.  
**Estimated size:** ~500 lines Rust

*(Full detail spec to be written when previous PRs are complete)*

---

### PR 20 — Corporation Dashboard UI
**Branch:** `feat/corp-dashboard`  
**Status:** ⬜ Not started  
**Depends on:** PR 19  
**Estimated size:** ~500 lines TypeScript/React, ~150 lines CSS

*(Full detail spec to be written when previous PRs are complete)*

---

### PR 21 — Multi-Character Aggregate View + Sync
**Branch:** `feat/multi-char-aggregate`  
**Status:** ⬜ Not started  
**Depends on:** PR 18  
**Estimated size:** ~400 lines Rust, ~350 lines TypeScript

*(Full detail spec to be written when previous PRs are complete)*

---

## Architectural Decisions Log

Decisions recorded here so context is never lost.

| # | Decision | Rationale | Date |
|---|---|---|---|
| ADR-01 | Tauri 2.0 + Rust backend + React frontend + SQLite | Native perf for concurrent ESI fetches; SQLite handles 30M+ rows easily; no server needed | 2026-08-18 |
| ADR-02 | Append-only tables for all time-series data | Never lose historical P&L/price data; SQLite writes are cheap | 2026-08-22 |
| ADR-03 | `jita_prices` stores only sell_min and buy_max, not full order book | Full order book (~400 pages × ~1000 orders) is too large to store; aggregated values are sufficient for 95% of use cases | 2026-08-22 |
| ADR-04 | FIFO trade matching for P&L | Matches standard accounting practice and how most EVE traders mentally track their trades | 2026-08-22 |
| ADR-05 | Fuzzwork SDE SQLite over CCP YAML SDE | Pre-converted, ~200MB vs YAML parsing complexity; trusted community source | 2026-08-22 |
| ADR-06 | Opportunity scorer runs every 15 minutes, not on-demand | Keeps UI instant-responsive; fresh enough for trading decisions; avoids compute spike on user interaction | 2026-08-22 |
| ADR-07 | Fetch market history only for tracked items, not all 10K types | Prevents DB bloat; 95% of value is in items you actually own or trade | 2026-08-22 |

---

## Test Coverage Targets

| Layer | Current | Target after all PRs |
|---|---|---|
| Rust (cargo test) | 47 passing | 150+ passing |
| TypeScript (vitest) | 34 passing | 100+ passing |
| Integration | 0 | E2E smoke test for sync pipeline |

---

## ESI Scopes — Cumulative

| Scope | Added in PR | Purpose |
|---|---|---|
| `publicData` | v0.1.0 | Character info |
| `esi-assets.read_assets.v1` | v0.1.0 | Asset ledger |
| `esi-universe.read_structures.v1` | v0.1.0 | Citadel names |
| `esi-ui.open_window.v1` | v0.1.0 | Open market window in-game |
| `esi-ui.write_waypoint.v1` | v0.1.0 | Set autopilot waypoint |
| `esi-skills.read_skills.v1` | PR 03 | Accurate margin calculation |
| `esi-wallet.read_character_wallet.v1` | PR 04 | Transactions + journal |
| `esi-markets.read_character_orders.v1` | PR 07 | Active order monitoring |
| `esi-characters.read_blueprints.v1` | PR 12 | Blueprint library |
| `esi-industry.read_character_jobs.v1` | PR 12 | Manufacturing jobs |
| `esi-corporations.read_wallets.v1` | PR 19 | Corp treasury |
| `esi-corporations.read_structures.v1` | PR 19 | Structure fuel |
| `esi-corporations.read_assets.v1` | PR 19 | Corp hangars |

---

## Dependencies to Add

| Package | Added in PR | Purpose |
|---|---|---|
| `recharts` (npm) | PR 06 | Charts (P&L, sparklines, net worth) |
| `tauri-plugin-notification` | PR 18 | System tray alerts |

---

## Version Plan

| Version | PRs included | Status |
|---|---|---|
| v0.1.0 | Initial release | ✅ Shipped |
| v0.2.0 | Jita price comparison | ✅ Shipped |
| v0.3.0 | PRs 01-03 (daemon + SDE + margin) | ⬜ |
| v0.4.0 | PRs 04-06 (wallet P&L) | ⬜ |
| v0.5.0 | PRs 07-08 (order monitor) | ⬜ |
| v0.6.0 | PRs 09-11 (all-region + arbitrage) | ⬜ |
| v0.7.0 | PRs 12-13 (manufacturing) | ⬜ |
| v0.8.0 | PRs 14-16 (killmails + net worth) | ⬜ |
| v1.0.0 | PRs 17-18 (intelligence engine + playbook) | ⬜ |
| v1.1.0 | PRs 19-20 (corporation) | ⬜ |
| v1.2.0 | PR 21 (multi-character aggregate) | ⬜ |
