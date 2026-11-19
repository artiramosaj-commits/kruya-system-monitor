# Feature map

This is a source review of the current tree. “Implemented” means the UI has a corresponding Tauri command or local implementation in source; native runtime behavior was not verified because Windows Application Control blocked the generated Cargo build script during cargo test.

| Area | Status | Source-grounded notes |
|---|---|---|
| CPU, memory, storage, network counters | Implemented in source | Rust collects system counters and the UI refreshes snapshots. Network values are byte counters, not packet capture or traffic inspection. |
| Process explorer | Implemented in source | Displays process data, supports filtering and process history, and exposes a terminate action. Termination is forceful. |
| System and hardware details | Implemented in source | Shows OS, CPU, memory, drives, displays, GPU, and selected hardware details. Some values depend on platform APIs and may be unavailable. |
| Battery and startup programs | Platform-limited | Battery detail and startup-program enumeration have Windows implementations; non-Windows paths return no extended battery data or an empty startup list. |
| Historical timeline | Implemented in source | Snapshots and per-process history are persisted to a local SQLite database. |
| App usage tracking | Implemented in source | Tracks the foreground application and idle intervals and persists totals to a local JSON file. |
| Threshold alerts | Implemented in source | CPU, memory, and disk limits can trigger desktop notifications. Threshold preferences are stored in browser local storage. |
| Predictions and health score | Implemented as heuristics | Uses fixed thresholds, recent values, trend calculations, and volatility. It does not call a trained model or an AI service. |
| CSV and JSON export | Implemented in source | Rust commands return recent snapshot data for export. |
| PDF report | Implemented in source | Rust report code generates a PDF using supplied snapshot, analysis, usage, startup, and summary data. |
| CPU stress control | Implemented in source | Tauri commands start worker threads and stop them after a configured duration or on request. This can make the machine temporarily busy. |
| Native application launch | Not verified | The frontend build passes, but the Rust test/build script was blocked by a Windows Application Control policy in the preparation environment. |

## Privacy note

A snapshot includes host and hardware details, process names and IDs, and foreground application/window information. The system details page includes the system serial number when the platform provides it. Snapshots are stored in sage_history.db, and app usage is stored in app_usage.json, under the app's local data directory. Exported reports can contain the same details. Review files before sharing them.
