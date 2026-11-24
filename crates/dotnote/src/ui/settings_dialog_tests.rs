//! UI tests for the Settings dialog: it opens with `secondary-,`'s action, and
//! its controls change the saved settings.

use dotnote_core::settings::Appearance;
use gpui_kit::TestAppContext;
use gpui_kit::component::WindowExt as _;
use gpui_kit::test::TestWindowExt as _;

use crate::actions::OpenSettings;
use crate::settings_store::SettingsStore;
use crate::ui::test_helpers::{in_window, setup};

#[gpui_kit::test]
fn settings_change_theme_and_update_checks(cx: &mut TestAppContext) {
    let s = setup(cx);
    in_window(&s, cx, |window, cx| {
        window.dispatch_action(Box::new(OpenSettings), cx)
    });
    in_window(&s, cx, |window, cx| assert!(window.has_active_dialog(cx)));

    in_window(&s, cx, |window, cx| window.click("theme-dark", cx));
    in_window(&s, cx, |window, cx| {
        window.click("settings-check-updates", cx)
    });

    let (appearance, check) = cx.update(|cx| {
        let settings = SettingsStore::get(cx);
        (settings.appearance, settings.check_for_updates)
    });
    assert_eq!(appearance, Appearance::Dark);
    assert!(!check, "the switch turns update checks off");

    // Clicking again turns it back on: the dialog shows the saved value.
    in_window(&s, cx, |window, cx| {
        window.click("settings-check-updates", cx)
    });
    assert!(cx.update(|cx| SettingsStore::get(cx).check_for_updates));
}
