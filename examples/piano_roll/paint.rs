use radiant::prelude::*;
pub(crate) use radiant::runtime::{
    push_fill_rect as push_rect, push_fill_rect_batch as push_rect_batch,
    push_stroke_rect as push_stroke, push_stroke_rect_batch as push_stroke_batch, push_text,
};

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8::new(r, g, b, a)
}

pub(crate) fn translucent(color: Rgba8, alpha: u8) -> Rgba8 {
    color.with_alpha(alpha)
}
