use super::*;

#[test]
fn performance_harness_is_registered_and_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let bench = fs::read_to_string(manifest_dir.join("benches/perf_harness.rs"))
        .expect("perf_harness bench should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");

    for required in [
        "[[bench]]",
        "name = \"perf_harness\"",
        "path = \"benches/perf_harness.rs\"",
        "harness = false",
    ] {
        assert!(
            manifest.contains(required),
            "Cargo.toml should register perf harness with `{required}`"
        );
    }
    let perf_scenarios = [
        "layout_deep_nesting",
        "layout_wrap_1k",
        "layout_virtualized_10k",
        "layout_virtualized_fixed_10k",
        "layout_virtualized_fixed_scroll_10k",
        "layout_mark_dirty_subtree_10k",
        "app_virtual_list_projection_10k",
        "app_virtual_list_projection_generated_child_ids_10k",
        "app_virtual_selectable_list_projection_10k",
        "app_virtual_list_window_projection_10k",
        "runtime_surface_large_tree",
        "runtime_text_paint_plan_1k",
        "runtime_horizontal_scroll_paint_1k",
        "runtime_virtualized_list_wheel_10k",
        "runtime_virtualized_list_hover_10k",
        "runtime_virtualized_list_stable_hover_10k",
        "runtime_virtualized_list_hover_paint_10k",
        "runtime_virtualized_nested_scroll_hover_10k",
        "runtime_refresh_large_tree",
        "runtime_resize_large_tree",
        "runtime_command_flattening_512",
        "runtime_command_drain_1k",
        "gpu_signal_summary",
        "gpu_surface_projection",
    ];
    for scenario in perf_scenarios {
        assert!(
            bench.contains(scenario),
            "perf_harness should include `{scenario}`"
        );
        assert!(
            docs.contains(scenario),
            "docs/API.md should document perf scenario `{scenario}`"
        );
    }
    assert!(
        bench.contains("radiant_perf scenario="),
        "perf_harness should print parseable metric lines"
    );
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_docs.contains("cargo bench --bench perf_harness")
            && normalized_docs
                .contains("does not enforce machine-dependent pass/fail timing thresholds"),
        "docs/API.md should describe how to run and interpret the perf harness"
    );
}

#[test]
fn clippy_quality_gate_is_documented_without_blanket_complexity_allow() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");

    assert!(
        docs.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "API docs should document the all-target Clippy quality gate"
    );
    assert!(
        !lib.contains("clippy::type_complexity"),
        "Radiant should not hide callback-shape drift behind a crate-level type_complexity allow"
    );
}

#[test]
fn pointer_move_repaint_contract_is_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let contract = fs::read_to_string(manifest_dir.join("src/widgets/contract.rs"))
        .expect("Radiant widget contract should be readable");

    for required in [
        "Widget::accepts_pointer_move()",
        "request repaint even when `handle_input` returns `None`",
        "without emitting host messages",
    ] {
        assert!(
            docs.contains(required),
            "API docs should explain the pointer-move repaint contract with `{required}`"
        );
    }
    for required in [
        "snapped timeline cursor",
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
fn runtime_diagnostics_use_tracing_outside_explicit_profile_artifacts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let diagnostic_sources = [
        "src/application/runtime/threading.rs",
        "src/application/runtime/timer.rs",
        "src/application/runtime/subscription.rs",
        "src/gui_runtime/native_vello/text_renderer.rs",
    ];

    for source_path in diagnostic_sources {
        let source = fs::read_to_string(manifest_dir.join(source_path))
            .unwrap_or_else(|err| panic!("{source_path} should be readable: {err}"));
        assert!(
            !source.contains("eprintln!"),
            "{source_path} should route ordinary runtime diagnostics through tracing"
        );
    }

    let startup_profile =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/startup/logging.rs"))
            .expect("native startup profile logging should be readable");
    assert!(
        startup_profile.contains("RADIANT_NATIVE_STARTUP_PROFILE")
            && startup_profile.contains("eprintln!"),
        "explicit startup profile artifacts may keep their opt-in stderr output"
    );
}

#[test]
fn api_docs_describe_the_structural_boundary_strategy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "# Radiant Core API",
        "Dependency Boundary",
        "host -> Radiant, never Radiant -> host",
        "Boundary tests prove that dependency direction, public exports, examples, and",
        "they intentionally avoid enforcing product",
        "Radiant now exposes only generic GUI and native runtime APIs",
        "Radiant exposes one public API with progressive control",
        "Application builders and explicit runtime objects are part of the same API surface",
        "same model with more explicit control",
        "Radiant's application API is designed to be easy to read without hiding the runtime model",
        "View, Element, And Widget",
        "VirtualListWindow",
        "virtual_list_view_start_after_scroll_delta",
        "SignalChromeState",
        "SignalToolState",
        "SignalRasterPreview",
        "TimelineViewport",
        "TimelineTransportState",
        "TimelineEditPreview",
        "TimelineFeedbackEvents",
        "TimelinePresentationState",
        "TimelineSurfaceState",
        "TimelineMotionState",
        "UiSurface",
        "SurfaceNode",
        "WidgetId",
        "Command<Message>",
        "Soft-Deprecated First-Use Boilerplate",
        "not a Rust `#[deprecated]` attribute on the explicit control objects",
        "RuntimeRunReport<Artifacts>",
        "RuntimeBridge",
        "ThemeTokens",
        "SurfacePaintPlan",
        "InvalidationMask",
        "RetainedSegmentMask",
        "VisualSnapshot",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should document `{required}`"
        );
    }
}

#[test]
fn api_docs_soft_deprecate_only_first_use_runtime_boilerplate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");

    for first_use_boilerplate in [
        "constructing `NativeRunOptions` directly for a hello-world app",
        "hand-writing a closure bridge before the app has meaningful state",
        "wrapping one label in `Arc<UiSurface<_>>`",
        "manually composing `SurfaceNode`, `SurfaceChild`, explicit numeric IDs, and",
    ] {
        assert!(
            docs.contains(first_use_boilerplate),
            "docs/API.md should soft-deprecate `{first_use_boilerplate}` for first-use application code"
        );
    }

    for explicit_control in [
        "The `radiant::runtime` module",
        "`RuntimeBridge`",
        "`UiSurface`",
        "`SurfaceNode`",
        "`SurfaceChild`",
        "`NativeRunOptions`",
        "`WidgetSizing`",
        "remain supported and non-deprecated for hosts",
    ] {
        assert!(
            docs.contains(explicit_control),
            "docs/API.md should preserve explicit-control API guidance for `{explicit_control}`"
        );
    }
    assert!(
        !runtime.contains("#[deprecated"),
        "radiant::runtime should remain supported, not a blanket-deprecated module"
    );
}
