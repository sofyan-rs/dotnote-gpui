//! The Live / Source / Split / Preview switch in the editor header, drawn as a
//! segmented control (`ui/segmented_control.rs`).

use dotnote_core::settings::ViewMode;
use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{Action, App, IntoElement, ParentElement as _, StatefulInteractiveElement as _};

use crate::actions::{ViewLive, ViewPreview, ViewSource, ViewSplit, WORKSPACE};
use crate::ui::segmented_control;

/// One segment: which mode it shows, its label and icon, and the action it sends.
fn segments() -> [(ViewMode, &'static str, IconName, Box<dyn Action>); 4] {
    [
        (
            ViewMode::Live,
            "Live",
            IconName::PenLine,
            Box::new(ViewLive),
        ),
        (
            ViewMode::Source,
            "Source",
            IconName::Code,
            Box::new(ViewSource),
        ),
        (
            ViewMode::Split,
            "Split",
            IconName::Columns2,
            Box::new(ViewSplit),
        ),
        (
            ViewMode::Preview,
            "Preview",
            IconName::Eye,
            Box::new(ViewPreview),
        ),
    ]
}

/// Clicking a segment sends the same action as its shortcut (⌘/Ctrl+1/2/3/4),
/// so the workspace handles all three ways of switching in one place.
pub fn render(active: ViewMode, cx: &App) -> impl IntoElement {
    segmented_control::track(cx).children(segments().into_iter().map(
        |(mode, label, icon, action)| {
            let tooltip_action = action.boxed_clone();
            segmented_control::segment(label, label, icon, mode == active, cx)
                .tooltip(move |window, cx| {
                    Tooltip::new(label)
                        .action(tooltip_action.as_ref(), Some(WORKSPACE))
                        .build(window, cx)
                })
                .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
        },
    ))
}
