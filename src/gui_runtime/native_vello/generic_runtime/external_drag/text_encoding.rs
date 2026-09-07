/// Encodes the standard `CF_UNICODETEXT` representation: CRLF-normalized
/// UTF-16 followed by exactly one NUL code unit.
pub(super) fn encode_unicode_text(text: &str) -> Vec<u8> {
    let mut utf16 = Vec::with_capacity(text.len().saturating_add(1));
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\r' => {
                utf16.push('\r' as u16);
                if chars.peek() == Some(&'\n') {
                    utf16.push('\n' as u16);
                    let _ = chars.next();
                } else {
                    utf16.push('\n' as u16);
                }
            }
            '\n' => {
                utf16.push('\r' as u16);
                utf16.push('\n' as u16);
            }
            _ => {
                let mut encoded = [0; 2];
                utf16.extend_from_slice(character.encode_utf16(&mut encoded));
            }
        }
    }
    utf16.push(0);
    utf16.into_iter().flat_map(u16::to_le_bytes).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_text_uses_crlf_utf16_and_one_terminator() {
        let encoded = encode_unicode_text("one\ntwo\rthree\r\nfour😀");
        let code_units = encoded
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();

        assert_eq!(
            String::from_utf16(&code_units[..code_units.len() - 1]).unwrap(),
            "one\r\ntwo\r\nthree\r\nfour😀"
        );
        assert_eq!(code_units.iter().filter(|unit| **unit == 0).count(), 1);
        assert_eq!(code_units.last(), Some(&0));
    }

    #[test]
    fn empty_text_is_one_terminator() {
        assert_eq!(encode_unicode_text(""), vec![0, 0]);
    }

    #[test]
    fn maximum_newline_expansion_stays_bounded() {
        let text = "\n".repeat(crate::runtime::MAX_EXTERNAL_OFFER_TEXT_BYTES);
        assert_eq!(encode_unicode_text(&text).len(), text.len() * 4 + 2);
    }
}
