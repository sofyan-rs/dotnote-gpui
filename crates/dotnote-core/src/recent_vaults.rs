//! The vaults opened before ("recent workspaces" in the UI), newest first, each
//! with the tabs it had open. Switching back to a vault reopens its tabs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::settings::SavedTab;

/// How many vaults the list remembers. Older ones drop off the end.
pub const MAX_RECENT_VAULTS: usize = 10;

/// A vault opened before, and the tabs it had open when it was left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentVault {
    pub path: PathBuf,
    #[serde(default)]
    pub open_tabs: Vec<SavedTab>,
    #[serde(default)]
    pub active_tab: Option<usize>,
}

impl RecentVault {
    fn new(path: &Path) -> Self {
        RecentVault {
            path: path.to_path_buf(),
            open_tabs: Vec::new(),
            active_tab: None,
        }
    }
}

/// The list itself. Saved in settings as a plain JSON array
/// (`#[serde(transparent)]` means "serialize the inner `Vec` directly").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecentVaults(Vec<RecentVault>);

impl RecentVaults {
    /// Newest first.
    pub fn list(&self) -> &[RecentVault] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The vault at `path`, if it is in the list.
    pub fn get(&self, path: &Path) -> Option<&RecentVault> {
        self.0.iter().find(|vault| vault.path == path)
    }

    /// Moves `path` to the front (adding it if new), keeping its saved tabs.
    pub fn remember(&mut self, path: &Path) {
        let entry = match self.0.iter().position(|vault| vault.path == path) {
            Some(index) => self.0.remove(index),
            None => RecentVault::new(path),
        };
        self.0.insert(0, entry);
        self.0.truncate(MAX_RECENT_VAULTS);
    }

    /// Removes `path` from the list (e.g. its folder is gone).
    pub fn forget(&mut self, path: &Path) {
        self.0.retain(|vault| vault.path != path);
    }

    /// Removes every vault except `keep` (the open one), which keeps its tabs.
    pub fn clear_except(&mut self, keep: Option<&Path>) {
        self.0.retain(|vault| Some(vault.path.as_path()) == keep);
    }

    /// Remembers the tabs open in `path`. Adds the vault (at the front) if missing.
    pub fn set_tabs(&mut self, path: &Path, open_tabs: Vec<SavedTab>, active_tab: Option<usize>) {
        if self.get(path).is_none() {
            self.remember(path);
        }
        if let Some(vault) = self.0.iter_mut().find(|vault| vault.path == path) {
            vault.open_tabs = open_tabs;
            vault.active_tab = active_tab;
        }
    }

    /// A label per vault, in list order: the folder name, plus its parent
    /// folder in parentheses when two vaults share a name.
    pub fn labels(&self) -> Vec<String> {
        let names: Vec<String> = self
            .0
            .iter()
            .map(|vault| folder_name(&vault.path))
            .collect();
        self.0
            .iter()
            .zip(&names)
            .map(|(vault, name)| {
                let shared = names.iter().filter(|other| *other == name).count() > 1;
                match vault.path.parent() {
                    Some(parent) if shared => format!("{name} ({})", parent.display()),
                    _ => name.clone(),
                }
            })
            .collect()
    }
}

/// The last part of a path, or the whole path for a drive root like `C:\`.
fn folder_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(name: &str) -> SavedTab {
        SavedTab {
            path: PathBuf::from(name),
            pinned: false,
            preview: false,
        }
    }

    fn paths(recent: &RecentVaults) -> Vec<PathBuf> {
        recent
            .list()
            .iter()
            .map(|vault| vault.path.clone())
            .collect()
    }

    #[test]
    fn remember_moves_to_front_without_duplicates() {
        let mut recent = RecentVaults::default();
        recent.remember(Path::new("a"));
        recent.remember(Path::new("b"));
        recent.remember(Path::new("a"));
        assert_eq!(paths(&recent), [PathBuf::from("a"), PathBuf::from("b")]);
    }

    #[test]
    fn remember_keeps_saved_tabs() {
        let mut recent = RecentVaults::default();
        recent.set_tabs(Path::new("a"), vec![tab("a/x.md")], Some(0));
        recent.remember(Path::new("b"));
        recent.remember(Path::new("a"));
        let a = recent.get(Path::new("a")).unwrap();
        assert_eq!(a.open_tabs, [tab("a/x.md")]);
        assert_eq!(a.active_tab, Some(0));
    }

    #[test]
    fn list_is_capped() {
        let mut recent = RecentVaults::default();
        for i in 0..MAX_RECENT_VAULTS + 3 {
            recent.remember(Path::new(&format!("v{i}")));
        }
        assert_eq!(recent.list().len(), MAX_RECENT_VAULTS);
        assert_eq!(
            recent.list()[0].path,
            PathBuf::from(format!("v{}", MAX_RECENT_VAULTS + 2))
        );
    }

    #[test]
    fn forget_and_clear() {
        let mut recent = RecentVaults::default();
        for name in ["a", "b", "c"] {
            recent.remember(Path::new(name));
        }
        recent.forget(Path::new("b"));
        assert_eq!(paths(&recent), [PathBuf::from("c"), PathBuf::from("a")]);
        recent.clear_except(Some(Path::new("a")));
        assert_eq!(paths(&recent), [PathBuf::from("a")]);
        recent.clear_except(None);
        assert!(recent.is_empty());
    }

    #[test]
    fn labels_add_the_parent_only_for_shared_names() {
        let mut recent = RecentVaults::default();
        let work = Path::new("work").join("Notes");
        let home = Path::new("home").join("Notes");
        recent.remember(&work);
        recent.remember(&home);
        recent.remember(Path::new("Journal 📓"));
        assert_eq!(
            recent.labels(),
            [
                "Journal 📓".to_string(),
                format!("Notes ({})", Path::new("home").display()),
                format!("Notes ({})", Path::new("work").display()),
            ]
        );
    }

    #[test]
    fn saves_as_a_plain_array() {
        let mut recent = RecentVaults::default();
        recent.remember(Path::new("a"));
        let json = serde_json::to_string(&recent).unwrap();
        assert!(json.starts_with('['), "{json}");
        let back: RecentVaults = serde_json::from_str(&json).unwrap();
        assert_eq!(back, recent);
    }
}
