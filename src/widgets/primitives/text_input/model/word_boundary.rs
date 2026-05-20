pub(super) fn previous_word_boundary(value: &str, caret: usize) -> usize {
    let mut previous_was_word = false;
    let mut last_word_start = 0;
    let mut saw_word = false;
    for (index, character) in value.chars().take(caret).enumerate() {
        let word_char = is_word_char(character);
        if word_char && !previous_was_word {
            last_word_start = index;
            saw_word = true;
        }
        previous_was_word = word_char;
    }
    if saw_word { last_word_start } else { 0 }
}

pub(super) fn next_word_boundary(value: &str, caret: usize) -> usize {
    let mut saw_word = false;
    for (offset, character) in value.chars().skip(caret).enumerate() {
        let word_char = is_word_char(character);
        if word_char {
            saw_word = true;
        } else if saw_word {
            return caret + offset;
        }
    }
    value.chars().count()
}

pub(super) fn word_range_at(value: &str, caret: usize) -> Option<(usize, usize)> {
    let char_len = value.chars().count();
    let caret = caret.min(char_len);
    let mut active_word_start = None;

    for (index, character) in value.chars().enumerate() {
        if is_word_char(character) {
            active_word_start.get_or_insert(index);
        } else if let Some(start) = active_word_start.take()
            && (start..=index).contains(&caret)
        {
            return Some((start, index));
        }
    }

    active_word_start.and_then(|start| {
        if (start..=char_len).contains(&caret) {
            Some((start, char_len))
        } else {
            None
        }
    })
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
}
