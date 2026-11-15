//! The Linux title bar with GNOME-style window buttons, used where the app draws
//! its own window frame (GNOME on Wayland). gpui-kit's `TitleBar` always draws
//! Windows-style minimize/maximize/close on the right, which looks out of place there.
//!
//! - Buttons follow GNOME's `button-layout` setting: which ones, and on which side.
//! - When the user's GTK theme draws them as colored rings (adw-colors), they are
//!   those rings in its colors (`dotnote_core::gtk_theme`); otherwise libadwaita's
//!   round grey buttons.
//!
//! Desktops that decorate the window themselves (KDE, X11 without a compositor…)
//! never get here and keep their own buttons (`platform::draws_gnome_title_bar`).

use std::sync::OnceLock;

use dotnote_core::gtk_theme::{self, RingColors};
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, InteractiveElementExt as _, TITLE_BAR_HEIGHT};
use gpui_kit::{
    AbsoluteLength, App, Background, ClickEvent, Div, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Rgba, Stateful, StatefulInteractiveElement as _, Styled,
    Window, WindowButton, WindowControlArea, div, linear_color_stop, linear_gradient,
    prelude::FluentBuilder as _, px, rgb,
};

/// A grey button's click target; the round background inside it is `DOT`.
const BUTTON: f32 = 34.;
const DOT: f32 = 24.;
/// A ring button's click target width and its ring (the themes' 18 px, 4 px apart).
const RING_BUTTON: f32 = 30.;
const RING: f32 = 18.;
/// Space between the buttons and the window edge or the bar's content.
const SIDE_PADDING: f32 = 6.;

/// The buttons to show on each side of the title bar.
pub struct ButtonLayout {
    pub left: Vec<WindowButton>,
    pub right: Vec<WindowButton>,
}

impl ButtonLayout {
    /// GNOME's `button-layout`, minus what the compositor can't do (a tiling one
    /// may offer neither minimize nor maximize). Close is always shown.
    pub fn current(window: &Window, cx: &App) -> Self {
        let supported = window.window_controls();
        let shown = |button: &WindowButton| match button {
            WindowButton::Minimize => supported.minimize,
            WindowButton::Maximize => supported.maximize,
            WindowButton::Close => true,
        };
        let side = |buttons: [Option<WindowButton>; 3]| {
            buttons.into_iter().flatten().filter(shown).collect()
        };
        match cx.button_layout() {
            Some(layout) => ButtonLayout {
                left: side(layout.left),
                right: side(layout.right),
            },
            None => ButtonLayout {
                left: Vec::new(),
                right: side([
                    Some(WindowButton::Minimize),
                    Some(WindowButton::Maximize),
                    Some(WindowButton::Close),
                ]),
            },
        }
    }
}

/// How wide a side's buttons are, padding included (0 for none).
pub fn side_width(buttons: &[WindowButton]) -> f32 {
    if buttons.is_empty() {
        return 0.0;
    }
    let each = if theme_rings().is_some() {
        RING_BUTTON
    } else {
        BUTTON
    };
    buttons.len() as f32 * each + 2.0 * SIDE_PADDING
}

/// Whether the press that may start a window move is still down.
struct DragState {
    pressed: bool,
}

/// The whole title bar: buttons on the sides, `content` (sidebar toggle, menus,
/// title) in the middle, which also moves the window when dragged.
pub fn title_bar(
    content: impl IntoElement,
    layout: ButtonLayout,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let state = window.use_state(cx, |_, _| DragState { pressed: false });
    let theme = cx.theme();
    let background = title_bar_background(theme.title_bar, theme.background);
    let (border, ink) = (theme.title_bar_border, theme.foreground);

    div().flex_shrink_0().child(
        div()
            .id("title-bar")
            .flex()
            .items_center()
            .h(TITLE_BAR_HEIGHT)
            .border_b_1()
            .border_color(border)
            .bg(background)
            .child(side(&layout.left, ink, window))
            .child(
                div()
                    .id("title-bar-drag")
                    .flex()
                    .items_center()
                    .flex_1()
                    .h_full()
                    .when(layout.left.is_empty(), |bar| bar.pl(px(5.)))
                    .window_control_area(WindowControlArea::Drag)
                    .on_double_click(|_, window, _| window.zoom_window())
                    // Start the move on the first motion, not on the press: moving on
                    // press hands the pointer to the compositor, so a double click
                    // would never arrive. `listener_for` updates `DragState`.
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&state, |state, _, _, _| state.pressed = true),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        window.listener_for(&state, |state, _, _, _| state.pressed = false),
                    )
                    .on_mouse_down_out(
                        window.listener_for(&state, |state, _, _, _| state.pressed = false),
                    )
                    .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
                        if state.pressed {
                            state.pressed = false;
                            window.start_window_move();
                        }
                    }))
                    // GNOME's window menu (Always on Top, Move to Workspace…).
                    .on_mouse_down(MouseButton::Right, |event, window, _| {
                        window.show_window_menu(event.position)
                    })
                    .child(content),
            )
            .child(side(&layout.right, ink, window)),
    )
}

/// One side's buttons, or nothing.
fn side(buttons: &[WindowButton], ink: Hsla, window: &Window) -> Div {
    let active = window.is_window_active();
    let maximized = window.is_maximized();
    div()
        .flex()
        .items_center()
        .when(!buttons.is_empty(), |side| side.px(px(SIDE_PADDING)))
        .children(buttons.iter().map(|&button| match theme_rings() {
            Some(rings) => {
                ring_button(button, ring_color(&rings, button), active, ink).into_any_element()
            }
            None => grey_button(button, maximized, ink).into_any_element(),
        }))
}

/// A press on a button must not reach the drag area behind it; a click does its job.
fn button_base(button: WindowButton) -> Stateful<Div> {
    div()
        .id(button.id())
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_: &ClickEvent, window, cx| {
            cx.stop_propagation();
            match button {
                WindowButton::Minimize => window.minimize_window(),
                WindowButton::Maximize => window.zoom_window(),
                // Same as gpui-kit's close button; the app then saves and quits.
                WindowButton::Close => window.remove_window(),
            }
        })
}

/// libadwaita's look: a 24 px grey circle with the icon, darker on hover.
fn grey_button(button: WindowButton, maximized: bool, ink: Hsla) -> impl IntoElement {
    let icon = match button {
        WindowButton::Minimize => IconName::WindowMinimize,
        WindowButton::Maximize if maximized => IconName::WindowRestore,
        WindowButton::Maximize => IconName::WindowMaximize,
        WindowButton::Close => IconName::WindowClose,
    };
    let tint = |alpha: f32| Hsla { a: alpha, ..ink };
    let (rest, hover, pressed) = (tint(0.10), tint(0.15), tint(0.30));
    // `group` lets the inner circle react to hovering the whole click target.
    let group = button.id();
    button_base(button)
        .size(px(BUTTON))
        .flex()
        .items_center()
        .justify_center()
        .group(group)
        .child(
            div()
                // An id makes the circle stateful, which `group_active` needs.
                .id(inner_id(button))
                .size(px(DOT))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rest)
                .group_hover(group, move |style| style.bg(hover))
                .group_active(group, move |style| style.bg(pressed))
                .text_color(ink)
                .child(Icon::new(icon).size(px(14.))),
        )
}

/// adw-colors' look: a ring in the button's color, thicker and lighter on hover
/// (thinner for maximize), filled while pressed; grey and faded while the window
/// is in the background.
fn ring_button(button: WindowButton, color: Hsla, active: bool, ink: Hsla) -> impl IntoElement {
    let rest = if active {
        color
    } else {
        Hsla { a: 0.15, ..ink }
    };
    let hover_width = if button == WindowButton::Maximize {
        4.
    } else {
        8.
    };
    let (hover, pressed) = (mix_white(color, 0.3), mix_white(color, 0.5));
    let group = button.id();
    button_base(button)
        .w(px(RING_BUTTON))
        .h(px(BUTTON))
        .flex()
        .items_center()
        .justify_center()
        .group(group)
        .child(
            ring(div().id(inner_id(button)).size(px(RING)).rounded_full(), 6.)
                .when(!active, |ring| ring.opacity(0.5))
                .border_color(rest)
                .group_hover(group, move |style| {
                    ring(style.opacity(1.), hover_width).border_color(hover)
                })
                .group_active(group, move |style| ring(style, 0.).bg(pressed)),
        )
}

/// Id of the circle or ring inside a button (the button itself uses `button.id()`).
fn inner_id(button: WindowButton) -> &'static str {
    match button {
        WindowButton::Minimize => "minimize-shape",
        WindowButton::Maximize => "maximize-shape",
        WindowButton::Close => "close-shape",
    }
}

/// Sets all four border widths at once: on a round element, a ring this thick.
/// (gpui's `border_N` helpers only come in a few fixed widths.)
fn ring<S: Styled>(mut element: S, width: f32) -> S {
    let width: Option<AbsoluteLength> = Some(px(width).into());
    let borders = &mut element.style().border_widths;
    (borders.top, borders.right, borders.bottom, borders.left) = (width, width, width, width);
    element
}

/// `amount` of white mixed into `color`, like CSS `color-mix(in srgb, white N%, color)`.
fn mix_white(color: Hsla, amount: f32) -> Hsla {
    let color = Rgba::from(color);
    let mix = |channel: f32| channel + (1.0 - channel) * amount;
    Rgba {
        r: mix(color.r),
        g: mix(color.g),
        b: mix(color.b),
        a: 1.0,
    }
    .into()
}

fn ring_color(rings: &RingColors, button: WindowButton) -> Hsla {
    let hex = match button {
        WindowButton::Close => rings.close,
        WindowButton::Maximize => rings.maximize,
        WindowButton::Minimize => rings.minimize,
    };
    rgb(hex).into()
}

/// The GTK theme's ring colors, read once (a small file, the first time the
/// title bar draws).
fn theme_rings() -> Option<RingColors> {
    static RINGS: OnceLock<Option<RingColors>> = OnceLock::new();
    *RINGS.get_or_init(gtk_theme::read_ring_colors)
}

/// The same top-to-bottom shade gpui-kit's `TitleBar` uses, so the bar looks
/// the same as on the other platforms.
fn title_bar_background(title_bar: Hsla, background: Hsla) -> Background {
    let (top, bottom) = (title_bar.to_rgb(), background.to_rgb());
    let mixed = Hsla::from(Rgba {
        r: top.r * 0.55 + bottom.r * 0.45,
        g: top.g * 0.55 + bottom.g * 0.45,
        b: top.b * 0.55 + bottom.b * 0.45,
        a: top.a * 0.55 + bottom.a * 0.45,
    });
    linear_gradient(
        180.,
        linear_color_stop(mixed, 0.),
        linear_color_stop(title_bar, 1.),
    )
}
