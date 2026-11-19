export interface HistoryPoint {
  cpu: number; mem: number; disk: number;
  diskRead: number; diskWrite: number;
  netDown: number; netUp: number;
  swapPct: number; battery: number; temperature: number;
  ts: string;
}

export interface Prediction {
  id: string;
  type: 'ram_saturation' | 'cpu_pressure' | 'thermal' | 'memory_leak' | 'background_bloat' | 'process_acceleration' | 'responsiveness_risk' | 'workflow_instability';
  title: string;
  description: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  confidence: number;
  estimatedTimeline: string;
  contributors: { label: string; value: string; impact: number }[];
  trend: 'increasing' | 'stable' | 'decreasing';
  metric: number;
  threshold: number;
  sparkline: number[];
}

interface ProcessSummary {
  name: string;
  cpu_percent: number;
  memory_mb: number;
}

function linreg(values: number[]): { slope: number; intercept: number } {
  const n = values.length;
  if (n < 2) return { slope: 0, intercept: values[0] || 0 };
  let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0;
  for (let i = 0; i < n; i++) {
    sumX += i; sumY += values[i];
    sumXY += i * values[i]; sumX2 += i * i;
  }
  const denom = n * sumX2 - sumX * sumX;
  if (denom === 0) return { slope: 0, intercept: sumY / n };
  const slope = (n * sumXY - sumX * sumY) / denom;
  const intercept = (sumY - slope * sumX) / n;
  return { slope, intercept };
}

function rollingAvg(values: number[], window: number): number[] {
  const result: number[] = [];
  for (let i = 0; i < values.length; i++) {
    const start = Math.max(0, i - window + 1);
    const slice = values.slice(start, i + 1);
    result.push(slice.reduce((a, b) => a + b, 0) / slice.length);
  }
  return result;
}

function timeToThreshold(values: number[], threshold: number): number | null {
  if (values.length < 3) return null;
  const recent = values.slice(-5);
  const { slope } = linreg(recent);
  if (slope <= 0) return null;
  const current = values[values.length - 1];
  const stepsToHit = (threshold - current) / slope;
  if (stepsToHit <= 0 || !isFinite(stepsToHit)) return null;
  return stepsToHit;
}

function confidenceFromR2(values: number[]): number {
  if (values.length < 3) return 20;
  const { slope } = linreg(values);
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  let ssRes = 0, ssTot = 0;
  for (let i = 0; i < values.length; i++) {
    const predicted = slope * i + (mean - slope * (values.length - 1) / 2);
    ssRes += (values[i] - predicted) ** 2;
    ssTot += (values[i] - mean) ** 2;
  }
  const r2 = ssTot === 0 ? 0 : 1 - ssRes / ssTot;
  return Math.min(95, Math.max(15, Math.round(r2 * 100)));
}

function volatility(values: number[]): number {
  if (values.length < 2) return 0;
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const variance = values.reduce((s, v) => s + (v - mean) ** 2, 0) / values.length;
  return Math.sqrt(variance) / (mean || 1);
}

function formatTimeline(minutes: number): string {
  if (minutes < 1) return '< 1 minute';
  if (minutes < 60) return `~${Math.round(minutes)} minute${Math.round(minutes) === 1 ? '' : 's'}`;
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return m > 0 ? `~${h}h ${m}m` : `~${h} hour${h === 1 ? '' : 's'}`;
}

function severityForConfidence(conf: number): 'critical' | 'high' | 'medium' | 'low' {
  if (conf >= 75) return 'critical';
  if (conf >= 55) return 'high';
  if (conf >= 35) return 'medium';
  return 'low';
}

export function analyzePredictions(
  history: HistoryPoint[],
  processes: ProcessSummary[],
  system: { memory_total_mb: number; swap_total_mb: number; cpu_cores: number }
): Prediction[] {
  const predictions: Prediction[] = [];
  if (history.length < 3) return predictions;

  const memValues = history.map(h => h.mem);
  const cpuValues = history.map(h => h.cpu);
  const tempValues = history.map(h => h.temperature).filter(t => t > 0);
  const cpuAvg = rollingAvg(cpuValues, 3);

  // 1. RAM Saturation
  if (memValues.length >= 3) {
    const recentMem = memValues.slice(-10);
    const memSlope = linreg(recentMem).slope;
    const stepsTo90 = timeToThreshold(memValues, 90);
    const conf = confidenceFromR2(recentMem);
    const currentMem = memValues[memValues.length - 1];
    if (memSlope > 0.1 && currentMem > 50) {
      predictions.push({
        id: 'ram-saturation',
        type: 'ram_saturation',
        title: 'RAM Saturation Risk',
        description: stepsTo90
          ? `Memory usage trending ${memSlope > 0.5 ? 'aggressively' : 'steadily'} upward. ${stepsTo90 <= 30 ? 'Saturation imminent.' : `Estimated to reach critical levels.`}`
          : `Memory at ${currentMem.toFixed(0)}% with gradual upward trend.`,
        severity: stepsTo90 !== null && stepsTo90 < 15 ? 'critical' : severityForConfidence(conf),
        confidence: conf,
        estimatedTimeline: stepsTo90 ? formatTimeline(stepsTo90 * 0.3) : 'Beyond forecast window',
        contributors: [
          { label: 'Current Usage', value: `${currentMem.toFixed(0)}%`, impact: currentMem },
          { label: 'Growth Rate', value: `${memSlope.toFixed(2)}%/step`, impact: Math.min(100, memSlope * 20) },
          { label: 'Total RAM', value: `${(system.memory_total_mb / 1024).toFixed(0)} GB`, impact: 0 },
        ],
        trend: memSlope > 0.2 ? 'increasing' : memSlope > 0.05 ? 'increasing' : 'stable',
        metric: currentMem,
        threshold: 90,
        sparkline: recentMem.slice(-15),
      });
    }
  }

  // 2. Sustained CPU Pressure
  if (cpuValues.length >= 5) {
    const recentCpu = cpuValues.slice(-8);
    const cpuSlope = linreg(recentCpu).slope;
    const avgCpu = cpuAvg[cpuAvg.length - 1];
    const sustainedHigh = cpuValues.slice(-6).filter(c => c > 65).length;
    const conf = confidenceFromR2(recentCpu);
    if (sustainedHigh >= 3 || (avgCpu > 60 && cpuSlope > 0)) {
      const stepsTo100 = timeToThreshold(cpuValues, 95);
      predictions.push({
        id: 'cpu-pressure',
        type: 'cpu_pressure',
        title: 'Sustained CPU Pressure',
        description: sustainedHigh >= 4
          ? `CPU consistently above 65% for extended period. ${system.cpu_cores > 4 ? 'Multi-core saturation likely.' : 'System responsiveness may degrade.'}`
          : `CPU trending upward at ${cpuSlope.toFixed(2)}%/step. ${avgCpu > 75 ? 'High load period detected.' : 'Moderate pressure building.'}`,
        severity: avgCpu > 75 ? 'high' : severityForConfidence(conf),
        confidence: conf,
        estimatedTimeline: stepsTo100 ? formatTimeline(stepsTo100 * 0.3) : 'Stable range',
        contributors: [
          { label: 'Current Load', value: `${avgCpu.toFixed(0)}%`, impact: avgCpu },
          { label: 'Peak (recent)', value: `${Math.max(...recentCpu).toFixed(0)}%`, impact: Math.max(...recentCpu) },
          { label: 'Sustained Spikes', value: `${sustainedHigh}/6 windows`, impact: Math.min(100, sustainedHigh * 20) },
        ],
        trend: cpuSlope > 0.1 ? 'increasing' : cpuSlope < -0.1 ? 'decreasing' : 'stable',
        metric: avgCpu,
        threshold: 75,
        sparkline: recentCpu.slice(-15),
      });
    }
  }

  // 3. Thermal Pressure
  if (tempValues.length >= 3) {
    const recentTemp = tempValues.slice(-8);
    const tempSlope = linreg(recentTemp).slope;
    const currentTemp = tempValues[tempValues.length - 1];
    const conf = confidenceFromR2(recentTemp);
    if (currentTemp > 60 || tempSlope > 0.3) {
      const stepsTo90 = timeToThreshold(tempValues, 90);
      predictions.push({
        id: 'thermal-pressure',
        type: 'thermal',
        title: 'Thermal Pressure Warning',
        description: tempSlope > 0.5
          ? `Temperature rising rapidly (${tempSlope.toFixed(1)}°C/step). Possible cooling limitation.`
          : currentTemp > 75
            ? `Temperature at ${currentTemp.toFixed(0)}°C — near thermal throttle threshold.`
            : `Temperature at ${currentTemp.toFixed(0)}°C with a ${tempSlope > 0 ? 'rising' : 'stable'} trend.`,
        severity: currentTemp > 75 ? 'critical' : currentTemp > 65 ? 'high' : 'medium',
        confidence: Math.min(90, conf + 10),
        estimatedTimeline: stepsTo90 ? formatTimeline(stepsTo90 * 0.3) : 'Stable',
        contributors: [
          { label: 'Current Temp', value: `${currentTemp.toFixed(0)}°C`, impact: currentTemp },
          { label: 'Rate of Change', value: `${tempSlope.toFixed(1)}°C/step`, impact: Math.min(100, tempSlope * 20) },
          { label: 'Max Recent', value: `${Math.max(...recentTemp).toFixed(0)}°C`, impact: Math.max(...recentTemp) },
        ],
        trend: tempSlope > 0.1 ? 'increasing' : tempSlope < -0.1 ? 'decreasing' : 'stable',
        metric: currentTemp,
        threshold: 80,
        sparkline: recentTemp.slice(-15),
      });
    }
  }

  // 4. Memory Leak Pattern
  if (memValues.length >= 10) {
    const recentMem = memValues.slice(-15);
    const memSlope = linreg(recentMem).slope;
    const vol = volatility(recentMem);
    const monotonicallyIncreasing = recentMem.slice(-8).every((v, i, a) => i === 0 || v >= a[i - 1]);
    const conf = confidenceFromR2(recentMem);
    if ((memSlope > 0.15 && vol < 0.08) || monotonicallyIncreasing) {
      predictions.push({
        id: 'memory-leak',
        type: 'memory_leak',
        title: 'Memory Leak Pattern Detected',
        description: monotonicallyIncreasing
          ? `Memory has increased monotonically for ${recentMem.slice(-8).length} consecutive polls — consistent with a memory leak.`
          : `Sustained linear memory growth (${memSlope.toFixed(2)}%/step) with low volatility (${(vol * 100).toFixed(0)}%) — leak-like pattern.`,
        severity: memSlope > 0.3 ? 'critical' : 'high',
        confidence: monotonicallyIncreasing ? Math.min(95, conf + 15) : conf,
        estimatedTimeline: memSlope > 0.2 ? formatTimeline(((90 - memValues[memValues.length - 1]) / memSlope) * 0.3) : 'Gradual',
        contributors: [
          { label: 'Growth Rate', value: `${memSlope.toFixed(2)}%/step`, impact: Math.min(100, memSlope * 30) },
          { label: 'Pattern', value: monotonicallyIncreasing ? 'Monotonic' : 'Linear', impact: monotonicallyIncreasing ? 80 : 50 },
          { label: 'Confidence', value: `${conf}%`, impact: conf },
        ],
        trend: 'increasing',
        metric: memValues[memValues.length - 1],
        threshold: 85,
        sparkline: recentMem.slice(-15),
      });
    }
  }

  // 5. Background Bloat
  if (processes.length > 0) {
    const bgProcesses = processes.filter(p =>
      !['explorer.exe', 'kruya.exe', 'svchost.exe'].includes(p.name.toLowerCase())
    );
    const bgMem = bgProcesses.reduce((s, p) => s + p.memory_mb, 0);
    const largeProcesses = bgProcesses.filter(p => p.memory_mb > 200);
    const conf = Math.min(80, Math.round((bgProcesses.length / Math.max(processes.length, 1)) * 100));
    if (bgProcesses.length > 20 || bgMem > system.memory_total_mb * 0.3) {
      predictions.push({
        id: 'background-bloat',
        type: 'background_bloat',
        title: 'Background Resource Bloat',
        description: bgProcesses.length > 40
          ? `${bgProcesses.length} background processes consuming ${(bgMem / 1024).toFixed(1)} GB — excessive background activity.`
          : `${bgProcesses.length} background processes using ${(bgMem / 1024).toFixed(1)} GB memory. ${largeProcesses.length} process(es) over 200 MB each.`,
        severity: bgMem > system.memory_total_mb * 0.5 ? 'critical' : bgMem > system.memory_total_mb * 0.3 ? 'high' : 'medium',
        confidence: conf,
        estimatedTimeline: 'Ongoing',
        contributors: [
          { label: 'Background Processes', value: `${bgProcesses.length}`, impact: Math.min(100, bgProcesses.length * 2) },
          { label: 'Background Memory', value: `${(bgMem / 1024).toFixed(1)} GB`, impact: Math.min(100, (bgMem / system.memory_total_mb) * 100) },
          { label: 'Large Consumers', value: `${largeProcesses.length}`, impact: Math.min(100, largeProcesses.length * 15) },
        ],
        trend: 'increasing',
        metric: bgMem / system.memory_total_mb * 100,
        threshold: 30,
        sparkline: history.slice(-10).map(h => h.mem),
      });
    }
  }

  // 6. Process Resource Acceleration
  if (processes.length > 0 && history.length >= 3) {
    const topByMem = [...processes].sort((a, b) => b.memory_mb - a.memory_mb).slice(0, 5);
    const memGrowth = memValues.length >= 2 ? memValues[memValues.length - 1] - memValues[memValues.length - 2] : 0;
    const bigProcesses = topByMem.filter(p => p.memory_mb > 300);
    if (bigProcesses.length > 0 && memGrowth > 1) {
      predictions.push({
        id: 'process-acceleration',
        type: 'process_acceleration',
        title: 'Process Resource Acceleration',
        description: `${bigProcesses[0].name} and ${bigProcesses.length - 1} other process(es) accelerating resource consumption. Combined: ${(bigProcesses.reduce((s, p) => s + p.memory_mb, 0) / 1024).toFixed(1)} GB.`,
        severity: bigProcesses.length >= 3 ? 'high' : 'medium',
        confidence: Math.min(75, confidenceFromR2(memValues) + 10),
        estimatedTimeline: 'Variable',
        contributors: bigProcesses.slice(0, 3).map(p => ({
          label: p.name.length > 20 ? p.name.slice(0, 20) + '…' : p.name,
          value: `${p.memory_mb} MB`,
          impact: Math.min(100, (p.memory_mb / system.memory_total_mb) * 100),
        })),
        trend: memGrowth > 0 ? 'increasing' : 'stable',
        metric: bigProcesses.reduce((s, p) => s + p.memory_mb, 0) / system.memory_total_mb * 100,
        threshold: 40,
        sparkline: memValues.slice(-15),
      });
    }
  }

  // 7. Responsiveness Degradation Risk
  {
    const currentMem = memValues[memValues.length - 1] || 0;
    const currentCpu = cpuValues[cpuValues.length - 1] || 0;
    const swapRatio = history.length > 0 ? history[history.length - 1].swapPct : 0;
    const memPressure = currentMem / 90;
    const cpuPressure = currentCpu / 80;
    const swapPressure = swapRatio / 50;
    const riskScore = Math.min(100, Math.round((memPressure * 0.45 + cpuPressure * 0.35 + swapPressure * 0.2) * 100));
    if (riskScore > 25) {
      predictions.push({
        id: 'responsiveness-risk',
        type: 'responsiveness_risk',
        title: 'Responsiveness Degradation Risk',
        description: riskScore > 70
          ? `System responsiveness at high risk. Memory (${currentMem.toFixed(0)}%), CPU (${currentCpu.toFixed(0)}%), and swap (${swapRatio.toFixed(0)}%) all elevated.`
          : riskScore > 50
            ? `Degradation risk detected: Memory ${currentMem.toFixed(0)}% | CPU ${currentCpu.toFixed(0)}% | Swap ${swapRatio.toFixed(0)}%`
            : `Early signs of responsiveness pressure: memory at ${currentMem.toFixed(0)}%, CPU at ${currentCpu.toFixed(0)}%.`,
        severity: riskScore > 70 ? 'critical' : riskScore > 50 ? 'high' : 'medium',
        confidence: Math.min(85, 50 + riskScore * 0.4),
        estimatedTimeline: riskScore > 60 ? 'Imminent' : 'Gradual',
        contributors: [
          { label: 'Memory Pressure', value: `${currentMem.toFixed(0)}%`, impact: currentMem },
          { label: 'CPU Pressure', value: `${currentCpu.toFixed(0)}%`, impact: currentCpu },
          { label: 'Swap Pressure', value: `${swapRatio.toFixed(0)}%`, impact: swapRatio },
        ],
        trend: riskScore > 60 ? 'increasing' : memPressure > cpuPressure ? 'increasing' : 'stable',
        metric: riskScore,
        threshold: 50,
        sparkline: history.slice(-15).map(h => h.cpu * 0.35 + h.mem * 0.45 + (h.swapPct / 50 * 20)),
      });
    }
  }

  // 8. Workflow Instability (inferred from resource volatility)
  if (history.length >= 8) {
    const recentCpu = cpuValues.slice(-8);
    const recentMem = memValues.slice(-8);
    const cpuVol = volatility(recentCpu);
    const memVol = volatility(recentMem);
    const instabilityScore = Math.min(100, Math.round((cpuVol * 40 + memVol * 60) * 100));
    if (instabilityScore > 30) {
      predictions.push({
        id: 'workflow-instability',
        type: 'workflow_instability',
        title: 'Workflow Instability Detected',
        description: instabilityScore > 70
          ? `High resource volatility (${instabilityScore}%) — frequent context switching or competing processes impacting stability.`
          : `Moderate workflow instability (${instabilityScore}%) — resource demand fluctuating. May cause perceived lag during app switches.`,
        severity: instabilityScore > 70 ? 'high' : 'medium',
        confidence: Math.min(80, 40 + instabilityScore * 0.4),
        estimatedTimeline: 'Current session',
        contributors: [
          { label: 'CPU Volatility', value: `${(cpuVol * 100).toFixed(0)}%`, impact: Math.min(100, cpuVol * 100) },
          { label: 'Memory Volatility', value: `${(memVol * 100).toFixed(0)}%`, impact: Math.min(100, memVol * 100) },
          { label: 'Instability Score', value: `${instabilityScore}%`, impact: instabilityScore },
        ],
        trend: instabilityScore > 60 ? 'increasing' : 'stable',
        metric: instabilityScore,
        threshold: 50,
        sparkline: history.slice(-15).map((h, i, a) => {
          if (i < 2) return 0;
          return Math.abs(h.cpu - a[i - 1].cpu) + Math.abs(h.mem - a[i - 1].mem) * 0.5;
        }),
      });
    }
  }

  return predictions.sort((a, b) => {
    const order = { critical: 0, high: 1, medium: 2, low: 3 };
    return order[a.severity] - order[b.severity];
  });
}

export function computeHealthScore(predictions: Prediction[]): number {
  if (predictions.length === 0) return 95;
  const penalties: Record<string, number> = {
    critical: 30, high: 15, medium: 7, low: 2,
  };
  let score = 100;
  for (const p of predictions) {
    score -= penalties[p.severity] * (p.confidence / 100);
  }
  return Math.max(0, Math.round(score));
}
