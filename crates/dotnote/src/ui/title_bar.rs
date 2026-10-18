//! The window title bar: sidebar toggle, app menu (Windows/Linux), and centered title.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, TitleBar, h_flex};
use gpui_kit::{
    App, ClickEvent, Entity, Global, InteractiveElement as _, IntoElement, ParentElement as _,
    Styled as _, Window, prelude::FluentBuilder as _,
};

use crate::actions::{ToggleSidebar, WORKSPACE};
use crate::platform;

/// The in-window menu bar, created once and shared (Windows and Linux only).
struct MenuBar(Entity<AppMenuBar>);
impl Global for MenuBar {}

/// `on_toggle` runs the sidebar toggle directly. Dispatching the action would
/// need keyboard focus inside the workspace, which a title bar click may not have.
pub fn render(
    title: Option<String>,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    _window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let menu_bar = (!platform::uses_native_menu_bar()).then(|| menu_bar(cx));
    // Room for the toggle + in-window menu (Windows/Linux) or the traffic lights (macOS).
    let title_inset = if menu_bar.is_some() { 260.0 } else { 110.0 };

    TitleBar::new()
        // gpui-kit pads the left side by 12px on Windows/Linux. Match the gap above and
        // below the 24px toggle button instead, so it sits evenly in the corner.
        // macOS keeps the default room for the traffic lights.
        .when(menu_bar.is_some(), |bar| bar.pl(gpui_kit::px(5.0)))
        .child(
            h_flex()
                .relative()
                .w_full()
                .pr_2()
                .gap_1()
                // Absolute overlay, drawn first so button and menu stay clickable on top.
                // Centers the title on the whole bar, like macOS.
                .child(
                    h_flex()
                        .absolute()
                        .inset_0()
                        .justify_center()
                        .items_center()
                        // Side padding keeps a long title clear of the button and menu.
                        .px(gpui_kit::px(title_inset))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            gpui_kit::div()
                                .truncate()
                                .child(title.unwrap_or_else(|| "Dotnote".to_string())),
                        ),
                )
                .child(
                    // `occlude` hides the title bar's drag region under the button. Without it,
                    // Windows treats the click as a window drag (HTCAPTION) and the button never
                    // sees it. The in-window menu bar does the same.
                    h_flex().items_center().occlude().child(
                        Button::new("toggle-sidebar")
                            .ghost()
                            .small()
                            .icon(IconName::PanelLeft)
                            .tooltip_with_action("Toggle Sidebar", &ToggleSidebar, Some(WORKSPACE))
                            .on_click(on_toggle),
                    ),
                )
                .children(menu_bar),
        )
}

fn menu_bar(cx: &mut App) -> Entity<AppMenuBar> {
    if let Some(MenuBar(bar)) = cx.try_global::<MenuBar>() {
        return bar.clone();
    }
    let bar = AppMenuBar::new(cx);
    cx.set_global(MenuBar(bar.clone()));
    bar
}
