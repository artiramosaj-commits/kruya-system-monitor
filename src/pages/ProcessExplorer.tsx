import { useState, useMemo, useEffect } from 'react';
import type { SnapshotResult, ProcessInfo, ProcessHistoryPoint } from '../types';
import { getProcessHistory, killProcess } from '../api';
import MiniChart from '../components/MiniChart';

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  if (d > 0) return `${d}d ${h}h`;
  return `${h}h ${Math.floor((seconds % 3600) / 60)}m`;
}

interface Props {
  data: SnapshotResult;
}

export default function ProcessExplorer({ data }: Props) {
  const { snapshot } = data;
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState('memory_mb');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');
  const [detailProc, setDetailProc] = useState<ProcessInfo | null>(null);
  const [procHistory, setProcHistory] = useState<ProcessHistoryPoint[]>([]);
  const [procHistoryLoading, setProcHistoryLoading] = useState(false);

  useEffect(() => {
    if (!detailProc) { setProcHistory([]); return; }
    let cancelled = false;
    setProcHistoryLoading(true);
    getProcessHistory(detailProc.name, 60).then(h => { if (!cancelled) setProcHistory(h); }).catch(() => {}).finally(() => { if (!cancelled) setProcHistoryLoading(false); });
    return () => { cancelled = true; };
  }, [detailProc]);

  const handleSort = (key: string) => {
    if (sortKey === key) setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    else { setSortKey(key); setSortDir('desc'); }
  };

  const arrow = (k: string) => sortKey === k ? (sortDir === 'asc' ? '▲' : '▼') : '';

  const sorted = useMemo(() => {
    let list = snapshot.processes;
    if (search) {
      const l = search.toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(l) || p.cmd.toLowerCase().includes(l) || String(p.pid).includes(l));
    }
    return [...list].sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case 'name': cmp = a.name.localeCompare(b.name); break;
        case 'pid': cmp = a.pid - b.pid; break;
        case 'cpu_percent': cmp = a.cpu_percent - b.cpu_percent; break;
        case 'memory_mb': cmp = a.memory_mb - b.memory_mb; break;
        case 'threads': cmp = a.threads - b.threads; break;
        case 'runtime_seconds': cmp = a.runtime_seconds - b.runtime_seconds; break;
      }
      return sortDir === 'desc' ? -cmp : cmp;
    });
  }, [snapshot.processes, search, sortKey, sortDir]);

  const totalCpu = snapshot.processes.reduce((s, p) => s + p.cpu_percent, 0);
  const totalMem = snapshot.processes.reduce((s, p) => s + p.memory_mb, 0);
  const totalThreads = snapshot.processes.reduce((s, p) => s + p.threads, 0);
  const fmt = (n: number) => n % 1 === 0 ? String(n) : n.toFixed(1);

  return (
    <div className="page page-process">
      <h2 className="section-title">Process Explorer — {snapshot.processes.length} processes</h2>
      <div className="summary-stats">
        <div className="summary-stat">
          <span className="summary-stat-value">{snapshot.processes.length}</span>
          <span className="summary-stat-label">Processes</span>
        </div>
        <div className="summary-stat">
          <span className="summary-stat-value">{totalThreads.toLocaleString()}</span>
          <span className="summary-stat-label">Threads</span>
        </div>
        <div className="summary-stat">
          <span className="summary-stat-value">{totalCpu.toFixed(1)}%</span>
          <span className="summary-stat-label">Total CPU</span>
        </div>
        <div className="summary-stat">
          <span className="summary-stat-value">{fmt(totalMem / 1024)} GB</span>
          <span className="summary-stat-label">Total Memory</span>
        </div>
      </div>
      <div className="section-header" style={{ marginBottom: 12 }}>
        <span className="table-header">Process List</span>
        <input className="search-input" placeholder="Filter by name, PID, or command line..." value={search} onChange={e => setSearch(e.target.value)} />
      </div>
      <div className="process-table-wrapper">
        <table className="process-table">
          <thead>
            <tr>
              <th className="sortable" onClick={() => handleSort('name')}>Process {arrow('name')}</th>
              <th className="sortable" onClick={() => handleSort('pid')}>PID {arrow('pid')}</th>
              <th className="sortable" onClick={() => handleSort('cpu_percent')}>CPU {arrow('cpu_percent')}</th>
              <th className="sortable" onClick={() => handleSort('memory_mb')}>Memory {arrow('memory_mb')}</th>
              <th className="sortable" onClick={() => handleSort('threads')}>Threads {arrow('threads')}</th>
              <th className="sortable" onClick={() => handleSort('runtime_seconds')}>Runtime {arrow('runtime_seconds')}</th>
              <th>Command Line</th>
              <th>Path</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map(p => (
              <tr key={p.pid} className="proc-row clickable" onClick={() => setDetailProc(p)}>
                <td className="proc-name">{p.name}</td>
                <td>{p.pid}</td>
                <td>{p.cpu_percent.toFixed(1)}%</td>
                <td>{p.memory_mb} MB</td>
                <td>{p.threads}</td>
                <td>{formatUptime(p.runtime_seconds)}</td>
                <td style={{ maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.cmd || '—'}</td>
                <td style={{ maxWidth: 250, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.path || '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {detailProc && (
        <div className="modal-overlay" onClick={() => setDetailProc(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{detailProc.name}</h3>
              <button className="modal-close" onClick={() => setDetailProc(null)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="modal-grid">
                <div className="modal-item">
                  <span className="modal-label">PID</span>
                  <span className="modal-val">{detailProc.pid}</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">CPU</span>
                  <span className="modal-val">{detailProc.cpu_percent.toFixed(1)}%</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Memory</span>
                  <span className="modal-val">{detailProc.memory_mb} MB</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Threads</span>
                  <span className="modal-val">{detailProc.threads}</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Runtime</span>
                  <span className="modal-val">{formatUptime(detailProc.runtime_seconds)}</span>
                </div>
              </div>
              {(detailProc.path || detailProc.cmd) && (
                <div className="modal-details">
                  {detailProc.path && (
                    <div className="modal-detail-item">
                      <span className="modal-label">Executable Path</span>
                      <span className="modal-detail-val mono">{detailProc.path}</span>
                    </div>
                  )}
                  {detailProc.cmd && (
                    <div className="modal-detail-item">
                      <span className="modal-label">Command Line</span>
                      <span className="modal-detail-val mono">{detailProc.cmd}</span>
                    </div>
                  )}
                </div>
              )}
              {procHistory.length > 0 && (
                <div className="modal-details">
                  <span className="modal-label" style={{ marginBottom: 6, display: 'block' }}>CPU / Memory History (60 snapshots)</span>
                  <div className="modal-chart-row">
                    <MiniChart data={procHistory.map(h => h.cpu_percent)} color="#3b82f6" width={280} height={24} />
                  </div>
                  <div className="modal-chart-row">
                    <MiniChart data={procHistory.map(h => h.memory_mb)} color="#8b5cf6" width={280} height={24} />
                  </div>
                </div>
              )}
              {procHistoryLoading && (
                <div className="modal-details">
                  <span className="modal-label" style={{ color: 'var(--text-muted)' }}>Loading history...</span>
                </div>
              )}
              <div className="modal-actions">
                <button className="kill-btn" onClick={async (e) => {
                  e.stopPropagation();
                  if (window.confirm(`Terminate ${detailProc.name} (PID ${detailProc.pid})?`)) {
                    try {
                      await killProcess(detailProc.pid);
                      setDetailProc(null);
                    } catch (err) {
                      alert('Failed to kill: ' + err);
                    }
                  }
                }}>
                  ✕ End Process
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
