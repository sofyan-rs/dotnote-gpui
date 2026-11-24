//! The workspace switcher at the top of the sidebar: the open vault's name.
//! Click it for a menu of recent workspaces (switch with one click), Open
//! Workspace…, and Close Workspace. "Workspace" is the UI word for a vault.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex};
use gpui_kit::{
    App, Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div, px,
};

use crate::settings_store::SettingsStore;
use crate::ui::dialogs;
use crate::vault_store::VaultStore;

pub fn render(vault: &Entity<VaultStore>, cx: &App) -> impl IntoElement {
    let name = vault
        .read(cx)
        .name()
        .unwrap_or_else(|| "No Workspace".to_string());
    let theme = cx.theme();
    let menu_vault = vault.clone();

    h_flex().px_2().pt_2().pb_1().child(
        Button::new("vault-switcher")
            .ghost()
            .small()
            .w_full()
            .tooltip("Switch Workspace")
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::Folder).text_color(theme.muted_foreground))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_semibold()
                            .child(name),
                    )
                    .child(
                        Icon::new(IconName::ChevronsUpDown)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .dropdown_menu(move |menu, window, cx| build_menu(menu, &menu_vault, window, cx)),
    )
}

/// The switcher's menu, built each time it opens so it shows the current list.
fn build_menu(
    menu: PopupMenu,
    vault: &Entity<VaultStore>,
    _: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let open_root = vault.read(cx).root().map(|root| root.to_path_buf());
    let recent = SettingsStore::get(cx).recent_vaults.clone();
    let mut menu = menu.min_w(px(220.)).max_w(px(360.));

    if !recent.is_empty() {
        menu = menu.label("Recent Workspaces");
    }
    for (entry, label) in recent.list().iter().zip(recent.labels()) {
        let is_open = open_root.as_deref() == Some(entry.path.as_path());
        let (vault, path) = (vault.clone(), entry.path.clone());
        menu = menu.item(
            PopupMenuItem::new(label)
                .checked(is_open)
                .on_click(move |_, _, cx| {
                    let path = path.clone();
                    vault.update(cx, |vault, cx| vault.open_recent(path, cx));
                }),
        );
    }
    if !recent.is_empty() {
        menu = menu.separator();
    }

    let picker_vault = vault.clone();
    menu = menu.item(
        PopupMenuItem::new("Open Workspace…").on_click(move |_, window, cx| {
            dialogs::pick_vault_folder(picker_vault.clone(), window, cx)
        }),
    );
    if open_root.is_some() {
        let close_vault = vault.clone();
        menu = menu.item(
            PopupMenuItem::new("Close Workspace").on_click(move |_, _, cx| {
                close_vault.update(cx, |vault, cx| vault.close_vault(cx))
            }),
        );
    }
    menu
}
