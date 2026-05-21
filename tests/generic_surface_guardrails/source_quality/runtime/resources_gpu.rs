use super::*;

#[test]
fn resource_completions_use_named_parts_for_request_results() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/resource/load.rs"))
        .expect("resource load module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let prelude = public_prelude_source(&manifest_dir);
    let update_context_tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");

    assert!(
        source.contains("pub struct ResourceCompletionParts")
            && source.contains("pub fn from_parts(parts: ResourceCompletionParts<T>) -> Self")
            && source.contains("Self::from_parts(ResourceCompletionParts { request, load })"),
        "resource completions should expose named parts and keep the compatibility constructor"
    );
    assert!(
        source.contains("ResourceCompletion::from_parts(ResourceCompletionParts {")
            && update_context_tasks.contains(
                "ResourceCompletion::from_parts(ResourceCompletionParts { request, load })"
            ),
        "resource completion mapping and spawn helpers should use the named-parts construction path"
    );
    assert!(
        resource.contains("ResourceCompletionParts")
            && runtime.contains("ResourceCompletionParts")
            && prelude.contains("ResourceCompletionParts"),
        "resource completion parts should remain publicly exported through runtime and prelude"
    );
}

#[test]
fn resource_slots_keep_load_state_and_lifecycle_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let slot = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot.rs"))
        .expect("resource slot module should be readable");
    let state = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot/state.rs"))
        .expect("resource slot state module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        slot.contains("mod state;")
            && slot.contains("pub use state::ResourceLoadState;")
            && slot.contains("pub struct ResourceSlot<T>")
            && slot.contains("pub fn begin_load")
            && slot.contains("pub fn apply_for"),
        "resource slot lifecycle should stay in slot.rs while delegating load-state model"
    );
    assert!(
        !slot.contains("pub enum ResourceLoadState")
            && state.contains("pub enum ResourceLoadState")
            && state.contains("Idle")
            && state.contains("Loading")
            && state.contains("Ready")
            && state.contains("Failed"),
        "resource load-state enum should live in runtime/resource/slot/state.rs"
    );
    assert!(
        resource.contains("pub use slot::{ResourceLoadState, ResourceSlot};")
            && runtime.contains("ResourceLoadState")
            && runtime.contains("ResourceSlot")
            && prelude.contains("ResourceLoadState")
            && prelude.contains("ResourceSlot"),
        "resource slot and load-state types should remain exported through runtime and prelude"
    );
}

#[test]
fn gpu_surface_widget_uses_named_parts_for_retained_resource_identity() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/widgets/primitives/gpu_surface.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");
    let application_builder =
        fs::read_to_string(manifest_dir.join("src/application/builders/leaf.rs"))
            .expect("application leaf builders should be readable");

    assert!(
        source.contains("pub struct GpuSurfaceParts")
            && source.contains("pub fn from_parts(parts: GpuSurfaceParts) -> Self"),
        "retained GPU surfaces should expose named parts for resource identity, revision, and content"
    );
    assert!(
        source.contains("Self::from_parts(GpuSurfaceParts {")
            && widgets.contains("GpuSurfaceParts")
            && application_builder.contains("pub fn gpu_surface_from_parts"),
        "GPU surface compatibility constructors, public exports, and application builders should keep the named-parts path available"
    );
}

#[test]
fn gpu_surface_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface.rs"))
        .expect("gpu-surface primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface/builders.rs"))
            .expect("gpu-surface primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct GpuSurfaceWidget")
            && root.contains("impl Widget for GpuSurfaceWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "gpu-surface primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn gpu_surface("),
        "gpu-surface runtime builder helper should live in gpu_surface/builders.rs"
    );
}

#[test]
fn gpu_surface_content_models_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let content = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content.rs"))
        .expect("GPU surface content module should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/model.rs"))
        .expect("GPU surface content model module should be readable");
    let validation =
        fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/validation.rs"))
            .expect("GPU surface content validation module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/tests.rs"))
        .expect("GPU surface content test root should be readable");
    let atlas_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/tests/atlas.rs"))
            .expect("GPU surface atlas tests should be readable");
    let signal_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/gpu_surface/content/tests/signal_shape.rs"),
    )
    .expect("GPU surface signal shape tests should be readable");
    let validation_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/gpu_surface/content/tests/validation.rs"),
    )
    .expect("GPU surface content validation tests should be readable");
    let gpu_surface = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface.rs"))
        .expect("GPU surface runtime facade should be readable");

    assert!(
        content.contains("mod model;")
            && content.contains("GpuShaderSurfaceDescriptor")
            && content.contains("GpuShaderSurfaceDescriptorParts")
            && content.contains("GpuSignalGainPreview")
            && content.contains("GpuSignalRenderShape")
            && content.contains("pub enum GpuSurfaceContent")
            && !content.contains("pub struct GpuSignalGainPreview")
            && !content.contains("pub struct GpuSignalRenderShape"),
        "GPU surface content root should expose the retained content enum while re-exporting focused signal content models"
    );
    assert!(
        model.contains("pub struct GpuShaderSurfaceDescriptor")
            && model.contains("pub struct GpuShaderSurfaceDescriptorParts")
            && model.contains("pub shader_key: String")
            && model.contains("pub wgsl_source: Option<Arc<str>>")
            && model.contains("pub fragment_entry_point: Option<String>")
            && model.contains("pub fn wgsl_source")
            && model.contains("pub fn fragment_entry_point")
            && model.contains("pub struct GpuSignalGainPreview")
            && model.contains("pub fade_in_length: f32")
            && model.contains("pub struct GpuSignalRenderShape")
            && model.contains("pub sample_count: usize"),
        "GPU shader descriptor, signal gain-preview, and render-shape DTOs should live in content/model.rs"
    );
    assert!(
        validation.contains("validate_signal_gain_preview")
            && validation.contains("validate_shader_descriptor")
            && validation.contains("validate_signal_render_shape"),
        "GPU surface content validation should stay in the validation module"
    );
    assert!(
        tests.contains("mod atlas;")
            && tests.contains("mod signal_shape;")
            && tests.contains("mod validation;")
            && !tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && !tests.contains("fn signal_render_shape_rejects_invalid_payload_contracts"),
        "GPU surface content test root should index focused content behavior groups instead of owning all cases"
    );
    assert!(
        atlas_tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && signal_tests.contains("fn signal_render_shape_uses_effective_available_frame_count")
            && validation_tests
                .contains("fn gpu_surface_content_validation_rejects_non_finite_gain_preview"),
        "GPU surface content behavior tests should stay grouped by atlas, signal shape, and validation concerns"
    );
    assert!(
        gpu_surface.contains("GpuShaderSurfaceDescriptor")
            && gpu_surface.contains("GpuShaderSurfaceDescriptorParts")
            && gpu_surface.contains("GpuSignalGainPreview")
            && gpu_surface.contains("GpuSignalRenderShape")
            && gpu_surface.contains("GpuSurfaceContent")
            && gpu_surface.contains("GpuSurfaceContentError"),
        "GPU surface content models and diagnostics should remain available through the runtime facade"
    );
}
