use super::*;

#[test]
fn pointer_move_repaint_contract_is_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let contract = fs::read_to_string(manifest_dir.join("src/widgets/contract/widget.rs"))
        .expect("Radiant widget trait contract should be readable");

    for required in [
        "Widget::accepts_pointer_move()",
        "Widget::prefers_pointer_move_paint_only()",
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
            "Widget contract should explain local pointer-move repaint behavior with `{required}`"
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
        "GpuSurfaceContent::SignalSummaryBands",
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
        "The public contract is `key` plus `revision` plus validated `GpuSurfaceContent`",
        "bump the revision only when the retained GPU payload changes",
        "keep transient cursor or drag previews in overlays or paint-only repaint paths",
        "one Radiant widget model instead of creating separate Vello and WGPU application models",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe the GPU-surface boundary contract with `{required}`"
        );
    }
}
