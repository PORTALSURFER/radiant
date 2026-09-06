use super::*;
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::persistent_storage::{
    PersistentStorageSelection, stage_selected_cursor, write_selected_range,
};
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::upload_plan::GpuSurfaceRenderCanvasUploadAction;

pub(super) fn upload(
    request: &mut CustomShaderBufferUploadRequest<'_, '_>,
    selection: &PersistentStorageSelection<'_>,
    mut plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
    stats: &mut GpuSurfaceRenderStats,
) -> bool {
    let selected_plan = match selection {
        PersistentStorageSelection::Mismatch => return false,
        PersistentStorageSelection::Absent => None,
        PersistentStorageSelection::Upload { plan, .. } => Some(plan),
    };
    if let Some(plan) = plan.as_mut()
        && !plan.consume_action(
            GpuSurfaceRenderCanvasUploadAction::CustomPersistentStorage {
                surface_index: request.surface_index,
                key: request.surface.key,
                plan: selected_plan.cloned(),
            },
        )
    {
        return false;
    }
    let PersistentStorageSelection::Upload {
        entry,
        plan: selected,
    } = selection
    else {
        request.binding.persistent_storage_cursor.stage_bulk_reset();
        return true;
    };
    stage_selected_cursor(request.binding, selection);
    for range in &selected.ranges {
        if let Some(plan) = plan.as_mut() {
            if plan.consume_upload(
                request.surface_index,
                GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            ) != Some(range.byte_len)
            {
                return false;
            }
            plan.mark_execution_mutated();
        }
        let Some(belt) = request.presentation_staging_belt.as_deref_mut() else {
            return false;
        };
        let Some(buffer) = request.binding.storage_buffer.as_ref() else {
            return false;
        };
        if !write_selected_range(belt, request.target.encoder, buffer, entry, *range) {
            return false;
        }
        stats.record_candidate_immutable_payload(range.byte_len);
        stats.custom_shader.static_writes += 1;
        stats.custom_shader.static_write_bytes += range.byte_len;
        record_custom_shader_storage_upload(stats, range.byte_len);
    }
    true
}

pub(super) fn write_bulk(
    request: &mut CustomShaderBufferUploadRequest<'_, '_>,
    bytes: &[u8],
) -> bool {
    let Some(buffer) = request.binding.storage_buffer.as_ref() else {
        return false;
    };
    if request
        .binding
        .persistent_storage_cursor
        .effective()
        .is_some()
    {
        request.binding.persistent_storage_cursor.stage_bulk_reset();
        let Some(belt) = request.presentation_staging_belt.as_deref_mut() else {
            return false;
        };
        let Some(size) = wgpu::BufferSize::new(bytes.len() as u64) else {
            return false;
        };
        belt.write_buffer(request.target.encoder, buffer, 0, size)
            .copy_from_slice(bytes);
    } else {
        request.target.queue.write_buffer(buffer, 0, bytes);
    }
    true
}
