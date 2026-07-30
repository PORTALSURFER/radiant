//! Public API coverage for user-defined Radiant widgets.

use radiant::runtime::{SurfaceWidget, UiSurface, WidgetMessageMapper};
use radiant::widgets::{Widget, WidgetRevision};
use std::sync::Arc;

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
    assert_eq!(widget.revision(), WidgetRevision::conservative());

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
