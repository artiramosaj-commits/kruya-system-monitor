# KruYa architecture

## Data flow

~~~mermaid
flowchart LR
  UI[React + TypeScript UI] --> Bridge[src/api.ts]
  Bridge -->|Tauri invoke commands| Commands[src-tauri/src/lib.rs]
  Commands --> Telemetry[telemetry.rs<br/>OS metrics and hardware information]
  Commands --> Analytics[analytics.rs<br/>Thresholds and trend heuristics]
  Commands --> Storage[storage.rs<br/>SQLite snapshots and process history]
  Commands --> Usage[app_tracker.rs + foreground.rs<br/>Foreground app usage and idle time]
  Telemetry --> Commands
  Analytics --> Commands
  Storage --> Commands
  Usage --> Commands
  Commands -->|JSON result| Bridge
  Bridge --> UI
  Commands -->|User-requested exports| Reports[report.rs<br/>PDF report]
~~~

## Component responsibilities

| Path | Responsibility |
|---|---|
| src/App.tsx | Main dashboard shell, navigation, polling state, alerts, exports, and settings. |
| src/api.ts | Typed wrappers around Tauri invoke calls. |
| src/pages/ | Dashboard views for processes, storage, network, system information, history, predictions, and app usage. |
| src/predictive.ts | Deterministic trend and threshold calculations used by the predictions view. |
| src-tauri/src/lib.rs | Tauri commands, application state, command registration, tray setup, and local data directory initialization. |
| src-tauri/src/telemetry.rs | Operating-system metrics and hardware details, with OS-specific code paths. |
| src-tauri/src/analytics.rs | System health checks, threshold assessments, and summary analysis. |
| src-tauri/src/storage.rs | SQLite schema and persistence for snapshots and process history. |
| src-tauri/src/app_tracker.rs and foreground.rs | Foreground application/window tracking, idle detection, and local JSON usage data. |
| src-tauri/src/report.rs | PDF report generation. |
| src-tauri/src/models.rs | Rust data types serialized to the frontend. |

## Request and storage flow

The React interface calls typed functions in src/api.ts. Those functions use Tauri invoke to call commands registered in src-tauri/src/lib.rs. Rust collects a system snapshot, runs deterministic analysis, saves the snapshot and process data to SQLite, and returns a result to the UI.

Application usage is tracked separately. The app records foreground application names and time information in app_usage.json; snapshots also contain foreground process/window information. The app data directory is selected from the platform data directory and a kruya subdirectory.

The checked application code contains no network upload path. Local telemetry and exports can still contain sensitive information; inspect the privacy section in the README before sharing reports or screenshots.
