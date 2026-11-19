import type { SnapshotResult } from '../types';

function formatBytes(b: number): string {
  if (b >= 1e12) return (b / 1e12).toFixed(1) + ' TB';
  if (b >= 1e9) return (b / 1e9).toFixed(1) + ' GB';
  if (b >= 1e6) return (b / 1e6).toFixed(1) + ' MB';
  if (b >= 1e3) return (b / 1e3).toFixed(0) + ' KB';
  return b + ' B';
}

interface Props {
  data: SnapshotResult;
}

export default function NetworkPage({ data }: Props) {
  const { interfaces, system } = data.snapshot;
  const totalTraffic = system.network_down_bytes + system.network_up_bytes;

  const downPct = totalTraffic > 0 ? (system.network_down_bytes / totalTraffic) * 100 : 50;

  return (
    <div className="page page-network">
      <h2 className="section-title">Network Interfaces — {interfaces.length} interface(s)</h2>
      <div className="storage-grid">
        {interfaces.map(nic => {
          const nicTotal = nic.total_received + nic.total_transmitted;
          return (
            <div key={nic.name} className="card">
              <div className="card-header">{nic.name}</div>
              <div className="nic-value">{formatBytes(nicTotal)}</div>
              <div className="metric-bar" style={{ margin: '8px 0' }}>
                <div className="metric-fill" style={{ width: `${nicTotal > 0 ? (nic.total_received / nicTotal) * 100 : 50}%`, background: '#06b6d4' }} />
              </div>
              <div className="nic-stats">
                <div className="nic-row"><span className="storage-label">↓ Received</span><span>{formatBytes(nic.total_received)}</span></div>
                <div className="nic-row"><span className="storage-label">↑ Transmitted</span><span>{formatBytes(nic.total_transmitted)}</span></div>
                <div className="nic-row"><span className="storage-label">Down/Up Ratio</span><span>{(nic.total_received / Math.max(nic.total_transmitted, 1)).toFixed(1)}x</span></div>
              </div>
            </div>
          );
        })}
      </div>
      <div className="card aggregate-card" style={{ marginTop: 12 }}>
        <div className="card-header">Aggregate Traffic</div>
        <div className="nic-value">{formatBytes(totalTraffic)}</div>
        <div className="metric-bar" style={{ margin: '8px 0' }}>
          <div className="metric-fill" style={{ width: `${downPct}%`, background: '#22c55e' }} />
        </div>
        <div className="nic-stats">
          <div className="nic-row"><span className="storage-label">↓ Total Received</span><span>{formatBytes(system.network_down_bytes)}</span></div>
          <div className="nic-row"><span className="storage-label">↑ Total Transmitted</span><span>{formatBytes(system.network_up_bytes)}</span></div>
          <div className="nic-row"><span className="storage-label">Interfaces</span><span>{interfaces.length}</span></div>
        </div>
      </div>
    </div>
  );
}
