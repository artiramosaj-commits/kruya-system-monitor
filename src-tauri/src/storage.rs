use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn new(app_dir: &std::path::Path) -> Result<Self, String> {
        std::fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let db_path = app_dir.join("sage_history.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                cpu_usage REAL NOT NULL,
                memory_usage INTEGER NOT NULL,
                memory_used_mb INTEGER NOT NULL,
                memory_total_mb INTEGER NOT NULL,
                disk_usage INTEGER NOT NULL,
                temperature REAL NOT NULL DEFAULT 0,
                battery_percent INTEGER NOT NULL DEFAULT -1,
                uptime_seconds INTEGER NOT NULL,
                total_processes INTEGER NOT NULL,
                severity_score INTEGER NOT NULL DEFAULT 0,
                snapshot_json TEXT NOT NULL,
                analysis_json TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_timestamp ON snapshots(timestamp);

            CREATE TABLE IF NOT EXISTS processes_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                name TEXT NOT NULL,
                cpu_percent REAL NOT NULL,
                memory_mb INTEGER NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id)
            );

            CREATE INDEX IF NOT EXISTS idx_proc_name ON processes_history(name);
            CREATE INDEX IF NOT EXISTS idx_proc_snapshot_id ON processes_history(snapshot_id);"
        ).map_err(|e| e.to_string())?;

        Ok(Storage {
            conn: Mutex::new(conn),
        })
    }

    pub fn save_snapshot(
        &self,
        snap: &crate::models::SystemSnapshot,
        analysis: &Option<crate::models::AnalysisOutput>,
    ) -> Result<i64, String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let snap_json = serde_json::to_string(snap).map_err(|e| e.to_string())?;
        let analysis_json = analysis
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default());

        let severity = analysis
            .as_ref()
            .map(|a| a.system_health.severity_score as i32)
            .unwrap_or(0);

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO snapshots (timestamp, cpu_usage, memory_usage, memory_used_mb, memory_total_mb,
             disk_usage, temperature, battery_percent, uptime_seconds, total_processes, severity_score,
             snapshot_json, analysis_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                snap.timestamp,
                snap.system.cpu_total_usage,
                snap.system.memory_total_usage,
                snap.system.memory_used_mb,
                snap.system.memory_total_mb,
                snap.system.disk_usage_percent,
                snap.system.temperature_celsius,
                snap.system.battery_percent,
                snap.system.uptime_seconds,
                snap.system.total_processes,
                severity,
                snap_json,
                analysis_json,
            ],
        )
        .map_err(|e| e.to_string())?;

        let snapshot_id = tx.last_insert_rowid();

        // Save process data
        for proc in &snap.processes {
            tx.execute(
                "INSERT INTO processes_history (snapshot_id, timestamp, name, cpu_percent, memory_mb)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    snapshot_id,
                    snap.timestamp,
                    proc.name,
                    proc.cpu_percent,
                    proc.memory_mb,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Prune history to last 2000 snapshots to prevent database bloat
        tx.execute(
            "DELETE FROM processes_history WHERE snapshot_id <= (SELECT MAX(id) - 2000 FROM snapshots)",
            [],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "DELETE FROM snapshots WHERE id <= (SELECT MAX(id) - 2000 FROM snapshots)",
            [],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;

        Ok(snapshot_id)
    }

    pub fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<crate::models::SystemSnapshot>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT snapshot_json FROM snapshots ORDER BY id DESC LIMIT ?1")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| e.to_string())?;

        let mut snapshots = Vec::new();
        for row in rows {
            let json = row.map_err(|e| e.to_string())?;
            if let Ok(snap) = serde_json::from_str(&json) {
                snapshots.push(snap);
            }
        }

        Ok(snapshots)
    }

    pub fn get_process_history(
        &self,
        process_name: &str,
        limit: usize,
    ) -> Result<Vec<(String, f32, u64)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, cpu_percent, memory_mb FROM processes_history
                 WHERE name = ?1 ORDER BY id DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![process_name, limit as i64], |row| {
                let ts: String = row.get(0)?;
                let cpu: f32 = row.get(1)?;
                let mem: u64 = row.get(2)?;
                Ok((ts, cpu, mem))
            })
            .map_err(|e| e.to_string())?;

        let mut history = Vec::new();
        for row in rows {
            history.push(row.map_err(|e| e.to_string())?);
        }

        Ok(history)
    }
}
