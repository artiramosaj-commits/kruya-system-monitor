# Roadmap

## Next

- Resolve the ESLint finding in ProcessExplorer without changing history or loading behavior.
- Run Rust tests and launch the native app on a machine where Cargo build scripts can execute.
- Verify a native build on each intended operating system and document which metrics are available there.
- Add tests for Tauri command contracts, storage, and deterministic prediction rules.

## Later

- Add clear controls for collecting, retaining, and deleting historical system and app-usage data.
- Redact serial numbers, hostnames, and foreground window titles from reports unless the user explicitly includes them.
- Set and verify a restrictive Tauri Content Security Policy; the current configuration has csp: null.
- Improve labels for metrics that are unavailable on a given operating system.
- Keep release notes and version numbers synchronized across the UI and manifests.
