//! The window title bar: sidebar toggle, app menu (Windows/Linux), and centered title.
//! On Linux with a self-drawn frame, the bar and its buttons come from `window_buttons`.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, TITLE_BAR_HEIGHT, TitleBar, h_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Entity, Global, InteractiveElement as _, IntoElement,
    ParentElement as _, Styled as _, Window, prelude::FluentBuilder as _,
};

use crate::actions::{ToggleSidebar, WORKSPACE};
use crate::platform;
use crate::ui::window_buttons::{self, ButtonLayout, side_width};

/// The in-window menu bar, created once and shared (Windows and Linux only).
struct MenuBar(Entity<AppMenuBar>);
impl Global for MenuBar {}

/// `on_toggle` runs the sidebar toggle directly. Dispatching the action would
/// need keyboard focus inside the workspace, which a title bar click may not have.
pub fn render(
    title: Option<String>,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let menu_bar = (!platform::uses_native_menu_bar()).then(|| menu_bar(cx));
    // Room for the toggle + in-window menu (Windows/Linux) or the traffic lights (macOS).
    let menu_bar_shown = menu_bar.is_some();
    let title_inset = if menu_bar_shown { 260.0 } else { 110.0 };
    let gnome_buttons =
        platform::draws_gnome_title_bar(window).then(|| ButtonLayout::current(window, cx));
    // Width of the window buttons on each side of the content row.
    let (left_buttons, right_buttons) = match &gnome_buttons {
        Some(layout) => (side_width(&layout.left), side_width(&layout.right)),
        None => (0.0, gpui_kit_controls_width(window)),
    };

    let content = h_flex()
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
                // This row ends where the window buttons begin. Stretch the
                // overlay under them so the title centers on the full width.
                .left(gpui_kit::px(-left_buttons))
                .right(gpui_kit::px(-right_buttons))
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
            // Both buttons are 24px tall and centered, but menu text sits low in its
            // line box. Nudge the icon down 1px so it lines up with the menu labels.
            h_flex()
                .items_center()
                .occlude()
                .relative()
                .top(gpui_kit::px(1.0))
                .child(
                    Button::new("toggle-sidebar")
                        .ghost()
                        .small()
                        .icon(IconName::PanelLeft)
                        .tooltip_with_action("Toggle Sidebar", &ToggleSidebar, Some(WORKSPACE))
                        .on_click(on_toggle),
                ),
        )
        .children(menu_bar);

    match gnome_buttons {
        Some(layout) => window_buttons::title_bar(content, layout, window, cx).into_any_element(),
        None => TitleBar::new()
            // gpui-kit pads the left side by 12px on Windows/Linux. Match the gap above and
            // below the 24px toggle button instead, so it sits evenly in the corner.
            // macOS keeps the default room for the traffic lights.
            .when(menu_bar_shown, |bar| bar.pl(gpui_kit::px(5.0)))
            .child(content)
            .into_any_element(),
    }
}

/// Width of gpui-kit's minimize/maximize/close buttons on the right, or 0 if there
/// are none. It draws each one as a square as tall as the title bar.
fn gpui_kit_controls_width(window: &Window) -> f32 {
    if !platform::draws_gpui_kit_window_controls() {
        return 0.0;
    }
    let controls = window.window_controls();
    let count = 1 + usize::from(controls.minimize) + usize::from(controls.maximize);
    count as f32 * f32::from(TITLE_BAR_HEIGHT)
}

fn menu_bar(cx: &mut App) -> Entity<AppMenuBar> {
    if let Some(MenuBar(bar)) = cx.try_global::<MenuBar>() {
        return bar.clone();
    }
    let bar = AppMenuBar::new(cx);
    cx.set_global(MenuBar(bar.clone()));
    bar
}

/// Rebuilds the in-window menu bar from the current app menus. Does nothing on
/// macOS, or before the title bar has been drawn the first time.
pub fn reload_menu_bar(cx: &mut App) {
    if let Some(MenuBar(bar)) = cx.try_global::<MenuBar>() {
        bar.clone().update(cx, |bar, cx| bar.reload(cx));
    }
}
