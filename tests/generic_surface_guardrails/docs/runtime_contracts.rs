use super::*;

#[test]
fn pointer_move_repaint_contract_is_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let contract = fs::read_to_string(manifest_dir.join("src/widgets/contract/pointer_motion.rs"))
        .expect("Radiant pointer-motion capability contract should be readable");

    for required in [
        "WidgetPointerMotion::accepts_pointer_move()",
        "WidgetPointerMotion::prefers_pointer_move_paint_only()",
        "WidgetPointerMotion::pointer_capture_policy()",
        "WidgetPointerMotion::pointer_move_overlay_is_valid()",
        "WidgetHitTest::hit_test(...)",
        "Widget::append_runtime_overlay_paint(...)",
        "WidgetCommon::with_pointer_focus()",
        "WidgetCommon::with_keyboard_focus()",
        "request repaint even when `handle_input` returns `None`",
        "cached scene on stable pointer motion",
        "without emitting host messages",
    ] {
        assert!(
            docs.contains(required),
            "API docs should explain the pointer-move repaint contract with `{required}`"
        );
    }
    for required in [
        "snapped timeline cursor",
        "append_runtime_overlay_paint",
        "rebuilding the base scene",
        "request repaint even when `handle_input` returns `None`",
        "emit host messages",
    ] {
        assert!(
            contract.contains(required),
            "WidgetPointerMotion contract should explain local pointer-move repaint behavior with `{required}`"
        );
    }
}

#[test]
fn ui_first_runtime_threading_contract_is_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let command = fs::read_to_string(manifest_dir.join("src/runtime/command.rs"))
        .expect("runtime command module should be readable");
    let threading = fs::read_to_string(manifest_dir.join("src/application/runtime/threading.rs"))
        .expect("application threading module should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "## UI-First Runtime Threading",
        "native UI/event/render owner as the priority path",
        "runtime-managed business threads",
        "bounded business worker lane",
        "default architecture is UI-first and non-blocking",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should document UI-first runtime threading with `{required}`"
        );
    }
    assert!(
        command.contains("UI reducers should stay short and non-blocking"),
        "Command docs should tell reducers to avoid blocking the UI path"
    );
    assert!(
        threading.contains("spawn_business_thread") && threading.contains("radiant-business"),
        "application runtime should expose explicit business-thread spawning internally"
    );
}

#[test]
fn numeric_adoption_sequence_and_shipped_surfaces_are_current() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = fs::read_to_string(manifest_dir.join("docs/DESIGN_DIRECTION.md"))
        .expect("Design Direction docs should be readable");
    let api =
        fs::read_to_string(manifest_dir.join("docs/API.md")).expect("API docs should be readable");
    let normalized_design = design.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_api = api.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "The shared edit-event foundation, Slider, Knob, PanelResizeState, and the public generic `NumericInput` control are shipped API.",
        "remaining continuous controls and separately unshipped native/product boundaries are follow-up work.",
    ] {
        assert!(
            normalized_design.contains(required),
            "Design Direction docs should record the completed numeric adoption sequence with `{required}`"
        );
    }

    for required in [
        "The shared edit-event adopters currently shipped are `Slider`, `Knob`, `PanelResizeState`, and the public generic `NumericInput`",
        "`radiant::application::{numeric_input, NumericInputBuilder}` exports",
        "`NumericInputEditBatch<T>` is the shipped bounded incremental carrier.",
        "Complete-mode PointerScrub and NumericInput wheel consumption are separate shipped consumers",
        "NumericInput IME/composition plus the widget-local accessibility policy and generic runtime accessibility dispatch are now shipped consumers",
    ] {
        assert!(
            normalized_api.contains(required),
            "API docs should record shipped NumericInput surfaces with `{required}`"
        );
    }

    for forbidden in [
        "generic numeric control is the next target adopter",
        "move the migration past Knob to the next shared-edit consumer",
        "`PanelResizeState` is the next shipped shared-edit consumer",
        "`PanelResizeState` is now the next shipped shared-edit consumer",
        "Slider/Knob adoption, native unit/phase adapters",
    ] {
        assert!(
            !normalized_design.contains(forbidden) && !normalized_api.contains(forbidden),
            "numeric adoption docs must not restore stale sequence claim `{forbidden}`"
        );
    }
}

#[test]
fn split_separator_native_publication_contract_is_current() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative_path in [
        "docs/DESIGN_DIRECTION.md",
        "docs/TARGET.md",
        "docs/API.md",
        "docs/ARCHITECTURE.md",
    ] {
        let source = fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"));
        let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized.contains("AXSplitter"),
            "{relative_path} should record shipped passive AXSplitter publication"
        );
        assert!(
            !normalized.contains("native-omitted")
                && !normalized.contains("omits native publication"),
            "{relative_path} must not retain the stale native-omitted separator contract"
        );
    }
}

#[test]
fn api_docs_describe_paint_only_overlay_composition_cache() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "paint-only presentation work",
        "caches the composed Vello scene plus retained GPU surfaces as a base frame",
        "composed-base refresh or cache hits for transient overlays",
        "transient-overlay paint callbacks",
        "transient-overlay primitive counts",
        "without refreshing the declarative surface, rebuilding the cached Vello scene, or recompositing",
        "`waveform_view` uses a generated synthetic signal",
        "RenderCanvasContent::SignalSummaryBands",
        "playback playhead",
        "instead of queueing app frame messages",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should document the paint-only overlay composition cache with `{required}`"
        );
    }
}

#[test]
fn api_docs_describe_scene_presentation_compatibility_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let lifecycle =
        fs::read_to_string(manifest_dir.join("src/application/launch/stateful/lifecycle.rs"))
            .expect("stateful lifecycle source should be readable");
    let overlays =
        fs::read_to_string(manifest_dir.join("src/application/launch/stateful/overlays.rs"))
            .expect("stateful overlay source should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "Compatibility policy: root-scoped app presentation should use `Scene::frame_clock(...)` and `Scene::overlay(...)`.",
        "App-builder `.presentation(...)` is the compatibility path",
        "remain public, supported, lower-level lifecycle APIs",
        "They are not deprecated in this phase",
        "new root-scoped application presentation should prefer the `Scene` descriptors",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe the scene presentation compatibility policy with `{required}`"
        );
    }

    for required in [
        "Advanced lifecycle hook for animation-driven native frames.",
        "Prefer [`crate::application::Scene::frame_clock`]",
        "Advanced lifecycle hook for messages emitted on active animation frames.",
    ] {
        assert!(
            lifecycle.contains(required),
            "launch lifecycle rustdoc should mark low-level animation hooks as advanced with `{required}`"
        );
    }

    for required in [
        "Advanced lifecycle hook for a lightweight frame-time overlay painter.",
        "Prefer [`crate::application::Scene::overlay`]",
        "Advanced lifecycle hook for transient-overlay timed frames.",
        "Advanced lifecycle hook for a transient overlay and its paint-only activity.",
        "Advanced lifecycle hook for a transient overlay with capped paint-only cadence.",
    ] {
        assert!(
            overlays.contains(required),
            "launch overlay rustdoc should mark low-level overlay hooks as advanced with `{required}`"
        );
    }
}

#[test]
fn api_docs_describe_lossless_view_projection_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "`IntoView::into_projection()` is the lossless stateful-application boundary.",
        "Custom wrappers must delegate this required method",
        "Bare `SurfaceNode` and `UiSurface` values do not implement `IntoView`",
        "metadata rejection explicit with `ViewProjection::from_surface(...)`",
        "intentionally strips application-only Scene lifecycle bindings",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe the lossless view projection contract with `{required}`"
        );
    }
}

#[test]
fn api_docs_describe_declarative_lifecycle_identity_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "The declarative lifecycle contract is snapshot based, not object-instance based.",
        "Application builders may create a fresh `View<Message>` or `UiSurface<Message>` on every refresh",
        "continuity comes from stable widget identity, host-owned state, retained resource identity, and runtime caches",
        "Use `.key(...)`, explicit widget IDs, or resource IDs for dynamic rows",
        "Generated IDs are suitable for static local structure",
        "dynamic collections should not depend on positional identity",
        "Reducers own all durable application state.",
        "runtime-local state is limited to GUI concerns such as focus, hover, pointer capture, scroll offsets, layout caches, repaint flags, and retained surface caches",
        "A reducer that changes durable state should request a normal surface repaint",
        "Use paint-only repaint scopes only for overlay motion",
        "without hiding a real state change",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe the declarative lifecycle identity contract with `{required}`"
        );
    }
}

#[test]
fn api_docs_describe_gpu_surface_boundary_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "Use retained GPU surfaces for dense visuals where the payload is naturally texture, signal, or shader data",
        "waveform bodies, meters, scopes, large preview atlases",
        "Keep normal panels, controls, labels, selection chrome, and editor overlays in standard Radiant widgets",
        "unless they need custom GPU resources",
        "The public contract is `key` plus `revision` plus validated `RenderCanvasContent`",
        "bump the revision only when the retained GPU payload changes",
        "keep transient cursor or drag previews in overlays or paint-only repaint paths",
        "one Radiant widget model instead of creating separate Vello and WGPU application models",
        "`RenderCanvasContent::CustomShader` for advanced surfaces",
        "explicit vertex and fragment entry-point names",
        "`fragment_entry_point(...)` names the color-producing fragment stage",
        "validation requires a fragment entry point",
        "execute WGSL-backed descriptors that use Radiant's built-in surface uniform ABI",
        "optional app uniform payload bytes",
        "optional read-only storage payload bytes",
        "optional volatile presentation-uniform bytes",
        "`@group(0) @binding(3)`",
        "`storage_identity`",
        "`storage_revision`",
        "`presentation_uniform_revision`",
        "`presentation_revision`",
        "non-empty descriptor or update payload must have a byte length divisible by four",
        "`GpuShaderPresentationUniformUpdate::try_new` reports an alignment error",
        "`RenderCanvasContent::validate()` reports a typed descriptor validation error",
        "`UiUpdateContext::update_gpu_shader_presentation_uniform`",
        "`Command::update_gpu_shader_presentation_uniform`",
        "paint-only updates",
        "do not enter application messages or force projection",
        "bounded and latest-only",
        "stale-generation updates are ignored unless their storage fence matches",
        "custom shader pipeline rebuilds",
        "`NativeGpuSurfaceDiagnostics::custom_shader`",
        "`surfaces_rendered`",
        "`pipeline_rebuilds`",
        "`binding_rebuilds`",
        "`binding_cache_hits`",
        "`static_writes`",
        "`static_write_bytes`",
        "`presentation_writes`",
        "`presentation_write_bytes`",
        "`custom_shader.failures.surfaces_failed`",
        "`custom_shader.failures.shader_module_failures`",
        "`custom_shader.failures.pipeline_failures`",
        "`custom_shader.failures.binding_failures`",
        "the native renderer also logs the backend validation error through tracing",
        "Descriptors that do not provide source or stage entry points report skipped surfaces",
        "`custom_shader.unsupported.surfaces`",
        "`custom_shader.unsupported.vertices`",
        "`custom_shader.unsupported.source_bytes`",
        "`custom_shader.unsupported.uniform_bytes`",
        "`custom_shader.unsupported.storage_bytes`",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe the GPU-surface boundary contract with `{required}`"
        );
    }
}
