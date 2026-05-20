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

pub(super) fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
