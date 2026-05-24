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
        runtime_diagnostics.contains("pub enum NativeGpuTimingStatus")
            && runtime_diagnostics.contains("CpuEnvelopeOnly")
            && runtime_diagnostics.contains("pub gpu_timing_status: NativeGpuTimingStatus")
            && runtime_diagnostics.contains("pub fn cpu_envelope_total"),
        "runtime timing diagnostics should expose an explicit native GPU timing availability status"
    );
    assert!(
        native_diagnostics
            .contains("gpu_timing_status: crate::runtime::NativeGpuTimingStatus::CpuEnvelopeOnly")
            && render_profile.contains("gpu_timing_status = \"cpu_envelope_only\"")
            && render_profile.contains("frame_cpu_envelope_total_us"),
        "native frame diagnostics and render profile should report CPU-envelope-only GPU timing status"
    );
}
