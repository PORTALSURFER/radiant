use super::*;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, DebugPrimitiveKind, LayoutDebugPrimitive,
        LayoutOmissionReason, LayoutOutput, LayoutPolicy, MeasureChildren, NodeId, PlaceChildren,
        SizeHint,
    },
    runtime::{PaintPrimitive, SurfaceChild, SurfaceContainer, SurfaceNode, UiSurface},
    theme::ThemeTokens,
    widgets::{
        TextWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetPaintContext,
        WidgetSizing, WidgetStyle,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default, PartialEq)]
struct PaintProbeRecord {
    bounds: Option<Rect>,
    environment: Option<crate::runtime::ResolvedEnvironment>,
    base_calls: usize,
    overlay_calls: usize,
}

#[derive(Clone)]
struct PaintProbe {
    common: WidgetCommon,
    record: Arc<Mutex<PaintProbeRecord>>,
}

impl PaintProbe {
    fn new(id: u64, record: Arc<Mutex<PaintProbeRecord>>) -> Self {
        Self {
            common: WidgetCommon::fixed(id, 80.0, 20.0).without_default_chrome(),
            record,
        }
    }
}

impl Widget for PaintProbe {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.record.lock().unwrap().base_calls += 1;
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let mut record = self.record.lock().unwrap();
        record.bounds = Some(context.bounds());
        record.environment = Some(context.environment().clone());
        record.base_calls += 1;
        drop(record);
    }

    fn append_runtime_overlay_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.record.lock().unwrap().overlay_calls += 1;
    }

    fn append_runtime_overlay_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let mut record = self.record.lock().unwrap();
        record.bounds = Some(context.bounds());
        record.environment = Some(context.environment().clone());
        record.overlay_calls += 1;
    }
}

#[derive(Clone)]
struct LegacyPaintProbe {
    common: WidgetCommon,
    record: Arc<Mutex<PaintProbeRecord>>,
}

impl LegacyPaintProbe {
    fn new(id: u64, record: Arc<Mutex<PaintProbeRecord>>) -> Self {
        Self {
            common: WidgetCommon::fixed(id, 80.0, 20.0).without_default_chrome(),
            record,
        }
    }
}

impl Widget for LegacyPaintProbe {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.record.lock().unwrap().base_calls += 1;
    }

    fn append_runtime_overlay_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.record.lock().unwrap().overlay_calls += 1;
    }
}

fn child_is_past_ordered_clip_for(
    kind: ContainerKind,
    clip_rect: Rect,
    child_id: NodeId,
    child_rect: Rect,
) -> bool {
    let mut layout = LayoutOutput::default();
    layout.rects.insert(child_id, child_rect);
    let theme = ThemeTokens::default();
    let context = SurfacePaintContext {
        layout: &layout,
        theme: &theme,
        hovered_container: None,
        active_scroll_affordance: None,
        auto_scroll_visible: &[],
        environment: crate::runtime::ResolvedEnvironment::default(),
        appearance: crate::theme::ResolvedAppearance::fixed(theme),
        clip_rect: Some(clip_rect),
    };
    let container = SurfaceContainer::<()>::new(
        1,
        ContainerPolicy {
            kind,
            ..ContainerPolicy::default()
        },
        Vec::new(),
    );
    context.child_is_past_ordered_clip(&container, child_id)
}

#[test]
fn ordered_clip_detects_row_children_past_right_edge() {
    let clip_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let child_rect = Rect::from_min_size(Point::new(100.0, 0.0), Vector2::new(24.0, 20.0));

    assert!(child_is_past_ordered_clip_for(
        ContainerKind::Row,
        clip_rect,
        20,
        child_rect
    ));
    assert!(!child_is_past_ordered_clip_for(
        ContainerKind::Column,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
        20,
        Rect::from_min_size(Point::new(100.0, 0.0), Vector2::new(24.0, 20.0))
    ));
}

#[test]
fn ordered_clip_detects_column_children_past_bottom_edge() {
    let clip_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let child_rect = Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(24.0, 20.0));

    assert!(child_is_past_ordered_clip_for(
        ContainerKind::Column,
        clip_rect,
        20,
        child_rect
    ));
    assert!(!child_is_past_ordered_clip_for(
        ContainerKind::Row,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
        20,
        Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(24.0, 20.0))
    ));
}

#[test]
fn clipped_container_wraps_child_paint_in_clip_primitives() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::static_widget(
            TextWidget::new(
                2,
                "Overflow",
                WidgetSizing::fixed(Vector2::new(160.0, 20.0)),
            ),
        ))],
    ));
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    assert!(matches!(
        plan.primitives.first(),
        Some(PaintPrimitive::ClipStart(clip))
            if clip.node_id == 1
                && clip.rect
                    == Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0))
    ));
    assert!(
        plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.widget_id == 2)
        )
    );
    assert!(matches!(
        plan.primitives.last(),
        Some(PaintPrimitive::ClipEnd(end)) if end.node_id == 1
    ));
}

#[test]
fn layout_debug_strokes_for_children_stay_inside_parent_clip_scope() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::static_widget(
            TextWidget::new(2, "Debug", WidgetSizing::fixed(Vector2::new(160.0, 20.0))),
        ))],
    ));
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    );
    layout.debug_primitives.push(LayoutDebugPrimitive {
        node_id: 2,
        kind: DebugPrimitiveKind::NodeBounds,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    });

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let child_debug_stroke = plan
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.widget_id == 2
                        && stroke.color == crate::gui::types::Rgba8::new(255, 0, 0, 255)
            )
        })
        .expect("child debug stroke should be painted");
    let parent_clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 1),
        )
        .expect("parent clip should start before child paint");
    let parent_clip_end = plan
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, PaintPrimitive::ClipEnd(end) if end.node_id == 1))
        .expect("parent clip should end after child paint");

    assert!(parent_clip_start < child_debug_stroke);
    assert!(child_debug_stroke < parent_clip_end);
}

#[test]
fn omitted_overlay_panel_is_absent_from_runtime_paint_plan() {
    struct OmitOverlayPolicy;

    impl LayoutPolicy for OmitOverlayPolicy {
        fn measure(
            &self,
            children: &mut MeasureChildren<'_>,
            constraints: Constraints,
        ) -> SizeHint {
            children
                .measure(0, constraints)
                .expect("the first overlay child should measure");
            children
                .measure(1, constraints)
                .expect("the second overlay child should measure");
            SizeHint::preferred(Vector2::new(40.0, 30.0))
        }

        fn place(&self, children: &mut PlaceChildren<'_>, _bounds: Rect) {
            children
                .omit(0, LayoutOmissionReason::Conditional)
                .expect("the overlay child should be omitted");
        }
    }

    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::layout(
        1,
        OmitOverlayPolicy,
        vec![
            SurfaceChild::fill(SurfaceNode::overlay_panel(
                2,
                Rect::from_xy_size(10.0, 12.0, 30.0, 20.0),
                "omitted",
                WidgetStyle::default(),
            )),
            SurfaceChild::fill(SurfaceNode::overlay_panel(
                3,
                Rect::from_xy_size(20.0, 22.0, 30.0, 20.0),
                "unresolved",
                WidgetStyle::default(),
            )),
        ],
    ));

    let frame = surface.frame_at_size_with_default_theme(Vector2::new(100.0, 80.0));

    assert!(!frame.layout.rects.contains_key(&2));
    assert!(!frame.layout.rects.contains_key(&3));
    assert!(!frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::OverlayPanel(panel) if panel.widget_id == 2 || panel.widget_id == 3)
    ));
}

#[test]
fn base_paint_context_receives_window_environment_without_changing_bounds() {
    let record = Arc::new(Mutex::new(PaintProbeRecord::default()));
    let probe = PaintProbe::new(2, Arc::clone(&record));
    let mut surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(probe));
    let environment = crate::runtime::WindowEnvironment::new(
        crate::theme::DpiScale::new(1.5),
        Some(crate::runtime::WindowColorScheme::Dark),
        true,
        true,
    );
    surface.set_window_environment(environment);
    let bounds = Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(80.0, 20.0));
    let mut layout = LayoutOutput::default();
    layout.rects.insert(2, bounds);

    let _ = surface.paint_plan(&layout, &ThemeTokens::default());
    let record = record.lock().unwrap().clone();
    assert_eq!(record.bounds, Some(bounds));
    assert_eq!(record.environment, Some(environment.resolved()));
    assert_eq!(record.base_calls, 1);
    assert_eq!(record.overlay_calls, 0);
    assert_eq!(layout.rects.get(&2), Some(&bounds));
}

#[test]
fn legacy_context_defaults_delegate_once_for_base_and_overlay() {
    let record = Arc::new(Mutex::new(PaintProbeRecord::default()));
    let probe = LegacyPaintProbe::new(4, Arc::clone(&record));
    let layout = LayoutOutput::default();
    let theme = ThemeTokens::default();
    let environment = crate::runtime::ResolvedEnvironment::default();
    let mut primitives = Vec::new();
    let context = WidgetPaintContext::new(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0)),
        &layout,
        &theme,
        &environment,
    );
    let mut context = context;
    probe.append_paint_with_context(&mut context);
    probe.append_runtime_overlay_paint_with_context(&mut context);

    let record = record.lock().unwrap();
    assert_eq!(record.base_calls, 1);
    assert_eq!(record.overlay_calls, 1);
}
