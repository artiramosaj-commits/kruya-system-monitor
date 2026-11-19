import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import { getSnapshot, exportCsv, exportJson, killProcess, getProcessHistory, setAutoStart } from './api';
import { save } from '@tauri-apps/plugin-dialog';
import type { SnapshotResult, ProcessInfo, ProcessHistoryPoint } from './types';
import { exportReport as exportReportApi, getAppUsage, getStartupPrograms } from './api';
import MiniChart from './components/MiniChart';
import ProcessExplorer from './pages/ProcessExplorer';
import StoragePage from './pages/StoragePage';
import NetworkPage from './pages/NetworkPage';
import SystemInfoPage from './pages/SystemInfoPage';
import TimelinePage from './pages/TimelinePage';
import PredictionsPage from './pages/PredictionsPage';
import UsagePage from './pages/UsagePage';
import { analyzePredictions, computeHealthScore } from './predictive';

function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1) + ' GB';
  if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + ' MB';
  if (bytes >= 1e3) return (bytes / 1e3).toFixed(0) + ' KB';
  return bytes + ' B';
}

function formatBytesPerSec(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1) + ' GB/s';
  if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + ' MB/s';
  if (bytes >= 1e3) return (bytes / 1e3).toFixed(0) + ' KB/s';
  return bytes.toFixed(1) + ' B/s';
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  if (d > 0) return `${d}d ${h}h`;
  return `${h}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function severityColor(score: number): string {
  if (score > 60) return '#ef4444';
  if (score > 40) return '#f59e0b';
  if (score > 20) return '#eab308';
  return '#22c55e';
}

function severityLabel(score: number): string {
  if (score > 80) return 'Critical';
  if (score > 60) return 'Significant';
  if (score > 40) return 'Moderate';
  if (score > 20) return 'Minor';
  return 'Normal';
}

function alertColor(severity: string): string {
  switch (severity) {
    case 'critical': return '#ef4444';
    case 'warning': return '#f59e0b';
    case 'info': return '#3b82f6';
    default: return '#6b7280';
  }
}

function anomalyColor(severity: string): string {
  switch (severity) {
    case 'critical': return '#ef4444';
    case 'high': return '#f97316';
    case 'medium': return '#f59e0b';
    case 'low': return '#eab308';
    default: return '#6b7280';
  }
}

export default function App() {
  const [data, setData] = useState<SnapshotResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<{ cpu: number; mem: number; disk: number; diskRead: number; diskWrite: number; netDown: number; netUp: number; swapPct: number; battery: number; temperature: number; ts: string }[]>([]);
  const [selectedProc, setSelectedProc] = useState<ProcessInfo | null>(null);
  const [paused, setPaused] = useState(false);
  const [diskReadRate, setDiskReadRate] = useState(0);
  const [diskWriteRate, setDiskWriteRate] = useState(0);
  const [netDownRate, setNetDownRate] = useState(0);
  const [netUpRate, setNetUpRate] = useState(0);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortKey, setSortKey] = useState<string>('memory_mb');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [pollInterval, setPollInterval] = useState(() => {
    try {
      const saved = localStorage.getItem('kruya_poll_interval');
      return saved ? parseInt(saved) || 5000 : 5000;
    } catch { return 5000; }
  });
  const [notificationsEnabled, setNotificationsEnabled] = useState(() => {
    try { return localStorage.getItem('kruya_notifications') !== 'false'; } catch { return true; }
  });
  const [autoStart, setAutoStartState] = useState(false);
  const [procHistory, setProcHistory] = useState<ProcessHistoryPoint[]>([]);
  const [procHistoryLoading, setProcHistoryLoading] = useState(false);
  const [theme, setTheme] = useState(() => {
    try { return (localStorage.getItem('kruya_theme') || 'dark') as 'dark' | 'light'; } catch { return 'dark'; }
  });
  const [thresholds, setThresholds] = useState(() => {
    try {
      const saved = localStorage.getItem('kruya_thresholds');
      return saved ? JSON.parse(saved) : { cpu: 90, memory: 85, disk: 95 };
    } catch { return { cpu: 90, memory: 85, disk: 95 }; }
  });
  const [alertHistory, setAlertHistory] = useState<{ msg: string; sev: string; ts: number }[]>([]);
  const [page, setPage] = useState<'dashboard' | 'processes' | 'storage' | 'network' | 'system' | 'timeline' | 'forecast' | 'usage'>('dashboard');
  const [exportLimit, setExportLimit] = useState(() => {
    try { return parseInt(localStorage.getItem('kruya_export_limit') || '120') || 120; } catch { return 120; }
  });
  const [historyRetention, setHistoryRetention] = useState(() => {
    try { return parseInt(localStorage.getItem('kruya_history_retention') || '120') || 120; } catch { return 120; }
  });
  const historyRetentionRef = useRef(historyRetention);
  useEffect(() => { historyRetentionRef.current = historyRetention; }, [historyRetention]);
  const [tempUnit, setTempUnit] = useState(() => {
    try { return (localStorage.getItem('kruya_temp_unit') || 'c') as 'c' | 'f'; } catch { return 'c'; }
  });
  const [forecastEnabled, setForecastEnabled] = useState(() => {
    try { return localStorage.getItem('kruya_forecast') !== 'false'; } catch { return true; }
  });
  const prevDisk = useRef({ read: 0, write: 0, ts: 0 });
  const prevNet = useRef({ down: 0, up: 0, ts: 0 });
  const notified = useRef(new Set<string>());

  // Apply theme
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'light') {
      root.classList.add('light');
    } else {
      root.classList.remove('light');
    }
  }, [theme]);

  // Request notification permission on mount
  useEffect(() => {
    if ('Notification' in window && Notification.permission === 'default' && notificationsEnabled) {
      Notification.requestPermission();
    }
  }, [notificationsEnabled]);

  const fetch = useCallback(async () => {
    try {
      const result = await getSnapshot();
      setData(result);
      setError(null);

      const sys = result.snapshot.system;
      const swapPct = sys.swap_total_mb > 0 ? Math.round((sys.swap_used_mb / sys.swap_total_mb) * 100) : 0;

      // Compute rates locally to avoid stale closure
      const prevDiskData = prevDisk.current;
      const dtDisk = prevDiskData.ts > 0 ? (Date.now() - prevDiskData.ts) / 1000 : 0;
      const newDiskReadRate = dtDisk > 0 && prevDiskData.read > 0
        ? Math.max(0, (sys.disk_read_bytes - prevDiskData.read) / dtDisk) : 0;
      const newDiskWriteRate = dtDisk > 0 && prevDiskData.read > 0
        ? Math.max(0, (sys.disk_write_bytes - prevDiskData.write) / dtDisk) : 0;
      prevDisk.current = { read: sys.disk_read_bytes, write: sys.disk_write_bytes, ts: Date.now() };
      setDiskReadRate(newDiskReadRate);
      setDiskWriteRate(newDiskWriteRate);

      const prevNetData = prevNet.current;
      const dtNet = prevNetData.ts > 0 ? (Date.now() - prevNetData.ts) / 1000 : 0;
      const newNetDownRate = dtNet > 0 && (prevNetData.down > 0 || prevNetData.up > 0)
        ? Math.max(0, (sys.network_down_bytes - prevNetData.down) / dtNet) : 0;
      const newNetUpRate = dtNet > 0 && (prevNetData.down > 0 || prevNetData.up > 0)
        ? Math.max(0, (sys.network_up_bytes - prevNetData.up) / dtNet) : 0;
      prevNet.current = { down: sys.network_down_bytes, up: sys.network_up_bytes, ts: Date.now() };
      setNetDownRate(newNetDownRate);
      setNetUpRate(newNetUpRate);

      setHistory(prev => {
        const next = [...prev, {
          cpu: sys.cpu_total_usage,
          mem: sys.memory_total_usage,
          disk: sys.disk_usage_percent,
          diskRead: newDiskReadRate,
          diskWrite: newDiskWriteRate,
          netDown: newNetDownRate,
          netUp: newNetUpRate,
          swapPct,
          battery: sys.battery_percent,
          temperature: sys.temperature_celsius,
          ts: result.snapshot.timestamp
        }];
        return next.length > historyRetentionRef.current ? next.slice(-historyRetentionRef.current) : next;
      });

      // Alert history log (dedup by alert_type, keep last 50)
      const alerts = result.analysis.alerts;
      for (const a of alerts) {
        if (!notified.current.has(`log:${a.alert_type}`)) {
          notified.current.add(`log:${a.alert_type}`);
          setAlertHistory(prev => {
            const next = [...prev, { msg: a.message, sev: a.severity, ts: Date.now() }];
            return next.slice(-50);
          });
        }
      }

      // Desktop notifications for thresholds (once per threshold crossing)
      if (notificationsEnabled && 'Notification' in window && Notification.permission === 'granted') {
        const cpuKey = `cpu_${thresholds.cpu}`;
        if (sys.cpu_total_usage > thresholds.cpu && !notified.current.has(cpuKey)) {
          new Notification('High CPU Usage', { body: `CPU at ${sys.cpu_total_usage.toFixed(1)}% (threshold: ${thresholds.cpu}%)` });
          notified.current.add(cpuKey);
        } else if (sys.cpu_total_usage <= thresholds.cpu - 10) {
          notified.current.delete(cpuKey);
        }
        const memKey = `mem_${thresholds.memory}`;
        if (sys.memory_total_usage > thresholds.memory && !notified.current.has(memKey)) {
          new Notification('High Memory Usage', { body: `Memory at ${sys.memory_total_usage}% (threshold: ${thresholds.memory}%)` });
          notified.current.add(memKey);
        } else if (sys.memory_total_usage <= thresholds.memory - 10) {
          notified.current.delete(memKey);
        }
        const diskKey = `disk_${thresholds.disk}`;
        if (sys.disk_usage_percent > thresholds.disk && !notified.current.has(diskKey)) {
          new Notification('Disk Nearly Full', { body: `Disk at ${sys.disk_usage_percent}% (threshold: ${thresholds.disk}%)` });
          notified.current.add(diskKey);
        } else if (sys.disk_usage_percent <= thresholds.disk - 5) {
          notified.current.delete(diskKey);
        }
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [notificationsEnabled, thresholds]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    fetch();
    if (paused) return;
    const interval = setInterval(fetch, pollInterval);
    return () => clearInterval(interval);
  }, [fetch, paused, pollInterval]);

  // Load process history when modal opens
  useEffect(() => {
    if (selectedProc) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setProcHistoryLoading(true);
      getProcessHistory(selectedProc.name, 60)
        .then(setProcHistory)
        .catch(() => setProcHistory([]))
        .finally(() => setProcHistoryLoading(false));
    } else {
      setProcHistory([]);
    }
  }, [selectedProc]);

  // Sorting helpers
  const handleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('desc');
    }
  };

  const sortArrow = (key: string) =>
    sortKey === key ? (sortDir === 'asc' ? ' ▲' : ' ▼') : '';

  // Filtered processes (memoized, guards against null data)
  const filteredProcesses = useMemo(() => {
    if (!data) return [];
    let list = data.snapshot.processes;
    if (searchTerm) {
      const lower = searchTerm.toLowerCase();
      list = list.filter(
        p => p.name.toLowerCase().includes(lower) || String(p.pid).includes(lower)
      );
    }
    list = [...list];
    list.sort((a, b) => {
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
    return list;
  }, [data, searchTerm, sortKey, sortDir]);

  const forecasts = useMemo(() => {
    if (!data) return [];
    return analyzePredictions(
      history,
      data.snapshot.processes.map(p => ({ name: p.name, cpu_percent: p.cpu_percent, memory_mb: p.memory_mb })),
      { memory_total_mb: data.snapshot.system.memory_total_mb, swap_total_mb: data.snapshot.system.swap_total_mb, cpu_cores: data.snapshot.sys_info.cpu_cores }
    );
  }, [data, history]);

  const forecastHealth = useMemo(() => computeHealthScore(forecasts), [forecasts]);

  if (loading && !data) {
    return (
      <div className="app">
        <div className="loading-screen">
          <div className="spinner" />
          <p>Collecting system telemetry...</p>
        </div>
      </div>
    );
  }

  if (error && !data) {
    return (
      <div className="app">
        <div className="error-screen">
          <h2>Connection Error</h2>
          <p>{error}</p>
          <button onClick={fetch}>Retry</button>
        </div>
      </div>
    );
  }

  if (!data) return null;

  const { snapshot, analysis } = data;
  const { system } = snapshot;
  const severity = analysis.system_health.severity_score;
  const uptimeDays = Math.floor(system.uptime_seconds / 86400);

  return (
    <div className="app">
      <header className="header">
        <div className="header-top">
          <div className="header-left">
            <div className="logo-badge">
              <span className="logo-icon">◈</span>
              <h1 className="title">KruYa</h1>
            </div>
            <span className={`subtitle ${paused ? 'inactive' : ''}`}>{paused ? 'Inactive' : 'KruYa System Monitor'}</span>
          </div>
          <div className="header-center">
            <span className={`live-dot ${paused ? 'paused' : ''}`} />
            <span className="timestamp">
              Updated {new Date(snapshot.timestamp).toLocaleTimeString()}
            </span>
          </div>
          <div className="header-right">
          <button className="settings-btn" onClick={() => setSettingsOpen(true)}>⚙</button>
          <button
            className={`pause-btn ${paused ? 'paused' : ''}`}
            onClick={() => setPaused(!paused)}
          >
            {paused ? '▶ Resume' : '⏸ Pause'}
          </button>
        </div>
        </div>
        <nav className="tabs">
          {[
            { id: 'dashboard' as const, label: 'Dashboard', icon: '◉' },
            { id: 'processes' as const, label: 'Processes', icon: '⚙' },
            { id: 'storage' as const, label: 'Storage', icon: '💾' },
            { id: 'network' as const, label: 'Network', icon: '🌐' },
            { id: 'system' as const, label: 'System', icon: '🖥' },
            { id: 'timeline' as const, label: 'Timeline', icon: '📊' },
    { id: 'forecast' as const, label: 'Forecast', icon: '📈' },
    { id: 'usage' as const, label: 'Usage', icon: '⏱' },
  ].map(t => (
            <button key={t.id} className={`tab ${page === t.id ? 'active' : ''}`} onClick={() => setPage(t.id)}>
              <span className="tab-icon">{t.icon}</span>
              {t.label}
            </button>
          ))}
        </nav>
      </header>

      {page === 'dashboard' ? (
        <>
      <div className="summary-bar">
        <p>{analysis.summary}</p>
        {snapshot.foreground.name !== 'unknown' && (
          <div className="foreground-badge">
            <span className="fg-label">FOCUS</span>
            <span className="fg-name">{snapshot.foreground.name}</span>
            {snapshot.foreground.title && <span className="fg-title"> — {snapshot.foreground.title}</span>}
          </div>
        )}
      </div>

      {forecasts.length > 0 && forecastEnabled && (
        <div className="forecast-bar">
          <div className="forecast-bar-left">
            <span className="forecast-bar-score" style={{ color: forecastHealth > 75 ? '#22c55e' : forecastHealth > 50 ? '#eab308' : forecastHealth > 25 ? '#f97316' : '#ef4444' }}>
              {forecastHealth}
            </span>
            <span className="forecast-bar-label">Forecast Health</span>
          </div>
          <div className="forecast-bar-items">
            {forecasts.slice(0, 3).map(f => (
              <div key={f.id} className="forecast-bar-item" style={{ borderLeftColor: f.severity === 'critical' ? '#ef4444' : f.severity === 'high' ? '#f97316' : f.severity === 'medium' ? '#eab308' : '#22c55e' }}>
                <span className="forecast-bar-item-title">{f.title}</span>
                <span className="forecast-bar-item-desc">{f.description.slice(0, 80)}{f.description.length > 80 ? '…' : ''}</span>
              </div>
            ))}
          </div>
          <button className="forecast-bar-btn" onClick={() => setPage('forecast')}>
            View Details →
          </button>
        </div>
      )}

      <div className="dashboard">
        {/* Severity Gauge */}
        <div className="card gauge-card">
          <div className="card-header">System Health</div>
          <div className="gauge-container">
            <svg viewBox="0 0 120 120" className="gauge">
              <circle cx="60" cy="60" r="54" fill="none" stroke="var(--border)" strokeWidth="10" />
              <circle
                cx="60" cy="60" r="54"
                fill="none"
                stroke={severityColor(severity)}
                strokeWidth="10"
                strokeDasharray={`${(severity / 100) * 339.3} 339.3`}
                strokeLinecap="round"
                transform="rotate(-90 60 60)"
                className="gauge-arc"
              />
              <text x="60" y="56" textAnchor="middle" className="gauge-value" fill="var(--text-primary)">
                {severity}
              </text>
              <text x="60" y="74" textAnchor="middle" className="gauge-label" fill="var(--text-muted)">
                {severityLabel(severity)}
              </text>
            </svg>
          </div>
          <div className="card-footer">{analysis.system_health.status}</div>
        </div>

        {/* CPU Card */}
        <div className="card metric-card">
          <div className="card-header">CPU</div>
          <div className="metric-value">{system.cpu_total_usage.toFixed(1)}%</div>
          <MiniChart data={history.map(h => h.cpu)} color="#3b82f6" />
          <div className="metric-bar">
            <div className="metric-fill" style={{ width: `${Math.min(system.cpu_total_usage, 100)}%`, background: system.cpu_total_usage > 75 ? '#ef4444' : '#3b82f6' }} />
          </div>
          {system.per_core_usage.length > 0 && (
            <div className="per-core-bar">
              {system.per_core_usage.map((u, i) => (
                <div key={i} className="core-bar-item" title={`Core ${i}: ${u.toFixed(1)}%  ${system.cpu_frequency_mhz[i] ? `${(system.cpu_frequency_mhz[i] / 1000).toFixed(1)} GHz` : ''}`}>
                  <div className="core-bar-fill" style={{ width: `${Math.min(u, 100)}%`, background: u > 75 ? '#ef4444' : u > 50 ? '#f59e0b' : '#3b82f6' }} />
                </div>
              ))}
            </div>
          )}
          <div className="card-footer">{system.cpu_brand || 'CPU'} — Trend: {analysis.trend_analysis.cpu_trend}</div>
        </div>

        {/* Memory Card */}
        <div className="card metric-card">
          <div className="card-header">Memory</div>
          <div className="metric-value">{system.memory_total_usage}%</div>
          <MiniChart data={history.map(h => h.mem)} color="#8b5cf6" />
          <div className="metric-bar">
            <div className="metric-fill" style={{ width: `${system.memory_total_usage}%`, background: system.memory_total_usage > 80 ? '#ef4444' : '#8b5cf6' }} />
          </div>
          <div className="memory-ram-map" style={{ margin: '8px 0', padding: '0 4px' }}>
            {(() => {
              const total = system.memory_total_mb || 1;
              const used = system.memory_used_mb;
              const free = system.memory_available_mb;
              const cached = Math.max(0, total - used - free);
              const usedPct = (used / total) * 100;
              const cachedPct = (cached / total) * 100;
              const freePct = (free / total) * 100;
              return (
                <>
                  <div className="ram-bar-container" style={{ height: 20, background: 'var(--surface)', borderRadius: 6, overflow: 'hidden', display: 'flex', marginBottom: 4 }}>
                    <div className="ram-bar-segment" style={{ width: `${usedPct}%`, background: 'var(--accent-purple)', minWidth: usedPct > 2 ? 4 : 0, transition: 'width 0.3s' }} title={`Used: ${used} MB`} />
                    <div className="ram-bar-segment" style={{ width: `${cachedPct}%`, background: '#f59e0b', minWidth: cachedPct > 2 ? 4 : 0, transition: 'width 0.3s' }} title={`Cached: ${cached} MB`} />
                    <div className="ram-bar-segment" style={{ width: `${freePct}%`, background: '#22c55e', minWidth: freePct > 2 ? 4 : 0, transition: 'width 0.3s' }} title={`Free: ${free} MB`} />
                  </div>
                  <div className="ram-legend" style={{ display: 'flex', gap: 10, fontSize: 10, color: 'var(--text-muted)', flexWrap: 'wrap' }}>
                    <span><span style={{ color: 'var(--accent-purple)' }}>▬</span> Used {usedPct.toFixed(1)}%</span>
                    <span><span style={{ color: '#f59e0b' }}>▬</span> Cached {cachedPct.toFixed(1)}%</span>
                    <span><span style={{ color: '#22c55e' }}>▬</span> Free {freePct.toFixed(1)}%</span>
                  </div>
                </>
              );
            })()}
          </div>
          <div className="card-footer">{system.memory_used_mb}MB / {system.memory_total_mb}MB</div>
        </div>

        {/* Disk Card */}
        <div className="card metric-card">
          <div className="card-header">Disk</div>
          <div className="metric-value">{system.disk_usage_percent}%</div>
          <MiniChart data={history.map(h => h.disk)} color="#22c55e" />
          <div className="metric-bar">
            <div className="metric-fill" style={{ width: `${system.disk_usage_percent}%`, background: system.disk_usage_percent > 90 ? '#ef4444' : '#22c55e' }} />
          </div>
          <div className="card-footer">{system.disk_used_gb}GB / {system.disk_total_gb}GB</div>
        </div>

        {/* Uptime Card */}
        <div className="card info-card">
          <div className="card-header">Uptime</div>
          <div className="info-value">{formatUptime(system.uptime_seconds)}</div>
          <div className="card-footer">{uptimeDays > 7 ? '⚠ Extended' : 'Normal'}</div>
        </div>

        {/* Battery Card */}
        <div className="card info-card">
          <div className="card-header">Battery</div>
          <div className="info-value">
            {system.battery_percent >= 0
              ? `🔋 ${system.battery_percent}%`
              : '⚡ AC Power'}
          </div>
          <MiniChart data={history.filter(h => h.battery >= 0).map(h => h.battery)} color="#22c55e" />
          <div className="card-footer">
            {system.battery_percent >= 0
              ? (system.ac_line_status === 1 ? '⚡ Charging'
                : system.ac_line_status === 0 ? '🔋 On Battery'
                : system.battery_percent < 20 ? '⚠ Low'
                : `${system.battery_percent}% charged`)
              : 'Desktop / No battery'}
          </div>
        </div>

        {/* Network Card */}
        <div className="card info-card">
          <div className="card-header">Network ↓ / ↑</div>
          <div className="info-value">{formatBytesPerSec(netDownRate)}</div>
          <MiniChart data={history.map(h => h.netDown)} color="#3b82f6" />
          <div className="card-footer">↓ {formatBytesPerSec(netDownRate)} ↑ {formatBytesPerSec(netUpRate)}</div>
        </div>

        {/* Swap Card */}
        <div className="card info-card">
          <div className="card-header">Swap</div>
          <div className="info-value">{system.swap_total_mb > 0 ? `${Math.round(system.swap_used_mb / system.swap_total_mb * 100)}%` : '—'}</div>
          <MiniChart data={history.map(h => h.swapPct)} color="#8b5cf6" />
          <div className="card-footer">{system.swap_used_mb}MB / {system.swap_total_mb}MB</div>
        </div>

        {/* Processes Card */}
        <div className="card info-card">
          <div className="card-header">Processes</div>
          <div className="info-value">{system.total_processes}</div>
          <div className="card-footer">{system.running_processes} running / {system.total_processes} total</div>
        </div>

        {/* GPU Card */}
        {snapshot.gpu.name !== 'Not available' && (
          <div className="card info-card">
            <div className="card-header">GPU</div>
            <div className="info-value" style={{ fontSize: 15 }}>{snapshot.gpu.name}</div>
            <div className="card-footer">
              {snapshot.gpu.memory_mb > 0 ? `${(snapshot.gpu.memory_mb / 1024).toFixed(0)} GB VRAM` : ''}
              {snapshot.gpu.driver_version ? ` · Driver ${snapshot.gpu.driver_version}` : ''}
            </div>
          </div>
        )}

        {/* Disk Read Card */}
        <div className="card info-card">
          <div className="card-header">Disk Read</div>
          <div className="info-value">{formatBytesPerSec(diskReadRate)}</div>
          <MiniChart data={history.map(h => h.diskRead)} color="#3b82f6" />
          <div className="card-footer">Cumulative: {formatBytes(system.disk_read_bytes)}</div>
        </div>

        {/* Disk Write Card */}
        <div className="card info-card">
          <div className="card-header">Disk Write</div>
          <div className="info-value">{formatBytesPerSec(diskWriteRate)}</div>
          <MiniChart data={history.map(h => h.diskWrite)} color="#f59e0b" />
          <div className="card-footer">Cumulative: {formatBytes(system.disk_write_bytes)}</div>
        </div>

        {/* Drive cards */}
        {snapshot.drives.filter(d => d.total_gb > 0).map((drive) => (
          <div key={drive.mount_point} className="card metric-card">
            <div className="card-header">Drive {drive.mount_point}</div>
            <div className="metric-value">{drive.usage_percent}%</div>
            <div className="metric-bar">
              <div className="metric-fill" style={{ width: `${drive.usage_percent}%`, background: drive.usage_percent > 90 ? '#ef4444' : '#22c55e' }} />
            </div>
            <div className="card-footer">{drive.used_gb}GB / {drive.total_gb}GB</div>
          </div>
        ))}
      </div>

      {/* Alerts (dedup by type) */}
      {analysis.alerts.length > 0 && (() => {
        const seen = new Set<string>();
        const unique = analysis.alerts.filter(a => {
          if (seen.has(a.alert_type)) return false;
          seen.add(a.alert_type);
          return true;
        });
        return (
        <div className="section">
          <h2 className="section-title">Alerts ({unique.length})</h2>
          <div className="alert-list">
            {unique.map((alert, i) => (
              <div key={i} className="alert-item" style={{ borderLeftColor: alertColor(alert.severity) }}>
                <span className="alert-badge" style={{ background: alertColor(alert.severity) }}>
                  {alert.severity}
                </span>
                <span className="alert-message">{alert.message}</span>
              </div>
            ))}
          </div>
        </div>
        );
      })()}

      {/* Anomalies */}
      {analysis.anomalies.length > 0 && (
        <div className="section">
          <h2 className="section-title">Anomalies</h2>
          <div className="anomaly-list">
            {analysis.anomalies.map((a, i) => (
              <div key={i} className="anomaly-item" style={{ borderLeftColor: anomalyColor(a.severity) }}>
                <div className="anomaly-header">
                  <span className="anomaly-type">{a.anomaly_type}</span>
                  <span className="anomaly-severity" style={{ color: anomalyColor(a.severity) }}>
                    {a.severity}
                  </span>
                </div>
                <div className="anomaly-detail">{a.detail}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Focus Analysis */}
      <div className="section">
        <h2 className="section-title">Focus Analysis</h2>
        <div className="focus-content">
          <p className="focus-text">{analysis.focus_analysis.assessment}</p>
        </div>
      </div>

      {/* Top Processes */}
      <div className="section">
        <div className="section-header">
          <h2 className="section-title">Processes ({filteredProcesses.length} shown)</h2>
          <input
            className="search-input"
            type="text"
            placeholder="Filter by name or PID..."
            value={searchTerm}
            onChange={e => setSearchTerm(e.target.value)}
          />
        </div>
        <div className="process-table-wrapper">
          <table className="process-table">
            <thead>
              <tr>
                <th className="sortable" onClick={() => handleSort('name')}>Process{sortArrow('name')}</th>
                <th className="sortable" onClick={() => handleSort('pid')}>PID{sortArrow('pid')}</th>
                <th className="sortable" onClick={() => handleSort('cpu_percent')}>CPU{sortArrow('cpu_percent')}</th>
                <th className="sortable" onClick={() => handleSort('memory_mb')}>Memory{sortArrow('memory_mb')}</th>
                <th className="sortable" onClick={() => handleSort('threads')}>Threads{sortArrow('threads')}</th>
                <th className="sortable" onClick={() => handleSort('runtime_seconds')}>Runtime{sortArrow('runtime_seconds')}</th>
              </tr>
            </thead>
            <tbody>
              {filteredProcesses.slice(0, 20).map((p) => (
                <tr key={p.pid} className="proc-row" onClick={() => setSelectedProc(p)}>
                  <td className="proc-name">{p.name}</td>
                  <td>{p.pid}</td>
                  <td>{p.cpu_percent.toFixed(1)}%</td>
                  <td>{p.memory_mb} MB</td>
                  <td>{p.threads}</td>
                  <td>{formatUptime(p.runtime_seconds)}</td>
                </tr>
              ))}
              {filteredProcesses.length === 0 && (
                <tr><td colSpan={6} className="empty-cell">No matching processes</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Observations & Recommendations */}
      <div className="columns">
        <div className="section">
          <h2 className="section-title">Recommendations</h2>
          <ul className="insight-list">
            {analysis.recommendations.map((r, i) => (
              <li key={i}>{r}</li>
            ))}
            {analysis.recommendations.length === 0 && (
              <li className="empty">No recommendations at this time.</li>
            )}
          </ul>
        </div>
        <div className="section">
          <h2 className="section-title">Performance Analysis</h2>
          <ul className="insight-list">
            {analysis.performance.map((p, i) => (
              <li key={i}>{p}</li>
            ))}
            {analysis.performance.length === 0 && (
              <li className="empty">All metrics within normal range.</li>
            )}
          </ul>
        </div>
      </div>

        </>
      ) : page === 'processes' ? (
        <ProcessExplorer data={data} />
      ) : page === 'storage' ? (
        <StoragePage data={data} />
      ) : page === 'network' ? (
        <NetworkPage data={data} />
      ) : page === 'system' ? (
        <SystemInfoPage data={data} />
      ) : page === 'timeline' ? (
        <TimelinePage history={history} />
      ) : page === 'forecast' ? (
        <PredictionsPage data={data} history={history} />
      ) : page === 'usage' ? (
        <UsagePage />
      ) : null}

      {/* Alert History */}
      {alertHistory.length > 0 && page === 'dashboard' && (
        <div className="section">
          <div className="section-header">
            <h2 className="section-title">Alert Log ({alertHistory.length})</h2>
            <button className="clear-btn" onClick={() => {
              setAlertHistory([]);
              notified.current.forEach(k => { if (k.startsWith('log:')) notified.current.delete(k); });
            }}>Clear</button>
          </div>
              <div className="alert-history-list" style={{ scrollbarWidth: 'none' }}>
            {alertHistory.slice(-20).reverse().map((a, i) => (
              <div key={i} className="alert-history-item" style={{ borderLeftColor: alertColor(a.sev) }}>
                <span className="alert-badge" style={{ background: alertColor(a.sev) }}>{a.sev}</span>
                <span className="alert-message">{a.msg}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Settings Drawer */}
      {settingsOpen && (
        <div className="drawer-overlay" onClick={() => setSettingsOpen(false)}>
          <div className="drawer" onClick={e => e.stopPropagation()}>
            <div className="drawer-header">
              <h3>Settings</h3>
              <button className="modal-close" onClick={() => setSettingsOpen(false)}>✕</button>
            </div>
            <div className="drawer-body">
              <div className="setting-group">
                <label className="setting-label">Polling Interval</label>
                <p className="setting-desc">How often KruYa collects a system metrics snapshot. Lower intervals give finer-grained data but use more CPU.</p>
                <div className="setting-row">
                  <input
                    type="range"
                    min="1000"
                    max="30000"
                    step="1000"
                    value={pollInterval}
                    onChange={e => {
                      const val = parseInt(e.target.value);
                      setPollInterval(val);
                      localStorage.setItem('kruya_poll_interval', String(val));
                    }}
                    className="range-input"
                  />
                  <span className="setting-value">{pollInterval / 1000}s</span>
                </div>
              </div>
              <div className="setting-group">
                <label className="setting-label">Appearance</label>
                <p className="setting-desc">Switch between dark and light mode. Dark mode reduces eye strain in low-light environments.</p>
                <div className="setting-row">
                  <button
                    className={`toggle-btn ${theme === 'dark' ? 'on' : 'off'}`}
                    style={theme === 'dark' ? { background: 'var(--accent-blue)', borderColor: 'var(--accent-blue)' } : {}}
                    onClick={() => {
                      setTheme('dark');
                      localStorage.setItem('kruya_theme', 'dark');
                    }}
                  >
                    🌙 Dark
                  </button>
                  <button
                    className={`toggle-btn ${theme === 'light' ? 'on' : 'off'}`}
                    style={theme === 'light' ? { background: '#eab308', borderColor: '#eab308', color: '#18181b' } : {}}
                    onClick={() => {
                      setTheme('light');
                      localStorage.setItem('kruya_theme', 'light');
                    }}
                  >
                    ☀ Light
                  </button>
                </div>
              </div>
              <div className="setting-group">
                <label className="setting-label">Desktop Notifications</label>
                <p className="setting-desc">Get native OS alerts when CPU, memory, or disk usage exceeds your configured thresholds below.</p>
                <div className="setting-row">
                  <button
                    className={`toggle-btn ${notificationsEnabled ? 'on' : 'off'}`}
                    onClick={() => {
                      const next = !notificationsEnabled;
                      setNotificationsEnabled(next);
                      localStorage.setItem('kruya_notifications', String(next));
                    }}
                  >
                    {notificationsEnabled ? 'ON' : 'OFF'}
                  </button>
                </div>
              </div>
              <div className="setting-group">
                <label className="setting-label">Alert Thresholds</label>
                <p className="setting-desc">KruYa triggers a desktop notification when any metric exceeds its threshold. Set levels appropriate for your workflow.</p>
                <div className="threshold-row">
                  <span className="threshold-label">CPU</span>
                  <input
                    type="range" min="30" max="99" value={thresholds.cpu}
                    onChange={e => {
                      const val = parseInt(e.target.value);
                      const next = { ...thresholds, cpu: val };
                      setThresholds(next);
                      localStorage.setItem('kruya_thresholds', JSON.stringify(next));
                    }}
                    className="range-input threshold-range"
                  />
                  <span className="setting-value">{thresholds.cpu}%</span>
                </div>
                <div className="threshold-row">
                  <span className="threshold-label">Memory</span>
                  <input
                    type="range" min="30" max="99" value={thresholds.memory}
                    onChange={e => {
                      const val = parseInt(e.target.value);
                      const next = { ...thresholds, memory: val };
                      setThresholds(next);
                      localStorage.setItem('kruya_thresholds', JSON.stringify(next));
                    }}
                    className="range-input threshold-range"
                  />
                  <span className="setting-value">{thresholds.memory}%</span>
                </div>
                <div className="threshold-row">
                  <span className="threshold-label">Disk</span>
                  <input
                    type="range" min="30" max="99" value={thresholds.disk}
                    onChange={e => {
                      const val = parseInt(e.target.value);
                      const next = { ...thresholds, disk: val };
                      setThresholds(next);
                      localStorage.setItem('kruya_thresholds', JSON.stringify(next));
                    }}
                    className="range-input threshold-range"
                  />
                  <span className="setting-value">{thresholds.disk}%</span>
                </div>
              </div>
              <div className="setting-group">
                <label className="setting-label">Auto-start on boot</label>
                <p className="setting-desc">Launch KruYa automatically when you sign in to Windows. Requires admin privileges for system monitoring.</p>
                <div className="setting-row">
                  <button
                    className={`toggle-btn ${autoStart ? 'on' : 'off'}`}
                    onClick={async () => {
                      const next = !autoStart;
                      try {
                        await setAutoStart(next);
                        setAutoStartState(next);
                      } catch { /* silent */ }
                    }}
                  >
                    {autoStart ? 'ON' : 'OFF'}
                  </button>
                </div>
              </div>

              <div className="setting-group">
                <label className="setting-label">Export Telemetry Data</label>
                <p className="setting-desc">Download recorded snapshots for external analysis. CSV is spreadsheet-friendly, JSON preserves full structure.</p>
                <div className="threshold-row">
                  <span className="threshold-label">Snapshots</span>
                  <input
                    type="range" min="10" max="500" step="10" value={exportLimit}
                    onChange={e => {
                      const v = parseInt(e.target.value);
                      setExportLimit(v);
                      localStorage.setItem('kruya_export_limit', String(v));
                    }}
                    className="range-input threshold-range"
                  />
                  <span className="setting-value">{exportLimit}</span>
                </div>
                <div className="settings-export-actions">
                  <button className="export-action-btn" onClick={async () => {
                    try {
                      const path = await save({ defaultPath: `kruya-export-${new Date().toISOString().slice(0, 10)}.csv`, filters: [{ name: 'CSV', extensions: ['csv'] }] });
                      if (!path) return;
                      const csv = await exportCsv(exportLimit).catch(() => null);
                      if (csv) {
                        const blob = new Blob([csv], { type: 'text/csv' });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement('a'); a.href = url; a.download = path.split('\\').pop() || 'export.csv'; a.click();
                        URL.revokeObjectURL(url);
                      } else {
                        const rows = history.slice(-exportLimit);
                        const header = 'timestamp,cpu,mem,disk,diskRead,diskWrite,netDown,netUp,swapPct,battery,temperature';
                        const data = rows.map(r => `${r.ts},${r.cpu},${r.mem},${r.disk},${r.diskRead},${r.diskWrite},${r.netDown},${r.netUp},${r.swapPct},${r.battery},${r.temperature}`).join('\n');
                        const blob = new Blob([header + '\n' + data], { type: 'text/csv' });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement('a'); a.href = url; a.download = path.split('\\').pop() || 'export.csv'; a.click();
                        URL.revokeObjectURL(url);
                      }
                    } catch { /* silent */ }
                  }}>
                    <span className="export-action-icon">↓</span>
                    <div className="export-label">
                      <span>Export CSV</span>
                      <span>Download snapshot data as spreadsheet</span>
                    </div>
                    <span className="export-chevron">→</span>
                  </button>
                  <button className="export-action-btn" onClick={async () => {
                    try {
                      const path = await save({ defaultPath: `kruya-export-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: 'JSON', extensions: ['json'] }] });
                      if (!path) return;
                      const json = await exportJson(exportLimit).catch(() => null);
                      if (json) {
                        const blob = new Blob([json], { type: 'application/json' });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement('a'); a.href = url; a.download = path.split('\\').pop() || 'export.json'; a.click();
                        URL.revokeObjectURL(url);
                      } else {
                        const rows = history.slice(-exportLimit);
                        const json = JSON.stringify(rows, null, 2);
                        const blob = new Blob([json], { type: 'application/json' });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement('a'); a.href = url; a.download = path.split('\\').pop() || 'export.json'; a.click();
                        URL.revokeObjectURL(url);
                      }
                    } catch { /* silent */ }
                  }}>
                    <span className="export-action-icon">↓</span>
                    <div className="export-label">
                      <span>Export JSON</span>
                      <span>Full structured telemetry data</span>
                    </div>
                    <span className="export-chevron">→</span>
                  </button>
                  <button className="export-action-btn" onClick={async () => {
                    try {
                      if (!data) return;
                      const path = await save({ defaultPath: `kruya-report-${new Date().toISOString().slice(0, 10)}.pdf`, filters: [{ name: 'PDF Report', extensions: ['pdf'] }] });
                      if (!path) return;
                      const [usage, startups] = await Promise.all([getAppUsage(), getStartupPrograms()]);
                      const temps = history.filter(h => h.temperature > 0).map(h => h.temperature);
                      const stats = {
                        avg_cpu: history.length > 0 ? history.reduce((s, h) => s + h.cpu, 0) / history.length : 0,
                        max_cpu: history.length > 0 ? Math.max(...history.map(h => h.cpu)) : 0,
                        avg_mem: history.length > 0 ? Math.round(history.reduce((s, h) => s + h.mem, 0) / history.length) : 0,
                        max_mem: history.length > 0 ? Math.max(...history.map(h => h.mem)) : 0,
                        avg_temp: temps.length > 0 ? temps.reduce((s, t) => s + t, 0) / temps.length : 0,
                        max_temp: temps.length > 0 ? Math.max(...temps) : 0,
                        samples: history.length,
                      };
                      await exportReportApi(path, data.snapshot, data.analysis, usage, startups, stats);
                    } catch { /* silent */ }
                  }}>
                    <span className="export-action-icon">↓</span>
                    <div className="export-label">
                      <span>Export Report (PDF)</span>
                      <span>System summary with historical averages</span>
                    </div>
                    <span className="export-chevron">→</span>
                  </button>
                </div>
              </div>

              <div className="setting-group">
                <label className="setting-label">Data Retention</label>
                <p className="setting-desc">Number of historical snapshots kept in memory for charting, export, and the Forecast engine. Higher values use more RAM.</p>
                <div className="threshold-row">
                  <span className="threshold-label">Max snapshots</span>
                  <input
                    type="range" min="30" max="480" step="10" value={historyRetention}
                    onChange={e => {
                      const v = parseInt(e.target.value);
                      setHistoryRetention(v);
                      localStorage.setItem('kruya_history_retention', String(v));
                    }}
                    className="range-input threshold-range"
                  />
                  <span className="setting-value">{historyRetention}</span>
                </div>
              </div>

              <div className="setting-group">
                <label className="setting-label">Temperature Unit</label>
                <div className="setting-row">
                  <button
                    className={`toggle-btn ${tempUnit === 'c' ? 'on' : 'off'}`}
                    onClick={() => {
                      const next = tempUnit === 'c' ? 'f' : 'c';
                      setTempUnit(next);
                      localStorage.setItem('kruya_temp_unit', next);
                    }}
                    style={tempUnit === 'c' ? {} : { background: 'var(--accent-orange)', borderColor: 'var(--accent-orange)' }}
                  >
                    {tempUnit === 'c' ? '°C' : '°F'}
                  </button>
                  <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>{tempUnit === 'c' ? 'Celsius' : 'Fahrenheit'}</span>
                </div>
              </div>

              <div className="setting-group">
                <label className="setting-label">Forecast Engine</label>
                <p className="setting-desc">Analyzes telemetry trends to predict future bottlenecks like RAM saturation, thermal pressure, and responsiveness degradation.</p>
                <div className="setting-row">
                  <button
                    className={`toggle-btn ${forecastEnabled ? 'on' : 'off'}`}
                    onClick={() => {
                      const next = !forecastEnabled;
                      setForecastEnabled(next);
                      localStorage.setItem('kruya_forecast', String(next));
                    }}
                  >
                    {forecastEnabled ? 'ON' : 'OFF'}
                  </button>
                  <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>Predictive trend analysis</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      <footer className="footer">
        <div className="footer-left">
          <span>KruYa v0.1.0-alpha.1</span>
          <span className="footer-sep">|</span>
          <span>{system.cpu_brand || '—'}</span>
          <span className="footer-sep">|</span>
          <span>{system.total_processes} procs</span>
        </div>
        <div className="footer-right">
          <span>Poll: {pollInterval / 1000}s</span>
          <span className="footer-sep">|</span>
          <span>{snapshot.timestamp}</span>
        </div>
      </footer>

      {/* Process Detail Modal */}
      {selectedProc && (
        <div className="modal-overlay" onClick={() => setSelectedProc(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{selectedProc.name}</h3>
              <button className="modal-close" onClick={() => setSelectedProc(null)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="modal-grid">
                <div className="modal-item">
                  <span className="modal-label">PID</span>
                  <span className="modal-val">{selectedProc.pid}</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">CPU</span>
                  <span className="modal-val">{selectedProc.cpu_percent.toFixed(1)}%</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Memory</span>
                  <span className="modal-val">{selectedProc.memory_mb} MB</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Threads</span>
                  <span className="modal-val">{selectedProc.threads}</span>
                </div>
                <div className="modal-item">
                  <span className="modal-label">Runtime</span>
                  <span className="modal-val">{formatUptime(selectedProc.runtime_seconds)}</span>
                </div>
              </div>
              {(selectedProc.path || selectedProc.cmd) && (
                <div className="modal-details">
                  {selectedProc.path && (
                    <div className="modal-detail-item">
                      <span className="modal-label">Path</span>
                      <span className="modal-detail-val">{selectedProc.path}</span>
                    </div>
                  )}
                  {selectedProc.cmd && (
                    <div className="modal-detail-item">
                      <span className="modal-label">Command Line</span>
                      <span className="modal-detail-val mono">{selectedProc.cmd}</span>
                    </div>
                  )}
                </div>
              )}
              {procHistory.length > 0 && (
                <div className="modal-details">
                  <span className="modal-label" style={{ marginBottom: 6, display: 'block' }}>CPU / Memory History (60 snapshots)</span>
                  <div className="modal-chart-row">
                    <MiniChart
                      data={procHistory.map(h => h.cpu_percent)}
                      color="#3b82f6"
                      width={280}
                      height={24}
                    />
                  </div>
                  <div className="modal-chart-row">
                    <MiniChart
                      data={procHistory.map(h => h.memory_mb)}
                      color="#8b5cf6"
                      width={280}
                      height={24}
                    />
                  </div>
                </div>
              )}
              {procHistoryLoading && (
                <div className="modal-details">
                  <span className="modal-label" style={{ color: 'var(--text-muted)' }}>Loading history...</span>
                </div>
              )}
              <div className="modal-actions">
                <button
                  className="kill-btn"
                  onClick={async (e) => {
                    e.stopPropagation();
                    if (window.confirm(`Terminate ${selectedProc.name} (PID ${selectedProc.pid})?`)) {
                      try {
                        await killProcess(selectedProc.pid);
                        setSelectedProc(null);
                      } catch (err) {
                        alert('Failed to kill: ' + err);
                      }
                    }
                  }}
                >
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


