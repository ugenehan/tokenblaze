# TokenBlaze

This is the single Tauri v2 desktop application. The dashboard and transparent
flame panel are two Tauri windows in one process and use the same Rust state.
The flame pixels are produced by the existing Rust fire engine and displayed
in the transparent panel's canvas; usage monitoring continues in that same
application process. Preferences are read and persisted by Rust through
`tauri-plugin-store`.

## Run

From this `app` directory, install the Tauri CLI dependency once with
`npm install`, then run `npm run dev` for development or `npm run build` for a
release build.
The frontend is static HTML/CSS/JS, so Tauri embeds it directly; there is no
separate frontend bundler or second TokenBlaze executable to build. The release
executable is named `tokenblaze.exe` on Windows.

The app exposes a local command boundary:

- `dashboard_snapshot` / `fire_frame` — shared usage, preferences, and live
  flame rendering state.
- `set_*` settings commands — update the in-memory Rust config and persist it
  through the Tauri store.
- Tray menu and dashboard commands — show/hide the flame panel and open/hide
  the dashboard without starting another process.
