use super::{normalized, read_project_file};

#[test]
fn api_docs_describe_bounded_cpu_frame_observation_evidence() {
    let docs = normalized(&read_project_file("docs/API.md"));
    let frame = read_project_file("src/runtime/diagnostics/frame.rs");

    for required in [
        "`NativeFrameDiagnostics::cpu_observation`",
        "bounded observational",
        "`NativeCpuFrameCompletionOutcome`",
        "exact routed interaction evidence",
        "`latest_exact_interaction`",
        "saturating admitted, successful, skipped-or-vetoed, incomplete, failed, and recovery-triggered totals",
        "`available` is false",
        "`latest_outcome` is `Unknown`",
        "zero/default values",
        "does not select work, change admission, route input, render, or alter publication ordering",
        "`publish_staged_frame_diagnostics`",
        "`forward_auxiliary_frame_diagnostics`",
    ] {
        assert!(
            docs.contains(required),
            "API docs should describe bounded CPU frame observation evidence with `{required}`"
        );
    }
    for required in [
        "pub enum NativeCpuFrameCompletionOutcome",
        "pub struct NativeCpuFrameObservationDiagnostics",
        "pub latest_outcome: NativeCpuFrameCompletionOutcome",
        "pub latest_exact_interaction: bool",
        "pub admitted_redraws: u64",
        "pub recovery_triggered_frames: u64",
    ] {
        assert!(
            frame.contains(required),
            "public CPU frame observation model should retain `{required}`"
        );
    }
}

#[test]
fn cpu_frame_observation_stays_private_and_projects_at_existing_boundaries() {
    let frame = read_project_file("src/runtime/diagnostics/frame.rs");
    let ledger =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/cpu_frame_observation.rs");
    let runner = read_project_file("src/gui_runtime/native_vello/generic_runtime/runner.rs");
    let lifecycle = read_project_file("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs");

    assert!(
        !frame.contains("CpuFrameStage")
            && !frame.contains("stage_mask")
            && !frame.contains("stages:"),
        "public CPU frame observation diagnostics must not expose private stage vocabulary"
    );
    assert!(
        ledger.contains("pub(super) fn project_frame_diagnostics")
            && ledger.contains("NativeCpuFrameObservationDiagnostics")
            && ledger.contains("public_completion_outcome"),
        "the private ledger should own the conservative public projection"
    );
    assert!(
        runner.contains("cpu_frame_observation: frame_diagnostics_enabled")
            && runner.contains("diagnostics.cpu_observation")
            && runner.contains("ledger.project_frame_diagnostics(&FrameScheduleKey::Primary)",),
        "primary observation evidence should remain opt-in and attach at publication"
    );
    assert!(
        lifecycle.contains("fn forward_auxiliary_frame_diagnostics")
            && lifecycle.contains("diagnostics.cpu_observation")
            && lifecycle.contains("runner\n            .cpu_frame_observation")
            && lifecycle.contains("ledger.project_frame_diagnostics(key)"),
        "auxiliary observation evidence should use the existing parent-owned forwarding boundary"
    );
}
