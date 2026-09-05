#![allow(missing_docs)]
#![allow(clippy::arc_with_non_send_sync)]

use radiant::gui::types::{Point, Vector2};
use radiant::layout::{
    Constraints, ContainerKind, ContainerPolicy, OverflowPolicy, ScrollAlignment, ScrollAxis,
    ScrollPolicy, ScrollRequest, ScrollTarget, SizeModeCross, SizeModeMain, SlotParams,
};
use radiant::runtime::testing::{DeterministicHost, DeterministicHostConfig, NormalizedRect};
use radiant::runtime::{
    Event, FocusTraversal, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface,
    WidgetMessageMapper, WindowEnvironment,
};
use radiant::theme::DpiScale;
use radiant::widgets::{ButtonWidget, TextWidget, WidgetKey, WidgetSizing};
use std::sync::Arc;
use std::time::Duration;

const OUTER_ID: u64 = 1;
const OUTER_CONTENT_ID: u64 = 2;
const INNER_ID: u64 = 3;
const INNER_CONTENT_ID: u64 = 4;
const TARGET_ID: u64 = 17;
const CANCEL_ID: u64 = 90;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Settled {
    node_id: u64,
    offset: Vector2,
}

#[derive(Default)]
struct ScrollAcceptanceBridge {
    settled: Vec<Settled>,
}

impl RuntimeBridge<Settled> for ScrollAcceptanceBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<Settled>> {
        let rows = (0..12)
            .map(|index| {
                let widget = if index == 7 {
                    SurfaceNode::widget(
                        ButtonWidget::new(
                            TARGET_ID,
                            "Nested target",
                            WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                        ),
                        WidgetMessageMapper::none(),
                    )
                } else {
                    SurfaceNode::widget(
                        TextWidget::new(
                            20 + index,
                            format!("Nested row {index}"),
                            WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                        ),
                        WidgetMessageMapper::none(),
                    )
                };
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(80.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    widget,
                )
            })
            .collect();

        let inner = SurfaceNode::container(
            INNER_ID,
            ContainerPolicy {
                kind: ContainerKind::ScrollView,
                overflow: OverflowPolicy::Scroll,
                scroll_policy: ScrollPolicy::default().axes(ScrollAxis::Vertical),
                initial_offset: Some(Vector2::new(0.0, 40.0)),
                scroll_request: Some(ScrollRequest::new(
                    ScrollTarget::Edge(radiant::layout::ScrollEdge::Top),
                    ScrollAlignment::Nearest,
                    7,
                )),
                ..ContainerPolicy::default()
            },
            vec![SurfaceChild::fill(SurfaceNode::column(
                INNER_CONTENT_ID,
                0.0,
                rows,
            ))],
        )
        .on_offset_settled(|offset| Settled {
            node_id: INNER_ID,
            offset,
        });
        let outer_content = SurfaceNode::column(
            OUTER_CONTENT_ID,
            0.0,
            vec![
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(120.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        TextWidget::new(
                            5,
                            "Prefix",
                            WidgetSizing::fixed(Vector2::new(80.0, 120.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(80.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    inner,
                ),
            ],
        );
        let outer = SurfaceNode::container(
            OUTER_ID,
            ContainerPolicy {
                kind: ContainerKind::ScrollView,
                overflow: OverflowPolicy::Scroll,
                scroll_policy: ScrollPolicy::default().axes(ScrollAxis::Vertical),
                initial_offset: Some(Vector2::new(0.0, 10.0)),
                ..ContainerPolicy::default()
            },
            vec![SurfaceChild::fill(outer_content)],
        )
        .on_offset_settled(|offset| Settled {
            node_id: OUTER_ID,
            offset,
        });
        let cancel = SurfaceNode::widget(
            ButtonWidget::new(
                CANCEL_ID,
                "Cancel capture",
                WidgetSizing::fixed(Vector2::new(100.0, 24.0)),
            ),
            WidgetMessageMapper::none(),
        );
        Arc::new(UiSurface::new(SurfaceNode::stack(
            100,
            vec![SurfaceChild::fill(outer), SurfaceChild::fill(cancel)],
        )))
    }

    fn reduce_message(&mut self, message: Settled) {
        self.settled.push(message);
    }
}

fn rect(snapshot: &radiant::runtime::testing::NormalizedSnapshot, node_id: u64) -> NormalizedRect {
    snapshot
        .layout
        .rects
        .iter()
        .find(|entry| entry.node_id == node_id)
        .map(|entry| entry.rect)
        .unwrap_or_else(|| panic!("missing normalized rect for node {node_id}"))
}

fn viewport_rect(
    snapshot: &radiant::runtime::testing::NormalizedSnapshot,
    node_id: u64,
) -> NormalizedRect {
    snapshot
        .layout
        .viewport_bounds
        .iter()
        .find(|entry| entry.node_id == node_id)
        .map(|entry| entry.rect)
        .unwrap_or_else(|| panic!("missing normalized viewport for node {node_id}"))
}

fn logical_offset(
    snapshot: &radiant::runtime::testing::NormalizedSnapshot,
    scroll_id: u64,
    content_id: u64,
) -> f32 {
    viewport_rect(snapshot, scroll_id).y - rect(snapshot, content_id).y
}

#[test]
fn deterministic_headless_scroll_trace_preserves_authority_across_runtime_changes() {
    let config = DeterministicHostConfig::new(Vector2::new(100.0, 80.0)).with_environment(
        WindowEnvironment::new(DpiScale::new(1.0), None, false, false),
    );
    let mut host = DeterministicHost::new(ScrollAcceptanceBridge::default(), config)
        .expect("deterministic host construction");

    assert_eq!(
        host.bridge().settled.as_slice(),
        &[Settled {
            node_id: INNER_ID,
            offset: Vector2::new(0.0, 0.0),
        }],
        "the declared request settles once at admission"
    );
    let initial = host.published_snapshot().clone();
    assert_eq!(initial.environment.display_scale, 1.0);
    assert!(!initial.environment.reduced_motion);
    let initial_inner_content = rect(&initial, INNER_CONTENT_ID);

    host.refresh().expect("refresh after consumed request");
    let refreshed = host.turn().expect("publish refreshed snapshot");
    assert_eq!(
        host.bridge().settled.len(),
        1,
        "generation 7 must not replay"
    );
    assert_eq!(rect(&refreshed, INNER_CONTENT_ID), initial_inner_content);

    let press_target = host
        .dispatch_event(Event::primary_press(Point::new(8.0, 8.0)))
        .expect("capture press");
    assert_eq!(press_target, Some(CANCEL_ID));
    let captured = host.snapshot().expect("capture snapshot");
    assert_eq!(captured.focus.pointer_capture, Some(CANCEL_ID));
    host.dispatch_event(Event::PointerCaptureCancelled)
        .expect("capture cancellation");
    let cancelled = host.snapshot().expect("cancelled snapshot");
    assert_eq!(cancelled.focus.pointer_capture, None);

    for _ in 0..3 {
        if host.runtime().focused_widget() == Some(TARGET_ID) {
            break;
        }
        host.dispatch_event(Event::traverse_focus(FocusTraversal::Forward))
            .expect("focus nested target");
    }
    assert_eq!(host.runtime().focused_widget(), Some(TARGET_ID));
    let focused = host.turn().expect("publish focus reveal");
    assert_eq!(focused.focus.focused_widget, Some(TARGET_ID));
    assert_eq!(
        host.bridge().settled.as_slice(),
        &[
            Settled {
                node_id: INNER_ID,
                offset: Vector2::new(0.0, 0.0),
            },
            Settled {
                node_id: INNER_ID,
                offset: Vector2::new(0.0, 560.0),
            },
            Settled {
                node_id: OUTER_ID,
                offset: Vector2::new(0.0, 120.0),
            },
        ],
        "focus reveal settles the actual nested ancestors exactly once, inside out"
    );

    let target = rect(&focused, TARGET_ID);
    let inner_viewport = rect(&focused, INNER_ID);
    assert!(
        target.x >= inner_viewport.x
            && target.y >= inner_viewport.y
            && target.x + target.width <= inner_viewport.x + inner_viewport.width
            && target.y + target.height <= inner_viewport.y + inner_viewport.height,
        "wheel point must be inside the visible focused inner viewport"
    );
    let before_wheel = host.bridge().settled.len();
    host.dispatch_event(Event::scroll(
        Point::new(target.x + 1.0, target.y + 1.0),
        Vector2::new(0.0, 12.0),
    ))
    .expect("nested wheel dispatch");
    host.advance_time(Duration::from_millis(100))
        .expect("wheel idle settlement");
    let wheeled = host.turn().expect("publish wheel settlement");
    assert_eq!(wheeled.focus.focused_widget, Some(TARGET_ID));
    assert_eq!(host.bridge().settled.len(), before_wheel + 1);
    assert_eq!(
        host.bridge().settled[before_wheel],
        Settled {
            node_id: INNER_ID,
            offset: Vector2::new(0.0, 572.0),
        },
        "wheel settles only the visible inner owner"
    );

    let before_key = host.bridge().settled.len();
    host.dispatch_event(Event::key_press(WidgetKey::PageDown))
        .expect("focused keyboard scroll");
    let keyed = host.turn().expect("publish keyboard settlement");
    assert_eq!(keyed.focus.focused_widget, Some(TARGET_ID));
    assert_eq!(host.bridge().settled.len(), before_key + 1);
    let keyed_settlement = host.bridge().settled[before_key];
    assert_eq!(keyed_settlement.node_id, INNER_ID);
    assert_eq!(keyed_settlement.offset.x, 0.0);
    assert!(
        keyed_settlement.offset.y > 572.0,
        "PageDown changes the focused inner ancestor"
    );
    let keyed_inner_offset = logical_offset(&keyed, INNER_ID, INNER_CONTENT_ID);
    let keyed_outer_offset = logical_offset(&keyed, OUTER_ID, OUTER_CONTENT_ID);
    assert_eq!(keyed_inner_offset, keyed_settlement.offset.y);
    assert_eq!(keyed_outer_offset, 120.0);

    let before_window_changes = host.bridge().settled.clone();
    host.dispatch_event(Event::resize(Vector2::new(120.0, 100.0)))
        .expect("logical resize");
    let resized = host.turn().expect("publish logical resize");
    assert_eq!(resized.focus.focused_widget, Some(TARGET_ID));
    assert_eq!(resized.environment.display_scale, 1.0);
    assert!(!resized.environment.reduced_motion);
    assert_eq!(logical_offset(&resized, OUTER_ID, OUTER_CONTENT_ID), 100.0);
    assert_eq!(
        logical_offset(&resized, INNER_ID, INNER_CONTENT_ID),
        keyed_inner_offset
    );
    assert_eq!(host.bridge().settled, before_window_changes);

    host.set_window_environment(WindowEnvironment::new(
        DpiScale::new(2.0),
        Some(radiant::runtime::WindowColorScheme::Dark),
        true,
        true,
    ))
    .expect("environment change");
    host.refresh().expect("refresh after environment change");
    let changed = host.turn().expect("publish environment change");
    assert_eq!(changed.environment.display_scale, 2.0);
    assert!(changed.environment.reduced_motion);
    assert_eq!(changed.focus.focused_widget, Some(TARGET_ID));
    assert_eq!(
        logical_offset(&changed, OUTER_ID, OUTER_CONTENT_ID),
        logical_offset(&resized, OUTER_ID, OUTER_CONTENT_ID)
    );
    assert_eq!(
        logical_offset(&changed, INNER_ID, INNER_CONTENT_ID),
        logical_offset(&resized, INNER_ID, INNER_CONTENT_ID)
    );
    assert_eq!(host.bridge().settled, before_window_changes);
}
