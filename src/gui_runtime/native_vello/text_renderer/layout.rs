//! Glyph-layout helpers for the native text renderer.

use super::{GlyphLayout, TextCursorStop, TextLayout, font::NativeFontStack};

pub(super) fn compute_layout(
    font_stack: &mut NativeFontStack,
    text: &str,
    font_size: f32,
) -> Option<TextLayout> {
    if font_stack.is_empty() {
        return None;
    }
    let rendered_metrics = rendered_line_metrics(text);
    let mut x = 0.0_f32;
    let mut fallback_glyphs = 0_u64;
    let mut missing_glyphs = 0_u64;
    let mut glyphs = Vec::with_capacity(rendered_metrics.capacity_hint);
    let mut cursor_stops = Vec::with_capacity(rendered_metrics.capacity_hint.saturating_add(1));
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
        let glyph = font_stack.resolve_glyph(ch).or_else(|| {
            let fallback = font_stack.fallback_glyph();
            if fallback.is_some() {
                fallback_glyphs = fallback_glyphs.saturating_add(1);
            } else {
                missing_glyphs = missing_glyphs.saturating_add(1);
            }
            fallback
        });
        let Some(glyph) = glyph else {
            x += font_size * 0.5;
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
            continue;
        };
        glyphs.push(GlyphLayout {
            face_index: glyph.face_index,
            id: glyph.glyph_id,
            x,
        });
        let advance = font_stack.glyph_advance(glyph, font_size);
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
        unsupported_shaping_runs: u64::from(rendered_metrics.requires_shaping),
        unsupported_shaping_scalars: if rendered_metrics.requires_shaping {
            rendered_metrics.capacity_hint as u64
        } else {
            0
        },
        fallback_glyphs,
        missing_glyphs,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderedLineMetrics {
    capacity_hint: usize,
    requires_shaping: bool,
}

#[cfg(test)]
fn rendered_line_capacity_hint(text: &str) -> usize {
    rendered_line_metrics(text).capacity_hint
}

#[cfg(test)]
fn rendered_line_requires_shaping(text: &str) -> bool {
    rendered_line_metrics(text).requires_shaping
}

fn rendered_line(text: &str) -> Option<&str> {
    let rendered_byte_len = text.find(['\n', '\r']).unwrap_or(text.len());
    let rendered = &text[..rendered_byte_len];
    (!rendered.is_empty()).then_some(rendered)
}

fn rendered_line_metrics(text: &str) -> RenderedLineMetrics {
    let Some(rendered) = rendered_line(text) else {
        return RenderedLineMetrics {
            capacity_hint: 0,
            requires_shaping: false,
        };
    };
    if rendered.is_ascii() {
        return RenderedLineMetrics {
            capacity_hint: rendered.len(),
            requires_shaping: false,
        };
    }
    let mut scalar_count = 0;
    let mut requires_shaping = false;
    for ch in rendered.chars() {
        scalar_count += 1;
        requires_shaping |= char_requires_shaping(ch);
    }
    RenderedLineMetrics {
        capacity_hint: scalar_count,
        requires_shaping,
    }
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
    use crate::gui_runtime::native_vello::text_renderer::font::NativeFontStack;

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

    #[test]
    fn rendered_line_metrics_combines_capacity_and_shaping_scan() {
        assert_eq!(
            rendered_line_metrics("plain\nمرحبا"),
            RenderedLineMetrics {
                capacity_hint: 5,
                requires_shaping: false,
            }
        );
        assert_eq!(
            rendered_line_metrics("øß猫"),
            RenderedLineMetrics {
                capacity_hint: 3,
                requires_shaping: false,
            }
        );
        assert_eq!(
            rendered_line_metrics("cafe\u{0301}"),
            RenderedLineMetrics {
                capacity_hint: 5,
                requires_shaping: true,
            }
        );
    }

    #[test]
    fn rendered_line_metrics_uses_byte_capacity_for_ascii_prefix() {
        assert_eq!(
            rendered_line_metrics("ASCII label\t42\rignored"),
            RenderedLineMetrics {
                capacity_hint: 14,
                requires_shaping: false,
            }
        );
    }

    #[test]
    fn complementary_faces_resolve_each_scalar_before_question_mark_fallback() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);

        let layout = compute_layout(&mut stack, "AΩA", 20.0).expect("fixture fonts load");

        assert_eq!(
            layout
                .glyphs
                .iter()
                .map(|glyph| glyph.face_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 0]
        );
        assert_eq!(layout.fallback_glyphs, 0);
        assert_eq!(layout.missing_glyphs, 0);
        let primary_advance = layout.glyphs[1].x - layout.glyphs[0].x;
        let secondary_advance = layout.glyphs[2].x - layout.glyphs[1].x;
        assert!(secondary_advance > primary_advance);
        assert!(layout.width > layout.glyphs[2].x);
        assert_eq!(
            layout.cursor_stops.last().map(|stop| stop.byte_index),
            Some(4)
        );
    }

    #[test]
    fn absent_scalar_uses_ordered_question_mark_and_tracks_missing_only_without_one() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let fallback = compute_layout(&mut stack, "Ж", 20.0).expect("fixture fonts load");
        assert_eq!(fallback.fallback_glyphs, 1);
        assert_eq!(fallback.missing_glyphs, 0);

        let mut empty_stack = NativeFontStack::from_test_bytes(&[]);
        assert!(compute_layout(&mut empty_stack, "Ж", 20.0).is_none());

        let mut no_question = NativeFontStack::from_test_bytes(&[include_bytes!(
            "../../../../tests/fixtures/fonts/no_question.ttf"
        )]);
        let missing = compute_layout(&mut no_question, "Ж", 20.0).expect("fixture font loads");
        assert_eq!(missing.fallback_glyphs, 0);
        assert_eq!(missing.missing_glyphs, 1);
        assert!(missing.glyphs.is_empty());
        assert_eq!(missing.width, 10.0);
    }
}
