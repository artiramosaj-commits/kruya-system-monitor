export interface AppUsage {
  name: string;
  total_seconds: number;
  last_seen: string;
}

export interface GpuInfo {
  name: string;
  memory_mb: number;
  driver_version: string;
  gpu_type: string;
}

export interface NetworkInterface {
  name: string;
  total_received: number;
  total_transmitted: number;
}

export interface DisplayInfo {
  name: string;
  resolution_width: number;
  resolution_height: number;
  refresh_rate: number;
  is_primary: boolean;
}

export interface SystemInfo {
  os_name: string;
  os_version: string;
  kernel_version: string;
  hostname: string;
  cpu_brand: string;
  cpu_cores: number;
  total_ram_mb: number;
  cpu_physical_cores: number;
  cpu_max_frequency_mhz: number;
  cpu_architecture: string;
  system_manufacturer: string;
  system_model: string;
  system_serial: string;
  system_sku: string;
  bios_vendor: string;
  bios_version: string;
  bios_date: string;
  motherboard_manufacturer: string;
  motherboard_model: string;
  ram_speed_mhz: number;
  ram_form_factor: string;
  ram_slots_used: number;
  ram_slots_total: number;
  os_edition: string;
  os_build: string;
  os_install_date: string;
  os_architecture: string;
}

export interface DriveInfo {
  mount_point: string;
  total_gb: number;
  used_gb: number;
  usage_percent: number;
  filesystem_type: string;
  is_removable: boolean;
}

export interface SystemSnapshot {
  timestamp: string;
  system: SystemMetrics;
  processes: ProcessInfo[];
  foreground: ForegroundInfo;
  drives: DriveInfo[];
  gpu: GpuInfo;
  interfaces: NetworkInterface[];
  sys_info: SystemInfo;
  displays: DisplayInfo[];
}

export interface SystemMetrics {
  cpu_total_usage: number;
  per_core_usage: number[];
  cpu_frequency_mhz: number[];
  cpu_brand: string;
  memory_total_mb: number;
  memory_used_mb: number;
  memory_available_mb: number;
  memory_total_usage: number;
  swap_total_mb: number;
  swap_used_mb: number;
  disk_total_gb: number;
  disk_used_gb: number;
  disk_usage_percent: number;
  disk_read_bytes: number;
  disk_write_bytes: number;
  network_down_bytes: number;
  network_up_bytes: number;
  temperature_celsius: number;
  battery_percent: number;
  ac_line_status: number;
  uptime_seconds: number;
  total_processes: number;
  running_processes: number;
}

export interface ProcessInfo {
  name: string;
  cpu_percent: number;
  memory_mb: number;
  threads: number;
  pid: number;
  runtime_seconds: number;
  cmd: string;
  path: string;
}

export interface ForegroundInfo {
  name: string;
  title: string;
  pid: number;
}

export interface AnalysisOutput {
  summary: string;
  system_health: HealthStatus;
  performance: string[];
  productivity: string[];
  anomalies: Anomaly[];
  recommendations: string[];
  observations: string[];
  top_processes: TopProcess[];
  focus_analysis: FocusAnalysis;
  resource_analysis: ResourceAnalysis;
  trend_analysis: TrendAnalysis;
  alerts: Alert[];
}

export interface HealthStatus {
  status: string;
  severity_score: number;
}

export interface Anomaly {
  anomaly_type: string;
  severity: string;
  detail: string;
}

export interface TopProcess {
  name: string;
  cpu_percent: number;
  memory_mb: number;
  threads: number;
  assessment: string;
}

export interface FocusAnalysis {
  primary_app: string;
  assessment: string;
}

export interface ResourceAnalysis {
  cpu: number;
  memory: number;
  bottleneck: string;
  assessment: string;
}

export interface TrendAnalysis {
  cpu_trend: string;
  memory_trend: string;
  assessment: string;
}

export interface Alert {
  alert_type: string;
  severity: string;
  message: string;
}

export interface SnapshotResult {
  snapshot: SystemSnapshot;
  analysis: AnalysisOutput;
}

export interface ProcessHistoryPoint {
  timestamp: string;
  cpu_percent: number;
  memory_mb: number;
}

export interface StartupProgram {
  name: string;
  command: string;
  location: string;
  enabled: boolean;
}

export interface BatteryInfo {
  design_capacity_mwh: number;
  full_charge_capacity_mwh: number;
  cycle_count: number;
  wear_level_pct: number;
}

export interface ReportStats {
  avg_cpu: number;
  max_cpu: number;
  avg_mem: number;
  max_mem: number;
  avg_temp: number;
  max_temp: number;
  samples: number;
}
