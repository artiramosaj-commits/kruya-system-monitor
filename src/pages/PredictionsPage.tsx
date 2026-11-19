import { useMemo } from 'react';
import MiniChart from '../components/MiniChart';
import { analyzePredictions, computeHealthScore, type HistoryPoint } from '../predictive';
import type { SnapshotResult } from '../types';

interface Props {
  data: SnapshotResult;
  history: HistoryPoint[];
}

const severityColors: Record<string, string> = {
  critical: '#ef4444', high: '#f97316', medium: '#eab308', low: '#22c55e',
};

function SeverityBadge({ severity }: { severity: string }) {
  return (
    <span className="pred-badge" style={{ background: `${severityColors[severity]}20`, color: severityColors[severity], borderColor: severityColors[severity] }}>
      {severity}
    </span>
  );
}

function ConfidenceBar({ value }: { value: number }) {
  const color = value > 75 ? '#ef4444' : value > 55 ? '#f97316' : value > 35 ? '#eab308' : '#22c55e';
  return (
    <div className="pred-confidence">
      <div className="pred-conf-bar">
        <div className="pred-conf-fill" style={{ width: `${value}%`, background: color }} />
      </div>
      <span className="pred-conf-label">{value}%</span>
    </div>
  );
}

export default function PredictionsPage({ data, history }: Props) {
  const { snapshot } = data;

  const predictions = useMemo(() => analyzePredictions(
    history,
    snapshot.processes.map(p => ({ name: p.name, cpu_percent: p.cpu_percent, memory_mb: p.memory_mb })),
    { memory_total_mb: snapshot.system.memory_total_mb, swap_total_mb: snapshot.system.swap_total_mb, cpu_cores: snapshot.sys_info.cpu_cores }
  ), [history, snapshot.processes, snapshot.system.memory_total_mb, snapshot.system.swap_total_mb, snapshot.sys_info.cpu_cores]);

  const healthScore = useMemo(() => computeHealthScore(predictions), [predictions]);

  const gaugeColor = healthScore > 75 ? '#22c55e' : healthScore > 50 ? '#eab308' : healthScore > 25 ? '#f97316' : '#ef4444';
  const gaugeCircumference = 339.292;

  const criticalCount = predictions.filter(p => p.severity === 'critical').length;
  const highCount = predictions.filter(p => p.severity === 'high').length;

  if (predictions.length === 0) {
    return (
      <div className="page page-predictions">
        <h2 className="section-title">Predictive Slowdown Engine</h2>
        <div className="pred-empty">
          <div className="pred-empty-icon">◉</div>
          <p>Collecting sufficient telemetry data for predictions...</p>
          <p className="pred-empty-sub">{history.length} snapshot{history.length !== 1 ? 's' : ''} recorded (minimum 3 required).</p>
        </div>
      </div>
    );
  }

  return (
    <div className="page page-predictions">
      <div className="pred-header">
        <h2 className="section-title">Predictive Slowdown Engine</h2>
        <span className="pred-subtitle">{predictions.length} prediction{predictions.length !== 1 ? 's' : ''}</span>
      </div>

      <div className="pred-top-row">
        <div className="pred-gauge-card">
          <svg viewBox="0 0 120 120" className="pred-gauge-svg">
            <circle cx="60" cy="60" r="54" fill="none" stroke="var(--border)" strokeWidth="8" />
            <circle
              cx="60" cy="60" r="54"
              fill="none"
              stroke={gaugeColor}
              strokeWidth="8"
              strokeDasharray={`${(healthScore / 100) * gaugeCircumference} ${gaugeCircumference}`}
              strokeLinecap="round"
              transform="rotate(-90 60 60)"
              className="pred-gauge-arc"
            />
            <text x="60" y="54" textAnchor="middle" fill="var(--text-primary)" fontSize="24" fontWeight="700">{healthScore}</text>
            <text x="60" y="72" textAnchor="middle" fill="var(--text-muted)" fontSize="10" fontWeight="600" letterSpacing="1" style={{ textTransform: 'uppercase' }}>Health</text>
          </svg>
          <div className="pred-gauge-footer">
            {predictions.length > 0 && (
              <span style={{ color: severityColors[predictions[0].severity], fontSize: 12, fontWeight: 600 }}>
                Top risk: {predictions[0].severity}
              </span>
            )}
          </div>
        </div>

        <div className="pred-summary-cards">
          <div className="pred-summary-card">
            <span className="pred-summary-val" style={{ color: '#ef4444' }}>{criticalCount}</span>
            <span className="pred-summary-label">Critical</span>
          </div>
          <div className="pred-summary-card">
            <span className="pred-summary-val" style={{ color: '#f97316' }}>{highCount}</span>
            <span className="pred-summary-label">High</span>
          </div>
          <div className="pred-summary-card">
            <span className="pred-summary-val" style={{ color: '#eab308' }}>{predictions.filter(p => p.severity === 'medium').length}</span>
            <span className="pred-summary-label">Medium</span>
          </div>
          <div className="pred-summary-card">
            <span className="pred-summary-val" style={{ color: '#22c55e' }}>{predictions.filter(p => p.severity === 'low').length}</span>
            <span className="pred-summary-label">Low</span>
          </div>
        </div>
      </div>

      <div className="pred-grid">
        {predictions.map(p => (
          <div key={p.id} className="pred-card" style={{ borderLeftColor: severityColors[p.severity] }}>
            <div className="pred-card-top">
              <SeverityBadge severity={p.severity} />
              <span className={`pred-trend pred-trend-${p.trend}`}>
                {p.trend === 'increasing' ? '▲' : p.trend === 'decreasing' ? '▼' : '■'}
              </span>
            </div>
            <h3 className="pred-card-title">{p.title}</h3>
            <p className="pred-card-desc">{p.description}</p>

            <div className="pred-meta-row">
              <ConfidenceBar value={p.confidence} />
              <div className="pred-timeline">
                <span className="pred-timeline-icon">◷</span>
                <span>{p.estimatedTimeline}</span>
              </div>
            </div>

            <div className="pred-contributors">
              <span className="pred-contrib-label">Contributors</span>
              {p.contributors.slice(0, 3).map((c, i) => (
                <div key={i} className="pred-contrib-row">
                  <span className="pred-contrib-name">{c.label}</span>
                  <span className="pred-contrib-value">{c.value}</span>
                  {c.impact > 0 && (
                    <div className="pred-contrib-bar-bg">
                      <div className="pred-contrib-bar" style={{ width: `${Math.min(c.impact, 100)}%`, background: severityColors[p.severity] }} />
                    </div>
                  )}
                </div>
              ))}
            </div>

            {p.sparkline.length > 1 && (
              <div className="pred-sparkline">
                <MiniChart data={p.sparkline} color={severityColors[p.severity]} width={250} height={36} />
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
