import { useState } from 'react';
import MiniChart from '../components/MiniChart';

interface HistoryPoint {
  cpu: number; mem: number; disk: number;
  diskRead: number; diskWrite: number;
  netDown: number; netUp: number;
  swapPct: number; battery: number; temperature: number;
  ts: string;
}

interface Props {
  history: HistoryPoint[];
}

type MetricKey = Exclude<keyof HistoryPoint, 'ts'>;

const metrics: { key: MetricKey; label: string; color: string }[] = [
  { key: 'cpu', label: 'CPU %', color: '#3b82f6' },
  { key: 'mem', label: 'Memory %', color: '#8b5cf6' },
  { key: 'disk', label: 'Disk %', color: '#22c55e' },
  { key: 'swapPct', label: 'Swap %', color: '#f59e0b' },
  { key: 'diskRead', label: 'Disk Read (MB/s)', color: '#3b82f6' },
  { key: 'diskWrite', label: 'Disk Write (MB/s)', color: '#f59e0b' },
  { key: 'netDown', label: 'Net Down (MB/s)', color: '#22c55e' },
  { key: 'netUp', label: 'Net Up (MB/s)', color: '#ef4444' },
  { key: 'temperature', label: 'Temperature °C', color: '#f97316' },
  { key: 'battery', label: 'Battery %', color: '#14b8a6' },
];

export default function TimelinePage({ history }: Props) {
  const [selected, setSelected] = useState<string[]>(['cpu', 'mem', 'disk']);

  const toggle = (key: string) => {
    setSelected(prev =>
      prev.includes(key) ? prev.filter(k => k !== key) : [...prev, key]
    );
  };

  if (history.length < 2) {
    return (
    <div className="page page-timeline">
        <h2 className="section-title">Performance Timeline</h2>
        <p className="empty-state">Collecting data... {history.length} snapshots so far.</p>
      </div>
    );
  }

  return (
    <div className="page page-timeline">
      <h2 className="section-title">Performance Timeline — {history.length} snapshots</h2>
      <div className="timeline-metrics">
        {metrics.map(m => (
          <button
            key={m.key}
            className={`metric-tag ${selected.includes(m.key) ? 'active' : ''}`}
            style={selected.includes(m.key) ? { borderColor: m.color, color: m.color } : {}}
            onClick={() => toggle(m.key)}
          >
            {m.label}
          </button>
        ))}
      </div>
      <div className="timeline-charts">
        {metrics.filter(m => selected.includes(m.key)).map(m => (
          <div key={m.key} className="card timeline-card">
            <div className="card-header">{m.label}</div>
            <MiniChart
              data={history.map(h => h[m.key] as number).filter(v => m.key === 'battery' || m.key === 'temperature' ? v > 0 : true)}
              color={m.color}
              width={500}
              height={60}
            />
            <div className="timeline-range">
              <span>Min: {Math.min(...history.map(h => h[m.key] as number).filter(v => m.key === 'battery' || m.key === 'temperature' ? v > 0 : true)).toFixed(1)}</span>
              <span>Now: {(history[history.length - 1][m.key] as number).toFixed(1)}</span>
              <span>Max: {Math.max(...history.map(h => h[m.key] as number).filter(v => m.key === 'battery' || m.key === 'temperature' ? v > 0 : true)).toFixed(1)}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
