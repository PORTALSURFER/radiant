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

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::stable_widget_id;

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
}
