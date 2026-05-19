//! Reusable raster image primitive.

use crate::gui::types::{ImageRgba, Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode};
use crate::theme::ThemeTokens;
use std::sync::Arc;

use super::support::WidgetCommon;
use crate::widgets::contract::{Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

mod model;
mod paint;

pub use model::ImageProps;

/// Public image primitive for raster content.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable image configuration.
    pub props: ImageProps,
}

/// Named construction fields for [`ImageWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ImageWidgetParts {
    /// Stable widget identity used by layout and paint projection.
    pub id: WidgetId,
    /// Shared raster image payload.
    pub image: Arc<ImageRgba>,
    /// Intrinsic image sizing contract.
    pub sizing: WidgetSizing,
}

impl ImageWidget {
    /// Build a non-interactive image descriptor from named identity, image, and sizing fields.
    pub fn from_parts(parts: ImageWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            props: ImageProps { image: parts.image },
        }
    }

    /// Build a non-interactive image descriptor that reuses shared pixel storage.
    pub fn new(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ImageWidgetParts { id, image, sizing })
    }
}

impl Widget for ImageWidget {
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
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        paint::push_image_widget_paint(primitives, self, bounds);
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting raster image leaf node.
    pub fn image(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ImageWidget::new(id, image, sizing))
    }
}
