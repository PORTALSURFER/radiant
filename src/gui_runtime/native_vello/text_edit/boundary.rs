//! UTF-8 byte-boundary navigation for single-line native text editing.

pub(super) fn clamp_to_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = byte_index.min(text.len());
    if text.is_char_boundary(clamped) {
        return clamped;
    }
    let mut last = 0;
    for (idx, _) in text.char_indices() {
        if idx >= clamped {
            break;
        }
        last = idx;
    }
    last
}

pub(super) fn previous_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    text[..clamped]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

pub(super) fn next_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    if clamped >= text.len() {
        return text.len();
    }
    let mut iter = text[clamped..].char_indices();
    let _ = iter.next();
    iter.next()
        .map(|(idx, _)| clamped + idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_to_char_boundary_snaps_inside_multibyte_characters_backward() {
        let text = "aé日";

        assert_eq!(clamp_to_char_boundary(text, 0), 0);
        assert_eq!(clamp_to_char_boundary(text, 2), 1);
        assert_eq!(clamp_to_char_boundary(text, 4), 3);
        assert_eq!(clamp_to_char_boundary(text, usize::MAX), text.len());
    }

    #[test]
    fn char_boundary_navigation_steps_over_complete_utf8_scalars() {
        let text = "aé日";

        assert_eq!(next_char_boundary(text, 0), 1);
        assert_eq!(next_char_boundary(text, 1), 3);
        assert_eq!(next_char_boundary(text, 3), text.len());
        assert_eq!(previous_char_boundary(text, text.len()), 3);
        assert_eq!(previous_char_boundary(text, 3), 1);
        assert_eq!(previous_char_boundary(text, 1), 0);
    }
}
