use super::*;

#[test]
fn preference_panel_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/form.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/form/tests.rs"))
        .expect("form root behavior tests should be readable");

    assert!(
        source.contains("pub struct PreferencePanelParts")
            && source.contains("pub enum PreferencePanelVisibility")
            && source.contains("pub visibility: PreferencePanelVisibility")
            && source.contains("pub fn from_parts(parts: PreferencePanelParts<TOGGLES>) -> Self")
            && source.contains("#[path = \"form/tests.rs\"]")
            && !source.contains(
                "fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label"
            ),
        "preference panel state should expose a named parts object for readable public construction while delegating behavior tests"
    );
    assert!(
        tests.contains("fn option_item_preserves_label_selection_and_value")
            && tests.contains("fn option_item_supports_named_selection_parts")
            && tests.contains(
                "fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label"
            ),
        "form root behavior coverage should live in form/tests.rs"
    );
    assert!(
        source.contains("pub struct OptionItemParts<Value>")
            && source.contains("pub enum OptionSelectionState")
            && source.contains("pub fn from_parts(parts: OptionItemParts<Value>) -> Self"),
        "generic form option items should expose named parts for readable selection construction"
    );
    assert!(
        source.contains("mod numeric;")
            && source.contains("mod paired;")
            && source.contains("DecimalTextInputPolicy")
            && source.contains("PairedStatusPanel"),
        "form root should remain the focused public facade for generic form helpers"
    );
    assert!(
        source.contains("Self::from_parts(PreferencePanelParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
    assert!(
        source.contains("PreferencePanelVisibility::from_visible(visible)")
            && tests.contains("fn preference_panel_visibility_round_trips_compatibility_flags"),
        "preference panel compatibility flags should round-trip through the named visibility state"
    );
}

#[test]
fn form_numeric_and_paired_helpers_keep_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let numeric = fs::read_to_string(manifest_dir.join("src/gui/form/numeric.rs"))
        .expect("form numeric helpers should be readable");
    let numeric_tests = fs::read_to_string(manifest_dir.join("src/gui/form/numeric/tests.rs"))
        .expect("form numeric behavior tests should be readable");
    let paired = fs::read_to_string(manifest_dir.join("src/gui/form/paired.rs"))
        .expect("form paired helpers should be readable");
    let paired_tests = fs::read_to_string(manifest_dir.join("src/gui/form/paired/tests.rs"))
        .expect("form paired behavior tests should be readable");

    assert!(
        numeric.contains("pub struct DecimalTextInputPolicy")
            && numeric.contains("pub fn sanitize_decimal_text_insert")
            && numeric.contains("#[path = \"numeric/tests.rs\"]")
            && !numeric.contains("fn decimal_text_insert_keeps_digits_and_one_decimal_point"),
        "numeric form helpers should keep behavior tests delegated"
    );
    assert!(
        numeric_tests.contains("fn decimal_text_insert_keeps_digits_and_one_decimal_point")
            && numeric_tests.contains("fn rounded_scaled_u16_clamps_non_finite_and_large_values"),
        "numeric form behavior coverage should live in form/numeric/tests.rs"
    );
    assert!(
        paired.contains("pub enum PairedPickerTarget")
            && paired.contains("pub struct PairedStatusPanel")
            && paired.contains("#[path = \"paired/tests.rs\"]")
            && !paired.contains("fn paired_picker_models_cover_primary_and_secondary_fields"),
        "paired form helpers should keep behavior tests delegated"
    );
    assert!(
        paired_tests.contains("fn paired_picker_models_cover_primary_and_secondary_fields")
            && paired_tests.contains("fn paired_status_panel_returns_options_for_target"),
        "paired form behavior coverage should live in form/paired/tests.rs"
    );
}

#[test]
fn gui_core_state_primitives_keep_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let focus = fs::read_to_string(manifest_dir.join("src/gui/focus.rs"))
        .expect("gui focus primitive should be readable");
    let focus_tests = fs::read_to_string(manifest_dir.join("src/gui/focus/tests.rs"))
        .expect("gui focus behavior tests should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/gui/frame.rs"))
        .expect("gui frame feedback primitive should be readable");
    let frame_tests = fs::read_to_string(manifest_dir.join("src/gui/frame/tests.rs"))
        .expect("gui frame behavior tests should be readable");
    let selection = fs::read_to_string(manifest_dir.join("src/gui/selection.rs"))
        .expect("gui selection primitive should be readable");
    let selection_tests = fs::read_to_string(manifest_dir.join("src/gui/selection/tests.rs"))
        .expect("gui selection behavior tests should be readable");

    assert!(
        focus.contains("pub enum FocusSurface")
            && focus.contains("#[path = \"focus/tests.rs\"]")
            && !focus.contains("fn focus_surface_defaults_to_none"),
        "focus surface state should live in gui/focus.rs while behavior tests stay delegated"
    );
    assert!(
        focus_tests.contains("fn focus_surface_defaults_to_none"),
        "focus behavior coverage should live in gui/focus/tests.rs"
    );
    assert!(
        frame.contains("pub struct FrameBuildCounts")
            && frame.contains("pub struct FrameRebuildFlags")
            && frame.contains("pub struct FrameAnimationRequest")
            && frame.contains("pub struct FrameBuildTiming")
            && frame.contains("pub struct FramePresentResult")
            && frame.contains("pub struct FrameBuildResult")
            && frame.contains("pub struct FrameCadenceMonitor")
            && frame.contains("pub struct FrameCadenceConfig")
            && frame.contains("pub enum FrameCadenceKind")
            && frame.contains("#[path = \"frame/tests.rs\"]")
            && !frame.contains("fn frame_build_result_defaults_to_no_work_observed"),
        "frame feedback state should stay grouped by counts, rebuilds, animation, timing, presentation, and cadence while behavior tests stay delegated"
    );
    assert!(
        frame_tests.contains("fn frame_build_result_defaults_to_no_work_observed")
            && frame_tests.contains("fn frame_build_result_groups_related_feedback")
            && frame_tests
                .contains("fn frame_cadence_monitor_classifies_start_normal_periodic_and_spikes"),
        "frame behavior coverage should live in gui/frame/tests.rs"
    );
    assert!(
        selection.contains("pub enum TriState")
            && selection.contains("pub enum TriageTarget")
            && selection.contains("#[path = \"selection/tests.rs\"]")
            && !selection.contains("fn tri_state_defaults_to_off"),
        "selection state should live in gui/selection.rs while behavior tests stay delegated"
    );
    assert!(
        selection_tests.contains("fn tri_state_defaults_to_off")
            && selection_tests.contains("fn triage_target_names_generic_three_way_selection"),
        "selection behavior coverage should live in gui/selection/tests.rs"
    );
}

#[test]
fn text_line_layout_keeps_insets_and_placement_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/text_layout/mod.rs"))
        .expect("text layout module root should be readable");
    let insets = fs::read_to_string(manifest_dir.join("src/gui/text_layout/insets.rs"))
        .expect("text layout insets model should be readable");
    let placement = fs::read_to_string(manifest_dir.join("src/gui/text_layout/placement.rs"))
        .expect("text layout placement helpers should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/text_layout/tests.rs"))
        .expect("text layout behavior tests should be readable");

    assert!(
        root.contains("mod insets;")
            && root.contains("mod placement;")
            && root.contains("pub use insets::TextLineInsets;")
            && root.contains("pub use placement::snap_text_baseline_to_pixel;")
            && !root.contains("pub struct TextLineInsets")
            && !root.contains("fn inset_rect"),
        "text layout root should own public API wiring while insets and placement stay delegated"
    );
    assert!(
        insets.contains("pub struct TextLineInsets")
            && insets.contains("pub fn symmetric")
            && insets.contains("pub fn horizontal"),
        "text-line inset data should live in gui/text_layout/insets.rs"
    );
    assert!(
        placement.contains("pub(super) fn compute_text_line")
            && placement.contains("fn clamp_min_top")
            && placement.contains("fn inset_rect")
            && placement.contains("pub fn snap_text_baseline_to_pixel"),
        "text-line placement math should live in gui/text_layout/placement.rs"
    );
    assert!(
        tests.contains("fn centered_line_reuses_cached_geometry_for_identical_inputs")
            && tests.contains("fn snap_text_baseline_to_pixel_keeps_height_and_rounds_bottom_edge"),
        "text-line cache and placement behavior coverage should stay in gui/text_layout/tests.rs"
    );
}
