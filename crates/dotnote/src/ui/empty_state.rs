//! Placeholder screens shown when no vault (workspace) or no note is open.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Icon, v_flex};
use gpui_kit::{App, Entity, IntoElement, ParentElement as _, Styled as _, div, px};

use crate::actions::OpenVault;
use crate::settings_store::SettingsStore;
use crate::vault_store::VaultStore;

/// "No Workspace Selected" with an Open Workspace button and, below it, the
/// recent workspaces to reopen with one click.
pub fn no_vault(vault: &Entity<VaultStore>, cx: &App) -> impl IntoElement {
    let recent = SettingsStore::get(cx).recent_vaults.clone();
    let recent_buttons =
        recent
            .list()
            .iter()
            .zip(recent.labels())
            .enumerate()
            .map(|(index, (entry, label))| {
                let (vault, path) = (vault.clone(), entry.path.clone());
                Button::new(("recent-vault", index))
                    .ghost()
                    .icon(IconName::Folder)
                    .label(label)
                    .tooltip(entry.path.display().to_string())
                    .on_click(move |_, _, cx| {
                        let path = path.clone();
                        vault.update(cx, |vault, cx| vault.open_recent(path, cx));
                    })
            });
    let recent_list = (!recent.is_empty()).then(|| {
        v_flex()
            .pt_4()
            .gap_1()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Recent Workspaces"),
            )
            .children(recent_buttons)
    });

    message(
        IconName::FolderPlus,
        "No Workspace Selected",
        "Open a folder to use it as your workspace.",
        cx,
    )
    .child(
        Button::new("open-vault")
            .primary()
            .label("Open Workspace…")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenVault), cx)),
    )
    .children(recent_list)
}

/// "No File Selected", shown while a vault is open but no note is.
pub fn no_file_selected(cx: &App) -> impl IntoElement {
    message(
        IconName::NotebookPen,
        "No File Selected",
        "Select a file from the sidebar to start writing.",
        cx,
    )
}

/// The shared layout: big muted icon, title, and a short hint, centered.
fn message(icon: IconName, title: &str, hint: &str, cx: &App) -> gpui_kit::Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            Icon::new(icon)
                .size(px(48.))
                .text_color(cx.theme().muted_foreground.opacity(0.5)),
        )
        .child(
            div()
                .text_lg()
                .text_color(cx.theme().muted_foreground)
                .child(title.to_string()),
        )
        .child(
            div()
                .max_w(px(260.))
                .text_sm()
                .text_center()
                .text_color(cx.theme().muted_foreground.opacity(0.8))
                .child(hint.to_string()),
        )
}
