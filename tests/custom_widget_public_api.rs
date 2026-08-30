//! Public API coverage for user-defined Radiant widgets.

use radiant::gui::automation::AutomationRole;
use radiant::layout::{Point, Rect, Vector2};
use radiant::runtime::{
    DevtoolsNodeSnapshot, Event, SurfaceNode, SurfaceRuntime, SurfaceWidget, UiSurface,
    WidgetMessageMapper, declarative_owned_runtime_bridge,
};
use radiant::widgets::{
    PointerCapturePolicy, WIDGET_CAPABILITIES_CONTRACT_VERSION, Widget, WidgetCapabilities,
    WidgetCursor, WidgetHitTest, WidgetHitTestResult, WidgetInput, WidgetPointerMotion,
    WidgetRevision, WidgetSemantics,
};
use std::{rc::Rc, sync::Arc};

fn arc_surface<Message>(surface: UiSurface<Message>) -> Arc<UiSurface<Message>> {
    Arc::new(surface)
}

#[path = "custom_widget_public_api/builders.rs"]
mod builders;
#[path = "custom_widget_public_api/hover.rs"]
mod hover;
#[path = "custom_widget_public_api/local_ownership.rs"]
mod local_ownership;
#[path = "custom_widget_public_api/runtime_paths.rs"]
mod runtime_paths;
#[path = "custom_widget_public_api/support.rs"]
mod support;

#[test]
fn custom_widgets_keep_the_conservative_revision_default_through_trait_objects() {
    let widget = support::CustomStatusWidget::new(41);
    assert_eq!(
        radiant::prelude::Widget::revision(&widget),
        WidgetRevision::conservative()
    );

    let boxed: Box<dyn Widget> = Box::new(widget.clone());
    assert_eq!(boxed.revision(), WidgetRevision::conservative());

    let surface_widget: SurfaceWidget<support::CustomWidgetMessage> =
        SurfaceWidget::custom(widget, WidgetMessageMapper::none());
    assert_eq!(surface_widget.revision(), WidgetRevision::conservative());
}

#[test]
fn custom_widgets_can_publish_typed_exact_revision_evidence() {
    let first = WidgetRevision::exact(
        ("status", 1_u8),
        vec![120_u16, 28],
        support::CustomWidgetMessage::Activated,
        Some("label"),
    );
    let equal = WidgetRevision::exact(
        ("status", 1_u8),
        vec![120_u16, 28],
        support::CustomWidgetMessage::Activated,
        Some("label"),
    );
    let changed = WidgetRevision::exact(
        ("status", 1_u8),
        vec![121_u16, 28],
        support::CustomWidgetMessage::Activated,
        Some("label"),
    );

    assert_eq!(first, equal);
    assert_ne!(first, changed);
}

#[test]
fn historical_v1_literals_and_legacy_pointer_hooks_remain_compatible() {
    let historical = WidgetCapabilities {
        contract_version: WIDGET_CAPABILITIES_CONTRACT_VERSION,
        semantics: None,
    };
    assert_eq!(historical.contract_version, 1);
    assert!(!historical.has_semantics());

    let legacy = support::LegacyHooksWidget::new(43);
    assert!(!radiant::prelude::Widget::accepts_pointer_move(&legacy));
    assert!(radiant::prelude::Widget::accepts_pointer_input(
        &legacy,
        &WidgetInput::pointer_move(Point::default()),
    ));
    assert_eq!(
        radiant::prelude::Widget::pointer_capture_policy(&legacy),
        PointerCapturePolicy::Exclusive
    );
    assert_eq!(
        radiant::prelude::Widget::cursor_for_point(&legacy, Rect::default(), Point::default()),
        Some(WidgetCursor::ResizeLeft)
    );
    assert!(radiant::prelude::Widget::prefers_pointer_move_paint_only(
        &legacy
    ));
    assert!(!legacy.capabilities_v2().has_pointer_motion());

    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::custom_widget(
        legacy,
        WidgetMessageMapper::none(),
    ));
    let bridge = declarative_owned_runtime_bridge(surface, |surface| surface.clone(), |_, _| {});
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 40.0));
    assert_eq!(runtime.widget_at(Point::new(8.0, 8.0)), Some(43));
    assert_eq!(
        runtime.cursor_at(Point::new(8.0, 8.0)),
        WidgetCursor::ResizeLeft
    );

    let unsupported = support::LegacyHooksWidget::with_unsupported_v2(44);
    assert_eq!(unsupported.capabilities_v2().contract_version(), 99);
    assert!(!unsupported.capabilities_v2().has_hit_test());
    assert!(!unsupported.capabilities_v2().has_pointer_motion());
}

#[test]
fn supported_v2_descriptors_take_precedence_over_legacy_pointer_hooks() {
    let moves = Rc::new(std::cell::Cell::new(0));
    let widget = support::DescriptorPrecedenceWidget::with_moves(45, Rc::clone(&moves));

    assert!(radiant::prelude::Widget::accepts_pointer_move(&widget));
    assert_eq!(
        radiant::prelude::Widget::pointer_capture_policy(&widget),
        PointerCapturePolicy::PassThrough
    );
    assert_eq!(
        radiant::prelude::Widget::cursor_for_point(&widget, Rect::default(), Point::default()),
        Some(WidgetCursor::ResizeLeft)
    );
    assert!(!radiant::prelude::Widget::prefers_pointer_move_paint_only(
        &widget
    ));

    let capabilities = widget.capabilities_v2();
    let hit_test = capabilities.hit_test().expect("v2 hit-test descriptor");
    let pointer_motion = capabilities
        .pointer_motion()
        .expect("v2 pointer-motion descriptor");
    assert_eq!(
        hit_test.cursor_for_point(Rect::default(), Point::default()),
        Some(WidgetCursor::ResizeRight)
    );
    assert!(!pointer_motion.accepts_pointer_move());
    assert_eq!(
        pointer_motion.pointer_capture_policy(),
        PointerCapturePolicy::Exclusive
    );
    assert!(pointer_motion.prefers_pointer_move_paint_only());
    assert!(pointer_motion.pointer_move_overlay_is_valid());

    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::custom_widget(
        widget,
        WidgetMessageMapper::none(),
    ));
    let bridge = declarative_owned_runtime_bridge(surface, |surface| surface.clone(), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 40.0));
    assert_eq!(
        runtime.cursor_at(Point::new(8.0, 8.0)),
        WidgetCursor::ResizeRight
    );
    runtime.dispatch_event(Event::pointer_move(Point::new(8.0, 8.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(12.0, 8.0)));
    assert_eq!(moves.get(), 1);
}

#[test]
fn direct_widget_automation_overrides_reach_snapshot_and_devtools() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::custom_widget(
        support::DirectAutomationWidget::new(46),
        WidgetMessageMapper::none(),
    ));
    let bridge = declarative_owned_runtime_bridge(surface, |surface| surface.clone(), |_, _| {});
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 40.0));

    let snapshot = runtime.automation_snapshot();
    let node = automation_node(&snapshot.root, "46").expect("direct automation node");
    assert_eq!(node.semantics.role, AutomationRole::Readout);
    assert_eq!(node.semantics.label.as_deref(), Some("direct override"));
    assert_eq!(node.available_actions, [String::from("direct-action")]);

    let devtools = runtime.devtools_snapshot();
    let node = devtools_node(&devtools.root, 46).expect("direct devtools node");
    let widget = node
        .widget
        .as_ref()
        .expect("direct widget devtools snapshot");
    assert_eq!(widget.semantics.role, AutomationRole::Readout);
    assert_eq!(widget.semantics.label.as_deref(), Some("direct override"));
}

#[test]
fn custom_widgets_publish_object_safe_capability_descriptors() {
    let widget = support::CustomStatusWidget::new(42);
    let capabilities = widget.capabilities();

    assert_eq!(
        capabilities.contract_version,
        WIDGET_CAPABILITIES_CONTRACT_VERSION
    );
    assert!(capabilities.has_semantics());
    assert_eq!(
        capabilities
            .semantics
            .expect("custom semantics descriptor")
            .automation_label(),
        Some(String::from("custom"))
    );
    let capabilities_v2 = widget.capabilities_v2();
    assert!(capabilities_v2.has_pointer_motion());
    assert!(capabilities_v2.has_hit_test());

    let semantics: &dyn WidgetSemantics = &widget;
    let hit_test: &dyn WidgetHitTest = capabilities_v2.hit_test().expect("custom hit-test");
    let pointer_motion: &dyn WidgetPointerMotion = capabilities_v2
        .pointer_motion()
        .expect("custom pointer motion");
    assert_eq!(
        semantics.automation_role(),
        radiant::gui::automation::AutomationRole::Button
    );
    assert_eq!(
        hit_test.hit_test(
            radiant::layout::Rect::default(),
            radiant::layout::Point::default(),
            &WidgetInput::pointer_move(radiant::layout::Point::default()),
        ),
        WidgetHitTestResult::Opaque
    );
    assert_eq!(
        hit_test.cursor_for_point(
            radiant::layout::Rect::default(),
            radiant::layout::Point::default(),
        ),
        None
    );
    assert!(pointer_motion.accepts_pointer_move());
}

fn automation_node<'a>(
    root: &'a radiant::gui::automation::AutomationNodeSnapshot,
    id: &str,
) -> Option<&'a radiant::gui::automation::AutomationNodeSnapshot> {
    if root.id.0 == id {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| automation_node(child, id))
}

fn devtools_node(root: &DevtoolsNodeSnapshot, node_id: u64) -> Option<&DevtoolsNodeSnapshot> {
    if root.node_id == node_id {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| devtools_node(child, node_id))
}
