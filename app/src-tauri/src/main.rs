#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, Wry};
use tauri_plugin_store::{Store, StoreExt};
use tokenblaze_core::config::{AppConfig, AppTheme, CostRates};
use tokenblaze_core::data::snapshot::{self, DashboardSnapshot, FireView};
use tokenblaze_core::data::{UsageMonitor, UsageSource, UsageStore};
use tokenblaze_core::fire::{
    compose_campfire_frame, FireEngine, FirePreviewStyle, FireStateMachine, FlameSize,
    SourceFlameColors,
};
use tokenblaze_core::l10n::{t, AppLanguage};
use tokenblaze_core::updater::{UpdateManager, UpdateStatus};

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "config";

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    #[link_name = "SetWindowPos"]
    fn set_window_pos(
        hwnd: isize,
        insert_after: isize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    #[link_name = "GetWindowLongW"]
    fn get_window_long(hwnd: isize, index: i32) -> i32;
    #[link_name = "SetWindowLongW"]
    fn set_window_long(hwnd: isize, index: i32, value: i32) -> i32;
    #[link_name = "SetWindowRgn"]
    fn set_window_rgn(hwnd: isize, region: isize, redraw: i32) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
extern "system" {
    #[link_name = "CreateRectRgn"]
    fn create_rect_rgn(left: i32, top: i32, right: i32, bottom: i32) -> isize;
    #[link_name = "DeleteObject"]
    fn delete_object(object: isize) -> i32;
}

#[cfg(windows)]
#[link(name = "comctl32")]
extern "system" {
    #[link_name = "SetWindowSubclass"]
    fn set_window_subclass(
        hwnd: isize,
        callback: unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize,
        subclass_id: usize,
        reference_data: usize,
    ) -> i32;
    #[link_name = "DefSubclassProc"]
    fn def_subclass_proc(hwnd: isize, message: u32, wparam: usize, lparam: isize) -> isize;
    #[link_name = "RemoveWindowSubclass"]
    fn remove_window_subclass(
        hwnd: isize,
        callback: unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize,
        subclass_id: usize,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "dwmapi")]
extern "system" {
    #[link_name = "DwmSetWindowAttribute"]
    fn dwm_set_window_attribute(hwnd: isize, attribute: u32, value: *const i32, size: u32) -> i32;
}

struct AppState {
    monitor: Mutex<UsageMonitor>,
    usage_store: Arc<UsageStore>,
    fire: Arc<Mutex<FireStateMachine>>,
    renderer: Mutex<FireRenderer>,
    config: Mutex<AppConfig>,
    last_tick: Mutex<Instant>,
    last_monitor_scan: Mutex<Instant>,
    updater: Mutex<UpdateManager>,
    settings_store: Arc<Store<Wry>>,
    tray_items: Mutex<Option<TrayItems>>,
}

struct TrayItems {
    toggle_flame: tauri::menu::MenuItem<Wry>,
    today_usage: tauri::menu::MenuItem<Wry>,
    open_console: tauri::menu::MenuItem<Wry>,
    reset_position: tauri::menu::MenuItem<Wry>,
    toggle_pause: tauri::menu::MenuItem<Wry>,
    check_updates: tauri::menu::MenuItem<Wry>,
    quit: tauri::menu::MenuItem<Wry>,
}

fn refresh_tray_usage(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok((tokens, by_source)) = state.usage_store.today_totals() else {
        return;
    };
    let active_sources = by_source.iter().filter(|tokens| **tokens > 0).count();
    let language = match state.config.lock() {
        Ok(config) => config.language.effective(),
        Err(_) => return,
    };
    let summary = format!(
        "{}: {} · {}: {}",
        t("menu.todayUsage", language),
        tokens,
        t("menu.activeSources", language),
        active_sources
    );
    if let Ok(items) = state.tray_items.lock() {
        if let Some(items) = items.as_ref() {
            let _ = items.today_usage.set_text(&summary);
        }
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(format!("TokenBlaze · {summary}")));
    }
}

struct FireRenderer {
    engine: FireEngine,
    last_step: Instant,
}

#[derive(Serialize)]
struct FireFrame {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    fuel: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FireVisualState {
    fire: FireView,
    reduce_motion: bool,
    show_live_rate: bool,
    today_tokens: i64,
    sources: Vec<snapshot::SourceView>,
    language: String,
    theme: String,
    config: FlameConfigView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FlameConfigView {
    flame_size: String,
    source_colors: [[u8; 3]; 7],
}

impl AppState {
    fn initialize(settings_store: Arc<Store<Wry>>) -> anyhow::Result<Self> {
        let mut config: AppConfig = settings_store
            .get(SETTINGS_KEY)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        if !matches!(config.retention_days, 0 | 30 | 90 | 365) {
            config.retention_days = 0;
        }
        SourceFlameColors::set_all(config.source_colors);

        let fire = Arc::new(Mutex::new(FireStateMachine::new()));
        fire.lock().expect("fire mutex poisoned").reduce_motion = config.reduce_motion;

        let store = Arc::new(UsageStore::open()?);
        store.set_retention_days(config.retention_days)?;
        let mut monitor = UsageMonitor::new(store.clone());
        monitor.configure_source_paths(&config.source_paths);
        monitor.attach_fire(&fire);
        monitor.start();

        Ok(Self {
            monitor: Mutex::new(monitor),
            usage_store: store,
            fire,
            renderer: Mutex::new(FireRenderer {
                engine: FireEngine::new(),
                last_step: Instant::now(),
            }),
            config: Mutex::new(config),
            last_tick: Mutex::new(Instant::now()),
            last_monitor_scan: Mutex::new(Instant::now()),
            updater: Mutex::new(UpdateManager::new()),
            settings_store,
            tray_items: Mutex::new(None),
        })
    }

    fn update_config(&self, update: impl FnOnce(&mut AppConfig)) -> Result<AppConfig, String> {
        let mut current = self.config.lock().map_err(|_| "config mutex poisoned")?;
        let mut next = current.clone();
        update(&mut next);
        let value = serde_json::to_value(&next).map_err(|error| error.to_string())?;
        self.settings_store.set(SETTINGS_KEY, value);
        self.settings_store
            .save()
            .map_err(|error| format!("failed to save settings: {error}"))?;
        *current = next.clone();
        Ok(next)
    }

    fn tick(&self) -> Result<(), String> {
        let dt = {
            let mut last_tick = self.last_tick.lock().map_err(|_| "tick mutex poisoned")?;
            let dt = last_tick.elapsed().as_secs_f64().min(1.0);
            *last_tick = Instant::now();
            dt
        };
        self.fire
            .lock()
            .map_err(|_| "fire mutex poisoned")?
            .tick(dt);
        let scan_interval = self
            .config
            .lock()
            .map_err(|_| "config mutex poisoned")?
            .token_poll_interval_seconds
            .clamp(1, 10);
        let scan_interval = if self
            .fire
            .lock()
            .map_err(|_| "fire mutex poisoned")?
            .animation_paused
        {
            scan_interval.max(15)
        } else {
            scan_interval
        };
        let scan_due = {
            let mut last_scan = self
                .last_monitor_scan
                .lock()
                .map_err(|_| "monitor scan mutex poisoned")?;
            if last_scan.elapsed() >= Duration::from_secs(scan_interval) {
                *last_scan = Instant::now();
                true
            } else {
                false
            }
        };
        let mut monitor = self.monitor.lock().map_err(|_| "monitor mutex poisoned")?;
        if scan_due {
            monitor.tick();
        } else {
            monitor.poll();
        }
        Ok(())
    }
}

#[tauri::command(async)]
fn dashboard_snapshot(state: State<'_, AppState>) -> Result<DashboardSnapshot, String> {
    state.tick()?;
    let monitor = state.monitor.lock().map_err(|_| "monitor mutex poisoned")?;
    let fire = state.fire.lock().map_err(|_| "fire mutex poisoned")?;
    let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
    let updater = state.updater.lock().map_err(|_| "updater mutex poisoned")?;
    let update = update_view(&updater);
    let mut dashboard = snapshot::from_runtime(&monitor, &fire, &config);
    dashboard.update = update;
    Ok(dashboard)
}

#[tauri::command(async)]
fn usage_history(
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<snapshot::DailyPoint>, String> {
    if !matches!(days, 30 | 90) {
        return Err("invalid history range".to_string());
    }
    state
        .usage_store
        .daily_usage(days)
        .map_err(|error| error.to_string())
        .map(|days| {
            days.into_iter()
                .map(|day| snapshot::DailyPoint {
                    date: day.date,
                    tokens: day.tokens,
                })
                .collect()
        })
}

#[tauri::command]
fn export_usage_events(
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<tokenblaze_core::data::store::ExportUsageEvent>, String> {
    state
        .usage_store
        .export_events(days)
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CostSummary {
    today: tokenblaze_core::data::store::CostEstimate,
    week: tokenblaze_core::data::store::CostEstimate,
    daily_budget_usd: Option<f64>,
    weekly_budget_usd: Option<f64>,
}

#[tauri::command(async)]
fn cost_summary(state: State<'_, AppState>) -> Result<Option<CostSummary>, String> {
    let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
    if !config.cost_enabled {
        return Ok(None);
    }
    let rates = config.cost_rates;
    let daily_budget_usd = config.daily_budget_usd;
    let weekly_budget_usd = config.weekly_budget_usd;
    drop(config);
    Ok(Some(CostSummary {
        today: state
            .usage_store
            .estimate_cost(1, &rates)
            .map_err(|error| error.to_string())?,
        week: state
            .usage_store
            .estimate_cost(7, &rates)
            .map_err(|error| error.to_string())?,
        daily_budget_usd,
        weekly_budget_usd,
    }))
}

#[tauri::command]
fn set_cost_settings(
    enabled: bool,
    rates: [CostRates; 7],
    daily_budget_usd: Option<f64>,
    weekly_budget_usd: Option<f64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let valid = |price: Option<f64>| {
        price.is_none_or(|value| value.is_finite() && (0.0..=1_000_000.0).contains(&value))
    };
    let valid_budget = |budget: Option<f64>| {
        budget.is_none_or(|value| value.is_finite() && value > 0.0 && value <= 1_000_000.0)
    };
    if !valid_budget(daily_budget_usd)
        || !valid_budget(weekly_budget_usd)
        || rates.iter().any(|rate| {
            !valid(rate.input)
                || !valid(rate.output)
                || !valid(rate.cache_read)
                || !valid(rate.cache_write)
        })
    {
        return Err("invalid price or budget".to_string());
    }
    state.update_config(|config| {
        config.cost_enabled = enabled;
        config.cost_rates = rates;
        config.daily_budget_usd = daily_budget_usd;
        config.weekly_budget_usd = weekly_budget_usd;
    })?;
    Ok(())
}

#[tauri::command(async)]
fn fire_visual_state(state: State<'_, AppState>) -> Result<FireVisualState, String> {
    state.tick()?;
    let monitor = state.monitor.lock().map_err(|_| "monitor mutex poisoned")?;
    let fire = state.fire.lock().map_err(|_| "fire mutex poisoned")?;
    let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
    let dashboard = snapshot::from_runtime(&monitor, &fire, &config);
    Ok(FireVisualState {
        fire: dashboard.fire,
        reduce_motion: config.reduce_motion,
        show_live_rate: config.show_live_rate,
        today_tokens: dashboard.today_tokens,
        sources: dashboard.sources,
        language: dashboard.config.language,
        theme: dashboard.config.theme,
        config: FlameConfigView {
            flame_size: dashboard.config.flame_size,
            source_colors: dashboard.config.source_colors,
        },
    })
}

#[tauri::command]
fn fire_frame(window: WebviewWindow, state: State<'_, AppState>) -> Result<FireFrame, String> {
    state.tick()?;
    let (snapshot, paused, palette_epoch) = {
        let fire = state.fire.lock().map_err(|_| "fire mutex poisoned")?;
        (
            fire.snapshot(),
            fire.animation_paused,
            fire.color_palette_epoch,
        )
    };
    let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
    let mut renderer = state
        .renderer
        .lock()
        .map_err(|_| "renderer mutex poisoned")?;
    renderer
        .engine
        .apply(snapshot, config.reduce_motion, palette_epoch);
    let step_interval = if config.reduce_motion {
        1.0 / 6.0
    } else {
        1.0 / 12.0
    };
    if !paused && renderer.last_step.elapsed().as_secs_f64() >= step_interval {
        renderer.engine.step();
        renderer.last_step = Instant::now();
    }
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let width = size.width.max(1) as usize;
    let height = size.height.max(1) as usize;
    let fire_pixels = renderer.engine.render();
    let pixels = compose_campfire_frame(fire_pixels, width, height, snapshot);
    Ok(FireFrame {
        width,
        height,
        pixels,
        fuel: snapshot.fuel,
    })
}

#[tauri::command]
fn set_reduce_motion(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.reduce_motion = enabled)?;
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .reduce_motion = enabled;
    Ok(())
}

#[tauri::command]
fn set_sound_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.sound_enabled = enabled)?;
    Ok(())
}

#[tauri::command]
fn set_sound_volume(volume: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.sound_volume = volume.clamp(0.0, 1.0))?;
    Ok(())
}

#[tauri::command]
fn set_show_live_rate(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.show_live_rate = enabled)?;
    Ok(())
}

#[tauri::command]
fn set_token_poll_interval(seconds: u64, state: State<'_, AppState>) -> Result<(), String> {
    if ![1, 2, 5, 10].contains(&seconds) {
        return Err(format!("unsupported token poll interval: {seconds}"));
    }
    state.update_config(|config| config.token_poll_interval_seconds = seconds)?;
    *state
        .last_monitor_scan
        .lock()
        .map_err(|_| "monitor scan mutex poisoned")? = Instant::now();
    Ok(())
}

#[tauri::command]
fn set_panel_visible(
    visible: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.update_config(|config| config.panel_visible = visible)?;
    let panel = app
        .get_webview_window("flame")
        .ok_or_else(|| "flame window is unavailable".to_string())?;
    set_flame_visibility(&panel, visible).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn initialize_flame_window(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if window.label() != "flame" {
        return Err("initialize_flame_window must be called by the flame window".into());
    }
    let (size, visible, position) = {
        let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
        (
            config.flame_size,
            config.panel_visible,
            config.window_position,
        )
    };

    #[cfg(windows)]
    {
        install_flame_frame_subclass(&window).map_err(|error| error.to_string())?;
        resize_flame_window(&window, size, true).map_err(|error| error.to_string())?;
        apply_flame_hit_region(&window, size, false).map_err(|error| error.to_string())?;
        if let Some((x, y)) = position {
            let position = flame_window_host_position(&window, size, x as i32, y as i32)
                .map_err(|error| error.to_string())?;
            window
                .set_position(position)
                .map_err(|error| error.to_string())?;
        } else if let Some(monitor) = window
            .primary_monitor()
            .map_err(|error| error.to_string())?
            .or(window
                .current_monitor()
                .map_err(|error| error.to_string())?)
        {
            let position =
                bottom_right_position(&window, &monitor).map_err(|error| error.to_string())?;
            window
                .set_position(position)
                .map_err(|error| error.to_string())?;
        }
    }
    #[cfg(not(windows))]
    {
        let (width, height) = size.content_size();
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|error| error.to_string())?;
        if let Some((x, y)) = position {
            window
                .set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
                .map_err(|error| error.to_string())?;
        } else if let Some(monitor) = window
            .primary_monitor()
            .map_err(|error| error.to_string())?
            .or(window
                .current_monitor()
                .map_err(|error| error.to_string())?)
        {
            let position =
                bottom_right_position(&window, &monitor).map_err(|error| error.to_string())?;
            window
                .set_position(position)
                .map_err(|error| error.to_string())?;
        }
    }

    set_flame_visibility(&window, visible).map_err(|error| error.to_string())
}

fn set_flame_visibility(window: &WebviewWindow, visible: bool) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        window.run_on_main_thread({
            let window = window.clone();
            move || {
                let result = if visible {
                    window.show()
                } else {
                    window.hide()
                }
                .and_then(|_| remove_flame_nonclient_styles(&window));
                if let Err(error) = result {
                    eprintln!("failed to update flame window visibility: {error}");
                }
            }
        })
    }
    #[cfg(not(windows))]
    {
        if visible {
            window.show()
        } else {
            window.hide()
        }
    }
}

#[tauri::command]
fn set_flame_size(size: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let size = match size.as_str() {
        "Small" => FlameSize::Small,
        "Medium" => FlameSize::Medium,
        "Large" => FlameSize::Large,
        _ => return Err(format!("unsupported flame size: {size}")),
    };
    state.update_config(|config| config.flame_size = size)?;
    let window = app
        .get_webview_window("flame")
        .ok_or_else(|| "flame window is unavailable".to_string())?;
    #[cfg(windows)]
    {
        resize_flame_window(&window, size, true).map_err(|error| error.to_string())?;
        apply_flame_hit_region(&window, size, false).map_err(|error| error.to_string())
    }
    #[cfg(not(windows))]
    {
        let (width, height) = size.content_size();
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn set_language(
    language: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let language = match language.as_str() {
        "System" => AppLanguage::System,
        "English" => AppLanguage::English,
        "Chinese" => AppLanguage::Chinese,
        "Japanese" => AppLanguage::Japanese,
        "Korean" => AppLanguage::Korean,
        _ => return Err(format!("unsupported language: {language}")),
    };
    let config = state.update_config(|config| config.language = language)?;
    let language = config.language.effective();
    let items = state
        .tray_items
        .lock()
        .map_err(|_| "tray menu mutex poisoned")?;
    if let Some(items) = items.as_ref() {
        items
            .toggle_flame
            .set_text(t(
                if config.panel_visible {
                    "menu.hideFlame"
                } else {
                    "menu.showFlame"
                },
                language,
            ))
            .map_err(|error| error.to_string())?;
        items
            .open_console
            .set_text(t("menu.openConsole", language))
            .map_err(|error| error.to_string())?;
        items
            .reset_position
            .set_text(t("menu.resetPosition", language))
            .map_err(|error| error.to_string())?;
        let paused = state
            .fire
            .lock()
            .map_err(|_| "fire mutex poisoned")?
            .animation_paused;
        items
            .toggle_pause
            .set_text(t(
                if paused {
                    "menu.resumeAnimation"
                } else {
                    "menu.pauseAnimation"
                },
                language,
            ))
            .map_err(|error| error.to_string())?;
        items
            .check_updates
            .set_text(t("menu.checkUpdates", language))
            .map_err(|error| error.to_string())?;
        items
            .quit
            .set_text(t("menu.quit", language))
            .map_err(|error| error.to_string())?;
    }
    drop(items);
    refresh_tray_usage(&app);
    if let Some(window) = app.get_webview_window("flame") {
        let _ = window.emit("language-changed", language.label());
    }
    Ok(())
}

#[tauri::command]
fn set_theme(theme: String, state: State<'_, AppState>) -> Result<(), String> {
    let theme = match theme.as_str() {
        "System" => AppTheme::System,
        "Dark" => AppTheme::Dark,
        "Light" => AppTheme::Light,
        _ => return Err(format!("unsupported theme: {theme}")),
    };
    state.update_config(|config| config.theme = theme)?;
    Ok(())
}

#[tauri::command]
fn set_source_color(
    index: usize,
    color: [u8; 3],
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = UsageSource::ALL
        .get(index)
        .copied()
        .ok_or_else(|| format!("invalid source index: {index}"))?;
    state.update_config(|config| config.source_colors[index] = color)?;
    SourceFlameColors::set(source, color);
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .color_palette_epoch += 1;
    Ok(())
}

#[tauri::command]
fn reset_source_colors(state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.source_colors = SourceFlameColors::BUILT_IN)?;
    SourceFlameColors::set_all(SourceFlameColors::BUILT_IN);
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .color_palette_epoch += 1;
    Ok(())
}

#[tauri::command]
fn rescan(state: State<'_, AppState>) -> Result<(), String> {
    state
        .monitor
        .lock()
        .map_err(|_| "monitor mutex poisoned")?
        .rescan();
    Ok(())
}

#[tauri::command]
fn set_retention_days(days: u32, state: State<'_, AppState>) -> Result<(), String> {
    if !matches!(days, 0 | 30 | 90 | 365) {
        return Err("invalid retention period".to_string());
    }
    state.update_config(|config| config.retention_days = days)?;
    state
        .usage_store
        .set_retention_days(days)
        .map_err(|error| error.to_string())?;
    state
        .monitor
        .lock()
        .map_err(|_| "monitor mutex poisoned")?
        .rescan();
    Ok(())
}

#[tauri::command]
fn clear_usage_data(state: State<'_, AppState>) -> Result<(), String> {
    state
        .monitor
        .lock()
        .map_err(|_| "monitor mutex poisoned")?
        .clear_usage_data()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn complete_onboarding(state: State<'_, AppState>) -> Result<(), String> {
    state.update_config(|config| config.onboarding_complete = true)?;
    Ok(())
}

#[tauri::command]
fn set_source_path(
    source: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = match source.as_str() {
        "ClaudeCode" => UsageSource::ClaudeCode,
        "Codex" => UsageSource::Codex,
        "Grok" => UsageSource::Grok,
        "Pi" => UsageSource::Pi,
        "Amp" => UsageSource::Amp,
        _ => {
            return Err(
                "manual folders are supported for Claude Code, Codex, Grok, Pi, and Amp"
                    .to_string(),
            )
        }
    };
    let path = path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let path_buf = path.as_deref().map(std::path::PathBuf::from);
    if let Some(directory) = path_buf.as_ref() {
        if !directory.is_dir() {
            return Err("selected path is not a directory".to_string());
        }
        std::fs::read_dir(directory).map_err(|error| error.to_string())?;
    }
    state.update_config(|config| config.source_paths[source.index()] = path)?;
    state
        .monitor
        .lock()
        .map_err(|_| "monitor mutex poisoned")?
        .set_source_path(source, path_buf);
    Ok(())
}

#[tauri::command]
fn set_animation_paused(paused: bool, state: State<'_, AppState>) -> Result<(), String> {
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .animation_paused = paused;
    Ok(())
}

#[tauri::command]
fn show_preview(style: String, state: State<'_, AppState>) -> Result<(), String> {
    let style = match style.as_str() {
        "Out" => FirePreviewStyle::Out,
        "Ember" => FirePreviewStyle::Ember,
        "Hush" => FirePreviewStyle::Hush,
        "Glow" => FirePreviewStyle::Glow,
        "Crackle" => FirePreviewStyle::Crackle,
        "Roar" => FirePreviewStyle::Roar,
        "Blaze" => FirePreviewStyle::Blaze,
        _ => return Err(format!("unsupported preview style: {style}")),
    };
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .show_preview(style);
    Ok(())
}

#[tauri::command]
fn show_custom_preview(intensity: f64, state: State<'_, AppState>) -> Result<(), String> {
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .show_custom_preview(intensity);
    Ok(())
}

#[tauri::command]
fn return_to_live(state: State<'_, AppState>) -> Result<(), String> {
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .return_to_live();
    Ok(())
}

#[tauri::command]
fn inject_tokens(tokens: f64, state: State<'_, AppState>) -> Result<(), String> {
    state
        .fire
        .lock()
        .map_err(|_| "fire mutex poisoned")?
        .ingest(tokens, Some(UsageSource::Codex), Instant::now(), true);
    Ok(())
}

#[tauri::command]
fn check_updates(state: State<'_, AppState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "updater mutex poisoned")?
        .check();
    Ok(())
}

#[tauri::command]
fn download_update(state: State<'_, AppState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "updater mutex poisoned")?
        .download();
    Ok(())
}

#[tauri::command]
fn dismiss_update(state: State<'_, AppState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "updater mutex poisoned")?
        .dismiss();
    Ok(())
}

#[tauri::command]
fn open_releases() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(tokenblaze_core::updater::releases_url())
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("release page opening is unavailable on this platform".to_string())
    }
}

#[tauri::command]
fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "updater mutex poisoned")?
        .install_and_restart()
        .map_err(|error| error.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn open_console(app: AppHandle) -> Result<(), String> {
    let console = app
        .get_webview_window("console")
        .ok_or_else(|| "console window is unavailable".to_string())?;
    console.show().map_err(|error| error.to_string())?;
    console.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn start_dragging(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
fn set_flame_hover(
    expanded: bool,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
    let flame_size = config.flame_size;
    drop(config);
    #[cfg(windows)]
    {
        return apply_flame_hit_region(&window, flame_size, expanded)
            .map_err(|error| error.to_string());
    }
    #[cfg(not(windows))]
    let (width, height) = if expanded {
        expanded_flame_window_size(flame_size)
    } else {
        flame_size.content_size()
    };
    #[cfg(not(windows))]
    {
        let old_position = window.outer_position().map_err(|error| error.to_string())?;
        let old_size = window.outer_size().map_err(|error| error.to_string())?;
        let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
        let target_width = (f64::from(width) * scale_factor).round() as u32;
        let target_height = (f64::from(height) * scale_factor).round() as u32;
        let x = old_position.x + old_size.width as i32 - target_width as i32;
        let y = old_position.y + old_size.height as i32 - target_height as i32;
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|error| error.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())
    }
}

fn expanded_flame_window_size(size: FlameSize) -> (f32, f32) {
    let (width, height) = size.content_size();
    (width + 354.0, height.max(224.0))
}

#[cfg(windows)]
fn flame_window_host_position(
    window: &WebviewWindow,
    size: FlameSize,
    flame_anchor_x: i32,
    flame_anchor_y: i32,
) -> tauri::Result<tauri::PhysicalPosition<i32>> {
    let (flame_width, flame_height) = size.content_size();
    let (window_width, window_height) = expanded_flame_window_size(size);
    let scale_factor = window.scale_factor()?;
    let offset_x = (f64::from(window_width - flame_width) * scale_factor).round() as i32;
    let offset_y = (f64::from(window_height - flame_height) * scale_factor).round() as i32;
    Ok(tauri::PhysicalPosition::new(
        flame_anchor_x - offset_x,
        flame_anchor_y - offset_y,
    ))
}

#[cfg(windows)]
fn flame_anchor_position(window: &WebviewWindow, size: FlameSize) -> tauri::Result<(i32, i32)> {
    let position = window.outer_position()?;
    let (flame_width, flame_height) = size.content_size();
    let (window_width, window_height) = expanded_flame_window_size(size);
    let scale_factor = window.scale_factor()?;
    let offset_x = (f64::from(window_width - flame_width) * scale_factor).round() as i32;
    let offset_y = (f64::from(window_height - flame_height) * scale_factor).round() as i32;
    Ok((position.x + offset_x, position.y + offset_y))
}

#[cfg(windows)]
fn resize_flame_window(
    window: &WebviewWindow,
    size: FlameSize,
    expanded: bool,
) -> tauri::Result<()> {
    let (width, height) = if expanded {
        expanded_flame_window_size(size)
    } else {
        size.content_size()
    };
    let old_position = window.outer_position()?;
    let old_size = window.outer_size()?;
    let scale_factor = window.scale_factor()?;
    let target_width = (f64::from(width) * scale_factor).round() as u32;
    let target_height = (f64::from(height) * scale_factor).round() as u32;
    let x = old_position.x + old_size.width as i32 - target_width as i32;
    let y = old_position.y + old_size.height as i32 - target_height as i32;
    const SWP_NO_ZORDER: u32 = 0x0004;
    const SWP_NO_ACTIVATE: u32 = 0x0010;
    let hwnd = window.hwnd()?.0 as isize;
    let result = unsafe {
        set_window_pos(
            hwnd,
            0,
            x,
            y,
            target_width as i32,
            target_height as i32,
            SWP_NO_ZORDER | SWP_NO_ACTIVATE,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
unsafe extern "system" fn flame_frame_subclass(
    hwnd: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    subclass_id: usize,
    _reference_data: usize,
) -> isize {
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_NCCALCSIZE: u32 = 0x0083;
    const WM_NCPAINT: u32 = 0x0085;
    const WM_NCACTIVATE: u32 = 0x0086;

    match message {
        // Tao may restore caption styles later; keep the entire flame host as client area.
        WM_NCCALCSIZE | WM_NCPAINT => 0,
        WM_NCACTIVATE => 1,
        WM_NCDESTROY => {
            remove_window_subclass(hwnd, flame_frame_subclass, subclass_id);
            def_subclass_proc(hwnd, message, wparam, lparam)
        }
        _ => def_subclass_proc(hwnd, message, wparam, lparam),
    }
}

#[cfg(windows)]
fn install_flame_frame_subclass(window: &WebviewWindow) -> tauri::Result<()> {
    const DWMWA_NCRENDERING_POLICY: u32 = 2;
    const DWMNCRP_DISABLED: i32 = 1;

    let hwnd = window.hwnd()?.0 as isize;
    let result = unsafe { set_window_subclass(hwnd, flame_frame_subclass, 1, 0) };
    if result == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let status = unsafe {
        dwm_set_window_attribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &DWMNCRP_DISABLED,
            std::mem::size_of::<i32>() as u32,
        )
    };
    if status != 0 {
        return Err(std::io::Error::other(format!(
            "failed to disable flame window frame rendering: {status:#x}"
        ))
        .into());
    }
    remove_flame_nonclient_styles(window)
}

#[cfg(windows)]
fn remove_flame_nonclient_styles(window: &WebviewWindow) -> tauri::Result<()> {
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_CAPTION: i32 = 0x00C0_0000;
    const WS_THICKFRAME: i32 = 0x0004_0000;
    const WS_SYSMENU: i32 = 0x0008_0000;
    const WS_MINIMIZEBOX: i32 = 0x0002_0000;
    const WS_MAXIMIZEBOX: i32 = 0x0001_0000;
    const WS_EX_WINDOWEDGE: i32 = 0x0000_0100;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;

    let hwnd = window.hwnd()?.0 as isize;
    let style = unsafe { get_window_long(hwnd, GWL_STYLE) };
    let extended_style = unsafe { get_window_long(hwnd, GWL_EXSTYLE) };
    let frameless_style =
        style & !(WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX);
    let frameless_extended_style = extended_style & !WS_EX_WINDOWEDGE;

    unsafe {
        set_window_long(hwnd, GWL_STYLE, frameless_style);
        set_window_long(hwnd, GWL_EXSTYLE, frameless_extended_style);
    }

    let result = unsafe {
        set_window_pos(
            hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn apply_flame_hit_region(
    window: &WebviewWindow,
    size: FlameSize,
    expanded: bool,
) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0 as isize;
    if expanded {
        let window_size = window.outer_size()?;
        let region =
            unsafe { create_rect_rgn(0, 0, window_size.width as i32, window_size.height as i32) };
        if region == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let result = unsafe { set_window_rgn(hwnd, region, 1) };
        if result == 0 {
            unsafe { delete_object(region) };
            return Err(std::io::Error::last_os_error().into());
        }
        return Ok(());
    }

    let window_size = window.outer_size()?;
    let scale_factor = window.scale_factor()?;
    let scale = f64::from(size.pixel_scale());
    let region_width = ((56.0 * scale + 28.0) * scale_factor).round() as i32;
    let region_height = ((96.0 * scale + 36.0) * scale_factor).round() as i32;
    let right = window_size.width as i32;
    let bottom = window_size.height as i32;
    let region =
        unsafe { create_rect_rgn(right - region_width, bottom - region_height, right, bottom) };
    if region == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let result = unsafe { set_window_rgn(hwnd, region, 1) };
    if result == 0 {
        unsafe { delete_object(region) };
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

fn bottom_right_position(
    window: &WebviewWindow,
    monitor: &tauri::Monitor,
) -> tauri::Result<tauri::PhysicalPosition<i32>> {
    let window_size = window.outer_size()?;
    let work_area = monitor.work_area();
    let margin = (36.0 * monitor.scale_factor()) as i32;
    Ok(tauri::PhysicalPosition::new(
        work_area.position.x + work_area.size.width as i32 - window_size.width as i32 - margin,
        work_area.position.y + work_area.size.height as i32 - window_size.height as i32 - margin,
    ))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let settings_store = app.store(SETTINGS_FILE)?;
            let state = AppState::initialize(settings_store)?;
            let (show_flame, lang) = {
                let config = state.config.lock().map_err(|_| "config mutex poisoned")?;
                (config.panel_visible, config.language)
            };
            #[cfg(not(debug_assertions))]
            state
                .updater
                .lock()
                .map_err(|_| "updater mutex poisoned")?
                .check();
            app.manage(state);
            let background_app = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(15));
                let _ = background_app.state::<AppState>().tick();
                refresh_tray_usage(&background_app);
            });

            let toggle_flame = tauri::menu::MenuItem::with_id(
                app,
                "toggle-flame",
                t(
                    if show_flame {
                        "menu.hideFlame"
                    } else {
                        "menu.showFlame"
                    },
                    lang,
                ),
                true,
                None::<&str>,
            )?;
            let open_console = tauri::menu::MenuItem::with_id(
                app,
                "open-console",
                t("menu.openConsole", lang),
                true,
                None::<&str>,
            )?;
            let today_usage = tauri::menu::MenuItem::with_id(
                app,
                "today-usage",
                t("menu.todayUsage", lang),
                false,
                None::<&str>,
            )?;
            let reset_position = tauri::menu::MenuItem::with_id(
                app,
                "reset-position",
                t("menu.resetPosition", lang),
                true,
                None::<&str>,
            )?;
            let toggle_pause = tauri::menu::MenuItem::with_id(
                app,
                "toggle-pause",
                t("menu.pauseAnimation", lang),
                true,
                None::<&str>,
            )?;
            let check_updates = tauri::menu::MenuItem::with_id(
                app,
                "check-updates",
                t("menu.checkUpdates", lang),
                true,
                None::<&str>,
            )?;
            let quit = tauri::menu::MenuItem::with_id(
                app,
                "quit",
                t("menu.quit", lang),
                true,
                None::<&str>,
            )?;
            let separator_1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let separator_2 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let separator_3 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let menu = tauri::menu::Menu::with_items(
                app,
                &[
                    &toggle_flame,
                    &separator_1,
                    &today_usage,
                    &open_console,
                    &reset_position,
                    &toggle_pause,
                    &separator_2,
                    &check_updates,
                    &separator_3,
                    &quit,
                ],
            )?;
            app.state::<AppState>()
                .tray_items
                .lock()
                .map_err(|_| "tray menu mutex poisoned")?
                .replace(TrayItems {
                    toggle_flame: toggle_flame.clone(),
                    today_usage: today_usage.clone(),
                    open_console: open_console.clone(),
                    reset_position: reset_position.clone(),
                    toggle_pause: toggle_pause.clone(),
                    check_updates: check_updates.clone(),
                    quit: quit.clone(),
                });
            let toggle_flame_for_event = toggle_flame.clone();
            let toggle_pause_for_event = toggle_pause.clone();
            let icon = tauri::image::Image::new_owned(flame_icon_rgba(), 16, 16);
            tauri::tray::TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("TokenBlaze")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, event| {
                    if matches!(
                        event,
                        tauri::tray::TrayIconEvent::DoubleClick {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        }
                    ) {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("console") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(move |app, event| {
                    if event.id().as_ref() == "open-console" {
                        if let Some(window) = app.get_webview_window("console") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if event.id().as_ref() == "toggle-flame" {
                        let state = app.state::<AppState>();
                        let (currently_visible, language) = state
                            .config
                            .lock()
                            .map(|config| (config.panel_visible, config.language.effective()))
                            .unwrap_or((false, AppLanguage::English));
                        let next_visible = !currently_visible;
                        let _ = state.update_config(|config| config.panel_visible = next_visible);
                        let _ = toggle_flame_for_event.set_text(if next_visible {
                            t("menu.hideFlame", language)
                        } else {
                            t("menu.showFlame", language)
                        });
                        if let Some(window) = app.get_webview_window("flame") {
                            let _ = set_flame_visibility(&window, next_visible);
                        }
                    } else if event.id().as_ref() == "reset-position" {
                        if let Some(window) = app.get_webview_window("flame") {
                            if let Ok(Some(monitor)) = window.current_monitor() {
                                if let Ok(position) = bottom_right_position(&window, &monitor) {
                                    if window.set_position(position).is_ok() {
                                        let state = app.state::<AppState>();
                                        #[cfg(windows)]
                                        let anchor = state.config.lock().ok().and_then(|config| {
                                            flame_anchor_position(&window, config.flame_size).ok()
                                        });
                                        #[cfg(not(windows))]
                                        let anchor = Some((position.x, position.y));
                                        let _ = state.update_config(|config| {
                                            config.window_position =
                                                anchor.map(|(x, y)| (f64::from(x), f64::from(y)))
                                        });
                                    }
                                }
                            }
                        }
                    } else if event.id().as_ref() == "toggle-pause" {
                        let state = app.state::<AppState>();
                        let language = state
                            .config
                            .lock()
                            .map(|config| config.language.effective())
                            .unwrap_or(AppLanguage::English);
                        if let Ok(mut fire) = state.fire.lock() {
                            fire.animation_paused = !fire.animation_paused;
                            let key = if fire.animation_paused {
                                "menu.resumeAnimation"
                            } else {
                                "menu.pauseAnimation"
                            };
                            let _ = toggle_pause_for_event.set_text(t(key, language));
                        };
                    } else if event.id().as_ref() == "check-updates" {
                        if let Some(window) = app.get_webview_window("console") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let state = app.state::<AppState>();
                        if let Ok(mut updater) = state.updater.lock() {
                            updater.check();
                        };
                    } else if event.id().as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;
            refresh_tray_usage(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard_snapshot,
            usage_history,
            export_usage_events,
            cost_summary,
            set_cost_settings,
            initialize_flame_window,
            fire_visual_state,
            fire_frame,
            set_reduce_motion,
            set_sound_enabled,
            set_sound_volume,
            set_show_live_rate,
            set_token_poll_interval,
            set_panel_visible,
            set_flame_size,
            set_language,
            set_theme,
            set_source_color,
            reset_source_colors,
            rescan,
            set_retention_days,
            clear_usage_data,
            complete_onboarding,
            set_source_path,
            set_animation_paused,
            show_preview,
            show_custom_preview,
            return_to_live,
            inject_tokens,
            check_updates,
            download_update,
            dismiss_update,
            open_releases,
            install_update,
            open_console,
            start_dragging,
            set_flame_hover,
        ])
        .on_window_event(|window, event| {
            if window.label() == "console" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running TokenBlaze");
}

fn flame_icon_rgba() -> Vec<u8> {
    let mut pixels = vec![0u8; 16 * 16 * 4];
    for y in 2..15 {
        for x in 2..14 {
            let dx = (x as f32 - 8.0) / 6.0;
            let dy = (14.0 - y as f32) / 12.0;
            let outer = dx * dx + (dy - 0.42).powi(2) < 0.42;
            let inner = dx * dx + (dy - 0.25).powi(2) < 0.16;
            if outer || inner {
                let index = (y * 16 + x) * 4;
                pixels[index] = if inner { 255 } else { 238 };
                pixels[index + 1] = if inner { 210 } else { 92 };
                pixels[index + 2] = if inner { 80 } else { 18 };
                pixels[index + 3] = 255;
            }
        }
    }
    pixels
}

fn update_view(updater: &UpdateManager) -> snapshot::UpdateView {
    match updater.status() {
        UpdateStatus::Idle => snapshot::UpdateView::default(),
        UpdateStatus::Checking => update_state("checking", None, None, None),
        UpdateStatus::UpToDate => update_state("upToDate", None, None, None),
        UpdateStatus::Available { version } => {
            update_state("available", Some(version.clone()), None, None)
        }
        UpdateStatus::Incompatible { version } => {
            update_state("incompatible", Some(version.clone()), None, None)
        }
        UpdateStatus::Downloading {
            version,
            downloaded,
            total,
        } => update_state(
            "downloading",
            Some(version.clone()),
            total.map(|total| *downloaded as f64 / total.max(1) as f64),
            None,
        ),
        UpdateStatus::Ready { version } => update_state("ready", Some(version.clone()), None, None),
        UpdateStatus::Failed { message } => {
            update_state("failed", None, None, Some(message.clone()))
        }
    }
}

fn update_state(
    state: &str,
    version: Option<String>,
    progress: Option<f64>,
    message: Option<String>,
) -> snapshot::UpdateView {
    snapshot::UpdateView {
        state: state.to_string(),
        version,
        progress,
        message,
    }
}
