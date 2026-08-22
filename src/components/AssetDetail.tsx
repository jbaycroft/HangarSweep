import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { AssetRow, LiquidityRow, formatIsk } from '../types';

interface Props {
  location: LiquidityRow;
  characterId: number;
}

type PriceMode = 'avg' | 'jita_sell' | 'jita_buy';

const PRICE_MODE_LABELS: Record<PriceMode, string> = {
  avg:       'ESI Avg',
  jita_sell: 'Jita Sell',
  jita_buy:  'Jita Buy',
};

/** Return the per-unit price for an asset under the active price mode. */
function unitPrice(asset: AssetRow, mode: PriceMode): number {
  switch (mode) {
    case 'jita_sell': return asset.jita_sell;
    case 'jita_buy':  return asset.jita_buy;
    default:          return asset.quantity > 0 ? asset.estimated_value / asset.quantity : 0;
  }
}

/** Return the total stack value for an asset under the active price mode. */
function stackValue(asset: AssetRow, mode: PriceMode): number {
  switch (mode) {
    case 'jita_sell': return asset.jita_sell * asset.quantity;
    case 'jita_buy':  return asset.jita_buy  * asset.quantity;
    default:          return asset.estimated_value;
  }
}

export default function AssetDetail({ location, characterId }: Props) {
  const [assets, setAssets] = useState<AssetRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [priceMode, setPriceMode] = useState<PriceMode>('avg');

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      setError(null);
      setCopied(false);
      try {
        const data = await invoke<AssetRow[]>('get_assets_at_location', {
          locationId: location.location_id,
          characterId: characterId,
        });
        // Sort descending by estimated value (ESI avg) on initial load
        data.sort((a, b) => b.estimated_value - a.estimated_value);
        setAssets(data);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [location.location_id, characterId]);

  const handleCopyMultibuy = async () => {
    try {
      const text = await invoke<string>('export_multibuy', {
        locationId: location.location_id,
        characterId: characterId,
      });
      await writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 3000);
    } catch (e) {
      setError(String(e));
    }
  };

  const totalValue = assets.reduce((sum, a) => sum + stackValue(a, priceMode), 0);

  // How many assets have Jita data at all?
  const jitaCoverage = assets.filter(
    (a) => a.jita_sell > 0 || a.jita_buy > 0
  ).length;
  const hasJitaData = jitaCoverage > 0;

  return (
    <div className="asset-detail">
      {/* ── Header ─────────────────────────────────────────────────────── */}
      <div className="detail-header">
        <div className="detail-title">
          <h3 title={location.location_name}>{location.location_name}</h3>
          <span className="detail-subtitle">
            <span className="detail-isk">{formatIsk(totalValue)}</span>
            &nbsp;across&nbsp;
            <strong>{assets.length}</strong> type{assets.length !== 1 ? 's' : ''}
            {priceMode !== 'avg' && !hasJitaData && (
              <span className="jita-no-data">&nbsp;· No Jita data yet — run Sync</span>
            )}
          </span>
        </div>
        <div className="detail-actions">
          {/* ── Price mode toggle ──────────────────────────── */}
          <div className="price-mode-toggle" role="group" aria-label="Price mode">
            {(Object.keys(PRICE_MODE_LABELS) as PriceMode[]).map((mode) => (
              <button
                key={mode}
                className={`price-mode-btn ${priceMode === mode ? 'active' : ''}`}
                onClick={() => setPriceMode(mode)}
                title={
                  mode === 'avg'       ? 'ESI global average price'            :
                  mode === 'jita_sell' ? 'Jita minimum sell order (list here)' :
                                         'Jita maximum buy order (instant ISK)'
                }
              >
                {PRICE_MODE_LABELS[mode]}
              </button>
            ))}
          </div>
          <button
            className={`btn ${copied ? 'btn-success' : 'btn-multibuy'}`}
            onClick={handleCopyMultibuy}
            disabled={loading || assets.length === 0}
          >
            {copied ? '✓ Copied!' : '📋 Copy Multibuy'}
          </button>
        </div>
      </div>

      {/* ── Jita hint bar — only shown when a non-avg mode is active ─────── */}
      {priceMode !== 'avg' && (
        <div className="jita-hint-bar">
          {priceMode === 'jita_sell' ? (
            <>
              <span className="jita-hint-icon">📈</span>
              <span>
                <strong>Jita Sell</strong> — the lowest active sell order in Jita.
                List your items at or just below this price to undercut.
              </span>
            </>
          ) : (
            <>
              <span className="jita-hint-icon">⚡</span>
              <span>
                <strong>Jita Buy</strong> — the highest active buy order in Jita.
                Haul to Jita and sell immediately for instant ISK.
              </span>
            </>
          )}
          {hasJitaData && (
            <span className="jita-coverage">
              {jitaCoverage}/{assets.length} types have Jita data
            </span>
          )}
        </div>
      )}

      {error && <div className="error-inline">⚠ {error}</div>}

      {/* ── Asset table ─────────────────────────────────────────────────── */}
      {loading ? (
        <div className="loading">Loading assets…</div>
      ) : assets.length === 0 ? (
        <div className="list-empty"><p>No tradeable assets at this location.</p></div>
      ) : (
        <div className="asset-table-wrap">
          <table className="data-table asset-table">
            <colgroup>
              <col style={{ width: '45%' }} />
              <col style={{ width: '12%' }} />
              <col style={{ width: '20%' }} />
              {priceMode !== 'avg' && <col style={{ width: '11%' }} />}
              {priceMode !== 'avg' && <col style={{ width: '12%' }} />}
              {priceMode === 'avg'  && <col style={{ width: '23%' }} />}
            </colgroup>
            <thead>
              <tr>
                <th>Type</th>
                <th className="num">Qty</th>
                <th className="num">
                  {priceMode === 'avg'       ? 'Est. Value'   :
                   priceMode === 'jita_sell' ? 'Jita Sell Total' :
                                               'Jita Buy Total'}
                </th>
                {priceMode !== 'avg' && <th className="num">Unit Price</th>}
                {priceMode !== 'avg' && <th className="num">vs Avg</th>}
                {priceMode === 'avg' && null}
              </tr>
            </thead>
            <tbody>
              {assets.map((asset) => {
                const sv = stackValue(asset, priceMode);
                const uv = unitPrice(asset, priceMode);
                const avgPerUnit = asset.quantity > 0
                  ? asset.estimated_value / asset.quantity
                  : 0;
                const delta = avgPerUnit > 0 ? ((uv - avgPerUnit) / avgPerUnit) * 100 : null;

                return (
                  <tr key={asset.item_id} className="data-row asset-row">
                    <td className="type-cell" title={asset.type_name}>{asset.type_name}</td>
                    <td className="num qty-cell">{asset.quantity.toLocaleString()}</td>
                    <td className={`num isk-value ${sv === 0 && priceMode !== 'avg' ? 'no-data' : ''}`}>
                      {sv > 0 ? formatIsk(sv) : priceMode !== 'avg' ? '—' : formatIsk(0)}
                    </td>
                    {priceMode !== 'avg' && (
                      <td className={`num qty-cell ${uv === 0 ? 'no-data' : ''}`}>
                        {uv > 0 ? formatIsk(uv) : '—'}
                      </td>
                    )}
                    {priceMode !== 'avg' && (
                      <td className={`num delta-cell ${
                        delta === null ? 'no-data' :
                        delta > 0     ? 'delta-pos' :
                        delta < 0     ? 'delta-neg' : ''
                      }`}>
                        {delta === null
                          ? '—'
                          : `${delta > 0 ? '+' : ''}${delta.toFixed(1)}%`}
                      </td>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
