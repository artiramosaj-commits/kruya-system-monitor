mod analytics;
mod app_tracker;
mod foreground;
mod models;
mod report;
mod storage;
mod telemetry;

use analytics::AnalyticsEngine;
use app_tracker::AppTracker;
use models::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use storage::Storage;
use tauri::{Manager, State};
use telemetry::TelemetryCollector;

pub struct AppState {
    telemetry: Mutex<TelemetryCollector>,
    analytics: Mutex<AnalyticsEngine>,
    storage: Mutex<Storage>,
    app_tracker: Mutex<AppTracker>,
}

#[tauri::command]
fn get_snapshot(state: State<AppState>) -> Result<SnapshotResult, String> {
    let mut tel = state.telemetry.lock().map_err(|e| e.to_string())?;
    let mut ana = state.analytics.lock().map_err(|e| e.to_string())?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut tracker = state.app_tracker.lock().map_err(|e| e.to_string())?;

    let snap = tel.collect_snapshot();
    let idle_secs = foreground::get_idle_seconds();
    tracker.update(&snap.foreground.name, idle_secs);

    let analysis = ana.analyze(&snap);
    ana.add_snapshot(snap.clone());

    storage
        .save_snapshot(&snap, &Some(analysis.clone()))
        .map_err(|e| format!("Storage error: {}", e))?;

    Ok(SnapshotResult {
        snapshot: snap,
        analysis,
    })
}

#[tauri::command]
fn get_app_usage(state: State<AppState>) -> Result<Vec<app_tracker::AppUsage>, String> {
    let tracker = state.app_tracker.lock().map_err(|e| e.to_string())?;
    Ok(tracker.get_usage())
}

#[tauri::command]
fn get_startup_programs() -> Result<Vec<StartupProgram>, String> {
    Ok(telemetry::get_startup_programs())
}

#[tauri::command]
fn get_battery_info_ext() -> Result<Option<BatteryInfo>, String> {
    Ok(telemetry::get_battery_info())
}

#[tauri::command]
fn export_report(path: String, snapshot: SystemSnapshot, analysis: AnalysisOutput, app_usage: Vec<app_tracker::AppUsage>, startup_programs: Vec<StartupProgram>, stats: ReportStats) -> Result<(), String> {
    report::generate_pdf(&path, &snapshot, &analysis, &app_usage, &startup_programs, &stats)
}

#[tauri::command]
fn reset_app_usage(state: State<AppState>, app_name: String) -> Result<(), String> {
    let mut tracker = state.app_tracker.lock().map_err(|e| e.to_string())?;
    tracker.reset(&app_name);
    Ok(())
}

#[tauri::command]
fn get_history(state: State<AppState>, limit: usize) -> Result<Vec<SystemSnapshot>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_recent_snapshots(limit)
}

#[tauri::command]
fn get_process_history(
    state: State<AppState>,
    name: String,
    limit: usize,
) -> Result<Vec<ProcessHistoryPoint>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let history = storage.get_process_history(&name, limit)?;
    Ok(history
        .into_iter()
        .map(|(ts, cpu, mem)| ProcessHistoryPoint {
            timestamp: ts,
            cpu_percent: cpu,
            memory_mb: mem,
        })
        .collect())
}

#[tauri::command]
fn export_csv(state: State<AppState>, limit: usize) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let snapshots = storage.get_recent_snapshots(limit)?;

    let mut csv = String::from(
        "timestamp,cpu_usage,memory_percent,memory_used_mb,memory_total_mb,\
         disk_percent,uptime_seconds,total_processes,battery_percent\n",
    );

    for snap in snapshots.iter().rev() {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            snap.timestamp,
            snap.system.cpu_total_usage,
            snap.system.memory_total_usage,
            snap.system.memory_used_mb,
            snap.system.memory_total_mb,
            snap.system.disk_usage_percent,
            snap.system.uptime_seconds,
            snap.system.total_processes,
            snap.system.battery_percent,
        ));
    }

    Ok(csv)
}

#[tauri::command]
fn export_json(state: State<AppState>, limit: usize) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let snapshots = storage.get_recent_snapshots(limit)?;
    serde_json::to_string_pretty(&snapshots).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_start(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        let path_str = exe_path.to_string_lossy();
        if enabled {
            let output = std::process::Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "KruYa",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &path_str,
                    "/f",
                ])
                .output()
                .map_err(|e| format!("Failed to set auto-start: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to set auto-start: {}", stderr));
            }
        } else {
            let output = std::process::Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "KruYa",
                    "/f",
                ])
                .output()
                .map_err(|e| format!("Failed to remove auto-start: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to remove auto-start: {}", stderr));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let config_dir = dirs_next::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("autostart");
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create autostart dir: {}", e))?;

        let desktop_file = config_dir.join("kruya.desktop");
        if enabled {
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=KruYa\nExec={}\nTerminal=false\nCategories=System;\n",
                exe_path.display()
            );
            std::fs::write(&desktop_file, content)
                .map_err(|e| format!("Failed to write autostart file: {}", e))?;
        } else if desktop_file.exists() {
            std::fs::remove_file(&desktop_file)
                .map_err(|e| format!("Failed to remove autostart file: {}", e))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        let launch_agents = dirs_next::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Library")
            .join("LaunchAgents");
        std::fs::create_dir_all(&launch_agents)
            .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;

        let plist_file = launch_agents.join("com.kruya.app.plist");
        if enabled {
            let content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.kruya.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
                exe_path.display()
            );
            std::fs::write(&plist_file, content)
                .map_err(|e| format!("Failed to write LaunchAgent plist: {}", e))?;
        } else if plist_file.exists() {
            std::fs::remove_file(&plist_file)
                .map_err(|e| format!("Failed to remove LaunchAgent plist: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| format!("Failed to execute taskkill: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to kill process {}: {}", pid, stderr));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map_err(|e| format!("Failed to execute kill: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to kill process {}: {}", pid, stderr));
        }
    }
    Ok(())
}

static STRESS_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn start_cpu_stress(cores: usize, duration_secs: u32) -> Result<(), String> {
    STRESS_ACTIVE.store(true, Ordering::SeqCst);
    
    let available_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);
    let cores_to_run = std::cmp::min(cores, available_cores);

    for _ in 0..cores_to_run {
        std::thread::spawn(|| {
            while STRESS_ACTIVE.load(Ordering::SeqCst) {
                // Keep the CPU core busy
                let mut _x = 0;
                for i in 0..1000 {
                    _x += i;
                }
            }
        });
    }

    // Auto-stop thread
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(duration_secs as u64));
        STRESS_ACTIVE.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
fn stop_cpu_stress() -> Result<(), String> {
    STRESS_ACTIVE.store(false, Ordering::SeqCst);
    Ok(())
}

#[derive(serde::Serialize)]
pub struct SnapshotResult {
    pub snapshot: SystemSnapshot,
    pub analysis: AnalysisOutput,
}

#[derive(serde::Serialize)]
pub struct ProcessHistoryPoint {
    pub timestamp: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kruya");

    let storage = Storage::new(&app_dir).expect("Failed to initialize storage");

    tauri::Builder::default()
        .manage(AppState {
            telemetry: Mutex::new(TelemetryCollector::new()),
            analytics: Mutex::new(AnalyticsEngine::new()),
            storage: Mutex::new(storage),
            app_tracker: Mutex::new(AppTracker::new(&app_dir)),
        })
        .setup(|app| {
            app.handle().plugin(tauri_plugin_dialog::init())?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // System tray
            let show = tauri::menu::MenuItem::with_id(app, "show", "Show KruYa", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;

            let tray = tauri::tray::TrayIconBuilder::new()
                .tooltip("KruYa System Monitor")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                });
            let tray = if let Some(icon) = app.default_window_icon().cloned() {
                tray.icon(icon)
            } else {
                tray
            };
            let _tray = tray.build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_history,
            get_process_history,
            export_csv,
            export_json,
            export_report,
            kill_process,
            set_auto_start,
            start_cpu_stress,
            stop_cpu_stress,
            get_app_usage,
            reset_app_usage,
            get_startup_programs,
            get_battery_info_ext,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

