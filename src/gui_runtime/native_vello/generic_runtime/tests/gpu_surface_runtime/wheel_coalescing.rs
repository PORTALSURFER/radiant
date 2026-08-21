use super::super::super::native_immediate_transient_stage::{
    NativeImmediateTransientKind, NativeImmediateTransientStageEvidence,
    admit_native_immediate_transient, complete_native_immediate_transient,
};
use super::super::super::runner_state::NativeTargetGeneration;
use super::super::super::{NativeAdapterGeneration, NativeLifecycle};
use super::*;
use crate::gui::input::InputTimestamp;
use crate::runtime::{RepaintScope, SurfaceInvalidation};
use crate::widgets::PointerModifiers;
use winit::event::TouchPhase;

#[test]
fn deferred_scroll_routes_message_without_refreshing_surface_until_requested() {
    let mut core =
        GenericNativeRuntimeCore::new(WheelRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let point = Point::new(12.0, 12.0);

    assert!(
        core.route_scroll_deferred_refresh_with_modifiers(
            point,
            Vector2::new(0.0, -40.0),
            Default::default(),
        )
        .routed
    );
    assert_eq!(core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "deferred wheel routing should not refresh the projected surface immediately"
    );

    core.refresh_surface();
    assert_eq!(core.runtime.bridge().project_count, 2);
}

#[test]
fn deferred_wheel_refresh_preserves_typed_scope_until_frame_preparation() {
    for (requested_scope, effective_scope) in [
        (RepaintScope::Projection, RepaintScope::Surface),
        (RepaintScope::Layout, RepaintScope::Surface),
    ] {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            WheelRefreshBridge {
                repaint_scope: Some(requested_scope),
                ..WheelRefreshBridge::default()
            },
            Vector2::new(240.0, 40.0),
        );
        // Discard startup diagnostics so this frame describes the requested
        // refresh and its classifier-selected effective scope.
        let _ = runner.core.runtime.take_frame_refresh_diagnostics();
        let layout_before = runner.core.runtime.refresh_counters().layout;
        let outcome = runner.core.route_scroll_deferred_refresh_with_modifiers(
            Point::new(12.0, 12.0),
            Vector2::new(0.0, -40.0),
            Default::default(),
        );

        assert!(outcome.is_deferred_surface_refresh());
        assert_eq!(outcome.surface_refresh_scope_or_surface(), requested_scope);
        runner.apply_route_outcome(outcome);
        assert_eq!(
            runner.timing.deferred_surface_refresh_scope,
            Some(requested_scope)
        );

        runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

        assert_eq!(
            runner.core.runtime.refresh_counters().layout,
            layout_before
                + if effective_scope.refreshes_layout() {
                    1
                } else {
                    0
                }
        );
        let frame = runner.core.runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, requested_scope);
        assert_eq!(frame.effective_scope, effective_scope);
        assert_eq!(
            runner.core.runtime.last_refresh_diagnostics().invalidation,
            match requested_scope {
                RepaintScope::Projection => SurfaceInvalidation::Projection,
                RepaintScope::Layout => SurfaceInvalidation::Layout,
                RepaintScope::Surface => SurfaceInvalidation::Surface,
                RepaintScope::PaintOnly => SurfaceInvalidation::PaintOnly,
            }
        );
    }
}

#[test]
fn deferred_scroll_fallback_requests_interactive_surface_refresh() {
    let mut core =
        GenericNativeRuntimeCore::new(ScrollRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let point = Point::new(12.0, 12.0);

    let outcome = core.route_scroll_deferred_refresh_with_modifiers(
        point,
        Vector2::new(0.0, 40.0),
        Default::default(),
    );

    assert!(outcome.routed);
    assert!(!outcome.is_deferred_surface_refresh());
    assert!(outcome.is_interactive_surface_refresh());
    assert!(outcome.is_interactive_scene_rebuild());
    assert!(outcome.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "route classification should leave projection to the native runner interactive refresh path"
    );

    core.refresh_surface();
    assert_eq!(core.runtime.bridge().project_count, 2);
}

#[test]
fn queued_gpu_surface_wheel_flushes_one_coalesced_update() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let project_count = runner.core.runtime.bridge().project_count;

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, -20.0), Default::default());
    runner.queue_gpu_surface_wheel(
        Point::new(80.0, 20.0),
        Vector2::new(0.0, -30.0),
        Default::default(),
    );
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "queued input should not claim frame work before it is flushed"
    );
    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().last_delta,
        Vector2::new(0.0, -50.0)
    );
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count,
        "coalesced wheel routing should not refresh until redraw applies deferred refresh"
    );
    assert!(runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RefreshSurface {
            reason: FrameWorkReason::DeferredSurfaceRefresh,
        },
        "flushed GPU wheel input should report the deferred refresh it schedules"
    );
}

#[test]
fn queued_gpu_surface_wheel_keeps_newest_sample_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let first_position = Point::new(40.0, 20.0);
    let newest_position = Point::new(80.0, 24.0);
    let first_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let newest_modifiers = PointerModifiers {
        command: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let first_timestamp = Some(InputTimestamp::capture());
    let newest_timestamp = Some(InputTimestamp::capture());
    let first_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("first wheel sample should receive a sequence range");
    let newest_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("newest wheel sample should receive a sequence range");

    runner.queue_gpu_surface_wheel_with_metadata(
        first_position,
        Vector2::new(0.0, -20.0),
        first_modifiers,
        first_timestamp,
        Some(first_sequence),
    );
    runner.queue_gpu_surface_wheel_with_metadata(
        newest_position,
        Vector2::new(0.0, -30.0),
        newest_modifiers,
        newest_timestamp,
        Some(newest_sequence),
    );

    let pending = runner
        .input
        .pending_gpu_surface_wheel
        .expect("same-axis wheel events should remain coalesced");
    assert_eq!(pending.position, newest_position);
    assert_eq!(pending.delta, Vector2::new(0.0, -50.0));
    assert_eq!(pending.modifiers, newest_modifiers);
    assert_eq!(pending.timestamp, newest_timestamp);
    let pending_sequence = pending
        .sequence_range
        .expect("coalesced wheel should retain sequence metadata");
    assert_eq!(pending_sequence.start(), first_sequence.start());
    assert_eq!(pending_sequence.end(), newest_sequence.end());

    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    let bridge = runner.core.runtime.bridge();
    assert_eq!(bridge.last_position, Some(newest_position));
    assert_eq!(bridge.last_modifiers, Some(newest_modifiers));
    assert_eq!(bridge.last_timestamp, newest_timestamp);
    assert_eq!(
        bridge
            .last_sequence_range
            .expect("flushed wheel should retain sequence metadata")
            .start(),
        first_sequence.start()
    );
    assert_eq!(
        bridge
            .last_sequence_range
            .expect("flushed wheel should retain sequence metadata")
            .end(),
        newest_sequence.end()
    );
}

#[test]
fn queued_gpu_surface_wheel_keeps_reversed_diagonal_horizontal_delta_out_of_vertical_axis() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);

    runner.queue_gpu_surface_wheel(point, Vector2::new(10.0, 9.0), Default::default());
    runner.queue_gpu_surface_wheel(point, Vector2::new(-10.0, 9.0), Default::default());

    let pending = runner
        .input
        .pending_gpu_surface_wheel
        .expect("same-axis wheel events should remain coalesced");
    assert_eq!(pending.delta, Vector2::new(0.0, 0.0));
}

#[test]
fn queued_gpu_surface_wheel_flushes_before_switching_semantic_axis() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let first_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let newest_modifiers = PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    };
    let first_timestamp = Some(InputTimestamp::capture());
    let newest_timestamp = Some(InputTimestamp::capture());
    let first_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("first axis sample should receive a sequence range");
    let newest_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("newest axis sample should receive a sequence range");

    runner.queue_gpu_surface_wheel_with_metadata(
        point,
        Vector2::new(20.0, 0.0),
        first_modifiers,
        first_timestamp,
        Some(first_sequence),
    );
    runner.queue_gpu_surface_wheel_with_metadata(
        point,
        Vector2::new(0.0, -30.0),
        newest_modifiers,
        newest_timestamp,
        Some(newest_sequence),
    );

    assert_eq!(runner.core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().last_delta,
        Vector2::new(20.0, 0.0),
        "changing semantic axis must route the prior pending delta before queuing the new one"
    );
    assert_eq!(
        runner.core.runtime.bridge().last_modifiers,
        Some(first_modifiers)
    );
    assert_eq!(runner.core.runtime.bridge().last_timestamp, first_timestamp);
    assert_eq!(
        runner
            .core
            .runtime
            .bridge()
            .last_sequence_range
            .expect("axis-flushed wheel should retain sequence metadata")
            .start(),
        first_sequence.start()
    );
    assert_eq!(
        runner
            .core
            .runtime
            .bridge()
            .last_sequence_range
            .expect("axis-flushed wheel should retain sequence metadata")
            .end(),
        first_sequence.end()
    );

    let pending = runner
        .input
        .pending_gpu_surface_wheel
        .expect("new semantic axis should become the pending owner");
    assert_eq!(pending.sequence_range, Some(newest_sequence));

    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().wheel_count, 2);
    assert_eq!(
        runner.core.runtime.bridge().last_delta,
        Vector2::new(0.0, -30.0)
    );
    assert_eq!(
        runner.core.runtime.bridge().last_modifiers,
        Some(newest_modifiers)
    );
    assert_eq!(
        runner.core.runtime.bridge().last_timestamp,
        newest_timestamp
    );
    assert_eq!(
        runner.core.runtime.bridge().last_sequence_range,
        Some(newest_sequence)
    );
}

#[test]
fn queued_scroll_container_wheel_keeps_newest_sample_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    let first_position = Point::new(40.0, 20.0);
    let newest_position = Point::new(80.0, 24.0);
    let first_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let newest_modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };
    let first_timestamp = Some(InputTimestamp::capture());
    let newest_timestamp = Some(InputTimestamp::capture());
    let first_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("first scroll-container sample should receive a sequence range");
    let newest_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("newest scroll-container sample should receive a sequence range");

    runner.queue_scroll_container_wheel_with_metadata(
        first_position,
        Vector2::new(0.0, 20.0),
        first_modifiers,
        first_timestamp,
        Some(first_sequence),
    );
    runner.queue_scroll_container_wheel_with_metadata(
        newest_position,
        Vector2::new(0.0, 30.0),
        newest_modifiers,
        newest_timestamp,
        Some(newest_sequence),
    );

    let pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("same-axis scroll-container wheels should remain coalesced");
    assert_eq!(pending.position, newest_position);
    assert_eq!(pending.delta, Vector2::new(0.0, 50.0));
    assert_eq!(pending.modifiers, newest_modifiers);
    assert_eq!(pending.timestamp, newest_timestamp);
    let pending_sequence = pending
        .sequence_range
        .expect("coalesced scroll-container wheel should retain sequence metadata");
    assert_eq!(pending_sequence.start(), first_sequence.start());
    assert_eq!(pending_sequence.end(), newest_sequence.end());
}

#[test]
fn fixed_alternating_transient_burst_keeps_owner_and_coalescers_bounded() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();

    const BURST_LEN: u64 = 512;
    let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
    let target_generation = NativeTargetGeneration::from_test_serial(1);
    let mut first_cursor_sequence = None;
    let mut newest_cursor_sequence = None;
    let mut newest_cursor_timestamp = None;
    let mut newest_cursor_position = None;
    let mut newest_cursor_modifiers = None;
    let mut first_wheel_sequence = None;
    let mut newest_wheel_sequence = None;
    let mut newest_wheel_timestamp = None;
    let mut newest_wheel_position = None;
    let mut newest_wheel_modifiers = None;

    for index in 0..BURST_LEN {
        let timestamp = InputTimestamp::capture();
        let sequence = runner
            .input
            .input_sequence_allocator
            .allocate()
            .expect("fixed burst sample should receive a sequence range");
        let position = Point::new(index as f32, (index % 31) as f32);
        let modifiers = if index % 4 == 0 {
            PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            }
        } else {
            PointerModifiers {
                command: true,
                alt: index % 3 == 0,
                ..PointerModifiers::default()
            }
        };
        let cursor = index % 2 == 0;
        let evidence = NativeImmediateTransientStageEvidence {
            key: runner.frame_stage_owner.schedule_key().clone(),
            kind: if cursor {
                NativeImmediateTransientKind::CursorMoved
            } else {
                NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved)
            },
            timestamp,
            window_id: Some(winit::window::WindowId::dummy()),
            adapter_generation,
            active_resource_generation: Some(adapter_generation),
            target_generation,
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        };
        let ticket = admit_native_immediate_transient(&mut runner.frame_stage_owner, evidence)
            .expect("alternating transient sample should admit");

        if cursor {
            runner.queue_scrollbar_drag_with_metadata_for_immediate_transient(
                position,
                modifiers,
                Some(timestamp),
                Some(sequence),
            );
            first_cursor_sequence.get_or_insert(sequence);
            newest_cursor_sequence = Some(sequence);
            newest_cursor_timestamp = Some(timestamp);
            newest_cursor_position = Some(position);
            newest_cursor_modifiers = Some(modifiers);
        } else {
            runner.queue_scroll_container_wheel_with_metadata_for_immediate_transient(
                position,
                Vector2::new(0.0, 1.0),
                modifiers,
                Some(timestamp),
                Some(sequence),
            );
            first_wheel_sequence.get_or_insert(sequence);
            newest_wheel_sequence = Some(sequence);
            newest_wheel_timestamp = Some(timestamp);
            newest_wheel_position = Some(position);
            newest_wheel_modifiers = Some(modifiers);
        }

        assert!(
            complete_native_immediate_transient(&mut runner.frame_stage_owner, ticket).is_success()
        );
        assert!(
            !runner.frame_stage_owner.has_in_flight(),
            "transient owner must be empty after every fixed-burst completion"
        );
        if cursor {
            assert!(runner.input.pending_scrollbar_drag.is_some());
        } else {
            assert!(runner.input.pending_scroll_container_wheel.is_some());
        }
        assert!(
            usize::from(runner.input.pending_scrollbar_drag.is_some())
                + usize::from(runner.input.pending_scroll_container_wheel.is_some())
                <= 2,
            "the fixed burst must retain at most one sample per existing bounded slot"
        );
    }

    let cursor = runner
        .input
        .pending_scrollbar_drag
        .expect("cursor samples should occupy one bounded latest-only slot");
    assert_eq!(
        cursor.position,
        newest_cursor_position.expect("cursor sample")
    );
    assert_eq!(cursor.timestamp, newest_cursor_timestamp);
    assert_eq!(
        cursor.modifiers,
        newest_cursor_modifiers.expect("cursor metadata")
    );
    let cursor_sequence = cursor
        .sequence_range
        .expect("cursor slot should retain sequence metadata");
    assert_eq!(
        cursor_sequence.start(),
        first_cursor_sequence
            .expect("first cursor sequence")
            .start()
    );
    assert_eq!(
        cursor_sequence.end(),
        newest_cursor_sequence
            .expect("newest cursor sequence")
            .end()
    );

    let wheel = runner
        .input
        .pending_scroll_container_wheel
        .expect("wheel samples should occupy one bounded coalescing slot");
    assert_eq!(wheel.position, newest_wheel_position.expect("wheel sample"));
    assert_eq!(wheel.timestamp, newest_wheel_timestamp);
    assert_eq!(
        wheel.modifiers,
        newest_wheel_modifiers.expect("wheel metadata")
    );
    assert_eq!(wheel.delta, Vector2::new(0.0, (BURST_LEN / 2) as f32));
    let wheel_sequence = wheel
        .sequence_range
        .expect("wheel slot should retain sequence metadata");
    assert_eq!(
        wheel_sequence.start(),
        first_wheel_sequence.expect("first wheel sequence").start()
    );
    assert_eq!(
        wheel_sequence.end(),
        newest_wheel_sequence.expect("newest wheel sequence").end()
    );
}

#[test]
fn focus_loss_discards_coalesced_input_without_retaining_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let gpu_timestamp = Some(InputTimestamp::capture());
    let scroll_timestamp = Some(InputTimestamp::capture());
    let gpu_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("GPU wheel sample should receive a sequence range");
    let scroll_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("scroll-container wheel sample should receive a sequence range");

    runner.queue_gpu_surface_wheel_with_metadata(
        point,
        Vector2::new(0.0, -20.0),
        Default::default(),
        gpu_timestamp,
        Some(gpu_sequence),
    );
    runner.queue_scroll_container_wheel_with_metadata(
        point,
        Vector2::new(0.0, -20.0),
        Default::default(),
        scroll_timestamp,
        Some(scroll_sequence),
    );
    runner.queue_scrollbar_drag(point);

    assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
    assert!(runner.input.pending_gpu_surface_wheel.is_some());
    assert!(runner.input.pending_scroll_container_wheel.is_some());
    assert!(runner.input.pending_scrollbar_drag.is_some());
    assert_eq!(
        runner
            .input
            .pending_gpu_surface_wheel
            .expect("GPU wheel metadata should be pending")
            .timestamp,
        gpu_timestamp
    );
    assert_eq!(
        runner
            .input
            .pending_scroll_container_wheel
            .expect("scroll-container wheel metadata should be pending")
            .timestamp,
        scroll_timestamp
    );
    assert_eq!(
        runner
            .input
            .pending_gpu_surface_wheel
            .expect("GPU wheel metadata should be pending")
            .sequence_range,
        Some(gpu_sequence)
    );
    assert_eq!(
        runner
            .input
            .pending_scroll_container_wheel
            .expect("scroll-container wheel metadata should be pending")
            .sequence_range,
        Some(scroll_sequence)
    );

    runner.handle_focus_lost_before_external_drag();

    assert!(runner.input.pending_gpu_surface_wheel.is_none());
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert!(runner.input.pending_scrollbar_drag.is_none());
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "canceled coalesced input must not leak work into presentation diagnostics"
    );
}

#[test]
fn queued_gpu_surface_wheel_refreshes_scroll_fallback_immediately() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, 40.0), Default::default());
    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        2,
        "scroll fallback from a coalesced GPU region must refresh before the next present"
    );
    assert!(
        !runner.timing.deferred_surface_refresh,
        "interactive scroll fallback should not leave stale materialized rows deferred"
    );
    assert!(
        !runner.timing.deferred_scene_rebuild,
        "interactive scroll fallback should not present a stale scene"
    );
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RebuildScene {
            reason: FrameWorkReason::InteractiveSurfaceRefresh,
            mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
        },
        "coalesced wheel diagnostics should report the frame work discovered while flushing input"
    );
}

#[test]
fn queued_gpu_surface_wheel_commits_ordinary_virtual_scroll_scene_before_present() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AppVirtualListBridge::retaining_materialized_window().with_coalescing_gpu_surface(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 10.0);
    let initial_scene_transforms = runner.frame.scene.encoding().transforms.clone();
    assert!(!runner.frame.last_paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Row 0")
    ));

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, 10.0), Default::default());
    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_ne!(
        runner.frame.scene.encoding().transforms,
        initial_scene_transforms,
        "ordinary virtual-list scroll fallback must re-encode the committed Vello scene"
    );

    let committed_row_rect = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "Row 1" => Some(text.rect),
            _ => None,
        })
        .expect("scroll fallback should retain the first row in the paint plan");
    assert!(
        committed_row_rect.min.y >= 0.0,
        "ordinary virtual-list scroll fallback must encode the moved scene before presentation"
    );
    assert_eq!(committed_row_rect.min.y, 14.0);
    assert!(!runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        }
    );
}

#[test]
fn coalesced_wheel_then_pointer_move_retargets_hover_after_virtual_rows_refresh() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        HoverVirtualListBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let pointer = Point::new(40.0, 30.0);

    runner.handle_cursor_moved(winit::dpi::PhysicalPosition::new(
        pointer.x as f64,
        pointer.y as f64,
    ));
    let first_hover = runner
        .core
        .runtime
        .hovered_widget()
        .expect("initial virtual row should be hovered");
    assert!(
        runner
            .core
            .runtime
            .surface()
            .find_widget(first_hover)
            .expect("initial hovered row")
            .widget()
            .common()
            .state
            .hovered
    );

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    let queued =
        runner.route_native_mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -100.0));
    assert_eq!(
        queued.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );

    runner.handle_cursor_moved(winit::dpi::PhysicalPosition::new(
        pointer.x as f64 + 1.0,
        pointer.y as f64,
    ));
    runner.flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());

    let hovered = runner
        .core
        .runtime
        .hovered_widget()
        .expect("post-scroll pointer should target a current virtual row");
    assert_ne!(hovered, first_hover);
    assert!(
        runner
            .core
            .runtime
            .surface()
            .find_widget(hovered)
            .expect("current hovered row")
            .widget()
            .common()
            .state
            .hovered,
        "hover state must follow the current materialized row"
    );
    assert!(
        runner
            .frame
            .last_paint_plan
            .primitives
            .iter()
            .any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::FillRect(fill)
                        if fill.widget_id == hovered && fill.rect.has_finite_positive_area()
                )
            }),
        "presented paint plan must expose hover chrome for the current row"
    );
    assert!(
        runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "coalesced wheel must present current virtual-list rows"
    );
}
