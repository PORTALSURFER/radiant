use super::super::gpu_surface_types::{
    CustomShaderPipeline, CustomShaderPipelineIdentity, CustomShaderPipelineKey,
};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::{GpuSurfaceRenderer, wgpu_device_id};
use super::diagnostics::custom_shader_validation_error;
#[path = "pipeline/layout.rs"]
mod layout;
use crate::runtime::GpuShaderSurfaceDescriptor;
use layout::{create_custom_shader_bind_group_layout, create_custom_shader_pipeline_layout};
use std::sync::Arc;
use tracing::warn;
use vello::wgpu;

pub(super) struct CustomShaderPipelineRequest<'a> {
    pub(super) surface_key: u64,
    pub(super) device: &'a wgpu::Device,
    pub(super) target_format: wgpu::TextureFormat,
    pub(super) key: CustomShaderPipelineKey,
}

/// An immutable, device-owned request which may be moved to the native host's
/// existing worker task. `device_identity` is captured on the UI thread; a
/// worker must never derive it from its clone of `device`.
#[derive(Clone)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct OwnedCustomShaderPipelineRequest {
    pub(in crate::gui_runtime::native_vello::generic_runtime) device: wgpu::Device,
    pub(in crate::gui_runtime::native_vello::generic_runtime) device_identity: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime) target_format: wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime) key: CustomShaderPipelineKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum CustomShaderPreparationFailure {
    Cancelled,
    ShaderModule,
    Pipeline,
    Panicked,
}

/// Builds both GPU objects with all validation scopes pushed and popped on the
/// calling thread. Driver calls themselves cannot be interrupted; cancellation
/// is therefore observed immediately before and after each stage.
pub(in crate::gui_runtime::native_vello::generic_runtime) fn prepare_custom_shader_pipeline(
    request: OwnedCustomShaderPipelineRequest,
    cancelled: impl Fn() -> bool,
) -> Result<CustomShaderPipeline, CustomShaderPreparationFailure> {
    // Preserve the UI-stamped identity as data. It is intentionally never
    // recomputed from this worker-owned device clone.
    let _captured_device_identity = request.device_identity;
    if cancelled() {
        return Err(CustomShaderPreparationFailure::Cancelled);
    }
    let borrowed = CustomShaderPipelineRequest {
        surface_key: 0,
        device: &request.device,
        target_format: request.target_format,
        key: request.key.clone(),
    };
    let module_scope = borrowed
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = borrowed
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_custom_shader_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(borrowed.key.wgsl_source.as_ref().into()),
        });
    if custom_shader_validation_error(module_scope).is_some() {
        return Err(CustomShaderPreparationFailure::ShaderModule);
    }
    if cancelled() {
        return Err(CustomShaderPreparationFailure::Cancelled);
    }
    let pipeline_scope = borrowed
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let bind_group_layout = create_custom_shader_bind_group_layout(&borrowed);
    let layout = create_custom_shader_pipeline_layout(borrowed.device, &bind_group_layout);
    let pipeline = create_custom_shader_render_pipeline(&borrowed, &shader, &layout);
    if custom_shader_validation_error(pipeline_scope).is_some() {
        return Err(CustomShaderPreparationFailure::Pipeline);
    }
    if cancelled() {
        return Err(CustomShaderPreparationFailure::Cancelled);
    }
    Ok(CustomShaderPipeline {
        key: request.key,
        bind_group_layout,
        pipeline,
    })
}

impl GpuSurfaceRenderer {
    pub(super) fn ensure_custom_shader_pipeline(
        &mut self,
        request: CustomShaderPipelineRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        if !self.custom_shader_pipeline_needs_rebuild(&request) {
            return true;
        }
        let identity = CustomShaderPipelineIdentity {
            device: wgpu_device_id(request.device),
            format: request.target_format,
            key: request.key.clone(),
        };
        if !self
            .resources
            .can_admit_custom_shader_pipeline(request.surface_key, &identity)
        {
            warn!(surface_key = request.surface_key, shader_key = %request.key.shader_key,
                "radiant custom shader pipeline cache admission limit reached");
            return false;
        }
        if self
            .resources
            .has_custom_shader_pipeline_identity(&identity)
        {
            self.resources
                .remove_custom_shader_binding(&request.surface_key);
            self.resources
                .associate_custom_shader_pipeline(request.surface_key, identity);
            return true;
        }
        let Some(created) = self.prepared_custom_shader_pipeline(&identity) else {
            match self.custom_shader_preparation_failure(&identity) {
                Some(CustomShaderPreparationFailure::ShaderModule) => {
                    stats.custom_shader.failures.shader_module_failures += 1;
                }
                Some(CustomShaderPreparationFailure::Pipeline) => {
                    stats.custom_shader.failures.pipeline_failures += 1;
                }
                _ => {}
            }
            return false;
        };
        // Worker creation is not attributed to demand-redraw pipeline builds.
        self.resources
            .remove_custom_shader_binding(&request.surface_key);
        self.resources
            .insert_custom_shader_pipeline(request.surface_key, identity, created);
        true
    }

    pub(super) fn custom_shader_pipeline_needs_rebuild(
        &self,
        request: &CustomShaderPipelineRequest<'_>,
    ) -> bool {
        self.resources
            .custom_shader_pipeline_identity(request.surface_key)
            .is_none_or(|pipeline| {
                pipeline.device != wgpu_device_id(request.device)
                    || pipeline.format != request.target_format
                    || pipeline.key != request.key
            })
    }
}

#[cfg(test)]
fn create_custom_shader_module(
    request: &CustomShaderPipelineRequest<'_>,
    stats: &mut GpuSurfaceRenderStats,
) -> Option<wgpu::ShaderModule> {
    let error_scope = request
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = request
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_custom_shader_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(request.key.wgsl_source.as_ref().into()),
        });
    if let Some(error) = custom_shader_validation_error(error_scope) {
        stats.custom_shader.failures.shader_module_failures += 1;
        warn!(
            surface_key = request.surface_key,
            shader_key = %request.key.shader_key,
            error = %error,
            "radiant custom shader WGSL module validation failed"
        );
        return None;
    }
    Some(shader)
}

fn create_custom_shader_render_pipeline(
    request: &CustomShaderPipelineRequest<'_>,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    request
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_custom_shader_surface_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(&request.key.vertex_entry_point),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(&request.key.fragment_entry_point),
                targets: &[Some(wgpu::ColorTargetState {
                    format: request.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn custom_shader_pipeline_key(
    descriptor: &GpuShaderSurfaceDescriptor,
) -> Option<CustomShaderPipelineKey> {
    Some(CustomShaderPipelineKey {
        shader_key: Arc::from(descriptor.shader_key.as_str()),
        wgsl_source: descriptor.wgsl_source.clone()?,
        vertex_entry_point: Arc::from(descriptor.entry_point.as_str()),
        fragment_entry_point: Arc::from(descriptor.fragment_entry_point.as_deref()?),
        has_uniform_payload: !descriptor.uniform_bytes.is_empty(),
        has_storage_payload: !descriptor.storage_bytes.is_empty(),
        has_presentation_uniform_payload: descriptor
            .presentation_uniform_bytes
            .as_ref()
            .is_some_and(|bytes| !bytes.is_empty()),
    })
}

#[cfg(test)]
#[path = "pipeline/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pipeline/measurement_native_tests.rs"]
mod measurement_native_tests;
