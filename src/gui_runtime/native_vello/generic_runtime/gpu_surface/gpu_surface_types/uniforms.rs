pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) const MAX_GPU_SURFACE_OVERLAYS: usize = 8;
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) const GPU_SURFACE_OVERLAY_VEC4_SLOTS: usize = MAX_GPU_SURFACE_OVERLAYS / 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceUniforms {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) dest: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) source: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) target_size: [f32; 2],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _padding: [f32; 2],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) overlay_ratios:
        [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) overlay_widths:
        [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) overlay_colors:
        [[f32; 4]; MAX_GPU_SURFACE_OVERLAYS],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalUniforms {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) dest: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frame_range: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) slide_preview: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) summary_meta: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) gain_preview_a: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) gain_preview_b: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) gain_preview_c: [f32; 4],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) target_size: [f32; 2],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cursor_ratio: f32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cursor_width: f32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cursor_color: [f32; 4],
}
