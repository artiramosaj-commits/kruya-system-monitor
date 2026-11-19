use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: String,
    pub system: SystemMetrics,
    pub processes: Vec<ProcessInfo>,
    pub foreground: ForegroundInfo,
    pub drives: Vec<DriveInfo>,
    pub gpu: GpuInfo,
    pub interfaces: Vec<NetworkInterface>,
    pub sys_info: SystemInfo,
    pub displays: Vec<DisplayInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub memory_mb: u64,
    pub driver_version: String,
    pub gpu_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub total_received: u64,
    pub total_transmitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub name: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub refresh_rate: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    // Existing
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub cpu_brand: String,
    pub cpu_cores: u32,
    pub total_ram_mb: u64,

    // CPU
    pub cpu_physical_cores: u32,
    pub cpu_max_frequency_mhz: u64,
    pub cpu_architecture: String,

    // System
    pub system_manufacturer: String,
    pub system_model: String,
    pub system_serial: String,
    pub system_sku: String,

    // BIOS
    pub bios_vendor: String,
    pub bios_version: String,
    pub bios_date: String,

    // Motherboard
    pub motherboard_manufacturer: String,
    pub motherboard_model: String,

    // RAM
    pub ram_speed_mhz: u32,
    pub ram_form_factor: String,
    pub ram_slots_used: u32,
    pub ram_slots_total: u32,

    // OS
    pub os_edition: String,
    pub os_build: String,
    pub os_install_date: String,
    pub os_architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub mount_point: String,
    pub total_gb: u64,
    pub used_gb: u64,
    pub usage_percent: u8,
    pub filesystem_type: String,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_total_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub cpu_frequency_mhz: Vec<u64>,
    pub cpu_brand: String,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_available_mb: u64,
    pub memory_total_usage: u8,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub disk_total_gb: u64,
    pub disk_used_gb: u64,
    pub disk_usage_percent: u8,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_down_bytes: u64,
    pub network_up_bytes: u64,
    pub temperature_celsius: f32,
    pub battery_percent: i8,
    pub ac_line_status: i8,
    pub uptime_seconds: u64,
    pub total_processes: u32,
    pub running_processes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub threads: u32,
    pub pid: u32,
    pub runtime_seconds: u64,
    pub cmd: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForegroundInfo {
    pub name: String,
    pub title: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOutput {
    pub summary: String,
    pub system_health: HealthStatus,
    pub performance: Vec<String>,
    pub productivity: Vec<String>,
    pub anomalies: Vec<Anomaly>,
    pub recommendations: Vec<String>,
    pub observations: Vec<String>,
    pub top_processes: Vec<TopProcess>,
    pub focus_analysis: FocusAnalysis,
    pub resource_analysis: ResourceAnalysis,
    pub trend_analysis: TrendAnalysis,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub severity_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub anomaly_type: String,
    pub severity: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProcess {
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub threads: u32,
    pub assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusAnalysis {
    pub primary_app: String,
    pub assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysis {
    pub cpu: f32,
    pub memory: u8,
    pub bottleneck: String,
    pub assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub cpu_trend: String,
    pub memory_trend: String,
    pub assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupProgram {
    pub name: String,
    pub command: String,
    pub location: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub design_capacity_mwh: u32,
    pub full_charge_capacity_mwh: u32,
    pub cycle_count: u32,
    pub wear_level_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStats {
    pub avg_cpu: f32,
    pub max_cpu: f32,
    pub avg_mem: u8,
    pub max_mem: u8,
    pub avg_temp: f32,
    pub max_temp: f32,
    pub samples: usize,
}
