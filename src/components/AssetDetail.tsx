import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { AssetRow, LiquidityRow, formatIsk } from '../types';

interface Props {
  location: LiquidityRow;
  characterId: number;
}

export default function AssetDetail({ location, characterId }: Props) {
  const [assets, setAssets] = useState<AssetRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      setError(null);
      setCopied(false);
      try {
        const data = await invoke<AssetRow[]>('get_assets_at_location', {
          location_id: location.location_id,
          character_id: characterId,
        });
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
        location_id: location.location_id,
        character_id: characterId,
      });
      await writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 3000);
    } catch (e) {
      setError(String(e));
    }
  };

  const totalValue = assets.reduce((sum, a) => sum + a.estimated_value, 0);

  return (
    <div className="asset-detail">
      <div className="detail-header">
        <div className="detail-title">
          <h3 title={location.location_name}>{location.location_name}</h3>
          <span className="detail-subtitle">
            {formatIsk(totalValue)} across {assets.length} type{assets.length !== 1 ? 's' : ''}
          </span>
        </div>
        <button
          className={`btn ${copied ? 'btn-success' : 'btn-multibuy'}`}
          onClick={handleCopyMultibuy}
          disabled={loading || assets.length === 0}
        >
          {copied ? '✓ Copied!' : '📋 Copy Multibuy'}
        </button>
      </div>

      {error && <div className="error-inline">⚠ {error}</div>}

      {loading ? (
        <div className="loading">Loading assets…</div>
      ) : assets.length === 0 ? (
        <div className="list-empty"><p>No tradeable assets at this location.</p></div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Type</th>
              <th className="num">Qty</th>
              <th className="num">Est. Value</th>
            </tr>
          </thead>
          <tbody>
            {assets.map((asset) => (
              <tr key={asset.item_id} className="data-row">
                <td>{asset.type_name}</td>
                <td className="num">{asset.quantity.toLocaleString()}</td>
                <td className="num isk-value">{formatIsk(asset.estimated_value)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
