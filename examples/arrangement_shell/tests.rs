use super::{
    ARRANGEMENT_WIDGET_ID, AppMessage, ArrangementOverviewWidget, DATA_SOURCE_NOTE,
    STATUS_WIDGET_ID, ShellMessage, geometry::x_for_beat, model::ArrangementShellState,
    project_surface, update,
};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

#[test]
fn arrangement_shell_tick_advances_playhead_and_meters_without_audio_or_dsp() {
    let mut state = ArrangementShellState::default();
    let initial_playhead = state.playhead_beat;
    let initial_level = state.mixer[0].level;

    state.tick();

    assert_eq!(state.frame, 2);
    assert!(state.playhead_beat > initial_playhead);
    assert_ne!(state.mixer[0].level, initial_level);
    assert_eq!(DATA_SOURCE_NOTE, "without_audio_or_dsp");
}

#[test]
fn arrangement_shell_projects_browser_arrangement_inspector_and_mixer() {
    let runtime = SurfaceRuntime::new(
        arrangement_shell_test_bridge(ArrangementShellState::default()),
        Vector2::new(1180.0, 700.0),
    );
    let paint_plan = runtime.paint_plan(&ThemeTokens::default());

    assert!(
        paint_plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("Browser")))
    );
    assert!(
        runtime
            .surface()
            .find_widget(ARRANGEMENT_WIDGET_ID)
            .is_some()
    );
    assert!(runtime.surface().find_widget(STATUS_WIDGET_ID).is_some());
}

#[test]
fn arrangement_overview_paints_lanes_clips_and_playhead_overlay() {
    let state = ArrangementShellState::default();
    let widget = ArrangementOverviewWidget::new(
        state.clips.clone(),
        state.selected_clip,
        state.playhead_beat,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
    let mut primitives = Vec::new();
    let mut overlay = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Bass A"))
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
        "playhead should paint as a lightweight runtime overlay"
    );
}

#[test]
fn arrangement_overview_click_selects_clip_or_seeks_empty_space() {
    let state = ArrangementShellState::default();
    let mut widget = ArrangementOverviewWidget::new(
        state.clips.clone(),
        state.selected_clip,
        state.playhead_beat,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
    let timeline = widget.timeline_rect(bounds);
    let clip = state.clips[1];

    let select = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: widget.clip_rect(timeline, clip).center(),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let seek = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(x_for_beat(timeline, 30.0), timeline.max.y - 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        select.and_then(|output| output.typed_ref::<ShellMessage>().copied()),
        Some(ShellMessage::SelectClip(2))
    );
    assert!(matches!(
        seek.and_then(|output| output.typed_ref::<ShellMessage>().copied()),
        Some(ShellMessage::Seek { beat }) if beat > 29.0
    ));
}

#[test]
fn arrangement_shell_hover_uses_paint_only_runtime_overlay() {
    let state = ArrangementShellState::default();
    let mut widget = ArrangementOverviewWidget::new(
        state.clips.clone(),
        state.selected_clip,
        state.playhead_beat,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
    let timeline = widget.timeline_rect(bounds);
    let clip = state.clips[1];

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: widget.clip_rect(timeline, clip).center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_clip, Some(2));
    assert!(widget.prefers_pointer_move_paint_only());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "hovered clip should paint as a lightweight runtime overlay"
    );
}

#[test]
fn arrangement_shell_runtime_hover_does_not_refresh_surface() {
    let bridge = arrangement_shell_test_bridge(ArrangementShellState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1180.0, 700.0));
    let bounds = runtime.layout().rects[&ARRANGEMENT_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 160.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 280.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable arrangement-shell hover should avoid reprojection and full scene rebuilds"
    );
}

#[test]
fn arrangement_shell_runtime_frame_messages_advance_status() {
    let bridge = arrangement_shell_test_bridge(ArrangementShellState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1180.0, 700.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

fn arrangement_shell_test_bridge(state: ArrangementShellState) -> impl RuntimeBridge<AppMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .into_bridge()
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, AppMessage>) -> String
where
    Bridge: RuntimeBridge<AppMessage>,
{
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                Some(text.text.as_str().to_string())
            }
            _ => None,
        })
        .expect("status text should be painted")
}
