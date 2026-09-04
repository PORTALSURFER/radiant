use unicode_segmentation::UnicodeSegmentation;

pub(super) fn previous_word_boundary(value: &str, caret: usize) -> usize {
    let caret = caret.min(value.chars().count());
    let mut previous_was_word = false;
    let mut last_word_start = 0;
    let mut saw_word = false;
    for (start, _, grapheme) in grapheme_ranges(value) {
        if start >= caret {
            break;
        }
        let word_grapheme = is_word_grapheme(grapheme);
        if word_grapheme && !previous_was_word {
            last_word_start = start;
            saw_word = true;
        }
        previous_was_word = word_grapheme;
    }
    if saw_word { last_word_start } else { 0 }
}

pub(super) fn next_word_boundary(value: &str, caret: usize) -> usize {
    let caret = caret.min(value.chars().count());
    let mut saw_word = false;
    for (start, end, grapheme) in grapheme_ranges(value) {
        if end <= caret {
            continue;
        }
        if is_word_grapheme(grapheme) {
            saw_word = true;
        } else if saw_word {
            return start;
        }
    }
    value.chars().count()
}

pub(super) fn word_range_at(value: &str, caret: usize) -> Option<(usize, usize)> {
    let char_len = value.chars().count();
    let caret = caret.min(char_len);
    let mut active_word_start = None;
    let mut active_word_end = 0;

    for (start, end, grapheme) in grapheme_ranges(value) {
        if is_word_grapheme(grapheme) {
            active_word_start.get_or_insert(start);
            active_word_end = end;
        } else if let Some(word_start) = active_word_start.take()
            && (word_start..=active_word_end).contains(&caret)
        {
            return Some((word_start, active_word_end));
        }
    }

    active_word_start.and_then(|word_start| {
        if (word_start..=active_word_end).contains(&caret) {
            Some((word_start, active_word_end))
        } else {
            None
        }
    })
}

fn grapheme_ranges(value: &str) -> Vec<(usize, usize, &str)> {
    let mut scalar = 0;
    value
        .graphemes(true)
        .map(|grapheme| {
            let start = scalar;
            scalar += grapheme.chars().count();
            (start, scalar, grapheme)
        })
        .collect()
}

fn is_word_grapheme(grapheme: &str) -> bool {
    grapheme.chars().any(is_word_char)
}

pub(super) fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_at_selects_current_or_previous_word() {
        let value = "alpha  beta_gamma.日文";

        assert_eq!(word_range_at(value, 9), Some((7, 17)));
        assert_eq!(word_range_at(value, 17), Some((7, 17)));
        assert_eq!(word_range_at(value, 19), Some((18, 20)));
    }

    #[test]
    fn word_range_at_rejects_separators_and_clamps_the_caret() {
        let value = "alpha  beta";

        assert_eq!(word_range_at(value, 5), Some((0, 5)));
        assert_eq!(word_range_at(value, 6), None);
        assert_eq!(word_range_at(value, 999), Some((7, 11)));
        assert_eq!(word_range_at("", 0), None);
    }

    #[test]
    fn word_boundaries_keep_combining_graphemes_together_in_scalar_offsets() {
        let value = "e\u{301} next";

        assert_eq!(previous_word_boundary(value, 1), 0);
        assert_eq!(next_word_boundary(value, 1), 2);
        assert_eq!(word_range_at(value, 1), Some((0, 2)));
        assert_eq!(word_range_at(value, 2), Some((0, 2)));
    }

    #[test]
    fn word_boundaries_keep_zwj_and_non_bmp_words_and_separators_intact() {
        let value = "क्\u{200d}ष 👩\u{200d}\u{1f52c} \u{10400}\u{301}";

        assert_eq!(word_range_at(value, 2), Some((0, 4)));
        assert_eq!(word_range_at(value, 4), Some((0, 4)));
        assert_eq!(word_range_at(value, 6), None);
        assert_eq!(word_range_at(value, 9), Some((9, 11)));
        assert_eq!(word_range_at(value, 10), Some((9, 11)));
        assert_eq!(next_word_boundary(value, 2), 4);
        assert_eq!(next_word_boundary(value, 9), 11);
        assert_eq!(previous_word_boundary(value, 10), 9);
    }
}
