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
    let chars: Vec<char> = value.chars().collect();
    let word_index = if caret < char_len && is_word_char(chars[caret]) {
        caret
    } else if caret > 0 && is_word_char(chars[caret - 1]) {
        caret - 1
    } else {
        return None;
    };
    let mut start = word_index;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = word_index + 1;
    while end < char_len && is_word_char(chars[end]) {
        end += 1;
    }
    Some((start, end))
}

pub(super) fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
