import { LiquidityRow, formatIsk } from '../types';

interface Props {
  rows: LiquidityRow[];
  selectedLocationId: number | null;
  onSelect: (row: LiquidityRow) => void;
}

export default function LocationList({ rows, selectedLocationId, onSelect }: Props) {
  if (rows.length === 0) {
    return (
      <div className="list-empty">
        <p>No locations with &gt;500M ISK value found.</p>
        <p className="hint">Sync your assets to populate the ledger.</p>
      </div>
    );
  }

  const totalValue = rows.reduce((sum, r) => sum + r.total_isk_value, 0);

  return (
    <div className="location-list">
      <div className="list-summary">
        <span>{rows.length} location{rows.length !== 1 ? 's' : ''}</span>
        <span className="isk-total">{formatIsk(totalValue)} total</span>
      </div>
      <table className="data-table">
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
              <td className="location-name">
                <span className={`loc-icon ${row.location_id >= 1_000_000_000_000 ? 'citadel' : 'station'}`} />
                {row.location_name}
              </td>
              <td className="num isk-value">{formatIsk(row.total_isk_value)}</td>
              <td className="num">{row.stack_count.toLocaleString()}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
