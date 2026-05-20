use super::*;

#[test]
fn canvas_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas.rs"))
        .expect("canvas primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas/builders.rs"))
            .expect("canvas primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CanvasWidget")
            && root.contains("pub struct RetainedSurfaceDescriptor")
            && root.contains("impl Widget for CanvasWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas primitive root should own widget behavior and retained-surface contract while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn canvas(")
            && builders.contains("pub fn canvas_mapped(")
            && builders.contains("pub fn retained_canvas_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas runtime builder helpers should live in canvas/builders.rs"
    );
}

#[test]
fn card_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card.rs"))
        .expect("card primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card/builders.rs"))
        .expect("card primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CardWidget")
            && root.contains("impl Widget for CardWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "card primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn card("),
        "card runtime builder helper should live in card/builders.rs"
    );
}

#[test]
fn image_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/image.rs"))
        .expect("image primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/image/builders.rs"))
            .expect("image primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ImageWidget")
            && root.contains("impl Widget for ImageWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "image primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn image("),
        "image runtime builder helper should live in image/builders.rs"
    );
}

#[test]
fn text_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text.rs"))
        .expect("text primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text/builders.rs"))
        .expect("text primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextWidget")
            && root.contains("impl Widget for TextWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "text primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text("),
        "text runtime builder helper should live in text/builders.rs"
    );
}
