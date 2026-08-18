import { LiquidityRow, formatIsk } from '../types';

interface Props {
  rows: LiquidityRow[];
  allRows: LiquidityRow[];
  selectedLocationId: number | null;
  onSelect: (row: LiquidityRow) => void;
}

export default function LocationList({ rows, allRows, selectedLocationId, onSelect }: Props) {
  if (rows.length === 0) {
    return (
      <div className="list-empty">
        {allRows.length === 0 ? (
          <>
            <p>No asset data yet.</p>
            <p className="hint">Click ⟳ Sync to fetch your assets from ESI.</p>
          </>
        ) : (
          <>
            <p>No locations match the current filter.</p>
            <p className="hint">Lower the Min Value slider to show more.</p>
            <p className="hint">{allRows.length} location{allRows.length !== 1 ? 's' : ''} in database — {formatIsk(allRows.reduce((s, r) => s + r.total_isk_value, 0))} total.</p>
          </>
        )}
      </div>
    );
  }

  const totalValue = rows.reduce((sum, r) => sum + r.total_isk_value, 0);

  return (
    <div className="location-list">
      <div className="list-summary">
        <span>{rows.length} location{rows.length !== 1 ? 's' : ''} shown</span>
        <span className="isk-total">{formatIsk(totalValue)}</span>
      </div>

      <div className="location-list-scroll">
        <table className="data-table">
          <colgroup>
            <col className="col-location" style={{ width: '55%' }} />
            <col className="col-value"    style={{ width: '32%' }} />
            <col className="col-stacks"   style={{ width: '13%' }} />
          </colgroup>
          <thead>
            <tr>
              <th>Location</th>
              <th className="num">Est. Value</th>
              <th className="num">Stacks</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.location_id}
                className={`data-row ${selectedLocationId === row.location_id ? 'selected' : ''}`}
                onClick={() => onSelect(row)}
              >
                <td>
                  <div className="location-name-cell">
                    <span className={`loc-icon ${row.location_id >= 1_000_000_000_000 ? 'citadel' : 'station'}`} />
                    <span className="location-name-text" title={row.location_name}>{row.location_name}</span>
                  </div>
                </td>
                <td className="num isk-value">{formatIsk(row.total_isk_value)}</td>
                <td className="num" style={{ color: 'var(--text-sec)', fontSize: '12px' }}>{row.stack_count.toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
