//! Public API coverage for Radiant application builder ergonomics.

use radiant::{
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{RuntimeBridge, UiSurface},
    widgets::{
        ButtonMessage, ButtonWidget, InteractionProvenance, TextWidget, Widget, WidgetSizing,
    },
};
use std::sync::Arc;

fn arc_surface<Message>(surface: UiSurface<Message>) -> Arc<UiSurface<Message>> {
    Arc::new(surface)
}

pub(crate) fn programmatic_button_message() -> ButtonMessage {
    ButtonMessage::Activate {
        provenance: InteractionProvenance::Programmatic,
    }
}

#[path = "application_builder_public_api/builder_core.rs"]
mod builder_core;
#[path = "application_builder_public_api/collection_layout.rs"]
mod collection_layout;
#[path = "application_builder_public_api/composition.rs"]
mod composition;
#[path = "application_builder_public_api/controls.rs"]
mod controls;
#[path = "application_builder_public_api/details.rs"]
mod details;
#[path = "application_builder_public_api/prelude_exports.rs"]
mod prelude_exports;
#[path = "application_builder_public_api/runtime_behavior.rs"]
mod runtime_behavior;
#[path = "application_builder_public_api/runtime_options.rs"]
mod runtime_options;
#[path = "application_builder_public_api/support.rs"]
mod support;
#[path = "application_builder_public_api/typography.rs"]
mod typography;

pub(crate) use support::{DemoMessage, DemoState, widget_ref};
