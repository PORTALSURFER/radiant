use super::super::*;
use radiant::application as app;

#[test]
fn application_split_pane_is_a_static_two_child_builder_with_identity_continuity() {
    use radiant::{layout::SplitPaneAxis, prelude as ui, prelude::IntoView};

    let builder: app::SplitPaneBuilder<()> =
        app::split_pane(ui::text("First").id(101), ui::text("Second").id(102))
            .axis(SplitPaneAxis::Vertical)
            .initial_ratio(0.25)
            .divider_extent(8.0)
            .min_first(24.0)
            .min_second(32.0);
    let view: ui::View<()> = builder.into_view();
    let surface = view.into_surface();
    let layout_node = surface.layout_node();
    let radiant::layout::LayoutNode::Container(container) = &layout_node else {
        panic!("split_pane should lower to a dedicated container");
    };

    assert_eq!(
        container.policy.kind,
        radiant::layout::ContainerKind::SplitPane
    );
    assert_eq!(container.children.len(), 2);
    assert_eq!(container.children[0].child.id(), 101);
    assert_eq!(container.children[1].child.id(), 102);

    let layout = layout_tree(
        &layout_node,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 160.0)),
    );
    assert_eq!(layout.rects[&101].height(), 38.0);
    assert_eq!(layout.rects[&102].min.y, 46.0);
    assert_eq!(layout.rects[&102].height(), 114.0);
    assert!(surface.find_widget(101).is_some());
    assert!(surface.find_widget(102).is_some());
}

#[test]
fn application_split_pane_defaults_match_the_shared_geometry_contract() {
    use radiant::{layout::ContainerKind, prelude as ui, prelude::IntoView};

    let view: ui::View<()> = ui::split_pane(ui::text("First"), ui::text("Second")).into_view();
    let layout_node = view.into_surface().layout_node();
    let radiant::layout::LayoutNode::Container(container) = layout_node else {
        panic!("split_pane should lower to a dedicated container");
    };

    assert_eq!(container.policy.kind, ContainerKind::SplitPane);
    assert_eq!(container.children.len(), 2);
    assert_eq!(
        container.policy.split_pane.axis,
        radiant::layout::SplitPaneAxis::Horizontal
    );
    assert_eq!(container.policy.split_pane.initial_ratio, 0.5);
    assert_eq!(container.policy.split_pane.divider_extent, 0.0);
    assert_eq!(container.policy.split_pane.first_min_extent, 0.0);
    assert_eq!(container.policy.split_pane.second_min_extent, 0.0);
}

#[test]
fn application_split_pane_collapse_policy_is_additive_and_runtime_owned() {
    use radiant::{layout::SplitPaneCollapsePolicy, prelude as ui, prelude::IntoView};

    let static_view: ui::View<()> = ui::split_pane(ui::text("First"), ui::text("Second"))
        .initial_ratio(0.25)
        .collapse_policy(SplitPaneCollapsePolicy::SecondPane)
        .into_view();
    let static_surface = static_view.into_surface();
    let static_node = static_surface.layout_node();
    let static_layout = layout_tree(
        &static_node,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
    );
    assert!(static_layout.rects.values().all(|rect| rect.is_finite()));

    let controlled_view: ui::View<()> = ui::split_pane(ui::text("First"), ui::text("Second"))
        .controlled_ratio(radiant::layout::Controlled::new(0.75, 1))
        .collapse_policy(SplitPaneCollapsePolicy::FirstPane)
        .into_view();
    let controlled_surface = controlled_view.into_surface();
    let controlled_bridge = radiant::runtime::declarative_runtime_bridge(
        (),
        move |_state: &mut ()| crate::arc_surface(controlled_surface.clone()),
        |_state: &mut (), _message: ()| {},
    );
    let controlled_runtime =
        radiant::runtime::SurfaceRuntime::new(controlled_bridge, Vector2::new(200.0, 80.0));
    assert_eq!(
        controlled_runtime.layout_target_at(Point::new(152.0, 40.0)),
        None
    );

    let runtime_view: ui::View<()> = ui::split_pane(ui::text("First"), ui::text("Second"))
        .initial_ratio(0.25)
        .divider_extent(8.0)
        .collapse_policy(SplitPaneCollapsePolicy::FirstPane)
        .runtime_owned_ratio()
        .into_view();
    let runtime_surface = runtime_view.into_surface();
    let runtime_node = runtime_surface.layout_node();
    let bridge = radiant::runtime::declarative_runtime_bridge(
        (),
        move |_state: &mut ()| crate::arc_surface(runtime_surface.clone()),
        |_state: &mut (), _message: ()| {},
    );
    let runtime = radiant::runtime::SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
    assert!(runtime.layout_target_at(Point::new(52.0, 40.0)).is_some());
    let radiant::layout::LayoutNode::Container(container) = runtime_node else {
        panic!("runtime split should lower to a container");
    };
    assert_eq!(container.children.len(), 2);
}

#[test]
fn application_split_pane_runtime_ratio_opt_ins_keep_static_policy_and_fallback_geometry() {
    use radiant::{layout::Controlled, prelude as ui, prelude::IntoView};

    for view in [
        ui::split_pane::<()>(ui::text("First"), ui::text("Second"))
            .initial_ratio(0.25)
            .runtime_owned_ratio()
            .into_view(),
        ui::split_pane::<()>(ui::text("First"), ui::text("Second"))
            .initial_ratio(0.25)
            .controlled_ratio(Controlled::new(0.75, 4))
            .into_view(),
    ] {
        let surface = view.into_surface();
        let layout_node = surface.layout_node();
        let radiant::layout::LayoutNode::Container(container) = &layout_node else {
            panic!("split_pane should lower to a dedicated container");
        };
        assert_eq!(container.policy.split_pane.initial_ratio, 0.25);
        let container_id = container.id;

        let layout = radiant::layout::layout_tree(
            &layout_node,
            radiant::layout::Rect::from_min_size(
                radiant::layout::Point::new(0.0, 0.0),
                radiant::layout::Vector2::new(100.0, 40.0),
            ),
        );
        assert_eq!(layout.rects[&container_id].width(), 100.0);
        assert_eq!(
            layout.rects[&container.children[0].child.id()].width(),
            25.0
        );
    }
}

#[test]
fn application_runtime_owned_split_lowers_the_private_divider_target() {
    use radiant::{
        layout::Point,
        prelude as ui,
        prelude::IntoView,
        runtime::{Event, SurfaceRuntime, declarative_runtime_bridge},
    };

    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            crate::arc_surface(
                ui::split_pane::<()>(ui::text("First"), ui::text("Second"))
                    .initial_ratio(0.25)
                    .divider_extent(8.0)
                    .runtime_owned_ratio()
                    .into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));

    let target = runtime
        .layout_target_at(Point::new(52.0, 40.0))
        .expect("runtime-owned application split should expose its divider target");
    assert_eq!(target.bounds, Rect::from_xy_size(48.0, 0.0, 8.0, 80.0));
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0))),
        None
    );
    assert!(runtime.layout_pointer_capture().is_some());
}

#[test]
fn application_runtime_owned_split_settled_mapper_survives_same_identity_refresh() {
    use radiant::{
        layout::Point,
        prelude as ui,
        prelude::IntoView,
        runtime::{Event, SurfaceRuntime, declarative_runtime_bridge},
    };
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum SettledMessage {
        RefreshMapper(u8),
        Ratio { mapper: u8, ratio: f32 },
    }

    let messages = Rc::new(RefCell::new(Vec::new()));
    let reduced_messages = Rc::clone(&messages);
    let bridge = declarative_runtime_bridge(
        1_u8,
        |mapper: &mut u8| {
            let mapper = *mapper;
            crate::arc_surface(
                ui::split_pane::<SettledMessage>(ui::text("First"), ui::text("Second"))
                    .initial_ratio(0.25)
                    .divider_extent(8.0)
                    .runtime_owned_ratio()
                    .on_ratio_settled(move |ratio| SettledMessage::Ratio { mapper, ratio })
                    .into_surface(),
            )
        },
        move |mapper: &mut u8, message| match message {
            SettledMessage::RefreshMapper(next) => *mapper = next,
            SettledMessage::Ratio { mapper, ratio } => {
                reduced_messages.borrow_mut().push((mapper, ratio));
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
    let moved = Point::new(130.0, 100.0);

    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(moved));
    runtime.dispatch_message(SettledMessage::RefreshMapper(2));
    assert!(runtime.layout_pointer_capture().is_some());
    assert_eq!(
        runtime
            .layout_target_at(Point::new(134.0, 40.0))
            .map(|target| target.bounds),
        Some(Rect::from_xy_size(130.0, 0.0, 8.0, 80.0))
    );
    runtime.dispatch_event(Event::pointer_release(
        moved,
        radiant::widgets::PointerButton::Primary,
        radiant::widgets::PointerModifiers::default(),
    ));

    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(messages.borrow().as_slice(), &[(1, 130.0_f32 / 192.0_f32)]);
}

#[derive(Clone, Copy)]
enum RuntimeRatioMode {
    Static,
    RuntimeOwned,
    Controlled,
}

struct RuntimeRatioState {
    mode: RuntimeRatioMode,
    initial_ratio: f32,
    ratio: f32,
    generation: u64,
}

#[derive(Clone, Copy)]
enum RuntimeRatioMessage {
    Static,
    RuntimeOwned(f32),
    Controlled(f32, u64),
}

fn runtime_ratio_view(state: &RuntimeRatioState) -> radiant::prelude::View<RuntimeRatioMessage> {
    use radiant::layout::Controlled;
    use radiant::prelude as ui;

    let builder =
        ui::split_pane::<RuntimeRatioMessage>(ui::text("First").id(11), ui::text("Second").id(12))
            .initial_ratio(state.initial_ratio);
    match state.mode {
        RuntimeRatioMode::Static => builder.into_view(),
        RuntimeRatioMode::RuntimeOwned => builder.runtime_owned_ratio().into_view(),
        RuntimeRatioMode::Controlled => builder
            .controlled_ratio(Controlled::new(state.ratio, state.generation))
            .into_view(),
    }
}

fn runtime_ratio_bridge() -> impl radiant::runtime::RuntimeBridge<RuntimeRatioMessage> {
    use radiant::prelude as ui;

    ui::app(RuntimeRatioState {
        mode: RuntimeRatioMode::Static,
        initial_ratio: 0.25,
        ratio: 0.4,
        generation: 1,
    })
    .view(runtime_ratio_view)
    .handle_message(|state, message, _| match message {
        RuntimeRatioMessage::Static => state.mode = RuntimeRatioMode::Static,
        RuntimeRatioMessage::RuntimeOwned(initial_ratio) => {
            state.mode = RuntimeRatioMode::RuntimeOwned;
            state.initial_ratio = initial_ratio;
        }
        RuntimeRatioMessage::Controlled(ratio, generation) => {
            state.mode = RuntimeRatioMode::Controlled;
            state.ratio = ratio;
            state.generation = generation;
        }
    })
    .into_bridge()
}

fn runtime_ratio_first_width<Bridge>(
    runtime: &radiant::runtime::SurfaceRuntime<Bridge, RuntimeRatioMessage>,
) -> f32
where
    Bridge: radiant::runtime::RuntimeBridge<RuntimeRatioMessage>,
{
    let radiant::layout::LayoutNode::Container(container) = runtime.surface().layout_node() else {
        panic!("runtime ratio view should lower to a split container");
    };
    runtime.layout().rects[&container.children[0].child.id()].width()
}

#[test]
fn runtime_ratio_modes_reconcile_through_one_mounted_slot() {
    use radiant::{layout::Vector2, runtime::SurfaceRuntime};

    let mut runtime = SurfaceRuntime::new(runtime_ratio_bridge(), Vector2::new(100.0, 40.0));
    assert_eq!(runtime_ratio_first_width(&runtime), 25.0);

    runtime.dispatch_message(RuntimeRatioMessage::Controlled(0.4, 1));
    assert_eq!(runtime_ratio_first_width(&runtime), 40.0);
    runtime.dispatch_message(RuntimeRatioMessage::Controlled(0.8, 1));
    assert_eq!(runtime_ratio_first_width(&runtime), 40.0);
    runtime.dispatch_message(RuntimeRatioMessage::Controlled(0.8, 2));
    assert_eq!(runtime_ratio_first_width(&runtime), 80.0);

    runtime.dispatch_message(RuntimeRatioMessage::RuntimeOwned(0.1));
    assert_eq!(runtime_ratio_first_width(&runtime), 10.0);
    runtime.dispatch_message(RuntimeRatioMessage::RuntimeOwned(0.9));
    assert_eq!(runtime_ratio_first_width(&runtime), 10.0);

    runtime.dispatch_message(RuntimeRatioMessage::Controlled(0.3, 1));
    assert_eq!(runtime_ratio_first_width(&runtime), 30.0);
    runtime.dispatch_message(RuntimeRatioMessage::Static);
    assert_eq!(runtime_ratio_first_width(&runtime), 90.0);
    runtime.dispatch_message(RuntimeRatioMessage::Controlled(0.2, 99));
    assert_eq!(runtime_ratio_first_width(&runtime), 20.0);
}

#[test]
fn application_builder_todo_layout_does_not_overlap_header_input_and_list() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::row([
            ui::text("Todos").id(10).size(140.0, 28.0),
            ui::text("1/3 done").id(11).size(120.0, 28.0),
        ])
        .id(2)
        .fill_width(),
        ui::row([
            ui::text_input("Review public API")
                .message(|_| ())
                .id(12)
                .min_size(260.0, 32.0)
                .preferred_size(420.0, 32.0)
                .fill_width(),
            ui::button("Add")
                .primary()
                .message(())
                .id(13)
                .size(80.0, 32.0),
        ])
        .id(3)
        .fill_width(),
        ui::list(0..3, |index| {
            ui::list_row(
                index,
                [
                    ui::checkbox(false)
                        .message(|_| ())
                        .id(20 + index)
                        .size(24.0, 24.0),
                    ui::text(format!("Item {index}"))
                        .id(60 + index)
                        .fill_width(),
                    ui::button("Delete")
                        .danger()
                        .message(())
                        .id(30 + index)
                        .size(84.0, 30.0),
                ],
            )
            .id(40 + index)
        })
        .id(4),
    ])
    .id(1)
    .padding(16.0)
    .spacing(12.0)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(560.0, 360.0)),
    );

    let header = layout.rects[&2];
    let input = layout.rects[&3];
    let list = layout.rects[&4];
    let first_row = layout.rects[&40];

    assert_eq!(header.height(), 28.0);
    assert_eq!(input.height(), 32.0);
    assert!(input.min.y >= header.max.y + 12.0);
    assert!(list.min.y >= input.max.y + 12.0);
    assert!(first_row.min.y >= list.min.y);
    assert_eq!(first_row.height(), 44.0);
}

#[test]
fn application_builder_centered_layer_centers_fixed_size_child() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::centered_layer(
        ui::text("Dialog").key("centered-dialog").id(2),
        Vector2::new(120.0, 80.0),
    )
    .id(1)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 300.0)),
    );
    let child = layout.rects[&2];

    assert_eq!(child.min.x, 140.0);
    assert_eq!(child.min.y, 110.0);
    assert_eq!(child.width(), 120.0);
    assert_eq!(child.height(), 80.0);
}

#[test]
fn centered_layer_parts_support_named_construction() {
    use radiant::prelude as ui;

    let parts: app::CenteredLayerParts<()> =
        app::CenteredLayerParts::new(ui::text("Dialog"), Vector2::new(320.0, 180.0));

    assert_eq!(parts.size, Vector2::new(320.0, 180.0));
}

#[test]
fn floating_layer_anchor_helpers_position_content_around_trigger() {
    use radiant::{prelude as ui, prelude::IntoView, runtime::PaintPrimitive};

    let frame = UiSurface::new(
        ui::stack([
            ui::text("").size(240.0, 140.0),
            ui::floating_layer_above::<()>(
                18.0,
                80.0,
                6.0,
                Vector2::new(90.0, 24.0),
                ui::text("Above").id(71),
            ),
            ui::floating_layer_below::<()>(
                18.0,
                80.0,
                20.0,
                6.0,
                Vector2::new(90.0, 24.0),
                ui::text("Below").id(72),
            ),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0)),
        &Default::default(),
    );

    let text_rect = |widget_id| {
        frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == widget_id => Some(text.rect),
                _ => None,
            })
            .expect("anchored floating-layer text should paint")
    };

    assert_eq!(text_rect(71).min, Point::new(18.0, 50.0));
    assert_eq!(text_rect(72).min, Point::new(18.0, 106.0));
}

#[test]
fn floating_layer_anchor_parts_support_named_interactive_construction() {
    use radiant::prelude as ui;

    let parts: app::FloatingLayerAnchorParts<()> = app::FloatingLayerAnchorParts::new(
        ui::text("Popup"),
        Vector2::new(160.0, 80.0),
        12.0,
        42.0,
        20.0,
        4.0,
        ui::FloatingLayerPlacement::Below,
    )
    .interactive(true);

    assert_eq!(parts.x, 12.0);
    assert_eq!(parts.trigger_y, 42.0);
    assert_eq!(parts.trigger_height, 20.0);
    assert_eq!(parts.gap, 4.0);
    assert_eq!(parts.size, Vector2::new(160.0, 80.0));
    assert_eq!(parts.placement, ui::FloatingLayerPlacement::Below);
    assert!(parts.interactive);
}
