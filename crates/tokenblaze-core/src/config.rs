//! App configuration / preferences.
//!
//! Stored as JSON in the user's config directory.

use crate::fire::{FlameSize, SourceFlameColors};
use crate::l10n::AppLanguage;
use serde::{Deserialize, Deserializer, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub language: AppLanguage,
    pub flame_size: FlameSize,
    pub sound_enabled: bool,
    pub sound_volume: f32,
    pub reduce_motion: bool,
    pub show_live_rate: bool,
    pub token_poll_interval_seconds: u64,
    pub panel_visible: bool,
    pub window_position: Option<(f64, f64)>,
    #[serde(
        default = "default_source_colors",
        deserialize_with = "deserialize_source_colors"
    )]
    pub source_colors: [[u8; 3]; 7],
    #[serde(default)]
    pub source_paths: [Option<String>; 7],
}

fn default_source_colors() -> [[u8; 3]; 7] {
    SourceFlameColors::BUILT_IN
}

fn deserialize_source_colors<'de, D>(deserializer: D) -> Result<[[u8; 3]; 7], D::Error>
where
    D: Deserializer<'de>,
{
    let saved = Vec::<[u8; 3]>::deserialize(deserializer)?;
    let mut colors = SourceFlameColors::BUILT_IN;
    for (target, source) in colors.iter_mut().zip(saved) {
        *target = source;
    }
    Ok(colors)
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            language: AppLanguage::English,
            flame_size: FlameSize::Medium,
            sound_enabled: true,
            sound_volume: 0.48,
            reduce_motion: false,
            show_live_rate: true,
            token_poll_interval_seconds: 2,
            panel_visible: true,
            window_position: None,
            source_colors: SourceFlameColors::BUILT_IN,
            source_paths: std::array::from_fn(|_| None),
        }
    }
}

impl AppConfig {
    /// Load config from disk, or return defaults if not found.
    pub fn load() -> Self {
        let path = match config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return config,
                Err(error) => tracing::warn!(
                    path = %path.display(),
                    %error,
                    "Ignoring invalid TokenBlaze configuration"
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(
                path = %path.display(),
                %error,
                "Failed to read TokenBlaze configuration"
            ),
        }
        Self::default()
    }

    /// Save config to disk.
    pub fn save(&self) {
        if let Some(path) = config_path() {
            if let Err(error) = save_atomic(self, &path) {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "Failed to save TokenBlaze configuration"
                );
            }
        }
    }
}

fn save_atomic(config: &AppConfig, path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("configuration path has no parent directory"))?;
    std::fs::create_dir_all(parent)?;

    let temp_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let result = (|| -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(config)?;
        let mut temp = std::fs::File::create(&temp_path)?;
        temp.write_all(&json)?;
        temp.sync_all()?;
        replace_file(&temp_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

fn config_path() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("ai", "createfun", "TokenBlaze")?;
    Some(dirs.config_dir().join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::UsageSource;

    #[test]
    fn missing_fields_are_filled_from_defaults() {
        let config: AppConfig = serde_json::from_str(r#"{"sound_enabled":false}"#).unwrap();
        let defaults = AppConfig::default();

        assert!(!config.sound_enabled);
        assert_eq!(config.language, defaults.language);
        assert_eq!(config.flame_size, defaults.flame_size);
        assert_eq!(config.sound_volume, defaults.sound_volume);
        assert_eq!(
            config.token_poll_interval_seconds,
            defaults.token_poll_interval_seconds
        );
        assert_eq!(config.window_position, defaults.window_position);
        assert_eq!(config.source_colors, defaults.source_colors);
    }

    #[test]
    fn console_preferences_survive_serialization() {
        let mut config = AppConfig {
            language: AppLanguage::Chinese,
            flame_size: FlameSize::Large,
            sound_enabled: false,
            sound_volume: 0.25,
            reduce_motion: true,
            show_live_rate: false,
            token_poll_interval_seconds: 5,
            ..AppConfig::default()
        };
        config.source_colors[UsageSource::Codex.index()] = [12, 34, 56];

        let json = serde_json::to_string(&config).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.language, AppLanguage::Chinese);
        assert_eq!(restored.flame_size, FlameSize::Large);
        assert!(!restored.sound_enabled);
        assert_eq!(restored.sound_volume, 0.25);
        assert!(restored.reduce_motion);
        assert!(!restored.show_live_rate);
        assert_eq!(restored.token_poll_interval_seconds, 5);
        assert_eq!(
            restored.source_colors[UsageSource::Codex.index()],
            [12, 34, 56]
        );
    }

    #[test]
    fn atomic_save_replaces_existing_configuration() {
        let directory = std::env::temp_dir().join(format!(
            "tokenblaze-config-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.json");
        std::fs::write(&path, b"stale configuration").unwrap();

        let config = AppConfig {
            language: AppLanguage::Chinese,
            sound_volume: 0.35,
            ..AppConfig::default()
        };
        save_atomic(&config, &path).unwrap();

        let restored: AppConfig = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(restored.language, AppLanguage::Chinese);
        assert_eq!(restored.sound_volume, 0.35);
        assert!(!path
            .with_extension(format!("json.{}.tmp", std::process::id()))
            .exists());

        std::fs::remove_dir_all(directory).unwrap();
    }
}
