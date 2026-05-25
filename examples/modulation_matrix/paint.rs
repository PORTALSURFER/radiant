use radiant::prelude::*;
pub(crate) use radiant::runtime::{
    push_fill_rect as push_rect, push_stroke_rect as push_stroke, push_text,
};

pub(crate) fn blend_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    a.with_alpha(255).blend_toward(b.with_alpha(255), t)
}

pub(crate) fn translucent(color: Rgba8, alpha: u8) -> Rgba8 {
    color.with_alpha(alpha)
}
