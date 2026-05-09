//! Text-input character selection normalization for scene encoding.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TextInputSelectionBytes {
    pub(super) caret_byte: usize,
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
    pub(super) has_selection: bool,
}

pub(super) fn resolve_text_input_selection(
    text: &str,
    caret_char: usize,
    anchor_char: usize,
) -> TextInputSelectionBytes {
    let char_len = text.chars().count();
    let caret_char = caret_char.min(char_len);
    let anchor_char = anchor_char.min(char_len);
    let has_selection = caret_char != anchor_char;
    let start_char = caret_char.min(anchor_char);
    let end_char = if has_selection {
        caret_char.max(anchor_char).saturating_add(1)
    } else {
        caret_char
    }
    .min(char_len);

    TextInputSelectionBytes {
        caret_byte: byte_index_for_char(text, caret_char),
        start_byte: byte_index_for_char(text, start_char),
        end_byte: byte_index_for_char(text, end_char),
        has_selection,
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_bytes_clamp_to_text_and_expand_active_selection() {
        assert_eq!(
            resolve_text_input_selection("abcdef", 3, 1),
            TextInputSelectionBytes {
                caret_byte: 3,
                start_byte: 1,
                end_byte: 4,
                has_selection: true,
            }
        );

        assert_eq!(
            resolve_text_input_selection("abcdef", 99, 99),
            TextInputSelectionBytes {
                caret_byte: 6,
                start_byte: 6,
                end_byte: 6,
                has_selection: false,
            }
        );
    }

    #[test]
    fn selection_bytes_preserve_utf8_boundaries() {
        assert_eq!(
            resolve_text_input_selection("aé日b", 2, 0),
            TextInputSelectionBytes {
                caret_byte: 3,
                start_byte: 0,
                end_byte: 6,
                has_selection: true,
            }
        );
    }
}
