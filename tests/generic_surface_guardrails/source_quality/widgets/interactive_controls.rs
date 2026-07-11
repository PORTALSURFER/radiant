use super::*;

#[test]
fn badge_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge.rs"))
        .expect("badge primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/builders.rs"))
            .expect("badge primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/tests.rs"))
        .expect("badge primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct BadgeWidget")
            && root.contains("impl Widget for BadgeWidget")
            && root.contains("#[path = \"badge/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn badge_releases_inside_bounds_emit_activation"),
        "badge primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn badge(")
            && builders.contains("pub fn badge_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "badge runtime builder helpers should live in badge/builders.rs"
    );
    assert!(
        tests.contains("fn badge_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_badge_enter_emits_activation"),
        "badge behavior tests should live in badge/tests.rs"
    );
}

#[test]
fn button_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button.rs"))
        .expect("button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/builders.rs"))
            .expect("button primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/tests.rs"))
        .expect("button primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ButtonWidget")
            && root.contains("impl Widget for ButtonWidget")
            && root.contains("mod tests;")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn button_releases_inside_bounds_emit_activation"),
        "button primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn button(")
            && builders.contains("pub fn button_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "button runtime builder helpers should live in button/builders.rs"
    );
    assert!(
        tests.contains("fn button_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_button_space_emits_activation"),
        "button behavior tests should stay in button/tests.rs"
    );
}

#[test]
fn icon_button_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button.rs"))
        .expect("icon-button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button/builders.rs"))
            .expect("icon-button primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct IconButtonWidget")
            && root.contains("impl Widget for IconButtonWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "icon-button primitive root should own widget behavior and delegate runtime mappers"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn icon_button("),
        "icon-button runtime mapper helper should live in icon_button/builders.rs"
    );
}

#[test]
fn interactive_row_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row.rs"))
        .expect("interactive-row primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/builders.rs"))
            .expect("interactive-row primitive builders should be readable");
    let input =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/input.rs"))
            .expect("interactive-row input module should be readable");
    let actions = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/actions/mod.rs"),
    )
    .expect("interactive-row actions model should be readable");
    let action_activation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/actions/activation.rs"),
    )
    .expect("interactive-row activation actions should be readable");
    let action_secondary = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/actions/secondary.rs"),
    )
    .expect("interactive-row secondary actions should be readable");
    let action_drag_drop = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/actions/drag_drop.rs"),
    )
    .expect("interactive-row drag/drop actions should be readable");
    let action_routing = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/actions/routing.rs"),
    )
    .expect("interactive-row action routing should be readable");
    let paint =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/paint.rs"))
            .expect("interactive-row paint module should be readable");
    let embedded =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/embedded.rs"))
            .expect("interactive-row embedded module should be readable");
    let widget_impl = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/interactive_row/widget_impl.rs"),
    )
    .expect("interactive-row widget impl module should be readable");

    assert!(
        root.contains("mod actions;")
            && root.contains("mod builders;")
            && root.contains("mod embedded;")
            && root.contains("mod input;")
            && root.contains("mod paint;")
            && root.contains("mod widget_impl;")
            && root.contains("pub struct InteractiveRowWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "interactive-row primitive root should own row state/configuration while delegating routing, paint, embedded wrappers, widget impl, and input routing"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn interactive_row("),
        "interactive-row runtime mapper helper should live in interactive_row/builders.rs"
    );
    assert!(
        input.contains("use super::InteractiveRowWidget;")
            && input.contains("gui::types::{Point, Rect}")
            && input.contains("InteractiveRowMessage")
            && input.contains("WidgetInput")
            && input.contains("DragHandleMessage")
            && input.contains("PointerButton")
            && input.contains("WidgetKey")
            && !input.contains("use super::*;")
            && input.contains("pub fn handle_input("),
        "interactive-row input routing should name row, geometry, input, key, pointer, drag, and output-message dependencies explicitly"
    );
    assert!(
        actions.contains("pub struct InteractiveRowActions")
            && actions.contains("mod activation;")
            && actions.contains("mod secondary;")
            && actions.contains("mod drag_drop;")
            && actions.contains("mod routing;")
            && action_activation.contains("pub fn activate_with_modifiers")
            && action_secondary.contains("pub fn primary_secondary_key")
            && action_drag_drop.contains("pub fn tracked_drop_candidate_key")
            && action_routing.contains("pub fn route(&self, message: InteractiveRowMessage)")
            && !actions.contains("InteractiveRowWidget"),
        "interactive-row action routing should live outside the primitive model and not depend on widget state"
    );
    assert!(
        paint.contains("pub fn dense_visual_state(")
            && paint.contains("pub fn push_dense_labeled_chrome(")
            && paint.contains("DenseRowPalette")
            && paint.contains("PaintPrimitive"),
        "interactive-row dense-row paint projection should live in the paint module"
    );
    assert!(
        embedded.contains("pub trait EmbeddedInteractiveRowWidget")
            && embedded.contains("impl<T> Widget for T")
            && embedded.contains("fn append_interactive_row_paint("),
        "interactive-row embedded custom-widget delegation should live in the embedded module"
    );
    assert!(
        widget_impl.contains("impl Widget for InteractiveRowWidget")
            && widget_impl.contains("fn accepts_pointer_move(&self)")
            && widget_impl.contains("push_control_chrome"),
        "interactive-row widget trait implementation should live in widget_impl.rs"
    );
}

#[test]
fn list_item_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item.rs"))
        .expect("list item primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item/builders.rs"))
            .expect("list item primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ListItemWidget")
            && root.contains("impl Widget for ListItemWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn list_item(")
            && builders.contains("pub fn list_item_action(")
            && builders.contains("pub fn list_item_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item runtime builder helpers should live in list_item/builders.rs"
    );
}

#[test]
fn selectable_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable.rs"))
        .expect("selectable primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable/builders.rs"))
            .expect("selectable primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SelectableWidget")
            && root.contains("impl Widget for SelectableWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn selectable(")
            && builders.contains("pub fn selectable_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable runtime builder helpers should live in selectable/builders.rs"
    );
}
