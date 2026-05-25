use super::*;

#[test]
fn runtime_surface_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/surface/node.rs"))
        .expect("runtime surface node module should be readable");
    let widget = fs::read_to_string(manifest_dir.join("src/runtime/surface/widget.rs"))
        .expect("runtime surface widget module should be readable");
    let widget_mapper =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/widget/mapper.rs"))
            .expect("runtime surface widget mapper module should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/runtime/surface/builders.rs"))
        .expect("runtime surface builders should be readable");
    let container_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/container.rs"))
            .expect("runtime surface container builders should be readable");
    let leaf_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/leaf.rs"))
            .expect("runtime surface leaf builders should be readable");
    let overlay_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/overlay.rs"))
            .expect("runtime surface overlay builders should be readable");
    let surface = fs::read_to_string(manifest_dir.join("src/runtime/surface.rs"))
        .expect("runtime surface module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SurfaceChildParts<Message>",
            "pub fn from_parts(parts: SurfaceChildParts<Message>) -> Self",
            "Self::from_parts(SurfaceChildParts {",
        ),
        (
            "pub struct SurfaceContainerParts<Message>",
            "pub fn from_parts(parts: SurfaceContainerParts<Message>) -> Self",
            "Self::from_parts(SurfaceContainerParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "runtime surface nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        builders.contains("mod container;")
            && builders.contains("mod leaf;")
            && builders.contains("mod overlay;")
            && !builders
                .contains("pub fn container_from_parts(parts: SurfaceContainerParts<Message>)")
            && !builders.contains("pub fn widget(")
            && !builders.contains("pub fn overlay_panel(")
            && container_builders.contains(
                "pub fn container_from_parts(parts: SurfaceContainerParts<Message>) -> Self"
            )
            && container_builders.contains("pub fn virtual_scroll_area(")
            && container_builders.contains("fn scroll_area_with_virtualization(")
            && leaf_builders.contains("pub fn widget(")
            && leaf_builders.contains("pub fn custom_widget_box(")
            && leaf_builders.contains("pub fn static_widget(")
            && overlay_builders.contains("pub fn overlay_panel(")
            && overlay_builders.contains("pub fn overlay_marker(")
            && widget.contains("mod mapper;")
            && widget.contains("pub use mapper::{MessageMapper, WidgetMessageMapper};")
            && widget_mapper.contains("pub struct WidgetMessageMapper<Message>")
            && widget_mapper.contains("pub type MessageMapper<Input, Message>")
            && surface.contains("SurfaceChildParts")
            && surface.contains("SurfaceContainerParts")
            && runtime.contains("SurfaceChildParts")
            && runtime.contains("SurfaceContainerParts"),
        "runtime surface builders should stay focused while named parts remain publicly available"
    );
}

#[test]
fn runtime_surface_focus_order_keeps_collection_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let focus = fs::read_to_string(manifest_dir.join("src/runtime/surface/focus.rs"))
        .expect("runtime surface focus order helper should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/surface/focus/tests.rs"))
        .expect("runtime surface focus order tests should be readable");

    assert!(
        focus.contains("pub fn keyboard_focus_order_into")
            && focus.contains("pub fn keyboard_focus_order(&self)")
            && focus.contains("use super::{SurfaceNode, UiSurface, WidgetId};")
            && !focus.starts_with("use super::*;")
            && focus.contains("fn append_keyboard_focus_order")
            && focus.contains("#[path = \"focus/tests.rs\"]")
            && !focus.contains("fn keyboard_focus_order_collects_only_keyboard_focusable_widgets"),
        "runtime surface focus ordering should name surface and widget identity dependencies while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn keyboard_focus_order_collects_only_keyboard_focusable_widgets")
            && tests.contains("fn keyboard_focus_order_into_reuses_existing_storage"),
        "runtime surface focus behavior coverage should live in surface/focus/tests.rs"
    );
    assert!(
        tests.contains("runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper}")
            && !tests.starts_with("use super::*;"),
        "runtime surface focus tests should name their surface construction dependencies instead of inheriting focus module imports"
    );
}
