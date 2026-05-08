//! Shared environment-flag parsing helpers for runtime configuration toggles.

/// Return whether a string value maps to the crate's canonical truthy tokens.
pub(crate) fn is_truthy(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

/// Return whether the named environment variable is present and truthy.
pub(crate) fn env_var_truthy(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| is_truthy(&value))
}

#[cfg(test)]
/// Parser behavior tests for shared runtime env flags.
mod tests {
    use super::*;

    #[test]
    /// Shared parser should accept canonical truthy variants.
    fn truthy_parser_accepts_supported_tokens() {
        assert!(is_truthy("1"));
        assert!(is_truthy("true"));
        assert!(is_truthy("TRUE"));
        assert!(is_truthy("On"));
        assert!(is_truthy(" yes "));
    }

    #[test]
    /// Shared parser should reject unsupported or empty values.
    fn truthy_parser_rejects_non_truthy_tokens() {
        assert!(!is_truthy("0"));
        assert!(!is_truthy("false"));
        assert!(!is_truthy("off"));
        assert!(!is_truthy("no"));
        assert!(!is_truthy(""));
    }
}
