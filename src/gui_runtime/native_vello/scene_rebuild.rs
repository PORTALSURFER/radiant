//! Static-scene rebuild policy and runtime env-flag parsing helpers.

use super::*;

pub(super) fn parse_truthy_env(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

/// Resolve static-scene rebuild behavior for one frame update.
///
/// `static_rebuild_requested` represents explicit runtime invalidations
/// (for example layout or full-static scopes). When no explicit static rebuild
/// is requested, bridge dirty segments decide whether static content must
/// rebuild during model refreshes.
pub(super) fn resolve_static_rebuild(
    model_refresh_requested: bool,
    static_rebuild_requested: bool,
    bridge_dirty_segments: DirtySegments,
) -> bool {
    if !model_refresh_requested {
        return static_rebuild_requested;
    }
    if bridge_dirty_segments.requires_static_rebuild() {
        return true;
    }
    static_rebuild_requested
}

/// Return whether bridge dirty segments forced a static rebuild for this refresh.
pub(super) fn static_rebuild_from_dirty_mask(
    model_refresh_requested: bool,
    static_rebuild_requested: bool,
    bridge_dirty_segments: DirtySegments,
) -> bool {
    model_refresh_requested
        && !static_rebuild_requested
        && bridge_dirty_segments.requires_static_rebuild()
}
