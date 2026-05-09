/// Sanitize pasted or typed text for single-line fields.
pub(in crate::gui_runtime::native_vello::text_edit) fn sanitize_single_line_insert(
    text: &str,
) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\r' | '\n' => {}
            '\t' => sanitized.push(' '),
            _ if ch.is_control() => {}
            _ => sanitized.push(ch),
        }
    }
    sanitized
}
