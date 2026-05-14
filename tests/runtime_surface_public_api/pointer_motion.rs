use super::{DemoMessage, widget_ref};
use radiant::{
    layout::{Point, Rect, Vector2},
    runtime::{
        Event, PaintPrimitive, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        WidgetMessageMapper, declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{PointerButton, Widget, WidgetCommon, WidgetInput, WidgetSizing},
};
use std::sync::Arc;

#[test]
fn surface_runtime_skips_stable_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(16.0, 16.0),
        }),
        Some(10)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(20.0, 20.0),
        }),
        Some(10)
    );

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 1);
    assert!(probe.common.state.hovered);
}

#[test]
fn surface_runtime_preserves_stable_pointer_motion_for_continuous_widgets() {
    let bridge = pointer_motion_bridge(true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(16.0, 16.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 2);
}

#[test]
fn surface_runtime_keeps_captured_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(16.0, 16.0),
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(18.0, 18.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 2);
    assert!(probe.common.state.pressed);
}

#[test]
fn surface_runtime_reports_paint_only_pointer_overlay_outcomes() {
    let bridge = pointer_motion_bridge_with_policy(true, true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let first = runtime.dispatch_pointer_move_with_outcome(Point::new(16.0, 16.0));
    assert!(first.routed());
    assert!(first.hover_changed);
    assert!(first.needs_scene_rebuild());

    let second = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert!(second.routed());
    assert!(!second.hover_changed);
    assert!(!second.pointer_captured);
    assert!(second.paint_only_requested);
    assert!(!second.repaint_requested);
    assert!(!second.needs_scene_rebuild());
    assert!(second.needs_redraw());

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 2);
}

#[test]
fn surface_runtime_reports_captured_paint_only_pointer_overlay_outcomes() {
    let bridge = pointer_motion_bridge_with_policy(true, true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(16.0, 16.0),
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let enter_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(18.0, 18.0));
    assert!(enter_drag.routed());
    assert!(enter_drag.pointer_captured);
    assert!(enter_drag.needs_scene_rebuild());

    let preview_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert!(preview_drag.routed());
    assert!(preview_drag.pointer_captured);
    assert!(preview_drag.paint_only_requested);
    assert!(!preview_drag.repaint_requested);
    assert!(!preview_drag.needs_scene_rebuild());
    assert!(preview_drag.needs_redraw());
}

#[test]
fn surface_runtime_keeps_captured_non_overlay_pointer_motion_on_scene_rebuild_path() {
    let bridge = pointer_motion_bridge_with_policy(true, false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(16.0, 16.0),
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let _ = runtime.dispatch_pointer_move_with_outcome(Point::new(18.0, 18.0));
    let preview_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));

    assert!(preview_drag.routed());
    assert!(preview_drag.pointer_captured);
    assert!(!preview_drag.paint_only_requested);
    assert!(preview_drag.repaint_requested);
    assert!(preview_drag.needs_scene_rebuild());
}

#[test]
fn surface_runtime_stable_pointer_motion_still_respects_higher_overlapping_widgets() {
    let mut runtime = SurfaceRuntime::new(OverlappingPointerBridge, Vector2::new(140.0, 60.0));

    let first = runtime.dispatch_pointer_move_with_outcome(Point::new(100.0, 20.0));
    assert_eq!(first.target, Some(10));
    assert!(first.hover_changed);

    let second = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert_eq!(second.target, Some(20));
    assert!(second.hover_changed);

    let lower = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "lower probe");
    let upper = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 20, "upper probe");
    assert_eq!(
        lower.moves, 2,
        "lower widget receives enter and leave motion"
    );
    assert_eq!(
        upper.moves, 1,
        "upper widget receives motion after becoming top target"
    );
}

fn pointer_motion_bridge(continuous_pointer_move: bool) -> impl RuntimeBridge<DemoMessage> {
    pointer_motion_bridge_with_policy(continuous_pointer_move, false)
}

fn pointer_motion_bridge_with_policy(
    continuous_pointer_move: bool,
    paint_only_pointer_move: bool,
) -> impl RuntimeBridge<DemoMessage> {
    declarative_runtime_bridge(
        (continuous_pointer_move, paint_only_pointer_move),
        |(continuous_pointer_move, paint_only_pointer_move): &mut (bool, bool)| {
            Arc::new(UiSurface::new(SurfaceNode::custom_widget(
                PointerMotionProbeWidget::new(
                    10,
                    *continuous_pointer_move,
                    *paint_only_pointer_move,
                ),
                WidgetMessageMapper::none(),
            )))
        },
        |_policy: &mut (bool, bool), _message| {},
    )
}

struct OverlappingPointerBridge;

impl RuntimeBridge<DemoMessage> for OverlappingPointerBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::stack(
            1,
            vec![
                SurfaceChild::fill(SurfaceNode::custom_widget(
                    PointerMotionProbeWidget::new_sized(10, true, false, Vector2::new(120.0, 40.0)),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::new(
                    constrained_stack_slot(Vector2::new(60.0, 40.0)),
                    SurfaceNode::custom_widget(
                        PointerMotionProbeWidget::new_sized(
                            20,
                            true,
                            false,
                            Vector2::new(60.0, 40.0),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                ),
            ],
        )))
    }
}

fn constrained_stack_slot(size: Vector2) -> radiant::layout::SlotParams {
    radiant::layout::SlotParams {
        size_main: radiant::layout::SizeModeMain::Fill(1.0),
        size_cross: radiant::layout::SizeModeCross::Fill,
        constraints: radiant::layout::Constraints::new(size.x, size.x, size.y, size.y),
        margin: Default::default(),
        align_cross_override: Some(radiant::layout::CrossAlign::Start),
        allow_fixed_compress: false,
    }
}

#[derive(Clone, Debug)]
struct PointerMotionProbeWidget {
    common: WidgetCommon,
    continuous_pointer_move: bool,
    paint_only_pointer_move: bool,
    moves: usize,
}

impl PointerMotionProbeWidget {
    fn new(id: u64, continuous_pointer_move: bool, paint_only_pointer_move: bool) -> Self {
        Self::new_sized(
            id,
            continuous_pointer_move,
            paint_only_pointer_move,
            Vector2::new(120.0, 40.0),
        )
    }

    fn new_sized(
        id: u64,
        continuous_pointer_move: bool,
        paint_only_pointer_move: bool,
        size: Vector2,
    ) -> Self {
        let common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(size).with_baseline(size.y.min(24.0)),
        );
        Self {
            common: common.with_pointer_focus(),
            continuous_pointer_move,
            paint_only_pointer_move,
            moves: 0,
        }
    }
}

impl Widget for PointerMotionProbeWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn accepts_pointer_move(&self) -> bool {
        self.continuous_pointer_move
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        self.paint_only_pointer_move
    }

    fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.moves += 1;
                self.common.state.hovered = bounds.contains(position);
            }
            WidgetInput::PointerPress { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                self.common.state.pressed = bounds.contains(position);
            }
            WidgetInput::PointerRelease { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                self.common.state.pressed = false;
            }
            _ => {}
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}
