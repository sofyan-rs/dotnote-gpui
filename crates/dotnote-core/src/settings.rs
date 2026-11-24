//! User settings saved between launches, stored as JSON in the OS config folder
//! (e.g. `~/Library/Application Support/dotnote/settings.json` on macOS).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fs_ops::atomic_write;
use crate::recent_vaults::RecentVaults;

/// How the editor pane shows the current note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Formatted while typing: markers dimmed, headings and emphasis styled.
    #[default]
    Live,
    /// Raw markdown with syntax highlighting.
    Source,
    /// Source and preview side by side.
    Split,
    /// Read-only rendered document.
    Preview,
}

/// Light or dark colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Follow the system light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

/// An open tab, remembered between launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTab {
    pub path: PathBuf,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub preview: bool,
}

/// Everything Dotnote remembers between launches.
///
/// `#[serde(default)]` means a missing field (e.g. from an older version)
/// gets its default value instead of failing to load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The vault open when the app quit (reopened at launch).
    pub last_vault: Option<PathBuf>,
    /// Vaults opened before, newest first, each with its open tabs.
    pub recent_vaults: RecentVaults,
    pub view_mode: ViewMode,
    pub sidebar_width: Option<f32>,
    pub sidebar_visible: bool,
    /// Before 1.2, the tabs of `last_vault` were kept here; they now live in
    /// `recent_vaults`. Read once by `load_from`, never written.
    #[serde(skip_serializing)]
    open_tabs: Vec<SavedTab>,
    #[serde(skip_serializing)]
    active_tab: Option<usize>,
    /// Vaults with git sync turned on (the remote itself is in each vault's `.git`).
    pub synced_vaults: Vec<PathBuf>,
    /// Look for a new Dotnote release on GitHub at launch.
    pub check_for_updates: bool,
    /// Light, dark, or follow the system.
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            last_vault: None,
            recent_vaults: RecentVaults::default(),
            view_mode: ViewMode::default(),
            sidebar_width: None,
            sidebar_visible: true,
            open_tabs: Vec::new(),
            active_tab: None,
            synced_vaults: Vec::new(),
            check_for_updates: true,
            appearance: Appearance::System,
        }
    }
}

impl Settings {
    /// Where settings are stored: `$DOTNOTE_CONFIG_DIR/settings.json` if that
    /// variable is set (handy for a separate dev/test instance), otherwise the OS
    /// config folder. `None` if neither exists.
    pub fn default_path() -> Option<PathBuf> {
        let dir = match std::env::var_os("DOTNOTE_CONFIG_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => dirs::config_dir()?.join("dotnote"),
        };
        Some(dir.join("settings.json"))
    }

    /// Where the app kept settings when it was called KuNotes. Read once, when
    /// `default_path` has no file yet, so saved vaults survive the rename.
    pub fn old_default_path() -> Option<PathBuf> {
        if std::env::var_os("DOTNOTE_CONFIG_DIR").is_some_and(|dir| !dir.is_empty()) {
            return None;
        }
        Some(dirs::config_dir()?.join("kunotes").join("settings.json"))
    }

    /// Loads settings from `path`. A missing or broken file gives the defaults,
    /// so a bad settings file can never stop the app from starting.
    pub fn load_from(path: &Path) -> Settings {
        let mut settings: Settings = fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        settings.move_old_tabs();
        settings
    }

    /// Settings from before 1.2 had one vault's tabs at the top level. Move
    /// them into that vault's `recent_vaults` entry, so they still reopen.
    fn move_old_tabs(&mut self) {
        let tabs = std::mem::take(&mut self.open_tabs);
        let active = self.active_tab.take();
        if let Some(vault) = self.last_vault.clone() {
            if self.recent_vaults.get(&vault).is_none() {
                self.recent_vaults.remember(&vault);
            }
            if !tabs.is_empty() {
                self.recent_vaults.set_tabs(&vault, tabs, active);
            }
        }
    }

    /// Saves settings to `path`, creating the folder if needed.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        atomic_write(path, json.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_sidebar_in_live_mode() {
        let settings = Settings::default();
        assert!(settings.sidebar_visible);
        assert!(settings.check_for_updates);
        assert_eq!(settings.view_mode, ViewMode::Live);
        assert_eq!(settings.last_vault, None);
        assert_eq!(settings.appearance, Appearance::System);
    }

    #[test]
    fn appearance_round_trips_as_lowercase() {
        let settings: Settings = serde_json::from_str(r#"{ "appearance": "dark" }"#).unwrap();
        assert_eq!(settings.appearance, Appearance::Dark);
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains(r#""appearance":"dark""#));
    }

    #[test]
    fn missing_fields_use_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "view_mode": "split" }"#).unwrap();
        assert_eq!(settings.view_mode, ViewMode::Split);
        assert!(settings.sidebar_visible);
    }
}
