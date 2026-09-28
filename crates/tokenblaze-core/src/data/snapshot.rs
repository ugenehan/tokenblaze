//! Shared, local dashboard snapshot exchanged by the native host and Tauri UI.

use crate::config::AppConfig;
use crate::data::{HourlyUsage, SourceConnectionState, UsageBreakdown, UsageMonitor, UsageSource};
use crate::fire::{FirePhase, FireStateMachine, FireTier, SourceFlameColors};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    #[serde(default)]
    pub debug_tools_enabled: bool,
    pub today_tokens: i64,
    pub today_by_source: [i64; 7],
    #[serde(default)]
    pub estimated_by_source: [i64; 7],
    pub hourly: Vec<HourlyPoint>,
    #[serde(default)]
    pub last_seven_days: Vec<DailyPoint>,
    pub breakdown: UsageBreakdown,
    pub fire: FireView,
    pub sources: Vec<SourceView>,
    #[serde(default)]
    pub is_rescanning: bool,
    pub config: ConfigView,
    #[serde(default)]
    pub update: UpdateView,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HourlyPoint {
    pub hour: u32,
    pub tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPoint {
    pub date: String,
    pub tokens: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FireView {
    /// Live flame intensity for geometry; historical daily totals do not raise it.
    pub intensity: f64,
    pub fuel: f64,
    pub ember_heat: f64,
    pub spark_burst: f64,
    pub phase: FirePhaseView,
    pub tier: FireTierView,
    pub color_mix: [f64; 7],
    pub palette: [[u8; 3]; 5],
    #[serde(default)]
    pub tokens_per_second: f64,
    #[serde(default)]
    pub animation_paused: bool,
    #[serde(default)]
    pub previewing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FirePhaseView {
    Unlit,
    Flame,
    Ember,
    Out,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FireTierView {
    Hush,
    Glow,
    Crackle,
    Roar,
    Blaze,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceView {
    pub id: String,
    pub name: String,
    pub tokens: i64,
    pub state: String,
    pub detail: String,
    #[serde(default)]
    pub last_read_at: Option<DateTime<Local>>,
    #[serde(default)]
    pub estimated_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    pub sound_enabled: bool,
    pub sound_volume: f32,
    pub reduce_motion: bool,
    pub show_live_rate: bool,
    pub panel_visible: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_flame_size")]
    pub flame_size: String,
    #[serde(default = "default_token_poll_interval_seconds")]
    pub token_poll_interval_seconds: u64,
    #[serde(default = "default_source_colors")]
    pub source_colors: [[u8; 3]; 7],
    #[serde(default)]
    pub source_paths: [Option<String>; 7],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    pub state: String,
    pub version: Option<String>,
    pub progress: Option<f64>,
    pub message: Option<String>,
}

impl Default for UpdateView {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            version: None,
            progress: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewStyleView {
    Out,
    Ember,
    Hush,
    Glow,
    Crackle,
    Roar,
    Blaze,
}

/// One-shot actions sent by the attached dashboard to the native host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum DashboardCommand {
    Rescan,
    SetAnimationPaused { paused: bool },
    ShowPreview { style: PreviewStyleView },
    ShowCustomPreview { intensity: f64 },
    ReturnToLive,
    InjectTokens { tokens: f64 },
    CheckUpdates,
    DownloadUpdate,
    DismissUpdate,
    InstallUpdate,
}

pub fn from_runtime(
    monitor: &UsageMonitor,
    fire: &FireStateMachine,
    config: &AppConfig,
) -> DashboardSnapshot {
    DashboardSnapshot {
        debug_tools_enabled: cfg!(debug_assertions),
        today_tokens: monitor.today_tokens(),
        today_by_source: monitor.today_by_source(),
        estimated_by_source: monitor.today_estimated_by_source(),
        hourly: monitor
            .today_hourly()
            .iter()
            .copied()
            .map(|HourlyUsage { hour, tokens }| HourlyPoint { hour, tokens })
            .collect(),
        last_seven_days: monitor
            .last_seven_days()
            .iter()
            .map(|day| DailyPoint {
                date: day.date.clone(),
                tokens: day.tokens,
            })
            .collect(),
        breakdown: monitor.today_breakdown(),
        fire: FireView::from_runtime(fire),
        sources: UsageSource::ALL
            .iter()
            .map(|source| {
                let status = monitor.statuses().get(source);
                SourceView {
                    id: source.display_name().to_lowercase().replace(' ', "-"),
                    name: source.display_name().to_string(),
                    tokens: status.map(|item| item.today_tokens).unwrap_or(0),
                    state: status
                        .map(|item| state_label(item.state))
                        .unwrap_or("notFound")
                        .to_string(),
                    detail: status.map(|item| item.detail.clone()).unwrap_or_default(),
                    last_read_at: status.and_then(|item| item.last_read_at),
                    estimated_tokens: status.map(|item| item.estimated_tokens).unwrap_or(0),
                }
            })
            .collect(),
        is_rescanning: monitor.is_rescanning(),
        config: config_view(config),
        update: UpdateView::default(),
    }
}

pub fn config_view(config: &AppConfig) -> ConfigView {
    ConfigView {
        sound_enabled: config.sound_enabled,
        sound_volume: config.sound_volume,
        reduce_motion: config.reduce_motion,
        show_live_rate: config.show_live_rate,
        panel_visible: config.panel_visible,
        language: language_label(config.language).to_string(),
        theme: match config.theme {
            crate::config::AppTheme::System => "System",
            crate::config::AppTheme::Dark => "Dark",
            crate::config::AppTheme::Light => "Light",
        }
        .to_string(),
        flame_size: flame_size_label(config.flame_size).to_string(),
        token_poll_interval_seconds: config.token_poll_interval_seconds,
        source_colors: config.source_colors,
        source_paths: config.source_paths.clone(),
    }
}

pub fn path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("ai", "createfun", "TokenBlaze")?;
    Some(dirs.data_dir().join("dashboard.json"))
}

pub fn write(snapshot: &DashboardSnapshot) -> anyhow::Result<()> {
    let path = path().ok_or_else(|| anyhow::anyhow!("TokenBlaze data directory is unavailable"))?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("snapshot path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec(snapshot)?)?;
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::rename(temp, path)?;
    Ok(())
}

/// Queue an action for the native host. Each file is consumed at most once.
#[allow(dead_code)]
pub fn send_command(command: &DashboardCommand) -> anyhow::Result<()> {
    let directory = command_path()?;
    std::fs::create_dir_all(&directory)?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = directory.join(format!("{}-{nonce}.json", std::process::id()));
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, serde_json::to_vec(command)?)?;
    std::fs::rename(temp, path)?;
    Ok(())
}

/// Drain actions queued by attached dashboards.
pub fn drain_commands() -> anyhow::Result<Vec<DashboardCommand>> {
    let directory = command_path()?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut commands = Vec::new();
    for path in paths {
        match std::fs::read(&path)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(anyhow::Error::from))
        {
            Ok(command) => commands.push(command),
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "Ignoring invalid dashboard command")
            }
        }
        if let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), %error, "Failed to remove dashboard command");
        }
    }
    Ok(commands)
}

#[allow(dead_code)]
pub fn read() -> anyhow::Result<DashboardSnapshot> {
    let path = path().ok_or_else(|| anyhow::anyhow!("TokenBlaze data directory is unavailable"))?;
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn state_label(state: SourceConnectionState) -> &'static str {
    match state {
        SourceConnectionState::Ok => "ok",
        SourceConnectionState::NotFound => "notFound",
        SourceConnectionState::NoPermission => "noPermission",
        SourceConnectionState::Unsupported => "unsupported",
        SourceConnectionState::ReadError => "readError",
    }
}

impl FireView {
    fn from_runtime(fire: &FireStateMachine) -> Self {
        let snapshot = fire.snapshot();
        Self {
            intensity: fire.visual_intensity(),
            fuel: snapshot.fuel,
            ember_heat: snapshot.ember_heat,
            spark_burst: snapshot.spark_burst,
            phase: snapshot.phase.into(),
            tier: snapshot.tier.into(),
            color_mix: snapshot.color_mix.weights,
            palette: [
                snapshot.color_mix.heat_to_rgb(0.0),
                snapshot.color_mix.heat_to_rgb(0.4),
                snapshot.color_mix.heat_to_rgb(0.65),
                snapshot.color_mix.heat_to_rgb(0.82),
                snapshot.color_mix.heat_to_rgb(0.98),
            ],
            tokens_per_second: fire.tokens_per_second(),
            animation_paused: fire.animation_paused,
            previewing: fire.is_previewing(),
        }
    }
}

pub fn fire_view(fire: &FireStateMachine) -> FireView {
    FireView::from_runtime(fire)
}

fn command_path() -> anyhow::Result<PathBuf> {
    let path = path().ok_or_else(|| anyhow::anyhow!("TokenBlaze data directory is unavailable"))?;
    Ok(path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("snapshot path has no parent"))?
        .join("commands"))
}

fn default_language() -> String {
    "English".to_string()
}

fn default_theme() -> String {
    "Dark".to_string()
}

fn default_flame_size() -> String {
    "Medium".to_string()
}

fn default_token_poll_interval_seconds() -> u64 {
    2
}

fn default_source_colors() -> [[u8; 3]; 7] {
    SourceFlameColors::BUILT_IN
}

fn language_label(language: crate::l10n::AppLanguage) -> &'static str {
    use crate::l10n::AppLanguage::*;
    match language {
        System => "System",
        English => "English",
        Chinese => "Chinese",
        Japanese => "Japanese",
        Korean => "Korean",
    }
}

fn flame_size_label(size: crate::fire::FlameSize) -> &'static str {
    match size {
        crate::fire::FlameSize::Small => "Small",
        crate::fire::FlameSize::Medium => "Medium",
        crate::fire::FlameSize::Large => "Large",
    }
}

impl From<FirePhase> for FirePhaseView {
    fn from(value: FirePhase) -> Self {
        match value {
            FirePhase::Unlit => Self::Unlit,
            FirePhase::Flame => Self::Flame,
            FirePhase::Ember => Self::Ember,
            FirePhase::Out => Self::Out,
        }
    }
}
impl From<FireTier> for FireTierView {
    fn from(value: FireTier) -> Self {
        match value {
            FireTier::Hush => Self::Hush,
            FireTier::Glow => Self::Glow,
            FireTier::Crackle => Self::Crackle,
            FireTier::Roar => Self::Roar,
            FireTier::Blaze => Self::Blaze,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fire::FirePreviewStyle;

    #[test]
    fn historical_totals_do_not_keep_flame_lit_but_preview_still_works() {
        let mut fire = FireStateMachine::new();
        fire.update_today_tokens(600_000, [0; 7]);

        assert_eq!(fire.snapshot().intensity, 0.0);
        assert!(matches!(fire.snapshot().phase, FirePhase::Unlit));
        assert_eq!(fire_view(&fire).intensity, 0.0);

        fire.show_preview(FirePreviewStyle::Blaze);
        assert_eq!(
            fire_view(&fire).intensity,
            FirePreviewStyle::Blaze.snapshot().intensity
        );
    }

    #[test]
    fn config_view_uses_frontend_field_names() {
        let value = serde_json::to_value(config_view(&AppConfig::default())).unwrap();
        assert!(value.get("soundEnabled").is_some());
        assert_eq!(
            value
                .get("tokenPollIntervalSeconds")
                .and_then(serde_json::Value::as_u64),
            Some(2)
        );
        assert!(value.get("flameSize").is_some());
        assert_eq!(
            value.get("theme").and_then(serde_json::Value::as_str),
            Some("Dark")
        );
        assert!(value.get("sourceColors").is_some());
        assert!(value.get("sound_enabled").is_none());
    }

    #[test]
    fn dashboard_command_round_trips() {
        let command = DashboardCommand::ShowPreview {
            style: PreviewStyleView::Blaze,
        };
        let json = serde_json::to_vec(&command).unwrap();
        let restored: DashboardCommand = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored, command);
    }
}
