// ─── IPC response types (mirror Rust structs) ─────────────────────────────────

export interface Character {
  id: number;
  name: string;
  access_token: string;
  refresh_token: string;
  token_expiry: number;
}

export interface LiquidityRow {
  location_id: number;
  location_name: string;
  total_isk_value: number;
  stack_count: number;
}

export interface AssetRow {
  item_id: number;
  type_id: number;
  type_name: string;
  quantity: number;
  estimated_value: number;
  location_flag: string;
  /** Jita minimum sell-order price per unit (0 = no Jita data) */
  jita_sell: number;
  /** Jita maximum buy-order price per unit (0 = no Jita data) */
  jita_buy: number;
}

// ─── Event payloads ───────────────────────────────────────────────────────────

export interface AuthCompletePayload {
  character_id: number;
  character_name: string;
}

export interface AuthErrorPayload {
  message: string;
}

export interface SyncProgressPayload {
  step: string;
  status: 'running' | 'complete' | 'error';
  message?: string;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

export function formatIsk(value: number): string {
  if (value >= 1_000_000_000_000) return `${(value / 1_000_000_000_000).toFixed(2)}T ISK`;
  if (value >= 1_000_000_000)     return `${(value / 1_000_000_000).toFixed(2)}B ISK`;
  if (value >= 1_000_000)         return `${(value / 1_000_000).toFixed(2)}M ISK`;
  if (value >= 1_000)             return `${(value / 1_000).toFixed(1)}K ISK`;
  return `${Math.round(value).toLocaleString()} ISK`;
}
