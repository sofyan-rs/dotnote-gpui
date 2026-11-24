//! Integration tests for saving and loading settings, using a real temporary folder.

use std::fs;
use std::path::Path;

use dotnote_core::recent_vaults::RecentVaults;
use dotnote_core::settings::{Appearance, SavedTab, Settings, ViewMode};
use tempfile::tempdir;

#[test]
fn settings_round_trip_and_fallback() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("settings.json");

    // Missing file -> defaults.
    assert_eq!(Settings::load_from(&path), Settings::default());

    let mut recent_vaults = RecentVaults::default();
    recent_vaults.remember(&dir.path().join("other"));
    recent_vaults.set_tabs(
        dir.path(),
        vec![SavedTab {
            path: dir.path().join("a.md"),
            pinned: true,
            preview: false,
        }],
        Some(0),
    );
    // Built field by field: the struct has private fields, so a literal won't compile here.
    let mut settings = Settings::default();
    settings.last_vault = Some(dir.path().to_path_buf());
    settings.recent_vaults = recent_vaults;
    settings.view_mode = ViewMode::Preview;
    settings.sidebar_width = Some(280.0);
    settings.sidebar_visible = false;
    settings.synced_vaults = vec![dir.path().to_path_buf()];
    settings.check_for_updates = false;
    settings.appearance = Appearance::Dark;
    settings.save_to(&path).unwrap();
    assert_eq!(Settings::load_from(&path), settings);

    // Broken file -> defaults instead of an error.
    fs::write(&path, "{ not json").unwrap();
    assert_eq!(Settings::load_from(&path), Settings::default());
}

#[test]
fn tabs_from_before_recent_vaults_move_to_the_last_vault() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let json = r#"{
        "last_vault": "notes",
        "open_tabs": [{ "path": "notes/a.md", "pinned": true }],
        "active_tab": 0
    }"#;
    fs::write(&path, json).unwrap();

    let settings = Settings::load_from(&path);
    let vault = settings.recent_vaults.get(Path::new("notes")).unwrap();
    assert_eq!(vault.open_tabs.len(), 1);
    assert!(vault.open_tabs[0].pinned);
    assert_eq!(vault.active_tab, Some(0));

    // Saved again, the old top-level fields are gone and nothing is lost.
    settings.save_to(&path).unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(saved.get("open_tabs").is_none(), "{saved}");
    assert_eq!(Settings::load_from(&path), settings);
}
