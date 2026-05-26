use radiant::prelude::*;
pub(crate) use radiant::runtime::{
    push_fill_rect as push_rect, push_stroke_rect as push_stroke, push_text,
};

pub(crate) fn translucent(color: Rgba8, alpha: u8) -> Rgba8 {
    color.with_alpha(alpha)
}
