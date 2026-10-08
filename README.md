# TokenBlaze

[English](README.md) | [简体中文](README.zh-CN.md)

TokenBlaze is a Windows desktop companion for viewing local AI coding assistant usage. It reads supported tools' local session files or databases, records normalized usage events in a local SQLite database, and visualizes activity in a dashboard and an optional animated flame panel.

> TokenBlaze is currently version `0.1.0`. Token counts and live activity rates depend on the data exposed by each source; some values, especially Cursor local estimates, are estimates rather than billing records.

## Contents

- [Features](#features)
- [Supported sources](#supported-sources)
- [Privacy and local data](#privacy-and-local-data)
- [Requirements](#requirements)
- [Run from source](#run-from-source)
- [Build a Windows installer](#build-a-windows-installer)
- [Repository layout](#repository-layout)
- [Troubleshooting](#troubleshooting)
- [Known limitations](#known-limitations)
- [License](#license)

## Features

- **Usage overview:** today’s token total, input/output/cache breakdown where available, per-source totals, hourly activity, and a seven-day history.
- **Desktop flame:** an optional transparent, always-on-top companion window that reflects recent activity. The dashboard and flame share the same application process and local state.
- **Source status:** see whether a source was detected, when it was last read, and whether its values include estimates.
- **Appearance controls:** reduce motion, pause flame animation, show the live-rate estimate, choose the flame size, set colors per source, and preview flame styles.
- **Tray controls:** open the dashboard, show or hide the flame, pause animation, check for updates, and quit from the Windows system tray.
- **Language settings:** the application UI supports System, English, Simplified Chinese, Japanese, and Korean. This repository also provides English and Simplified Chinese README files; use the links at the top of either file to switch documentation language.
- **Export:** save the usage chart as a PNG image.

## Supported sources

TokenBlaze currently scans local data for these tools:

| Source | Local input | Notes |
| --- | --- | --- |
| Claude Code | Session files | A session folder can be entered in Settings if automatic discovery does not find it. |
| Codex | Session files | A session folder can be entered in Settings if automatic discovery does not find it. |
| Cursor | Local application database and related usage data | Depending on available data, TokenBlaze may use dashboard records or a local usage estimate. Check the source details for the active mode. |
| Grok | Session files | A session folder can be entered in Settings if automatic discovery does not find it. |
| Pi | Session files | A session folder can be entered in Settings if automatic discovery does not find it. |
| Amp | Session files | A session folder can be entered in Settings if automatic discovery does not find it. |
| OpenCode | Local SQLite database | TokenBlaze reads the local database; it does not require an OpenCode cloud API. |

Source formats and installation locations can change between upstream tool versions. A source may remain disconnected until the tool has created local records. In **Settings**, enter a session folder for Claude Code, Codex, Grok, Pi, or Amp; leave it empty to return to automatic discovery. Cursor and OpenCode use automatic database discovery in the current implementation.

TokenBlaze imports local usage data; it does not start or control the source applications. It does not promise parity with provider invoices or dashboards. Source data can be missing, delayed, cumulative, or estimated, and cache fields are shown only when the source provides them.

## Privacy and local data

- Session files and source databases are read from this computer.
- Normalized usage events are stored in a local SQLite database. Application preferences are stored locally through Tauri's store plugin.
- Usage/session contents are not uploaded by the monitoring feature.
- The **Check for updates** action needs a network connection and reads the Sparkle-style XML feed at [appcast.xml](https://raw.githubusercontent.com/ugenehan/tokenblaze/main/appcast.xml) in this repository. The feed must contain a Windows executable signed with the updater public key before an update can be downloaded and installed. The repository release page is [GitHub Releases](https://github.com/ugenehan/tokenblaze/releases).
- Do not share the local database or source session files without reviewing them first. They may contain project names, prompts, paths, or other private information.

## Requirements

- Windows 10 or later is the intended desktop platform. The current Tauri bundle target is the Windows NSIS installer.
- Rust stable toolchain with Cargo.
- Node.js and npm, used to run the Tauri CLI.
- The Windows build prerequisites required by Tauri 2 (including the Microsoft C++ build tools and WebView2 runtime).

The application reads data only from tools installed and used on the same machine. No API keys or provider credentials are needed for local log ingestion.

## Run from source

Open PowerShell in the repository root:

```powershell
cargo test --workspace
cargo check -p tokenblaze-app
```

Run the desktop application in development mode:

```powershell
cd .\app
npm install
npm run dev
```

`npm run dev` launches the Tauri application and serves the frontend from `app/src`. The frontend is plain HTML, CSS, and JavaScript; the Tauri CLI is the package's only npm development dependency.

## Build a Windows installer

From the `app` directory:

```powershell
npm install
npm run build
```

The Tauri configuration packages an NSIS installer. Release artifacts are produced under the Tauri build output beneath `app/src-tauri/target`; exact filenames depend on the target and build configuration.

For a Rust-only validation pass from the repository root:

```powershell
cargo test --workspace
cargo check -p tokenblaze-app
```

## Repository layout

```text
.
├── app/
│   ├── src/                 # Dashboard and flame-panel HTML, CSS, and JavaScript
│   └── src-tauri/           # Tauri desktop executable, windows, tray, and commands
├── crates/
│   └── tokenblaze-core/     # Configuration, source adapters, SQLite store, charts data,
│                            # localization, flame engine, and update logic
├── Cargo.toml               # Rust workspace and release profile
├── Cargo.lock
├── LICENSE
├── README.md                # English documentation
└── README.zh-CN.md          # Simplified Chinese documentation
```

The Rust workspace has two members: `app/src-tauri` (`tokenblaze-app`) and `crates/tokenblaze-core` (`tokenblaze-core`). The Tauri process owns the dashboard, flame panel, tray menu, monitor, and shared application state.

## Troubleshooting

### A source is disconnected or shows no usage

1. Use the source application and confirm that it has created local sessions or database records.
2. Open **Settings** and select **Rescan sources**.
3. For Claude Code, Codex, Grok, Pi, or Amp, enter the folder that contains that tool's session files. Clear the field to restore automatic discovery.
4. Confirm that your Windows account can read the selected folder or database.

### Cursor values differ from its dashboard

Cursor ingestion can select between dashboard records and a local estimate according to what is available on this machine. Estimates are intended to show activity trends, not to reproduce a provider's billing total. Review the Cursor source detail in the dashboard for the detected mode and estimate status.

### The flame is not visible

Enable the flame panel in Settings or use the tray menu to show it. The flame can also be paused independently of background usage monitoring. If it is still absent, restart TokenBlaze and check whether the window is behind another full-screen application.

### Build fails on Windows

Verify that Rust/Cargo and Node.js/npm are available in PowerShell, then install the Windows prerequisites for Tauri 2. If a native dependency fails to compile, check the Tauri prerequisite setup for the installed Windows toolchain before changing project dependencies.

## Known limitations

- Source discovery depends on each tool's local file and database formats, which may change upstream.
- TokenBlaze does not validate local counts against provider billing records.
- Cursor can report estimates when exact usage records are unavailable.
- The update checker targets this repository main/appcast.xml feed and GitHub Releases page. This checkout does not yet contain appcast.xml, so online checks will fail until the feed is published. Update delivery also requires a valid signed Windows release entry. After signing the raw `tokenblaze.exe` (not the NSIS installer) with the matching Ed25519 key, `cargo run -p tokenblaze-core --bin appcast_release -- VERSION PATH_TO_EXE RELEASE_ASSET_URL PATH_TO_BASE64_SIGNATURE` verifies the detached signature and prints an appcast entry for review. It does not publish a release or feed.
- The current installer configuration targets Windows NSIS; macOS and Linux packaging are not configured here.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
