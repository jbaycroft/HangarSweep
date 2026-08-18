import { LiquidityRow, formatIsk } from '../types';

interface Props {
  rows: LiquidityRow[];          // filtered rows to display
  allRows: LiquidityRow[];       // full unfiltered set (for context in empty state)
  selectedLocationId: number | null;
  onSelect: (row: LiquidityRow) => void;
}

export default function LocationList({ rows, allRows, selectedLocationId, onSelect }: Props) {
  if (rows.length === 0) {
    return (
      <div className="list-empty">
        {allRows.length === 0 ? (
          <>
            <p>No asset data found.</p>
            <p className="hint">Click ⟳ Sync to fetch your assets from ESI.</p>
          </>
        ) : (
          <>
            <p>No locations match the current threshold.</p>
            <p className="hint">Lower the Min Value slider to see more locations.</p>
            <p className="hint">{allRows.length} total location{allRows.length !== 1 ? 's' : ''} in database — {formatIsk(allRows.reduce((s, r) => s + r.total_isk_value, 0))} combined.</p>
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
