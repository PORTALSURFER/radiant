use crate::{gui::types::Vector2, gui_runtime::native_vello::wgpu};

pub(in crate::gui_runtime::native_vello::generic_runtime) struct PostGpuOverlayRenderTarget<'a> {
    pub(in crate::gui_runtime::native_vello::generic_runtime) device: &'a wgpu::Device,
    pub(in crate::gui_runtime::native_vello::generic_runtime) queue: &'a wgpu::Queue,
    pub(in crate::gui_runtime::native_vello::generic_runtime) encoder: &'a mut wgpu::CommandEncoder,
    pub(in crate::gui_runtime::native_vello::generic_runtime) target_view: &'a wgpu::TextureView,
    pub(in crate::gui_runtime::native_vello::generic_runtime) format: wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime) size: Vector2,
}
