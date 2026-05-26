use radiant::prelude::*;
pub(crate) use radiant::runtime::{
    push_fill_rect as push_rect, push_stroke_rect as push_stroke, push_text,
};

pub(crate) fn meter_color(db: f32) -> Rgba8 {
    if db > -3.0 {
        rgba(255, 82, 52, 255)
    } else if db > -10.0 {
        rgba(255, 190, 72, 255)
    } else {
        rgba(60, 214, 154, 255)
    }
}

pub(crate) fn group_color(group: usize, theme: &ThemeTokens) -> Rgba8 {
    match group % 4 {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        2 => theme.accent_warning,
        _ => theme.highlight_orange,
    }
}

pub(crate) fn send_color(send: usize, theme: &ThemeTokens) -> Rgba8 {
    match send % 3 {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        _ => theme.highlight_orange,
    }
}

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8::new(r, g, b, a)
}

pub(crate) fn translucent(color: Rgba8, alpha: u8) -> Rgba8 {
    color.with_alpha(alpha)
}
