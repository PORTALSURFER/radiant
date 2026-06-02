use super::WidgetId;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Derive a stable widget id from a caller-owned scope and key.
///
/// Use this for dynamic controls whose runtime state must survive projection
/// order changes, such as row hit targets, inline editors, and retained custom
/// widgets. The scope is caller-owned so independent widget families can use
/// the same app key without colliding.
pub fn stable_widget_id(scope: u64, key: impl AsRef<str>) -> WidgetId {
    let mut hash = FNV_OFFSET;
    hash = hash_bytes(hash, &scope.to_le_bytes());
    hash = hash_bytes(hash, key.as_ref().as_bytes());
    if hash == 0 { 1 } else { hash }
}

/// Derive a stable widget id from a caller-owned scope and numeric key.
///
/// Use this for dynamic controls keyed by durable numeric app IDs or compact
/// enum indexes. It follows the same scope-separation contract as
/// [`stable_widget_id`] without requiring callers to allocate a temporary
/// string during projection.
pub fn stable_widget_id_u64(scope: u64, key: u64) -> WidgetId {
    let mut hash = FNV_OFFSET;
    hash = hash_bytes(hash, &scope.to_le_bytes());
    hash = hash_bytes(hash, &[1]);
    hash = hash_bytes(hash, &key.to_le_bytes());
    if hash == 0 { 1 } else { hash }
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{stable_widget_id, stable_widget_id_u64};

    #[test]
    fn stable_widget_id_is_deterministic_for_same_scope_and_key() {
        assert_eq!(stable_widget_id(42, "row-a"), stable_widget_id(42, "row-a"));
    }

    #[test]
    fn stable_widget_id_separates_scopes() {
        assert_ne!(stable_widget_id(42, "row-a"), stable_widget_id(43, "row-a"));
    }

    #[test]
    fn stable_widget_id_separates_keys() {
        assert_ne!(stable_widget_id(42, "row-a"), stable_widget_id(42, "row-b"));
    }

    #[test]
    fn stable_widget_id_u64_is_deterministic_for_same_scope_and_key() {
        assert_eq!(stable_widget_id_u64(42, 7), stable_widget_id_u64(42, 7));
    }

    #[test]
    fn stable_widget_id_u64_separates_scopes() {
        assert_ne!(stable_widget_id_u64(42, 7), stable_widget_id_u64(43, 7));
    }

    #[test]
    fn stable_widget_id_u64_separates_keys() {
        assert_ne!(stable_widget_id_u64(42, 7), stable_widget_id_u64(42, 8));
    }

    #[test]
    fn stable_widget_id_u64_uses_a_distinct_key_domain_from_text_keys() {
        assert_ne!(
            stable_widget_id_u64(42, u64::from_le_bytes(*b"numeric!")),
            stable_widget_id(42, "numeric!")
        );
    }
}
