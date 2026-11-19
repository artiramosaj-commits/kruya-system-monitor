import { invoke } from '@tauri-apps/api/core';
import type { SnapshotResult, SystemSnapshot, ProcessHistoryPoint, AppUsage, StartupProgram, BatteryInfo } from './types';

export async function getSnapshot(): Promise<SnapshotResult> {
  return invoke<SnapshotResult>('get_snapshot');
}

export async function getHistory(limit: number = 60): Promise<SystemSnapshot[]> {
  return invoke<SystemSnapshot[]>('get_history', { limit });
}

export async function getProcessHistory(
  name: string,
  limit: number = 60
): Promise<ProcessHistoryPoint[]> {
  return invoke<ProcessHistoryPoint[]>('get_process_history', { name, limit });
}

export async function exportCsv(limit: number = 60): Promise<string> {
  return invoke<string>('export_csv', { limit });
}

export async function killProcess(pid: number): Promise<void> {
  return invoke<void>('kill_process', { pid });
}

export async function setAutoStart(enabled: boolean): Promise<void> {
  return invoke<void>('set_auto_start', { enabled });
}

export async function exportJson(limit: number = 60): Promise<string> {
  return invoke<string>('export_json', { limit });
}

export async function startCpuStress(cores: number, durationSecs: number): Promise<void> {
  return invoke<void>('start_cpu_stress', { cores, durationSecs });
}

export async function stopCpuStress(): Promise<void> {
  return invoke<void>('stop_cpu_stress');
}

export async function getAppUsage(): Promise<AppUsage[]> {
  return invoke<AppUsage[]>('get_app_usage');
}

export async function resetAppUsage(appName: string): Promise<void> {
  return invoke<void>('reset_app_usage', { appName });
}

export async function getStartupPrograms(): Promise<StartupProgram[]> {
  return invoke<StartupProgram[]>('get_startup_programs');
}

export async function getBatteryInfoExt(): Promise<BatteryInfo | null> {
  return invoke<BatteryInfo | null>('get_battery_info_ext');
}

export async function exportReport(
  path: string,
  snapshot: SystemSnapshot,
  analysis: SnapshotResult['analysis'],
  appUsage: AppUsage[],
  startupPrograms: StartupProgram[],
  stats: { avg_cpu: number; max_cpu: number; avg_mem: number; max_mem: number; avg_temp: number; max_temp: number; samples: number }
): Promise<void> {
  return invoke<void>('export_report', { path, snapshot, analysis, appUsage, startupPrograms, stats });
}

