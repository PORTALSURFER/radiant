//! Immutable text-dependent size declarations for framework-owned overlays.

use crate::{
    gui::text_layout::{TextWidthEstimate, estimated_text_width_for_char_count_in_range},
    layout::{ContainerPolicy, Vector2},
    runtime::ResolvedEnvironment,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextScaledExtent {
    pub characters: usize,
    pub metrics: TextWidthEstimate,
    pub minimum: f32,
    pub maximum: f32,
}

impl TextScaledExtent {
    fn resolve(self, scale: f32) -> f32 {
        estimated_text_width_for_char_count_in_range(
            self.characters,
            TextWidthEstimate::new(
                self.metrics.character_advance * scale,
                self.metrics.horizontal_padding,
            ),
            self.minimum,
            self.maximum,
        )
        .max(1.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TextScaledSize {
    pub width: Option<TextScaledExtent>,
    pub height: Option<TextScaledExtent>,
}

impl TextScaledSize {
    fn resolve(self, declared: Vector2, scale: f32) -> Vector2 {
        Vector2::new(
            self.width.map_or(declared.x, |width| width.resolve(scale)),
            self.height
                .map_or(declared.y, |height| height.resolve(scale)),
        )
    }
}

impl<Message> super::SurfaceFloatingLayer<Message> {
    pub(in crate::runtime::surface) fn resolved_policy(
        &self,
        environment: &ResolvedEnvironment,
    ) -> ContainerPolicy {
        let mut policy = self.container.policy.clone();
        if let Some(size) = self.text_scaled_size {
            policy.floating.size =
                size.resolve(policy.floating.size, environment.text_scale().factor());
        }
        policy
    }
}

impl<Message> super::SurfaceNode<Message> {
    pub(crate) fn with_text_scaled_floating_size(mut self, size: Option<TextScaledSize>) -> Self {
        if let Self::FloatingLayer(layer) = &mut self {
            layer.text_scaled_size = size;
        }
        self
    }
}
