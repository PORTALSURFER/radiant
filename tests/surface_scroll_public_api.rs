//! Public API coverage for scroll and virtual-scroll surface behavior.

use radiant::{
    layout::{
        Constraints, ContainerKind, ContainerPolicy, LayoutDebugOptions, LayoutState, Point, Rect,
        SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis, layout_tree,
        layout_tree_with_state,
    },
    runtime::{
        Event, PaintPrimitive, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        declarative_runtime_bridge,
    },
    widgets::{PointerButton, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

#[test]
fn surface_node_scroll_area_helpers_project_scroll_view_layout() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(220.0, 160.0)),
        ),
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );
    let overflow = output
        .overflow_flags
        .get(&31)
        .expect("scroll area should report overflow");

    assert!(surface.find_widget(32).is_some());
    assert!(overflow.x);
    assert!(overflow.y);
}

#[test]
fn surface_paint_plan_clips_scroll_content_and_draws_scrollbar_affordance() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
            (0..8)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should start a clip");
    let PaintPrimitive::ClipStart(clip) = &plan.primitives[clip_start] else {
        unreachable!("clip_start index was matched above");
    };
    let clip_end = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should end the clip");
    let scrollbar_fills: Vec<_> = plan
        .primitives
        .iter()
        .skip(clip_end + 1)
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill),
            _ => None,
        })
        .collect();

    assert!(clip_start < clip_end);
    assert_eq!(clip.rect, layout.rects[&31]);
    assert_eq!(scrollbar_fills.len(), 1);
    assert!(scrollbar_fills[0].rect.width() <= 3.0);
    assert_eq!(scrollbar_fills[0].rect.max.x, layout.rects[&31].max.x);
}

#[test]
fn surface_paint_plan_culls_scroll_content_outside_visible_clip() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
            (0..64)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let stats = plan.stats();

    assert!(stats.text > 0);
    assert!(
        stats.text < 8,
        "normal scroll paint should emit only text rows intersecting the visible clip"
    );
}

#[test]
fn styled_scroll_container_paints_own_chrome_then_clips_content() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::styled_container(
        31,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: radiant::layout::OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        },
        vec![SurfaceChild::fill(SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(220.0, 160.0)),
        ))],
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let chrome = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 31),
        )
        .expect("styled scroll container should paint its own chrome");
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 31),
        )
        .expect("styled scroll container should clip its content after chrome");

    assert!(chrome < clip_start);
}

#[test]
fn surface_runtime_routes_scroll_delta_to_scroll_view_under_pointer() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
                (0..12)
                    .map(|index| {
                        SurfaceChild::new(
                            intrinsic_slot(),
                            SurfaceNode::text(
                                100 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                            ),
                        )
                    })
                    .collect(),
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let before = runtime.layout().rects[&100];

    assert!(runtime.scroll_at(Point::new(20.0, 20.0), Vector2::new(0.0, 48.0)));
    let after = runtime.layout().rects[&100];

    assert!(after.min.y < before.min.y);
    assert_eq!(before.min.y - after.min.y, 48.0);
}

#[test]
fn surface_runtime_drags_painted_scrollbar_thumb() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
                (0..20)
                    .map(|index| {
                        SurfaceChild::new(
                            intrinsic_slot(),
                            SurfaceNode::text(
                                100 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                            ),
                        )
                    })
                    .collect(),
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let before = runtime.layout().rects[&100];
    let thumb = runtime
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a draggable thumb");

    runtime.dispatch_event(Event::PointerPress {
        position: thumb.center(),
        button: PointerButton::Primary,
    });
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));
    assert!(
        runtime.take_repaint_requested(),
        "pressing the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
    });
    assert!(
        runtime.take_repaint_requested(),
        "dragging the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
        button: PointerButton::Primary,
    });

    let after = runtime.layout().rects[&100];
    assert!(after.min.y < before.min.y);
}

#[test]
fn surface_runtime_highlights_painted_scrollbar_thumb_on_hover() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
                (0..20)
                    .map(|index| {
                        SurfaceChild::new(
                            intrinsic_slot(),
                            SurfaceNode::text(
                                100 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                            ),
                        )
                    })
                    .collect(),
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let theme = radiant::theme::ThemeTokens::default();
    let thumb = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a hoverable thumb");

    runtime.dispatch_event(Event::PointerMove {
        position: thumb.center(),
    });

    let hovered_color = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.color),
            _ => None,
        })
        .expect("hovered scroll area should still paint a thumb");
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));
    assert!(runtime.take_repaint_requested());
    assert_eq!(hovered_color, theme.accent_copper);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });
    let idle_color = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.color),
            _ => None,
        })
        .expect("idle scroll area should still paint a thumb");
    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert!(runtime.take_repaint_requested());
    assert_eq!(idle_color, theme.grid_strong);
}

#[test]
fn surface_runtime_does_not_hit_scrolled_content_outside_scroll_viewport() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::column(
            1,
            8.0,
            vec![
                SurfaceChild::new(
                    intrinsic_slot(),
                    SurfaceNode::button(
                        10,
                        "Header",
                        WidgetSizing::fixed(Vector2::new(220.0, 32.0)),
                        DemoMessage::Increment,
                    ),
                ),
                SurfaceChild::fill(SurfaceNode::scroll_area(
                    20,
                    SurfaceNode::column(
                        21,
                        2.0,
                        (0..12)
                            .map(|index| {
                                SurfaceChild::new(
                                    intrinsic_slot(),
                                    SurfaceNode::button(
                                        100 + index,
                                        format!("Row {index}"),
                                        WidgetSizing::fixed(Vector2::new(220.0, 30.0)),
                                        DemoMessage::Increment,
                                    ),
                                )
                            })
                            .collect(),
                    ),
                )),
            ],
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));

    assert!(runtime.scroll_at(Point::new(20.0, 60.0), Vector2::new(0.0, 80.0)));

    assert_eq!(runtime.widget_at(Point::new(20.0, 16.0)), Some(10));
}

#[test]
fn surface_runtime_does_not_scroll_nested_view_outside_parent_clip() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            20,
            SurfaceNode::column(
                21,
                0.0,
                vec![
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            30,
                            "Spacer",
                            WidgetSizing::fixed(Vector2::new(220.0, 100.0)),
                        ),
                    ),
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::scroll_area(
                            40,
                            SurfaceNode::column(
                                41,
                                0.0,
                                (0..8)
                                    .map(|index| {
                                        SurfaceChild::new(
                                            intrinsic_slot(),
                                            SurfaceNode::text(
                                                100 + index,
                                                format!("Nested {index}"),
                                                WidgetSizing::fixed(Vector2::new(220.0, 24.0)),
                                            ),
                                        )
                                    })
                                    .collect(),
                            ),
                        ),
                    ),
                ],
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));

    assert!(!runtime.scroll_at(Point::new(20.0, 110.0), Vector2::new(0.0, 24.0)));
}

#[test]
fn surface_node_virtual_scroll_area_helper_records_virtual_window() {
    let rows = (0..256)
        .map(|index| {
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::text(
                    1000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 10.0)),
                ),
            )
        })
        .collect::<Vec<_>>();
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        33,
        SurfaceNode::column(34, 1.0, rows),
        VirtualizationAxis::Vertical,
        0.0,
    ));
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(33, Vector2::new(0.0, 400.0));

    let output = layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&33)
        .expect("virtual scroll area should report a virtual window");

    assert_eq!(window.total_children, 256);
    assert!(window.first_index > 0);
    assert!(window.culled_after > 0);
}
