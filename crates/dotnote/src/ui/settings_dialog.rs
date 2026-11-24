//! The Settings dialog (Dotnote › Settings…, `secondary-,`): app-wide choices
//! (theme, update checks) and shortcuts to the open workspace's own dialogs
//! (git sync, notes password, recent list).
//!
//! It only gathers settings that already exist. Choices the app remembers by
//! itself (view mode, sidebar width) stay where they are used.

use dotnote_core::settings::Appearance;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::{
    AnyElement, App, Entity, IntoElement, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};

use crate::git_sync::GitSync;
use crate::settings_store::SettingsStore;
use crate::ui::workspace::set_appearance;
use crate::ui::{change_password_dialog, git_sync_dialog, segmented_control};
use crate::vault_lock::VaultLock;
use crate::vault_store::VaultStore;

/// What the dialog needs to reach: the open vault and its sync and lock.
#[derive(Clone)]
pub struct SettingsTargets {
    pub vault: Entity<VaultStore>,
    pub git_sync: Entity<GitSync>,
    pub vault_lock: Entity<VaultLock>,
}

/// Opens the dialog. Its content is rebuilt on every frame, so it always
/// shows the current settings without keeping a copy of them.
pub fn open(targets: SettingsTargets, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog.w(px(460.)).title("Settings").child(
            v_flex()
                .gap_5()
                .child(general_section(cx))
                .child(workspace_section(&targets, cx)),
        )
    });
}

fn general_section(cx: &App) -> impl IntoElement {
    let settings = SettingsStore::get(cx);
    let appearance = settings.appearance;
    let check_for_updates = settings.check_for_updates;

    let choices = [
        (
            "theme-system",
            "System",
            IconName::Monitor,
            Appearance::System,
        ),
        ("theme-light", "Light", IconName::Sun, Appearance::Light),
        ("theme-dark", "Dark", IconName::Moon, Appearance::Dark),
    ];
    // Same look as the editor's view mode switch.
    let theme = segmented_control::track(cx).children(choices.into_iter().map(
        |(id, label, icon, choice)| {
            segmented_control::segment(id, label, icon, choice == appearance, cx)
                .on_click(move |_, window, cx| set_appearance(choice, window, cx))
        },
    ));

    let updates = Switch::new("settings-check-updates")
        .small()
        .checked(check_for_updates)
        // A Switch only shows the value it is given: save the new one, then redraw.
        .on_change(|checked, window, cx| {
            let checked = *checked;
            SettingsStore::update(cx, |settings| settings.check_for_updates = checked);
            window.refresh(); // the dialog is rebuilt on the next frame
        });

    section(
        "General",
        vec![
            row("Theme", None, theme.into_any_element(), cx),
            row(
                "Check for updates at launch",
                Some("Dotnote looks for a new release on GitHub when it starts."),
                updates.into_any_element(),
                cx,
            ),
        ],
        cx,
    )
}

fn workspace_section(targets: &SettingsTargets, cx: &App) -> impl IntoElement {
    let vault = targets.vault.read(cx);
    let Some(root) = vault.root() else {
        let hint = div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child("Open a workspace to see its settings.");
        return section("Workspace", vec![hint.into_any_element()], cx);
    };
    let location = row(
        vault.name().unwrap_or_default().as_str(),
        Some(&root.display().to_string()),
        div().into_any_element(),
        cx,
    );

    let sync_state = match targets.git_sync.read(cx).remote() {
        Some(_) => "On",
        None => "Off",
    };
    let git_sync = targets.git_sync.clone();
    let sync_button = Button::new("settings-git-sync")
        .small()
        .label("Git Sync…")
        .on_click(move |_, window, cx| {
            // Close this dialog first, so the next one isn't stacked on top.
            window.close_dialog(cx);
            git_sync_dialog::open(git_sync.clone(), window, cx);
        });

    let vault_lock = targets.vault_lock.clone();
    let password_button = Button::new("settings-password")
        .small()
        .label("Change…")
        .on_click(move |_, window, cx| {
            window.close_dialog(cx);
            change_password_dialog::open(vault_lock.clone(), window, cx);
        });

    let others = SettingsStore::get(cx)
        .recent_vaults
        .list()
        .iter()
        .filter(|entry| entry.path != root)
        .count();
    let clear_vault = targets.vault.clone();
    let clear_button = Button::new("settings-clear-recent")
        .small()
        .label("Clear")
        .disabled(others == 0)
        .on_click(move |_, window, cx| {
            clear_vault.update(cx, |vault, cx| vault.clear_recent(cx));
            window.refresh(); // redraw the dialog with the shorter list
        });
    let recent_hint = match others {
        0 => "No other workspaces in the list.".to_string(),
        1 => "1 other workspace in the switcher list.".to_string(),
        count => format!("{count} other workspaces in the switcher list."),
    };

    section(
        "Workspace",
        vec![
            location,
            row(
                "Git sync",
                Some(sync_state),
                sync_button.into_any_element(),
                cx,
            ),
            row(
                "Notes password",
                Some("Protects locked notes in this workspace."),
                password_button.into_any_element(),
                cx,
            ),
            row(
                "Recent workspaces",
                Some(&recent_hint),
                clear_button.into_any_element(),
                cx,
            ),
        ],
        cx,
    )
}

/// A titled group of rows.
fn section(title: &str, rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    v_flex()
        .gap_3()
        .child(
            div()
                .text_xs()
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(cx.theme().muted_foreground)
                .child(title.to_uppercase()),
        )
        .children(rows)
        .into_any_element()
}

/// A label (with an optional muted hint below it) on the left, a control on the right.
fn row(label: &str, hint: Option<&str>, control: AnyElement, cx: &App) -> AnyElement {
    let text = v_flex()
        .flex_1()
        .min_w_0()
        .child(div().text_sm().truncate().child(label.to_string()))
        .children(hint.map(|hint| {
            div()
                .text_xs()
                .truncate()
                .text_color(cx.theme().muted_foreground)
                .child(hint.to_string())
        }));
    h_flex()
        .gap_4()
        .items_center()
        .child(text)
        .child(control)
        .into_any_element()
}
