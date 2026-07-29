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
