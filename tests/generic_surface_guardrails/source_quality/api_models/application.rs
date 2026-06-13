use super::*;

const MAX_APPLICATION_ROOT_FACADE_LINES: usize = 80;
const MAX_APPLICATION_EXPORT_GROUP_LINES: usize = 32;

const APPLICATION_FACADE_LEAVES: &[&str] = &[
    "controls", "details", "layout", "menus", "overlays", "panels", "runtime", "surfaces", "view",
];

#[test]
fn application_facade_uses_explicit_public_exports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application facade should be readable");
    let view_facade = fs::read_to_string(manifest_dir.join("src/application/facade/view.rs"))
        .expect("application view facade should be readable");
    let layout_facade = fs::read_to_string(manifest_dir.join("src/application/facade/layout.rs"))
        .expect("application layout facade should be readable");
    let details_facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");
    let menus_facade = fs::read_to_string(manifest_dir.join("src/application/facade/menus.rs"))
        .expect("application menus facade should be readable");
    let overlays_facade =
        fs::read_to_string(manifest_dir.join("src/application/facade/overlays.rs"))
            .expect("application overlays facade should be readable");
    let surfaces_facade =
        fs::read_to_string(manifest_dir.join("src/application/facade/surfaces.rs"))
            .expect("application surfaces facade should be readable");

    assert!(
        source.contains("mod facade;")
            && source.contains("pub use facade::*;")
            && !source.contains("pub use launch::{")
            && !source.contains("pub use layout_builders::{"),
        "application root should delegate public export ownership to focused facades"
    );
    assert!(
        view_facade.contains("pub use super::super::launch::{")
            && view_facade.contains("WindowBuilder")
            && view_facade.contains("StatefulAppBuilder")
            && layout_facade.contains("pub use super::super::layout_builders::{")
            && layout_facade.contains("virtual_list_window")
            && layout_facade.contains("DEFAULT_COLUMN_SPACING"),
        "focused application facades should name the launch and layout API surfaces explicitly"
    );
    assert!(
        !view_facade.contains("pub use super::super::launch::*;")
            && !layout_facade.contains("pub use super::super::layout_builders::*;"),
        "application facades should not wildcard-export public launch or layout builders"
    );
    assert!(
        details_facade.contains("CompactOptionListParts")
            && !menus_facade.contains("CompactOptionListParts")
            && overlays_facade.contains("DropdownMenuOverlayBelowParts")
            && overlays_facade.contains("floating_layer_below")
            && overlays_facade.contains("PointerShieldBuilder")
            && !layout_facade.contains("DropdownMenuOverlayBelowParts")
            && surfaces_facade.contains("DynamicWidgetParts")
            && !view_facade.contains("DynamicWidgetParts"),
        "application facade groups should follow prelude API roles instead of implementation-module ownership"
    );
}

#[test]
fn application_root_stays_small_and_delegates_export_ownership() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let direct_public_exports = source
        .lines()
        .filter_map(direct_public_application_export)
        .collect::<Vec<_>>();

    assert!(
        source.lines().count() <= MAX_APPLICATION_ROOT_FACADE_LINES,
        "application.rs should stay a small subsystem root; move public export groups to src/application/facade/* before it becomes a registry again"
    );
    assert!(
        source.contains("mod facade;") && source.contains("pub use facade::*;"),
        "application.rs should delegate public app-facing export ownership to application/facade"
    );
    assert!(
        direct_public_exports.is_empty(),
        "application.rs should not directly reintroduce public export lists; add exports to the smallest application/facade leaf:\n{}",
        direct_public_exports.join("\n")
    );
}

#[test]
fn application_facade_export_groups_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_dir = manifest_dir.join("src/application/facade");

    let oversized = rust_sources_under(&facade_dir)
        .into_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "application facade group {} should be readable: {err}",
                    path.display()
                )
            });
            let line_count = source.lines().count();
            (line_count > MAX_APPLICATION_EXPORT_GROUP_LINES).then(|| {
                format!(
                    "{} ({line_count} lines)",
                    relative_path(&manifest_dir, &path)
                )
            })
        })
        .collect::<Vec<_>>();

    assert!(
        oversized.is_empty(),
        "application facade export groups should stay small enough to scan; split or regroup broad leaves before they become catch-all registries:\n{}",
        oversized.join("\n")
    );
}

#[test]
fn application_facade_leaves_match_prelude_roles() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_root = fs::read_to_string(manifest_dir.join("src/application/facade.rs"))
        .expect("application facade root should be readable");

    for leaf in APPLICATION_FACADE_LEAVES {
        assert!(
            facade_root.contains(&format!("mod {leaf};"))
                && facade_root.contains(&format!("pub use {leaf}::{{"))
                && manifest_dir
                    .join("src/application/facade")
                    .join(format!("{leaf}.rs"))
                    .is_file(),
            "application facade leaf `{leaf}` should be declared in the root and backed by src/application/facade/{leaf}.rs"
        );
    }

    assert!(
        facade_root.contains("mirror the application prelude's API roles"),
        "application facade root should document that new app-facing exports follow the prelude-aligned ownership model"
    );
    assert!(
        APPLICATION_FACADE_LEAVES
            .iter()
            .all(|leaf| !facade_root.contains(&format!("pub use {leaf}::*;"))),
        "application facade root should explicitly name each grouped public export instead of wildcarding child facades"
    );
}

#[test]
fn application_facade_leaves_do_not_wildcard_export_implementation_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_dir = manifest_dir.join("src/application/facade");

    let offenders = rust_sources_under(&facade_dir)
        .into_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "application facade group {} should be readable: {err}",
                    path.display()
                )
            });
            let relative = relative_path(&manifest_dir, &path);
            source
                .lines()
                .filter(|line| application_facade_wildcard_export(line))
                .map(move |line| format!("{relative}: {}", line.trim()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "application facade leaves must name exported APIs explicitly instead of wildcard-exporting implementation modules:\n{}",
        offenders.join("\n")
    );
}

fn direct_public_application_export(line: &str) -> Option<String> {
    let trimmed = line.trim();
    (trimmed.starts_with("pub use ") && trimmed != "pub use facade::*;").then(|| trimmed.to_owned())
}

fn application_facade_wildcard_export(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("pub use ")
        && trimmed.ends_with("::*;")
        && (trimmed.contains("super::super::") || trimmed.contains("crate::application"))
}

#[test]
fn application_view_lowering_keeps_container_defaults_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/application/view_node.rs"))
        .expect("application view node module should be readable");
    let lowering = fs::read_to_string(manifest_dir.join("src/application/view_node/lowering.rs"))
        .expect("application view lowering should be readable");
    let defaults =
        fs::read_to_string(manifest_dir.join("src/application/view_node/lowering_defaults.rs"))
            .expect("application view lowering defaults should be readable");
    let defaults_tests = fs::read_to_string(
        manifest_dir.join("src/application/view_node/lowering_defaults/tests.rs"),
    )
    .expect("application view lowering default tests should be readable");

    assert!(
        module.contains("mod lowering_defaults;")
            && lowering.contains("ViewNodeContainerDefaults::new("),
        "view lowering should consume container defaults from a focused helper"
    );
    assert!(
        !lowering.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("fn default_container_padding")
            && defaults.contains("fn base_policy")
            && defaults.contains("#[path = \"lowering_defaults/tests.rs\"]")
            && !defaults.contains("fn styled_container_defaults_to_panel_padding"),
        "declarative container default policy should stay outside the main view lowering match"
    );
    assert!(
        defaults_tests.contains("fn styled_container_defaults_to_panel_padding")
            && defaults_tests
                .contains("fn explicit_container_defaults_override_style_padding_and_alignment"),
        "view lowering default behavior tests should live in lowering_defaults/tests.rs"
    );
}

#[test]
fn application_view_identity_keeps_reserved_id_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let identity = fs::read_to_string(manifest_dir.join("src/application/view_node/identity.rs"))
        .expect("application view node identity module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/view_node/identity/tests.rs"))
            .expect("application view node identity tests should be readable");

    assert!(
        identity.contains("pub(super) fn collect_reserved_ids")
            && identity.contains("fn reserve_child_identity_capacity")
            && identity.contains("fn reserved_identity_capacity_hint")
            && identity.contains("#[path = \"identity/tests.rs\"]")
            && !identity.contains("fn reserved_id_collection_presizes_for_large_child_groups"),
        "view-node identity collection should live in view_node/identity.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn reserved_id_collection_presizes_for_large_child_groups")
            && tests.contains("fn reserved_id_collection_includes_grid_child_identities")
            && tests.contains("fn reserved_id_collection_presizes_wrapped_runtime_identities"),
        "view-node identity behavior coverage should live in view_node/identity/tests.rs"
    );
}

#[test]
fn application_list_builders_keep_virtualization_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lists = fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists.rs"))
        .expect("application list builders should be readable");
    let virtual_builder = fs::read_to_string(
        manifest_dir.join("src/application/layout_builders/lists/virtual_builder.rs"),
    )
    .expect("virtual list builder module should be readable");
    let row_chrome = fs::read_to_string(
        manifest_dir.join("src/application/layout_builders/lists/row_chrome.rs"),
    )
    .expect("row chrome module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists/tests.rs"))
            .expect("application list builder tests should be readable");
    let virtual_window_tests = fs::read_to_string(
        manifest_dir.join("src/application/layout_builders/lists/tests/virtual_window.rs"),
    )
    .expect("virtual-window list tests should be readable");
    let bounded_scroll_tests = fs::read_to_string(
        manifest_dir.join("src/application/layout_builders/lists/tests/bounded_scroll.rs"),
    )
    .expect("bounded-scroll list tests should be readable");

    assert!(
        lists.contains("mod row_chrome;")
            && lists.contains("mod scroll_columns;")
            && lists.contains("mod scroll_update;")
            && lists.contains("mod tree_window;")
            && lists.contains("mod virtual_builder;")
            && lists.contains("mod virtual_window;")
            && lists.contains(
                "pub use virtual_window::{virtual_list_window, virtual_list_window_body};"
            )
            && lists.contains("#[path = \"lists/tests.rs\"]")
            && !lists.contains("pub fn virtual_list<Message, Item>")
            && !lists.contains("fn virtual_list_window_projects_only_materialized_range"),
        "application list builders should stay a focused facade over row, scroll, virtual-window, and tree-list owners"
    );
    assert!(
        virtual_builder.contains("pub struct VirtualListBuilder<Message, Project>")
            && virtual_builder.contains("pub fn virtual_list_windowed<Message, Project>")
            && row_chrome.contains("fn apply_list_row_chrome<Message>"),
        "virtual-list builder and row chrome helpers should live in their focused owner modules"
    );
    assert!(
        tests.contains("mod bounded_scroll;")
            && tests.contains("mod row_chrome;")
            && tests.contains("mod scroll_update;")
            && tests.contains("mod tree_window;")
            && tests.contains("mod virtual_window;")
            && !tests.contains("fn virtual_list_uses_packed_rows")
            && virtual_window_tests
                .contains("fn virtual_list_window_projects_only_materialized_range")
            && bounded_scroll_tests.contains("fn bounded_scroll_column_caps_visible_row_height"),
        "application list builder behavior coverage should be split by list-builder concern"
    );
}

#[test]
fn application_dropdown_builder_keeps_menu_overlay_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown.rs"))
            .expect("application dropdown builder root should be readable");
    let model =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/model.rs"))
            .expect("application dropdown model should be readable");
    let menu =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/menu.rs"))
            .expect("application dropdown menu module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/tests.rs"))
            .expect("application dropdown tests should be readable");

    assert!(
        root.contains("mod menu;")
            && root.contains("mod model;")
            && root.contains("DropdownOptionSelection")
            && root.contains("#[path = \"dropdown/tests.rs\"]")
            && root.contains("pub struct DropdownBuilder<Message>")
            && root.contains("pub struct DropdownTriggerBuilder<Message>")
            && root.contains("pub fn option_from_parts")
            && root.contains("pub fn dropdown_from_parts")
            && root.contains("pub fn dropdown_trigger_from_parts")
            && !root.contains("pub struct DropdownOptionParts<Message>")
            && !root.contains("fn dropdown_option_button")
            && !root.contains("fn dropdown_builder_accepts_toggle_and_options"),
        "dropdown builder root should own builder entry points while delegating public DTOs, menu overlay, and tests"
    );
    assert!(
        model.contains("pub struct DropdownOption<Message>")
            && model.contains("pub struct DropdownOptionParts<Message>")
            && model.contains("pub enum DropdownOptionSelection")
            && model.contains("pub struct DropdownParts<Message>")
            && model.contains("pub struct DropdownTriggerParts<Message>")
            && model.contains("pub fn from_parts(parts: DropdownOptionParts<Message>) -> Self"),
        "dropdown public DTOs should live in the focused model module"
    );
    assert!(
        menu.contains("pub fn dropdown_menu<")
            && menu.contains("pub fn dropdown_menu_overlay<")
            && menu.contains("fn dropdown_option_button"),
        "dropdown menu and overlay assembly should live in the focused menu module"
    );
    assert!(
        tests.contains("fn dropdown_height_tracks_expanded_options")
            && tests.contains("fn dropdown_builder_accepts_toggle_and_options")
            && tests.contains("fn dropdown_trigger_builds_external_overlay_toggle")
            && tests
                .contains("fn dropdown_option_compatibility_constructor_delegates_to_named_parts"),
        "dropdown builder and named-option behavior tests should live in dropdown/tests.rs"
    );
}

#[test]
fn application_interactive_row_builder_keeps_policy_and_underlay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/interactive_row/builder.rs"),
    )
    .expect("interactive-row builder root should be readable");
    let drag_drop = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/interactive_row/builder/drag_drop.rs"),
    )
    .expect("interactive-row drag/drop policy module should be readable");
    let messages = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/interactive_row/builder/messages.rs"),
    )
    .expect("interactive-row message mapping module should be readable");
    let underlay = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/interactive_row/builder/underlay.rs"),
    )
    .expect("interactive-row underlay module should be readable");
    let widget = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/interactive_row/builder/widget.rs"),
    )
    .expect("interactive-row widget lowering module should be readable");

    assert!(
        root.contains("#[path = \"builder/drag_drop.rs\"]")
            && root.contains("#[path = \"builder/messages.rs\"]")
            && root.contains("#[path = \"builder/underlay.rs\"]")
            && root.contains("#[path = \"builder/widget.rs\"]")
            && root.contains("pub struct InteractiveRowBuilder")
            && !root.contains("pub struct InteractiveRowUnderlayBuilder")
            && !root.contains("pub fn tracked_drag_source")
            && !root.contains("fn with_message_mapper"),
        "interactive-row builder root should own row configuration and delegate policy, routing, underlay, and widget lowering"
    );
    assert!(
        drag_drop.contains("pub fn tracked_drag_source")
            && drag_drop.contains("pub fn tracked_drop_target")
            && drag_drop.contains("pub fn tracked_drop_candidate")
            && messages.contains("fn with_message_mapper")
            && underlay.contains("pub struct InteractiveRowUnderlayBuilder")
            && underlay.contains("pub fn interactive_row_underlay")
            && widget.contains("pub fn widget(self) -> InteractiveRowWidget"),
        "interactive-row drag/drop policy, message mapping, underlay composition, and widget lowering should have focused owners"
    );
}

#[test]
fn application_tree_row_builder_keeps_defaults_state_and_composition_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/tree_row/mod.rs"))
            .expect("tree-row builder root should be readable");
    let builder = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/tree_row/builder.rs"),
    )
    .expect("tree-row builder module should be readable");
    let defaults = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/tree_row/defaults.rs"),
    )
    .expect("tree-row defaults module should be readable");
    let drag_drop = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/tree_row/drag_drop.rs"),
    )
    .expect("tree-row drag/drop module should be readable");
    let composition = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/tree_row/composition.rs"),
    )
    .expect("tree-row composition module should be readable");
    let hit_target = fs::read_to_string(
        manifest_dir.join("src/application/control_builders/tree_row/hit_target.rs"),
    )
    .expect("tree-row hit-target module should be readable");

    assert!(
        root.contains("mod builder;")
            && root.contains("mod composition;")
            && root.contains("mod defaults;")
            && root.contains("mod drag_drop;")
            && root.contains("mod hit_target;")
            && root.contains("pub use builder::{TreeRowBuilder, TreeRowMessageBuilder, tree_row};")
            && root.contains("pub use drag_drop::TreeRowDragDropState;")
            && !root.contains("pub struct TreeRowBuilder")
            && !root.contains("DEFAULT_TREE_ROW_HEIGHT"),
        "tree-row root should stay a small facade over builder state, defaults, drag/drop state, composition, and hit-target lowering"
    );
    assert!(
        builder.contains("pub struct TreeRowBuilder")
            && builder.contains("pub struct TreeRowMessageBuilder")
            && builder.contains("pub fn tree_row(")
            && !builder.contains("fn expander<")
            && !builder.contains("TreeRowHitTarget::new"),
        "tree-row builder module should own public state and fluent setters while delegating composition and hit-target lowering"
    );
    assert!(
        defaults.contains("DEFAULT_TREE_ROW_HEIGHT")
            && defaults.contains("fn default_palette")
            && defaults.contains("fn default_drop_target_outline")
            && drag_drop.contains("pub struct TreeRowDragDropState")
            && drag_drop.contains("pub const fn clears_hover_on_sync"),
        "tree-row defaults and host-owned drag/drop state should stay in focused modules"
    );
    assert!(
        composition.contains("fn expander<")
            && composition.contains("fn hit_target<")
            && composition.contains("TreeRowHitTarget::new")
            && hit_target.contains("pub(super) struct TreeRowHitTarget")
            && hit_target.contains("impl<Message> EmbeddedInteractiveRowWidget"),
        "tree-row composition should own row/expander assembly and delegate custom hit-target painting to hit_target.rs"
    );
}
