# KruYa System Monitor

**Early Access — actively developing**

KruYa is a Tauri desktop application for viewing local system metrics and resource history. It explores collecting operating-system information in Rust and presenting it through a React and TypeScript interface.

The UI calls Rust commands through Tauri. Rust gathers metrics, calculates threshold and trend summaries, stores snapshots locally, and returns the result to the dashboard. The predictions screen uses deterministic rules and recent trends; this is not machine learning or an AI integration.

## Current features

- Live CPU, memory, disk, network-counter, and process views.
- System, hardware, drive, and display details.
- Process search, per-process history, and a process termination action.
- Historical snapshots and process history stored locally in SQLite.
- Foreground application usage totals stored locally in JSON.
- Threshold-based CPU, memory, and disk notifications.
- Deterministic trend summaries, resource warnings, and a health score.
- CSV, JSON, and PDF report export.
- Battery and startup-program details where the platform implementation is available.
- An optional timed CPU stress utility.

See [docs/FEATURES.md](docs/FEATURES.md) for implementation status and platform limits.

## Architecture

~~~mermaid
flowchart LR
  UI[React + TypeScript UI] --> Bridge[src/api.ts]
  Bridge -->|Tauri invoke| Rust[src-tauri/src/lib.rs]
  Rust --> Telemetry[telemetry.rs<br/>OS metrics]
  Rust --> Analysis[analytics.rs + src/predictive.ts<br/>Threshold and trend heuristics]
  Rust --> Storage[storage.rs<br/>SQLite history]
  Rust --> Usage[foreground.rs + app_tracker.rs<br/>Local app-usage tracking]
  Rust --> Reports[report.rs<br/>PDF export]
  Telemetry --> Rust
  Analysis --> Rust
  Storage --> Rust
  Usage --> Rust
  Rust -->|Snapshot and analysis| Bridge
  Bridge --> UI
~~~

React renders the screens and keeps presentation state. src/api.ts is the boundary to Tauri commands. src-tauri/src/lib.rs registers those commands and coordinates the Rust modules. The telemetry collector uses Rust libraries and platform-specific APIs; analytics runs calculations over collected snapshots; storage persists local history.

For module responsibilities and data flow, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Technologies

- **Tauri 2** hosts the web UI in a native desktop window and exposes Rust commands to the frontend.
- **Rust** gathers operating-system data, writes local history, runs analysis, and creates reports.
- **React 19 and TypeScript** compose the dashboard pages and call the typed frontend API.
- **Vite 8** serves the development UI and builds frontend assets.
- **SQLite (rusqlite)** stores system snapshots and process history.
- **Serde** serializes data exchanged between Rust and TypeScript.
- **Vitest and Testing Library** run the current frontend component tests.

## Run locally

### Requirements

- Node.js ^20.19.0 or >=22.12.0, and npm.
- Rust 1.77.2 or newer.
- Tauri platform prerequisites. See the [official Tauri guide](https://v2.tauri.app/start/prerequisites/).

### Start the desktop app

~~~bash
npm ci
npx tauri dev
~~~

The Tauri configuration starts Vite on port 5173 and loads it in the native window. Running npm run dev alone opens only the frontend; native system-data commands need the Tauri shell.

### Build

~~~bash
npm run build
npx tauri build
~~~

npm run build checks TypeScript and produces the frontend bundle. npx tauri build also compiles and packages the native application for the current host operating system. A bundle target of all does not verify or cross-compile the app for every operating system.

## Tests and current checks

~~~bash
npm test
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml --locked
~~~

The existing JavaScript suite contains six MiniChart tests. In the current portfolio copy, npm test passed 6/6 and npm run build passed. npm run lint reports one react-hooks/set-state-in-effect error in src/pages/ProcessExplorer.tsx. The Rust test command could not start: Windows Application Control blocked execution of the generated Cargo build script. No native desktop build or runtime is claimed as verified.

## Screenshot

No application screenshot is included yet. The native Rust shell could not be launched in the available environment, and a browser-only Vite preview would not show the operating-system telemetry returned by Tauri commands. The architecture diagram above describes the implementation; it is not a screenshot.

## Local data and privacy

KruYa reads system metrics, process details, foreground application/window information, and selected hardware identifiers. The system-information screen can show the machine serial number when the platform provides it. The application writes:

- sage_history.db for system snapshots and process history.
- app_usage.json for foreground app usage totals.

These files are placed under the platform data directory in a kruya subdirectory. CSV, JSON, and PDF exports can contain host, hardware, process, and foreground-window details. Review exports and screenshots before sharing. The checked source contains local storage and no telemetry upload endpoint.

No application environment variables are referenced by the checked source. No .env file is required.

## Known limitations

- The app has not been launched as a native desktop application in this environment.
- Only the existing chart component has frontend tests; telemetry, Rust storage, and frontend/backend integration lack verified coverage.
- One lint finding remains in ProcessExplorer.
- Battery and startup-program support varies by operating system.
- The Tauri configuration currently sets csp to null; tightening and verifying the application policy is a security follow-up.
- The dashboard collects sensitive local system data and does not yet provide a documented retention/redaction workflow.

See the [roadmap](docs/ROADMAP.md) and [development guide](docs/DEVELOPMENT.md) for planned improvements and commands.

## Version

0.1.0-alpha.1 is the first public pre-release baseline prepared from the current source tree. No earlier release tags were available, and no earlier release history has been reconstructed. This describes source code; it does not imply that a packaged binary has been published.

## License

No project license is included. No license choice has been added as part of this preparation.
