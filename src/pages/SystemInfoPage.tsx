import { useState, useEffect, useRef } from 'react';
import type { SnapshotResult, StartupProgram, BatteryInfo } from '../types';
import { startCpuStress, stopCpuStress, getStartupPrograms, getBatteryInfoExt } from '../api';

interface Props {
  data: SnapshotResult;
}

function BlurValue({ value }: { value: string }) {
  const [revealed, setRevealed] = useState(false);
  return (
    <span
      className="sysinfo-value"
      onClick={() => setRevealed(!revealed)}
      style={{
        cursor: 'pointer',
        filter: revealed ? 'none' : 'blur(6px)',
        transition: 'filter 0.15s',
        userSelect: revealed ? 'text' : 'none',
      }}
    >
      {value}
    </span>
  );
}

function Row({ label, value, empty, sensitive }: { label: string; value: string | number; empty?: string; sensitive?: boolean }) {
  const show = empty !== undefined ? empty : '—';
  const v = value === '' || value == null ? show : String(value);
  return (
    <div className="sysinfo-row">
      <span className="sysinfo-label">{label}</span>
      {sensitive && v !== show ? (
        <BlurValue value={v} />
      ) : (
        <span className="sysinfo-value">{v}</span>
      )}
    </div>
  );
}

function fmt(n: number, unit: string) {
  if (n <= 0) return '—';
  return `${n} ${unit}`;
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="card sysinfo-card" style={{ flex: '1 1 320px', minWidth: '280px' }}>
      <div className="card-header">{title}</div>
      <div className="sysinfo-table">{children}</div>
    </div>
  );
}

export default function SystemInfoPage({ data }: Props) {
  const { sys_info, gpu, displays, drives, system } = data.snapshot;

  const [isStressing, setIsStressing] = useState(false);
  const [stressCores, setStressCores] = useState(sys_info.cpu_cores || 4);
  const [stressDuration, setStressDuration] = useState(10);
  const [timeLeft, setTimeLeft] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [startupPrograms, setStartupPrograms] = useState<StartupProgram[]>([]);
  const [batteryInfo, setBatteryInfo] = useState<BatteryInfo | null>(null);
  const [startupOpen, setStartupOpen] = useState(false);

  useEffect(() => {
    getStartupPrograms().then(setStartupPrograms).catch(() => {});
    getBatteryInfoExt().then(setBatteryInfo).catch(() => {});
  }, []);

  const handleStopStress = async () => {
    try {
      await stopCpuStress();
      setIsStressing(false);
      setTimeLeft(0);
      if (timerRef.current) clearTimeout(timerRef.current);
    } catch (err) {
      console.error("Failed to stop stress test:", err);
    }
  };

  useEffect(() => {
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      stopCpuStress().catch(console.error);
    };
  }, []);

  useEffect(() => {
    if (timeLeft > 0 && isStressing) {
      timerRef.current = setTimeout(() => {
        setTimeLeft(prev => prev - 1);
      }, 1000);
    }
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [timeLeft, isStressing]);

  useEffect(() => {
    if (timeLeft === 0 && isStressing) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      handleStopStress();
    }
  }, [timeLeft, isStressing]);

  const handleStartStress = async () => {
    try {
      await startCpuStress(stressCores, stressDuration);
      setIsStressing(true);
      setTimeLeft(stressDuration);
    } catch (err) {
      console.error("Failed to start stress test:", err);
    }
  };

  return (
    <div className="page page-sysinfo">
      <h2 className="section-title">System Information</h2>
      <div className="sysinfo-grid">

        <Section title="Operating System">
          <Row label="Edition" value={sys_info.os_edition || sys_info.os_name} />
          <Row label="OS Version" value={sys_info.os_version} />
          <Row label="Build" value={sys_info.os_build || (sys_info.kernel_version.includes('.') ? sys_info.kernel_version.split('.').slice(-1)[0] : '')} />
          <Row label="Install Date" value={sys_info.os_install_date || 'Not available'} empty="Not available" />
          <Row label="Architecture" value={sys_info.os_architecture || sys_info.cpu_architecture} />
          <Row label="Kernel" value={sys_info.kernel_version} />
          <Row label="Hostname" value={sys_info.hostname} />
        </Section>

        <Section title="CPU">
          <Row label="Model" value={sys_info.cpu_brand} />
          <Row label="Logical Cores" value={String(sys_info.cpu_cores)} />
          <Row label="Physical Cores" value={sys_info.cpu_physical_cores > 0 ? String(sys_info.cpu_physical_cores) : '—'} />
          <Row label="Max Frequency" value={fmt(sys_info.cpu_max_frequency_mhz, 'MHz')} />
          <Row label="Current Usage" value={`${system.cpu_total_usage.toFixed(1)}%`} />
          <Row label="Architecture" value={sys_info.cpu_architecture} />
          <Row label="Temperature" value={system.temperature_celsius > 0 ? `${system.temperature_celsius.toFixed(1)}°C` : '—'} />
        </Section>

        <Section title="Memory">
          <Row label="Total RAM" value={`${sys_info.total_ram_mb} MB (${(sys_info.total_ram_mb / 1024).toFixed(0)} GB)`} />
          <Row label="Used" value={`${system.memory_used_mb} MB`} />
          <Row label="Available" value={`${system.memory_available_mb} MB`} />
          <Row label="Usage" value={`${system.memory_total_usage}%`} />
          <Row label="Speed" value={fmt(sys_info.ram_speed_mhz, 'MHz')} />
          <Row label="Form Factor" value={sys_info.ram_form_factor} />
          <Row label="Slots" value={sys_info.ram_slots_total > 0 ? `${sys_info.ram_slots_used} / ${sys_info.ram_slots_total} used` : '—'} />
          <Row label="Swap Total" value={fmt(system.swap_total_mb, 'MB')} />
          <Row label="Swap Used" value={fmt(system.swap_used_mb, 'MB')} />
        </Section>

        <Section title="Motherboard">
          <Row label="Manufacturer" value={sys_info.motherboard_manufacturer} />
          <Row label="Model" value={sys_info.motherboard_model} />
        </Section>

        <Section title="System Chassis">
          <Row label="Manufacturer" value={sys_info.system_manufacturer} />
          <Row label="Model" value={sys_info.system_model} />
          <Row label="Serial" value={sys_info.system_serial} sensitive />
          <Row label="SKU" value={sys_info.system_sku} />
        </Section>

        <Section title="BIOS / UEFI">
          <Row label="Vendor" value={sys_info.bios_vendor} />
          <Row label="Version" value={sys_info.bios_version} />
          <Row label="Date" value={sys_info.bios_date} />
        </Section>

        <Section title="GPU">
          <Row label="Name" value={gpu.name} />
          <div className="sysinfo-row">
            <span className="sysinfo-label">Type</span>
            <span>
              <span className={`gpu-badge ${gpu.gpu_type.toLowerCase()}`}>{gpu.gpu_type}</span>
            </span>
          </div>
          <Row label="VRAM" value={gpu.memory_mb > 0 ? `${(gpu.memory_mb / 1024).toFixed(0)} GB` : '—'} />
          <Row label="Driver" value={gpu.driver_version} />
        </Section>

        <Section title="Displays">
          {displays.length === 0 ? (
            <Row label="—" value="No display info" />
          ) : (
            displays.map((d, i) => (
              <div key={i}>
                <Row label={`Display ${i + 1}`} value={`${d.name}`} />
                <Row label="Resolution" value={`${d.resolution_width} × ${d.resolution_height}`} />
                <Row label="Refresh Rate" value={d.refresh_rate > 0 ? `${d.refresh_rate} Hz` : '—'} />
                <Row label="Primary" value={d.is_primary ? 'Yes' : 'No'} />
                {i < displays.length - 1 && <hr style={{ borderColor: 'var(--border)', margin: '8px 0' }} />}
              </div>
            ))
          )}
        </Section>

        <Section title="Storage">
          <Row label="Total Capacity" value={`${system.disk_total_gb} GB`} />
          <Row label="Used" value={`${system.disk_used_gb} GB`} />
          <Row label="Usage" value={`${system.disk_usage_percent}%`} />
          {drives.map((d, i) => (
            <div key={i}>
              <Row label={d.mount_point} value={`${d.total_gb} GB (${d.usage_percent}% used)`} />
              <Row label="Filesystem" value={d.filesystem_type} />
              <Row label="Removable" value={d.is_removable ? 'Yes' : 'No'} />
              <Row label="I/O Read" value={`${(data.snapshot.system.disk_read_bytes / (1024 * 1024)).toFixed(1)} MB`} />
              <Row label="I/O Write" value={`${(data.snapshot.system.disk_write_bytes / (1024 * 1024)).toFixed(1)} MB`} />
              {i < drives.length - 1 && <hr style={{ borderColor: 'var(--border)', margin: '8px 0' }} />}
            </div>
          ))}
        </Section>

        <Section title="Runtime">
          <Row label="Uptime" value={`${Math.floor(system.uptime_seconds / 86400)}d ${Math.floor((system.uptime_seconds % 86400) / 3600)}h ${Math.floor((system.uptime_seconds % 3600) / 60)}m`} />
          <Row label="Battery" value={system.battery_percent >= 0
            ? (system.ac_line_status === 1 ? `⚡ ${system.battery_percent}% (Charging)`
              : system.ac_line_status === 0 ? `🔋 ${system.battery_percent}% (On Battery)`
              : `${system.battery_percent}%`)
            : 'AC Power / No Battery'} />
          <Row label="Processes" value={`${system.total_processes} total`} />
          <Row label="Running" value={`${system.running_processes} running`} />
          <Row label="Temperature" value={system.temperature_celsius > 0 ? `${system.temperature_celsius.toFixed(1)}°C` : '—'} />
        </Section>

        {batteryInfo && batteryInfo.design_capacity_mwh > 0 && (
          <Section title="Battery Health">
            <Row label="Design Capacity" value={`${(batteryInfo.design_capacity_mwh / 1000).toFixed(0)} Wh`} />
            <Row label="Full Charge" value={`${(batteryInfo.full_charge_capacity_mwh / 1000).toFixed(0)} Wh`} />
            <Row label="Cycle Count" value={String(batteryInfo.cycle_count)} />
            <Row label="Wear Level" value={`${batteryInfo.wear_level_pct.toFixed(1)}%`} />
          </Section>
        )}

        <Section title="Startup Programs">
          {startupPrograms.length === 0 ? (
            <Row label="—" value="No startup programs found" />
          ) : startupPrograms.length <= 5 ? (
            startupPrograms.map((sp, i) => (
              <div key={i}>
                <Row label={sp.name} value={sp.location} />
                <Row label="Command" value={sp.command.length > 60 ? sp.command.slice(0, 60) + '…' : sp.command} />
                {i < startupPrograms.length - 1 && <hr style={{ borderColor: 'var(--border)', margin: '6px 0' }} />}
              </div>
            ))
          ) : (
            <>
              <Row label="Total" value={`${startupPrograms.length} programs`} />
              <Row label="—" value={`Last: ${startupPrograms[startupPrograms.length - 1].name}`} />
              <div style={{ marginTop: 8 }}>
                <button
                  className="export-btn"
                  style={{ width: '100%', padding: '6px 12px', fontSize: 12 }}
                  onClick={() => setStartupOpen(true)}
                >
                  View All ({startupPrograms.length})
                </button>
              </div>
            </>
          )}
        </Section>

        <Section title="Network">
          <Row label="Total Down" value={system.network_down_bytes > 0 ? `${(system.network_down_bytes / (1024 * 1024 * 1024)).toFixed(2)} GB` : '—'} />
          <Row label="Total Up" value={system.network_up_bytes > 0 ? `${(system.network_up_bytes / (1024 * 1024 * 1024)).toFixed(2)} GB` : '—'} />
          {data.snapshot.interfaces.slice(0, 5).map((ni, i) => (
            <Row key={i} label={ni.name} value={`↓ ${(ni.total_received / (1024 * 1024)).toFixed(0)} MB / ↑ ${(ni.total_transmitted / (1024 * 1024)).toFixed(0)} MB`} />
          ))}
        </Section>

      </div>

      <div className="section" style={{ marginTop: '24px' }}>
        <h3 className="section-title">CPU Performance Stress Diagnostics</h3>
        <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '16px', lineHeight: '1.5' }}>
          Simulate high CPU workloads directly from KruYa to test thermal performance, cooling capacity, and trigger diagnostic alert limits.
        </p>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '20px', alignItems: 'center' }}>
          <div className="setting-group" style={{ margin: 0, flex: '1 1 200px' }}>
            <span className="setting-label">Cores to Stress: {stressCores}</span>
            <input 
              type="range" 
              className="range-input" 
              min={1} 
              max={sys_info.cpu_cores || 8} 
              value={stressCores} 
              disabled={isStressing}
              onChange={(e) => setStressCores(parseInt(e.target.value))} 
            />
          </div>

          <div className="setting-group" style={{ margin: 0, flex: '1 1 200px' }}>
            <span className="setting-label">Duration: {stressDuration}s</span>
            <input 
              type="range" 
              className="range-input" 
              min={5} 
              max={60} 
              step={5}
              value={stressDuration} 
              disabled={isStressing}
              onChange={(e) => setStressDuration(parseInt(e.target.value))} 
            />
          </div>

          <div style={{ flex: '1 1 200px', display: 'flex', alignItems: 'center', gap: '12px' }}>
            {!isStressing ? (
              <button 
                className="export-btn" 
                style={{ width: '100%', padding: '10px 16px', fontSize: '13px', display: 'flex', justifyContent: 'center', alignItems: 'center', gap: '8px', background: 'var(--accent-purple)', borderColor: 'var(--accent-purple)' }}
                onClick={handleStartStress}
              >
                🔥 Run Stress Test
              </button>
            ) : (
              <button 
                className="kill-btn" 
                style={{ width: '100%', padding: '10px 16px', fontSize: '13px', display: 'flex', justifyContent: 'center', alignItems: 'center', gap: '8px' }}
                onClick={handleStopStress}
              >
                🛑 Stop Test ({timeLeft}s)
              </button>
            )}
          </div>
        </div>
      </div>

      {startupOpen && (
        <div className="modal-overlay" onClick={() => setStartupOpen(false)}>
          <div className="modal" onClick={e => e.stopPropagation()} style={{ maxWidth: 600, maxHeight: '80vh', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
            <div className="modal-header">
              <h3>Startup Programs ({startupPrograms.length})</h3>
              <button className="modal-close" onClick={() => setStartupOpen(false)}>✕</button>
            </div>
            <div className="modal-body" style={{ overflow: 'auto', flex: 1, padding: 0 }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
                <thead>
                  <tr style={{ background: 'var(--surface)', position: 'sticky', top: 0 }}>
                    <th style={{ padding: '8px 10px', textAlign: 'left', borderBottom: '1px solid var(--border)' }}>Name</th>
                    <th style={{ padding: '8px 10px', textAlign: 'left', borderBottom: '1px solid var(--border)' }}>Command</th>
                    <th style={{ padding: '8px 10px', textAlign: 'left', borderBottom: '1px solid var(--border)' }}>Location</th>
                  </tr>
                </thead>
                <tbody>
                  {startupPrograms.map((sp, i) => (
                    <tr key={i} style={{ borderBottom: '1px solid var(--border)' }}>
                      <td style={{ padding: '6px 10px', whiteSpace: 'nowrap', fontWeight: 500 }}>{sp.name}</td>
                      <td style={{ padding: '6px 10px', maxWidth: 250, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={sp.command}>{sp.command}</td>
                      <td style={{ padding: '6px 10px', whiteSpace: 'nowrap' }}>{sp.location}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
