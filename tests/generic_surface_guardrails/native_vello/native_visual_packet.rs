//! Structural guardrails for the private native visual request handoff.

use std::{fs, path::PathBuf};

#[test]
fn native_visual_packets_have_one_bounded_redraw_authority() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src/gui_runtime/native_vello/generic_runtime");
    let packet = fs::read_to_string(source_root.join("native_visual_packet.rs"))
        .expect("native visual packet source should be readable");
    let runner = fs::read_to_string(source_root.join("runner.rs"))
        .expect("native runner source should be readable");
    let auxiliary = fs::read_to_string(source_root.join("auxiliary.rs"))
        .expect("native auxiliary source should be readable");
    let lifecycle = fs::read_to_string(source_root.join("lifecycle.rs"))
        .expect("native lifecycle source should be readable");
    let present = fs::read_to_string(source_root.join("present.rs"))
        .expect("native presentation source should be readable");
    let surface = fs::read_to_string(source_root.join("surface.rs"))
        .expect("native surface source should be readable");
    let encode_present = fs::read_to_string(source_root.join("native_encode_present.rs"))
        .expect("native encode/present source should be readable");

    assert!(
        packet.contains("pub(super) struct NativeVisualRequestPacket")
            && packet.contains("requested: Option<NativeVisualRequestPacket>")
            && packet.contains("consuming: Option<NativeVisualRequestIdentity>")
            && packet.contains("pending: Option<NativeVisualRequestPacket>")
            && packet
                .contains("const _: [(); NATIVE_VISUAL_MAILBOX_MAX_RETAINED_DEPTH] = [(); 2];",)
            && packet.contains("struct NativeVisualOwnerGeneration(NonZeroU64)")
            && packet.contains("struct NativeVisualRevision(NonZeroU64)")
            && packet.contains("ScheduledOrRuntime")
            && packet.contains("NativeInvalidationFallback")
            && !packet.contains("adapter_generation:")
            && !packet.contains("target_generation:")
            && packet.contains("checked_add"),
        "the private packet must retain bounded requested/consuming/pending ownership with checked identities"
    );
    assert!(
        packet.contains(
            "#[derive(Debug, PartialEq, Eq)]\npub(super) struct NativeVisualRequestPacket"
        ) && !packet.contains(
            "#[derive(Clone, Debug, PartialEq, Eq)]\npub(super) struct NativeVisualRequestPacket"
        ),
        "native visual request packets must remain deliberately non-Clone"
    );

    let raw_redraw_call_count = [
        packet.as_str(),
        runner.as_str(),
        auxiliary.as_str(),
        lifecycle.as_str(),
        present.as_str(),
    ]
    .into_iter()
    .map(|source| source.matches("request_redraw();").count())
    .sum::<usize>();
    assert_eq!(
        raw_redraw_call_count, 1,
        "only the central native visual request adapter may call Window::request_redraw"
    );
    assert!(
        packet.contains("fn issue(window: &Window)")
            && runner.contains("pub(super) fn begin_native_visual_request(")
            && runner.contains("pub(super) fn finish_native_visual_request(")
            && auxiliary.contains("begin_native_visual_request(adapter)")
            && auxiliary.contains("finish_native_visual_request(packet, disposition);")
            && lifecycle.contains("WindowEvent::RedrawRequested")
            && present.contains("begin_native_visual_request(&adapter)"),
        "primary and auxiliary redraw delivery must share the packet begin/finish boundary"
    );
    assert!(
        !runner.contains("is_visible(")
            && !auxiliary.contains("is_visible(")
            && !present.contains("is_visible(")
            && !surface.contains("is_visible(")
            && auxiliary.contains("if !self.active || !self.is_admitted()")
            && frame_scheduler_has_logical_eligibility(source_root.join("frame_scheduler.rs")),
        "native presentation eligibility must remain logical and independent of host visibility"
    );
    assert!(
        present.contains("Result<NativeVisualRequestDisposition, NativeFrameRenderFailure>")
            && surface.contains("Result<wgpu::SurfaceTexture, NativeSurfaceAcquireError>")
            && present.contains("NativeVisualRequestDisposition::Presented")
            && present.contains("NativeVisualRequestDisposition::DropPacket")
            && surface.contains("NativeVisualRequestDisposition::RetrySamePacket")
            && present.contains("let (disposition, redraw_failed) = match result")
            && auxiliary.contains("let (disposition, redraw_failed) = match redraw_result"),
        "primary and auxiliary wrappers must consume the typed native packet disposition"
    );
    assert!(
        encode_present.contains("pub(super) struct NativeFrameSnapshotRevision")
            && encode_present.contains("pub(super) struct NativeEncodePresentTicket")
            && encode_present.contains("NativeVisualRequestIdentity")
            && encode_present.contains("NativeEncodePresentPath")
            && encode_present.contains("EncodePresentStageTicket")
            && encode_present.contains(
                "#[derive(Debug, PartialEq, Eq)]\npub(super) struct NativeFrameSnapshotRevision",
            )
            && !encode_present.contains(
                "#[derive(Clone, Debug, PartialEq, Eq)]\npub(super) struct NativeEncodePresentTicket",
            )
            && !encode_present.contains("thread::")
            && !encode_present.contains("Mutex")
            && !encode_present.contains("Condvar")
            && !encode_present.contains("sleep(")
            && runner.contains("admit_native_encode_present(")
            && runner.contains("complete_native_encode_present(")
            && runner.contains("veto_native_encode_present(")
            && present.contains("snapshot_gpu_shader_presentation_updates()")
            && present.contains("commit_gpu_shader_presentation_updates()")
            && present.contains("abort_gpu_shader_presentation_updates()")
            && present.contains("acquire_present_surface_texture(")
            && present.contains("admit_native_encode_present(")
            && present.contains("NativeEncodePresentPath::DirectResize")
            && present.contains("NativeEncodePresentPath::Composited"),
        "native presentation must stage losslessly, admit one exact ticket immediately before acquisition, and share direct/composited ownership without waits or locks"
    );

    let redraw_start = present
        .find("pub(super) fn redraw(")
        .expect("redraw should remain the consuming presentation boundary");
    let redraw = &present[redraw_start..];
    let deferred_resume = redraw
        .find("self.resume_deferred_deadline_before_redraw(event_loop, adapter)")
        .expect("redraw should resume a deferred Deadline before preparation");
    let deferred_resize = redraw
        .find("self.apply_pending_surface_resize_if_needed(adapter);")
        .expect("redraw should retain deferred resize application");
    assert!(
        deferred_resume < deferred_resize,
        "deferred Deadline completion must precede deferred resize and CPU frame preparation"
    );
    let resume_start = runner
        .find("pub(super) fn resume_deferred_deadline_before_redraw(")
        .expect("runner should own the redraw-boundary deferred Deadline resume");
    let resume = &runner[resume_start..];
    let resume_admission = resume
        .find("self.admit_deferred_timed_frame_deadline(")
        .expect("redraw-boundary resume should consume the exact retained Deadline");
    let resume_route = resume
        .find("self.handle_route_outcome_deferred_publication(event_loop, admission.outcome);")
        .expect("deferred Deadline route outcomes should dispatch at the redraw boundary");
    let resume_continue_check = resume[resume_route..]
        .find("self.is_running()")
        .map(|offset| resume_route + offset)
        .expect("redraw should continue only after the deferred route handler updates lifecycle");
    assert!(
        resume_admission < resume_route && resume_route < resume_continue_check,
        "deferred Deadline route dispatch must follow exact resume and complete before redraw continuation"
    );
    let acquire_start = redraw
        .find("let surface_texture = match self.acquire_present_surface_texture(adapter)")
        .expect("redraw should use the raw acquisition seam");
    let acquire = &redraw[acquire_start..];
    let acquire_veto = acquire
        .find("self.veto_native_encode_present(ticket)")
        .expect("every acquisition error must retire the exact encode/present ticket");
    let acquire_abort = acquire
        .find("self.core.runtime.abort_gpu_shader_presentation_updates();")
        .expect("every acquisition error must abort the volatile snapshot");
    let acquire_policy = acquire
        .find("self.handle_present_surface_acquire_error(")
        .expect("surface recovery policy must run after ticket retirement");
    assert!(
        acquire_veto < acquire_abort && acquire_abort < acquire_policy,
        "Lost/Outdated/Timeout/Other/OOM policy must follow exact ticket veto and snapshot abort"
    );
    let admitted_path = redraw
        .find("let mut ticket = Some(ticket);")
        .expect("redraw should retain the admitted ticket across native work");
    assert!(
        !redraw[admitted_path..].contains("self.admit_native_resources(adapter)"),
        "post-admission paths must use pure ticket/resource currentness checks before vetoing, never recovery-capable resource admission"
    );

    let transition_start = surface
        .find("fn complete_target_transition(&mut self)")
        .expect("ordinary target transition should remain explicit");
    let transition_tail = &surface[transition_start..];
    let transition_end = transition_tail
        .find("pub(super) fn complete_native_recovery_target_transition")
        .expect("ordinary target transition should have a bounded body");
    let transition = &transition_tail[..transition_end];
    assert!(
        transition.contains("fence_native_surface_target_for_transition")
            && !transition.contains("native_visual_requests.invalidate()"),
        "ordinary deferred resize must advance target evidence without invalidating its claimed packet"
    );

    let recovery_start = surface
        .find("fn resize_surface_now_for_recovery(")
        .expect("surface recovery resize should remain explicit");
    let recovery_tail = &surface[recovery_start..];
    let recovery_end = recovery_tail
        .find("fn complete_target_transition(&mut self)")
        .expect("surface recovery resize should precede target transition");
    assert!(
        recovery_tail[..recovery_end].contains("self.fence_native_surface_target();"),
        "surface-loss reconfiguration must retain its explicit resource-failure fence"
    );
}

fn frame_scheduler_has_logical_eligibility(path: PathBuf) -> bool {
    let source = fs::read_to_string(path).expect("frame scheduler source should be readable");
    source.contains("pub(super) struct AuxiliaryScheduleEligibility")
        && !source.contains("pub(super) visible:")
        && source.contains("active")
        && source.contains("admitted")
}
