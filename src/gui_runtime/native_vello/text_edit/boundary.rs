//! UTF-8 byte-boundary helpers for native text-input rendering.

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
}
