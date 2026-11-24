//! The segmented control look shared by the editor's view mode switch and the
//! Settings theme picker: a muted track with the chosen segment raised on it.
//!
//! `track` draws the track; `segment` draws one option. Callers add their own
//! `.on_click` (and `.tooltip`) to each segment, so this file only holds the style.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::{
    App, Div, InteractiveElement as _, IntoElement, ParentElement as _, StatefulInteractiveElement,
    Styled as _, TestSupportExt as _, prelude::FluentBuilder as _, px,
};

/// Space between the track's edge and the segments, the same on every side.
const TRACK_PADDING: f32 = 3.;
/// Segment corner radius. The track's radius is this plus the padding, so the
/// corners are concentric and the gap looks even all the way around.
const SEGMENT_RADIUS: f32 = 5.;

/// The muted, rounded track that holds the segments.
pub fn track(cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .p(px(TRACK_PADDING))
        .gap(px(TRACK_PADDING))
        .rounded(px(SEGMENT_RADIUS + TRACK_PADDING))
        .bg(theme.muted)
        .border_1()
        .border_color(theme.border)
}

/// One option: icon and label, raised when `active`. `id` also lets UI tests click it.
pub fn segment(
    id: &'static str,
    label: &'static str,
    icon: IconName,
    active: bool,
    cx: &App,
) -> impl IntoElement + StatefulInteractiveElement {
    // `impl ..` because `test_support()` wraps the element in UI tests.
    let theme = cx.theme();
    let (active_text, idle_text) = (theme.foreground, theme.muted_foreground);
    let text = if active { active_text } else { idle_text };
    let raised = theme.background;
    h_flex()
        .id(id)
        .test_support()
        .h(px(24.))
        .px_2p5()
        .gap_1p5()
        .rounded(px(SEGMENT_RADIUS))
        .text_xs()
        .text_color(text)
        .cursor_pointer()
        .when(active, |segment| segment.bg(raised).shadow_sm())
        .when(!active, |segment| {
            segment.hover(move |segment| segment.text_color(active_text))
        })
        .child(Icon::new(icon).xsmall().text_color(text))
        .child(label)
}
