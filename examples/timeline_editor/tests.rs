use super::timeline_widget::TimelineGeometry;
use super::*;
use radiant::{
    gui::visualization::ChannelViewMode,
    layout::{LayoutOutput, Point, Rect, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextRun, RuntimeBridge, SurfaceRuntime,
    },
    theme::ThemeTokens,
    widgets::{PointerButton, TextWidget, Widget, WidgetInput, WidgetKey, WidgetOutput},
};

#[path = "tests/input.rs"]
mod input;
#[path = "tests/paint.rs"]
mod paint;
#[path = "tests/projection.rs"]
mod projection;
#[path = "tests/runtime.rs"]
mod runtime;
#[path = "tests/state.rs"]
mod state;

fn assert_surface_message(
    output: &WidgetOutput,
    matches: impl FnOnce(&TimelineSurfaceMessage) -> bool,
) {
    let message = output
        .typed_ref::<TimelineSurfaceMessage>()
        .expect("timeline widget emits timeline messages");
    assert!(matches(message), "unexpected message: {message:?}");
}

fn assert_clip_preview(
    primitives: &[PaintPrimitive],
    preview_rect: Rect,
    preview_color: radiant::gui::types::Rgba8,
    label: &str,
    theme: &ThemeTokens,
) {
    let preview_fill = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                if *rect == preview_rect && *color == preview_color
        )
    });
    let preview_stroke = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::StrokeRect(PaintStrokeRect { rect, color, width, .. })
                if *rect == preview_rect && *color == theme.text_primary && *width == 2.0
        )
    });
    let preview_label = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(PaintTextRun { text, rect, color, .. })
                if text.as_str() == label
                    && rect.min.x > preview_rect.min.x
                    && rect.max.x <= preview_rect.max.x
                    && *color == theme.text_primary
        )
    });

    assert!(preview_fill);
    assert!(preview_stroke);
    assert!(preview_label);
}

fn assert_clip(state: &TimelineEditorState, id: u32, lane: usize, range: BeatRange) {
    let clip = state
        .clips
        .iter()
        .find(|clip| clip.id == id)
        .unwrap_or_else(|| panic!("clip {id} should exist"));
    assert_eq!(clip.lane, lane);
    assert_eq!(clip.range, range);
}

fn assert_lane_has_no_overlaps(state: &TimelineEditorState, lane: usize) {
    let clips = state
        .clips
        .iter()
        .filter(|clip| clip.lane == lane)
        .collect::<Vec<_>>();
    for (index, clip) in clips.iter().enumerate() {
        for other in clips.iter().skip(index + 1) {
            assert!(
                clip.range.end <= other.range.start || other.range.end <= clip.range.start,
                "clips {} and {} overlap on lane {lane}",
                clip.id,
                other.id
            );
        }
    }
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, TimelineMessage>) -> String
where
    Bridge: RuntimeBridge<TimelineMessage>,
{
    runtime
        .surface()
        .find_widget(STATUS_WIDGET_ID)
        .expect("status widget exists")
        .widget_object()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("status widget is text")
        .text
        .to_string()
}
