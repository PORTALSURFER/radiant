use super::{normalized, read_project_file};

#[test]
fn api_docs_describe_native_gpu_timing_status() {
    let docs = read_project_file("docs/API.md");
    let runtime_diagnostics = read_project_file("src/runtime/diagnostics/timing.rs");
    let native_diagnostics =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs");
    let render_profile =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/render_profile.rs");

    let normalized_docs = normalized(&docs);
    assert!(
        normalized_docs.contains("`NativeFrameTimingDiagnostics::gpu_timing_status`")
            && normalized_docs.contains("`NativeGpuTimingStatus::CpuEnvelopeOnly`")
            && normalized_docs.contains("CPU-side encode/submit/present envelopes")
            && normalized_docs.contains("not backend GPU timestamp query durations")
            && normalized_docs.contains("`NativeFrameTimingDiagnostics::cpu_envelope_total()`")
            && normalized_docs.contains("`frame_cpu_envelope_total_us`"),
        "API docs should distinguish CPU timing envelopes from backend GPU timestamp timing"
    );
    assert!(
        normalized_docs.contains("`NativeFrameDiagnostics::frame_sequence`")
            && normalized_docs.contains("scoped to one native window")
            && normalized_docs.contains("successful presentation")
            && normalized_docs.contains("counter is exhausted")
            && normalized_docs.contains("never wraps or reuses"),
        "API docs should define the native frame sequence scope, assignment point, and exhaustion behavior"
    );
    assert!(
        normalized_docs.contains("`NativeFrameDiagnostics::window_identity`")
            && normalized_docs.contains("`(window_identity, frame_sequence)`")
            && normalized_docs.contains("native runtime run")
            && normalized_docs.contains("hide/show")
            && normalized_docs.contains("cache-on-close")
            && normalized_docs.contains("public logical `WindowKey`")
            && normalized_docs.contains("auxiliary projection key"),
        "API docs should define native-window identity lifetime, correlation, exhaustion, and key separation"
    );
    assert!(
        runtime_diagnostics.contains("pub enum NativeGpuTimingStatus")
            && runtime_diagnostics.contains("CpuEnvelopeOnly")
            && runtime_diagnostics.contains("pub gpu_timing_status: NativeGpuTimingStatus")
            && runtime_diagnostics.contains("pub frame_work: NativeFrameWorkTimings")
            && runtime_diagnostics.contains("pub composited_base: NativeCompositedBaseTiming")
            && runtime_diagnostics.contains("pub transient_overlay: NativeTransientOverlayTiming")
            && runtime_diagnostics.contains("pub fn cpu_envelope_total"),
        "runtime timing diagnostics should expose an explicit native GPU timing availability status"
    );
    assert!(
        native_diagnostics
            .contains("gpu_timing_status: crate::runtime::NativeGpuTimingStatus::CpuEnvelopeOnly")
            && native_diagnostics.contains("window_identity: parts.profile.window_identity")
            && native_diagnostics.contains("frame_sequence: parts.profile.frame_sequence")
            && render_profile.contains("gpu_timing_status = \"cpu_envelope_only\"")
            && render_profile.contains(
                "window_identity = frame.window_identity.map(NativeWindowDiagnosticIdentity::get)",
            )
            && render_profile.contains("frame_sequence = frame.frame_sequence")
            && render_profile.contains("frame_cpu_envelope_total_us"),
        "native frame diagnostics and render profile should report CPU-envelope-only GPU timing status"
    );
}

#[test]
fn api_docs_describe_cpu_due_lateness_as_observational() {
    let docs = normalized(&read_project_file("docs/API.md"));
    let diagnostics = read_project_file("src/runtime/diagnostics/frame.rs");

    assert!(
        docs.contains("`NativeCpuFrameFairnessDiagnostics::latest_due_lateness_us`")
            && docs.contains("missed-presentation-deadline evidence")
            && docs.contains("original cadence `due_at` boundary")
            && docs.contains("never changes scheduling policy")
            && diagnostics.contains("pub latest_due_lateness_us: Option<u64>"),
        "CPU due lateness should be documented as bounded observational evidence"
    );
}
