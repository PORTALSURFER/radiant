//! Portable MIME tag preparation for native format registration.

pub(super) fn normalized_external_drag_mime_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_mime_tag_normalization_preserves_the_caller_name() {
        let name = String::from("Application/X-Radiant");
        assert_eq!(
            normalized_external_drag_mime_name(&name),
            "application/x-radiant"
        );
        assert_eq!(name, "Application/X-Radiant");
    }
}
