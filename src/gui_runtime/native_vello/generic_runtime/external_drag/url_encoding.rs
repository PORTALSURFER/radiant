/// Encodes the standard Unicode URL drag representation: UTF-16 followed by
/// exactly one NUL code unit. URL bytes are deliberately not line-ending
/// normalized.
pub(super) fn encode_unicode_url(url: &str) -> Vec<u8> {
    url.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_url_keeps_code_units_and_one_terminator_without_crlf_rewrite() {
        let url = "custom://example.test/a\nb?label=Jöhn😀";
        let encoded = encode_unicode_url(url);
        let code_units = encoded
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();

        assert_eq!(
            String::from_utf16(&code_units[..code_units.len() - 1]).unwrap(),
            url
        );
        assert_eq!(code_units.iter().filter(|unit| **unit == 0).count(), 1);
        assert_eq!(code_units.last(), Some(&0));
    }
}
