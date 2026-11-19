import type { SnapshotResult } from '../types';

interface Props {
  data: SnapshotResult;
}

function fmt(n: number): string {
  return n % 1 === 0 ? String(n) : n.toFixed(1);
}

export default function StoragePage({ data }: Props) {
  const { system, drives } = data.snapshot;
  const activeDrives = drives.filter(d => d.total_gb > 0);
  const swapPct = system.swap_total_mb > 0 ? Math.round((system.swap_used_mb / system.swap_total_mb) * 100) : 0;

  return (
    <div className="page page-storage">
      <h2 className="section-title">Storage — {activeDrives.length} drive(s)</h2>
      <div className="storage-grid">
        {activeDrives.map(drive => (
          <div key={drive.mount_point} className="card storage-card">
            <div className="card-header">
              Drive {drive.mount_point}
              {drive.filesystem_type && <span className="storage-badge">{drive.filesystem_type}</span>}
              {drive.is_removable && <span className="storage-badge removable">Removable</span>}
            </div>
            <div className="storage-value">{drive.usage_percent}% used</div>
            <div className="metric-bar" style={{ margin: '8px 0' }}>
              <div className="metric-fill" style={{ width: `${drive.usage_percent}%`, background: drive.usage_percent > 90 ? '#ef4444' : '#22c55e' }} />
            </div>
            <div className="storage-details">
              <div className="storage-row"><span className="storage-label">Used</span><span>{drive.used_gb} GB</span></div>
              <div className="storage-row"><span className="storage-label">Free</span><span>{fmt(drive.total_gb - drive.used_gb)} GB</span></div>
              <div className="storage-row"><span className="storage-label">Total</span><span>{drive.total_gb} GB</span></div>
            </div>
          </div>
        ))}
      </div>

      <div className="storage-grid" style={{ marginTop: 12 }}>
        <div className="card">
          <div className="card-header">Aggregate Storage</div>
          <div className="storage-value">{system.disk_usage_percent}% used</div>
          <div className="metric-bar" style={{ margin: '8px 0' }}>
            <div className="metric-fill" style={{ width: `${system.disk_usage_percent}%`, background: system.disk_usage_percent > 90 ? '#ef4444' : '#22c55e' }} />
          </div>
          <div className="storage-details">
            <div className="storage-row"><span className="storage-label">Total Used</span><span>{system.disk_used_gb} GB</span></div>
            <div className="storage-row"><span className="storage-label">Total Size</span><span>{system.disk_total_gb} GB</span></div>
            <div className="storage-row"><span className="storage-label">Read</span><span>{fmt(system.disk_read_bytes / 1e12)} TB cumulative</span></div>
            <div className="storage-row"><span className="storage-label">Written</span><span>{fmt(system.disk_write_bytes / 1e12)} TB cumulative</span></div>
          </div>
        </div>

        <div className="card swap-card">
          <div className="card-header">Swap Memory</div>
          <div className="storage-value">{swapPct}% used</div>
          <div className="metric-bar" style={{ margin: '8px 0' }}>
            {system.swap_total_mb > 0 && (
              <div className="metric-fill" style={{ width: `${swapPct}%`, background: swapPct > 70 ? '#ef4444' : swapPct > 40 ? '#f59e0b' : '#22c55e' }} />
            )}
          </div>
          <div className="storage-details">
            <div className="storage-row"><span className="storage-label">Used</span><span>{system.swap_used_mb} MB</span></div>
            <div className="storage-row"><span className="storage-label">Total</span><span>{system.swap_total_mb} MB</span></div>
            <div className="storage-row"><span className="storage-label">Status</span><span>{system.swap_total_mb <= 0 ? 'No swap' : swapPct > 50 ? 'Heavy usage' : 'Normal'}</span></div>
          </div>
        </div>
      </div>
    </div>
  );
}
