//! Public API coverage for user-defined Radiant widgets.

use radiant::runtime::UiSurface;
use std::sync::Arc;

fn arc_surface<Message>(surface: UiSurface<Message>) -> Arc<UiSurface<Message>> {
    Arc::new(surface)
}

#[path = "custom_widget_public_api/builders.rs"]
mod builders;
#[path = "custom_widget_public_api/hover.rs"]
mod hover;
#[path = "custom_widget_public_api/runtime_paths.rs"]
mod runtime_paths;
#[path = "custom_widget_public_api/support.rs"]
mod support;
