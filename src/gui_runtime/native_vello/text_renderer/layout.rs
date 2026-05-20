//! Glyph-layout helpers for the native text renderer.

use super::{GlyphLayout, TextCursorStop, TextLayout};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use vello::peniko::FontData;

pub(super) fn compute_layout(font: &FontData, text: &str, font_size: f32) -> Option<TextLayout> {
    let font_ref = skrifa::FontRef::from_index(font.data.as_ref(), font.index).ok()?;
    let charmap = font_ref.charmap();
    let metrics = font_ref.glyph_metrics(FontSize::new(font_size), LocationRef::default());
    let fallback_glyph = charmap.map('?');

    let rendered_len = rendered_line_capacity_hint(text);
    let requires_shaping = rendered_line_requires_shaping(text);
    let mut x = 0.0_f32;
    let mut fallback_glyphs = 0_u64;
    let mut missing_glyphs = 0_u64;
    let mut glyphs = Vec::with_capacity(rendered_len);
    let mut cursor_stops = Vec::with_capacity(rendered_len.saturating_add(1));
    cursor_stops.push(TextCursorStop {
        byte_index: 0,
        x: 0.0,
    });
    for (byte_index, ch) in text.char_indices() {
        if ch == '\n' || ch == '\r' {
            break;
        }
        if ch == '\t' {
            x += font_size * 2.0;
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
            continue;
        }
        if ch == ' ' {
            x += font_size * 0.33;
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
            continue;
        }
        if ch.is_control() {
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
            continue;
        }
        let glyph_id = match charmap.map(ch) {
            Some(glyph_id) => Some(glyph_id),
            None => {
                if fallback_glyph.is_some() {
                    fallback_glyphs = fallback_glyphs.saturating_add(1);
                } else {
                    missing_glyphs = missing_glyphs.saturating_add(1);
                }
                fallback_glyph
            }
        };
        let Some(glyph_id) = glyph_id else {
            x += font_size * 0.5;
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
            continue;
        };
        glyphs.push(GlyphLayout {
            id: glyph_id.to_u32(),
            x,
        });
        let advance = metrics
            .advance_width(glyph_id)
            .unwrap_or(font_size * 0.55)
            .max(0.0);
        x += advance;
        cursor_stops.push(TextCursorStop {
            byte_index: byte_index + ch.len_utf8(),
            x,
        });
    }

    Some(TextLayout {
        width: x,
        glyphs,
        cursor_stops,
        unsupported_shaping_runs: u64::from(requires_shaping),
        unsupported_shaping_scalars: if requires_shaping {
            rendered_len as u64
        } else {
            0
        },
        fallback_glyphs,
        missing_glyphs,
    })
}

fn rendered_line_capacity_hint(text: &str) -> usize {
    rendered_line(text).map_or(0, |rendered| {
        if rendered.is_ascii() {
            rendered.len()
        } else {
            rendered.chars().count()
        }
    })
}

fn rendered_line_requires_shaping(text: &str) -> bool {
    rendered_line(text).is_some_and(|line| line.chars().any(char_requires_shaping))
}

fn rendered_line(text: &str) -> Option<&str> {
    let rendered_byte_len = text.find(['\n', '\r']).unwrap_or(text.len());
    let rendered = &text[..rendered_byte_len];
    (!rendered.is_empty()).then_some(rendered)
}

fn char_requires_shaping(ch: char) -> bool {
    matches!(
        ch,
        '\u{0300}'..='\u{036f}'
            | '\u{0590}'..='\u{05ff}'
            | '\u{0600}'..='\u{06ff}'
            | '\u{0750}'..='\u{077f}'
            | '\u{08a0}'..='\u{08ff}'
            | '\u{0900}'..='\u{0d7f}'
            | '\u{0e00}'..='\u{0e7f}'
            | '\u{200c}'..='\u{200d}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{1f3fb}'..='\u{1f3ff}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_line_capacity_hint_stops_at_line_breaks() {
        assert_eq!(rendered_line_capacity_hint("title\nignored"), 5);
        assert_eq!(rendered_line_capacity_hint("title\rignored"), 5);
        assert_eq!(rendered_line_capacity_hint("plain"), 5);
    }

    #[test]
    fn rendered_line_capacity_hint_counts_unicode_scalars_not_bytes() {
        assert_eq!("øß猫".len(), 7);
        assert_eq!(rendered_line_capacity_hint("øß猫"), 3);
    }

    #[test]
    fn rendered_line_requires_shaping_for_complex_unicode_sequences() {
        assert!(!rendered_line_requires_shaping("plain ascii"));
        assert!(!rendered_line_requires_shaping("øß猫"));
        assert!(rendered_line_requires_shaping("cafe\u{0301}"));
        assert!(rendered_line_requires_shaping("مرحبا"));
        assert!(rendered_line_requires_shaping("👋\u{1f3fd}"));
    }

    #[test]
    fn rendered_line_requires_shaping_stops_at_line_breaks() {
        assert!(!rendered_line_requires_shaping("plain\nمرحبا"));
        assert!(rendered_line_requires_shaping("مرحبا\nplain"));
    }
}
