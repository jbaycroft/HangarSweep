import { describe, it, expect } from 'vitest';
import { formatIsk } from '../types';

// ─── formatIsk ────────────────────────────────────────────────────────────────

describe('formatIsk', () => {
  it('renders trillions', () => {
    expect(formatIsk(2_500_000_000_000)).toBe('2.50T ISK');
  });

  it('renders billions', () => {
    expect(formatIsk(1_500_000_000)).toBe('1.50B ISK');
  });

  it('renders millions', () => {
    expect(formatIsk(42_000_000)).toBe('42.00M ISK');
  });

  it('renders thousands', () => {
    expect(formatIsk(5_500)).toBe('5.5K ISK');
  });

  it('renders exact ISK below 1K', () => {
    expect(formatIsk(999)).toBe('999 ISK');
  });

  it('rounds sub-1K values', () => {
    expect(formatIsk(0)).toBe('0 ISK');
  });
});

// ─── Jita price helpers (mirrors AssetDetail logic) ───────────────────────────

type PriceMode = 'avg' | 'jita_sell' | 'jita_buy';

interface MockAsset {
  quantity: number;
  estimated_value: number;
  jita_sell: number;
  jita_buy: number;
}

function unitPrice(asset: MockAsset, mode: PriceMode): number {
  switch (mode) {
    case 'jita_sell': return asset.jita_sell;
    case 'jita_buy':  return asset.jita_buy;
    default:          return asset.quantity > 0 ? asset.estimated_value / asset.quantity : 0;
  }
}

function stackValue(asset: MockAsset, mode: PriceMode): number {
  switch (mode) {
    case 'jita_sell': return asset.jita_sell * asset.quantity;
    case 'jita_buy':  return asset.jita_buy  * asset.quantity;
    default:          return asset.estimated_value;
  }
}

describe('Jita price mode helpers', () => {
  const asset: MockAsset = {
    quantity: 1000,
    estimated_value: 5000,   // 5 ISK avg
    jita_sell: 6.5,
    jita_buy:  5.8,
  };

  // ── unitPrice ──────────────────────────────────────────────────────────────

  it('unitPrice(avg) divides estimated_value by quantity', () => {
    expect(unitPrice(asset, 'avg')).toBeCloseTo(5.0);
  });

  it('unitPrice(jita_sell) returns jita_sell directly', () => {
    expect(unitPrice(asset, 'jita_sell')).toBe(6.5);
  });

  it('unitPrice(jita_buy) returns jita_buy directly', () => {
    expect(unitPrice(asset, 'jita_buy')).toBe(5.8);
  });

  it('unitPrice(avg) returns 0 when quantity is 0', () => {
    expect(unitPrice({ ...asset, quantity: 0 }, 'avg')).toBe(0);
  });

  // ── stackValue ─────────────────────────────────────────────────────────────

  it('stackValue(avg) returns estimated_value', () => {
    expect(stackValue(asset, 'avg')).toBe(5000);
  });

  it('stackValue(jita_sell) multiplies jita_sell × quantity', () => {
    expect(stackValue(asset, 'jita_sell')).toBeCloseTo(6500);
  });

  it('stackValue(jita_buy) multiplies jita_buy × quantity', () => {
    expect(stackValue(asset, 'jita_buy')).toBeCloseTo(5800);
  });

  // ── delta computation ──────────────────────────────────────────────────────

  it('computes positive delta when jita_sell > avg', () => {
    const avgPerUnit = asset.quantity > 0 ? asset.estimated_value / asset.quantity : 0;
    const uv = unitPrice(asset, 'jita_sell');
    const delta = avgPerUnit > 0 ? ((uv - avgPerUnit) / avgPerUnit) * 100 : null;
    expect(delta).not.toBeNull();
    expect(delta!).toBeCloseTo(30.0); // (6.5 - 5) / 5 * 100 = 30%
  });

  it('computes positive delta when jita_buy > avg', () => {
    const avgPerUnit = asset.quantity > 0 ? asset.estimated_value / asset.quantity : 0;
    const uv = unitPrice(asset, 'jita_buy');
    const delta = avgPerUnit > 0 ? ((uv - avgPerUnit) / avgPerUnit) * 100 : null;
    expect(delta).not.toBeNull();
    expect(delta!).toBeCloseTo(16.0); // (5.8 - 5) / 5 * 100 = 16%
  });

  it('returns null delta when avgPerUnit is 0', () => {
    const zeroAsset: MockAsset = { ...asset, estimated_value: 0 };
    const avgPerUnit = zeroAsset.quantity > 0 ? zeroAsset.estimated_value / zeroAsset.quantity : 0;
    const uv = unitPrice(zeroAsset, 'jita_sell');
    const delta = avgPerUnit > 0 ? ((uv - avgPerUnit) / avgPerUnit) * 100 : null;
    expect(delta).toBeNull();
  });

  // ── no-data zero sentinel ──────────────────────────────────────────────────

  it('stackValue is 0 when jita prices are 0 (no data)', () => {
    const noData: MockAsset = { ...asset, jita_sell: 0, jita_buy: 0 };
    expect(stackValue(noData, 'jita_sell')).toBe(0);
    expect(stackValue(noData, 'jita_buy')).toBe(0);
  });

  it('unitPrice is 0 when jita_sell is 0 (no data)', () => {
    const noData: MockAsset = { ...asset, jita_sell: 0 };
    expect(unitPrice(noData, 'jita_sell')).toBe(0);
  });
});
