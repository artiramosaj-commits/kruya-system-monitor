import { useState, useEffect } from 'react';
import { getAppUsage, resetAppUsage } from '../api';
import type { AppUsage } from '../types';

function fmtDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const h = Math.floor(m / 60);
  const rem = m % 60;
  if (h > 0) return `${h}h ${rem}m`;
  return `${m}m`;
}

export default function UsagePage() {
  const [usage, setUsage] = useState<AppUsage[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [confirmClear, setConfirmClear] = useState(false);

  const fetch = async () => {
    setLoading(true);
    try {
      const data = await getAppUsage();
      setUsage(data);
    } catch (err) {
      console.error('Failed to load app usage:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    fetch();
    const interval = setInterval(fetch, 5000);
    return () => clearInterval(interval);
  }, []);

  const clearAll = async () => {
    for (const app of usage) {
      await resetAppUsage(app.name);
    }
    setConfirmClear(false);
    fetch();
  };

  const filtered = search.trim()
    ? usage.filter(a => a.name.toLowerCase().includes(search.toLowerCase()))
    : usage;

  const totalMin = Math.floor(usage.reduce((s, a) => s + a.total_seconds, 0) / 60);

  const fmtTotal = () => {
    const h = Math.floor(totalMin / 60);
    const m = totalMin % 60;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  };

  if (loading && usage.length === 0) {
    return <div className="page page-usage"><p style={{ color: 'var(--text-muted)', padding: '2rem' }}>Loading...</p></div>;
  }

  return (
    <div className="page page-usage">
      <div style={{ marginBottom: '1.5rem' }}>
        <h2 className="section-title" style={{ margin: '0 0 0.25rem' }}>App Usage</h2>
        <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-muted)' }}>
          {fmtTotal()} across {usage.length} app{usage.length !== 1 ? 's' : ''}
        </p>
      </div>

      <div style={{
        display: 'flex', alignItems: 'center', gap: '0.75rem',
        marginBottom: '1rem', flexWrap: 'wrap',
      }}>
        <input
          type="text"
          placeholder="Search applications..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          style={{
            flex: '1 1 200px', padding: '0.55rem 0.8rem', borderRadius: '8px',
            border: '1px solid var(--border)', background: 'var(--bg-secondary)',
            color: 'var(--text-primary)', fontSize: '0.88rem', outline: 'none',
          }}
        />
        {usage.length > 0 && !confirmClear && (
          <button onClick={() => setConfirmClear(true)} style={{
            background: 'none', border: '1px solid var(--border)', cursor: 'pointer',
            padding: '0.55rem 1rem', borderRadius: '8px', color: 'var(--text-muted)',
            fontSize: '0.85rem',
          }}>
            Clear All
          </button>
        )}
        {confirmClear && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontSize: '0.85rem', color: 'var(--accent-red)' }}>This will permanently delete all usage data.</span>
            <button onClick={clearAll} style={{
              background: 'var(--accent-red)', border: 'none', cursor: 'pointer',
              padding: '0.5rem 0.9rem', borderRadius: '6px', color: '#fff',
              fontSize: '0.85rem', fontWeight: 600,
            }}>
              Delete All
            </button>
            <button onClick={() => setConfirmClear(false)} style={{
              background: 'none', border: '1px solid var(--border)', cursor: 'pointer',
              padding: '0.5rem 0.9rem', borderRadius: '6px', color: 'var(--text-muted)',
              fontSize: '0.85rem',
            }}>
              Cancel
            </button>
          </div>
        )}
      </div>

      {usage.length === 0 ? (
        <div style={{
          textAlign: 'center', padding: '3rem 1rem',
          color: 'var(--text-muted)', border: '1px dashed var(--border)',
          borderRadius: '12px', fontSize: '0.9rem',
        }}>
          No usage data yet.
        </div>
      ) : (
        <div className="card" style={{ overflow: 'hidden', padding: 0, borderRadius: '12px' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.9rem' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                {['Application', 'Time Used', 'Last Seen'].map(h => (
                  <th key={h} style={{
                    textAlign: h === 'Application' ? 'left' : 'right',
                    padding: '0.8rem 1.2rem', color: 'var(--text-muted)',
                    fontWeight: 600, fontSize: '0.72rem',
                    textTransform: 'uppercase', letterSpacing: '0.6px',
                  }}>
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {filtered.map((app, i) => (
                <tr key={app.name} style={{
                  borderBottom: i < filtered.length - 1 ? '1px solid var(--border)' : 'none',
                  transition: 'background 0.15s',
                }}
                  onMouseEnter={e => (e.currentTarget.style.background = 'var(--bg-hover)')}
                  onMouseLeave={e => (e.currentTarget.style.background = 'transparent')}
                >
                  <td style={{ padding: '0.75rem 1.2rem', fontWeight: 500 }}>{app.name}</td>
                  <td style={{
                    padding: '0.75rem 1.2rem', textAlign: 'right',
                    fontVariantNumeric: 'tabular-nums', fontWeight: 600,
                  }}>
                    {fmtDuration(app.total_seconds)}
                  </td>
                  <td style={{
                    padding: '0.75rem 1.2rem', textAlign: 'right',
                    color: 'var(--text-muted)', fontSize: '0.84rem',
                  }}>
                    {app.last_seen || '—'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {search && filtered.length === 0 && usage.length > 0 && (
        <p style={{ textAlign: 'center', color: 'var(--text-muted)', marginTop: '1rem', fontSize: '0.85rem' }}>
          No results for "{search}"
        </p>
      )}
    </div>
  );
}
