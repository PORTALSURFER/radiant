use super::super::{DemoMessage, widget_ref};
use radiant::{
    layout::{Point, Rect, Vector2},
    runtime::{
        Event, PaintPrimitive, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface,
        WidgetMessageMapper, declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{PointerButton, Widget, WidgetCommon, WidgetInput, WidgetSizing},
};
use std::sync::Arc;

pub(crate) use radiant::runtime::SurfaceRuntime;

pub(crate) fn pointer_motion_bridge(
    continuous_pointer_move: bool,
) -> impl RuntimeBridge<DemoMessage> {
    pointer_motion_bridge_with_policy(continuous_pointer_move, false)
}

pub(crate) fn pointer_motion_bridge_with_policy(
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

pub(crate) struct OverlappingPointerBridge;

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

pub(crate) fn motion_probe<'a, Bridge>(
    runtime: &'a SurfaceRuntime<Bridge, DemoMessage>,
    id: u64,
    expected: &str,
) -> &'a PointerMotionProbeWidget
where
    Bridge: RuntimeBridge<DemoMessage>,
{
    widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), id, expected)
}

pub(crate) fn primary_press(position: Point) -> Event {
    Event::PointerPress {
        position,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    }
}

fn constrained_stack_slot(size: Vector2) -> radiant::layout::SlotParams {
    radiant::layout::SlotParams {
        size_main: radiant::layout::SizeModeMain::Fill(1.0),
        size_cross: radiant::layout::SizeModeCross::Fill,
        constraints: radiant::layout::Constraints::from_parts(radiant::layout::ConstraintsParts {
            min_w: size.x,
            max_w: size.x,
            min_h: size.y,
            max_h: size.y,
        }),
        margin: Default::default(),
        align_cross_override: Some(radiant::layout::CrossAlign::Start),
        allow_fixed_compress: false,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PointerMotionProbeWidget {
    pub(crate) common: WidgetCommon,
    continuous_pointer_move: bool,
    paint_only_pointer_move: bool,
    pub(crate) moves: usize,
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
