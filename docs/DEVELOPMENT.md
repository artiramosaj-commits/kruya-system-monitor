# Development guide

## Requirements

- Node.js accepted by the installed Vite version and npm.
- Rust toolchain 1.77.2 or newer, as declared in src-tauri/Cargo.toml.
- Tauri platform prerequisites. On Windows this includes native build tools and the WebView2 runtime; follow the official platform guide for the machine being used.

The checked Vite package declares Node ^20.19.0 or >=22.12.0. The project lockfile is used to keep JavaScript dependencies reproducible.

## Install and run

~~~bash
npm ci
npx tauri dev
~~~

The Tauri configuration starts the Vite dev server on port 5173 and loads that URL in the native window. npm run dev starts only the browser frontend; Tauri invoke calls that read operating-system metrics require the desktop shell.

## Build

~~~bash
npm run build
npx tauri build
~~~

npm run build compiles TypeScript and builds the frontend. npx tauri build additionally builds and bundles the native application for the current host. The bundle target setting does not make a cross-platform build; each operating system needs its own native toolchain and verification.

## Checks

~~~bash
npm test
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml --locked
~~~

The existing JavaScript suite covers MiniChart; it does not cover Tauri telemetry or UI/backend integration. During portfolio preparation, npm test passed 6 tests and npm run build passed. npm run lint reported one react-hooks/set-state-in-effect error in src/pages/ProcessExplorer.tsx. Rust tests did not run: Windows Application Control blocked Cargo from executing the generated kruya build script.

## Current development setup fixes

The original Tauri configuration had no build.devUrl or build.beforeDevCommand, despite the frontend using Vite. The portfolio copy now sets the Vite URL to http://localhost:5173 and starts npm run dev -- --strictPort. Vite documents 5173 as its default port; strictPort prevents it silently moving to another port when the configured port is occupied.

## Official references

- [Tauri v2 development](https://v2.tauri.app/develop/)
- [Tauri v2 configuration reference](https://v2.tauri.app/reference/config/#buildconfig)
- [Tauri v2 platform prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Vite server options](https://vite.dev/config/server-options)
