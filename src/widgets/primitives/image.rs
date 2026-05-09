//! Reusable raster image primitive.

use crate::gui::types::{ImageRgba, Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode};
use crate::theme::ThemeTokens;
use std::sync::Arc;

use super::support::WidgetCommon;
use crate::widgets::contract::{Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

mod paint;

/// Immutable public properties for a reusable image widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageProps {
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Public image primitive for raster content.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable image configuration.
    pub props: ImageProps,
}

impl ImageWidget {
    /// Build a non-interactive image descriptor that reuses shared pixel storage.
    pub fn new(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            props: ImageProps { image },
        }
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
