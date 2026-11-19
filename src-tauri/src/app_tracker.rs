use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Utc;

static LOADED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppUsage {
    pub name: String,
    pub total_seconds: u64,
    pub last_seen: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UsageStore {
    apps: HashMap<String, UsageEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UsageEntry {
    total_seconds: u64,
    last_seen: String,
}

const AWAY_THRESHOLD_SECS: u64 = 300;

pub struct AppTracker {
    apps: HashMap<String, UsageEntry>,
    path: PathBuf,
    prev_foreground: String,
    prev_timestamp: chrono::DateTime<Utc>,
    initialized: bool,
}

impl AppTracker {
    pub fn new(app_dir: &PathBuf) -> Self {
        let path = app_dir.join("app_usage.json");
        let mut tracker = AppTracker {
            apps: HashMap::new(),
            path,
            prev_foreground: String::new(),
            prev_timestamp: Utc::now(),
            initialized: false,
        };
        tracker.load();
        tracker
    }

    fn load(&mut self) {
        if LOADED.load(Ordering::SeqCst) { return; }
        if let Ok(data) = std::fs::read_to_string(&self.path) {
            match serde_json::from_str::<UsageStore>(&data) {
                Ok(store) => { self.apps = store.apps; }
                Err(e) => {
                    eprintln!("[app_tracker] Failed to parse app_usage.json: {}. Data preserved.", e);
                }
            }
        }
        LOADED.store(true, Ordering::SeqCst);
    }

    fn save(&self) {
        let store = UsageStore {
            apps: self.apps.clone(),
        };
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[app_tracker] Failed to create data directory: {}", e);
                return;
            }
        }
        match serde_json::to_string_pretty(&store) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.path, json) {
                    eprintln!("[app_tracker] Failed to write app_usage.json: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[app_tracker] Failed to serialize usage data: {}", e);
            }
        }
    }

    pub fn update(&mut self, foreground_name: &str, idle_seconds: u64) {
        let now = Utc::now();
        let elapsed = (now - self.prev_timestamp).num_seconds().max(0) as u64;

        if !foreground_name.is_empty() && foreground_name != "unknown" && self.initialized && elapsed > 0 {
            if idle_seconds > AWAY_THRESHOLD_SECS {
                let entry = self.apps.entry("⚡ Away".to_string()).or_insert(UsageEntry {
                    total_seconds: 0,
                    last_seen: String::new(),
                });
                entry.total_seconds += elapsed;
                entry.last_seen = self.prev_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            } else if !self.prev_foreground.is_empty() {
                let entry = self.apps.entry(self.prev_foreground.clone()).or_insert(UsageEntry {
                    total_seconds: 0,
                    last_seen: String::new(),
                });
                entry.total_seconds += elapsed;
                entry.last_seen = self.prev_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }

        if !foreground_name.is_empty() && foreground_name != "unknown" {
            self.prev_foreground = foreground_name.to_string();
        } else {
            self.prev_foreground.clear();
        }
        self.prev_timestamp = now;
        self.initialized = true;
        self.save();
    }

    pub fn get_usage(&self) -> Vec<AppUsage> {
        let mut list: Vec<AppUsage> = self
            .apps
            .iter()
            .map(|(name, entry)| AppUsage {
                name: name.clone(),
                total_seconds: entry.total_seconds,
                last_seen: entry.last_seen.clone(),
            })
            .collect();
        list.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));
        list
    }

    pub fn reset(&mut self, app_name: &str) {
        self.apps.remove(app_name);
        self.save();
    }
}
