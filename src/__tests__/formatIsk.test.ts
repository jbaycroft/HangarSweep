import { describe, it, expect } from 'vitest';
import { formatIsk } from '../types';

describe('formatIsk', () => {
  // ── Zero and near-zero ──────────────────────────────────────────────────────
  it('returns "0 ISK" for zero', () => {
    expect(formatIsk(0)).toBe('0 ISK');
  });

  it('handles negative values (debt)', () => {
    // Negative values are formatted as-is — no special handling required
    const result = formatIsk(-500);
    expect(result).toContain('ISK');
  });

  // ── Raw ISK (< 1 000) ───────────────────────────────────────────────────────
  it('shows raw ISK for values under 1000', () => {
    expect(formatIsk(1)).toBe('1 ISK');
    expect(formatIsk(999)).toBe('999 ISK');
    expect(formatIsk(42)).toBe('42 ISK');
  });

  it('rounds fractional ISK values under 1000', () => {
    expect(formatIsk(99.7)).toBe('100 ISK');
    expect(formatIsk(0.4)).toBe('0 ISK');
  });

  // ── Thousands (K) ───────────────────────────────────────────────────────────
  it('formats exactly 1 000 as K', () => {
    expect(formatIsk(1_000)).toBe('1.0K ISK');
  });

  it('formats thousands with one decimal', () => {
    expect(formatIsk(1_500)).toBe('1.5K ISK');
    expect(formatIsk(42_700)).toBe('42.7K ISK');
    expect(formatIsk(999_900)).toBe('999.9K ISK');
  });

  it('formats the K/M boundary correctly', () => {
    // Just under 1M → K
    expect(formatIsk(999_999)).toMatch(/K ISK$/);
    // At 1M → M
    expect(formatIsk(1_000_000)).toMatch(/M ISK$/);
  });

  // ── Millions (M) ────────────────────────────────────────────────────────────
  it('formats exactly 1 million', () => {
    expect(formatIsk(1_000_000)).toBe('1.00M ISK');
  });

  it('formats millions with two decimals', () => {
    expect(formatIsk(1_500_000)).toBe('1.50M ISK');
    expect(formatIsk(119_560_000)).toBe('119.56M ISK');
    expect(formatIsk(494_940_000)).toBe('494.94M ISK');
  });

  it('formats the M/B boundary correctly', () => {
    expect(formatIsk(999_999_999)).toMatch(/M ISK$/);
    expect(formatIsk(1_000_000_000)).toMatch(/B ISK$/);
  });

  // ── Billions (B) ────────────────────────────────────────────────────────────
  it('formats exactly 1 billion', () => {
    expect(formatIsk(1_000_000_000)).toBe('1.00B ISK');
  });

  it('formats billions with two decimals', () => {
    expect(formatIsk(2_500_000_000)).toBe('2.50B ISK');
    expect(formatIsk(44_860_000_000)).toBe('44.86B ISK');
  });

  // ── Trillions (T) ───────────────────────────────────────────────────────────
  it('formats exactly 1 trillion', () => {
    expect(formatIsk(1_000_000_000_000)).toBe('1.00T ISK');
  });

  it('formats trillions with two decimals', () => {
    expect(formatIsk(3_500_000_000_000)).toBe('3.50T ISK');
    expect(formatIsk(10_000_000_000_000)).toBe('10.00T ISK');
  });

  // ── Suffix correctness ───────────────────────────────────────────────────────
  it('always ends with " ISK"', () => {
    const values = [0, 1, 999, 1_000, 500_000, 1_000_000, 1_000_000_000, 1_000_000_000_000];
    values.forEach(v => {
      expect(formatIsk(v)).toMatch(/ ISK$/);
    });
  });

  it('never produces NaN or undefined in output', () => {
    [0, 1, 1e6, 1e9, 1e12].forEach(v => {
      expect(formatIsk(v)).not.toContain('NaN');
      expect(formatIsk(v)).not.toContain('undefined');
    });
  });
});
