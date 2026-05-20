//! Public model types for retained GPU-surface content.

use std::sync::Arc;

/// Named backend-neutral payload for a custom shader-backed GPU surface.
///
/// The descriptor intentionally avoids exposing `wgpu` handles. Applications
/// describe stable shader identity and opaque payload bytes while backend
/// adapters decide whether and how to compile a matching native pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuShaderSurfaceDescriptor {
    /// Stable application-defined shader or pipeline identity.
    pub shader_key: String,
    /// Shader entry point requested by this surface.
    pub entry_point: String,
    /// Opaque uniform payload consumed by a backend-specific pipeline.
    pub uniform_bytes: Arc<[u8]>,
    /// Opaque storage/data payload consumed by a backend-specific pipeline.
    pub storage_bytes: Arc<[u8]>,
    /// Number of vertices the backend should draw for this surface.
    pub vertex_count: u32,
}

/// Named construction parts for [`GpuShaderSurfaceDescriptor`].
#[derive(Clone, Debug, PartialEq)]
pub struct GpuShaderSurfaceDescriptorParts {
    /// Stable application-defined shader or pipeline identity.
    pub shader_key: String,
    /// Shader entry point requested by this surface.
    pub entry_point: String,
    /// Opaque uniform payload consumed by a backend-specific pipeline.
    pub uniform_bytes: Arc<[u8]>,
    /// Opaque storage/data payload consumed by a backend-specific pipeline.
    pub storage_bytes: Arc<[u8]>,
    /// Number of vertices the backend should draw for this surface.
    pub vertex_count: u32,
}

impl GpuShaderSurfaceDescriptor {
    /// Build a descriptor from named parts.
    pub fn from_parts(parts: GpuShaderSurfaceDescriptorParts) -> Self {
        Self {
            shader_key: parts.shader_key,
            entry_point: parts.entry_point,
            uniform_bytes: parts.uniform_bytes,
            storage_bytes: parts.storage_bytes,
            vertex_count: parts.vertex_count,
        }
    }

    /// Build a descriptor with the conventional `main` entry point.
    pub fn new(shader_key: impl Into<String>) -> Self {
        Self::from_parts(GpuShaderSurfaceDescriptorParts {
            shader_key: shader_key.into(),
            entry_point: String::from("main"),
            uniform_bytes: Arc::<[u8]>::from([]),
            storage_bytes: Arc::<[u8]>::from([]),
            vertex_count: 3,
        })
    }

    /// Set the shader entry point.
    pub fn entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.entry_point = entry_point.into();
        self
    }

    /// Set opaque uniform bytes for the backend pipeline.
    pub fn uniform_bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.uniform_bytes = Arc::from(bytes.as_ref());
        self
    }

    /// Set opaque storage/data bytes for the backend pipeline.
    pub fn storage_bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.storage_bytes = Arc::from(bytes.as_ref());
        self
    }

    /// Set the vertex count to draw for this shader surface.
    pub fn vertex_count(mut self, vertex_count: u32) -> Self {
        self.vertex_count = vertex_count;
        self
    }
}

/// Optional GPU-side gain envelope for retained signal rendering.
///
/// The preview is intentionally normalized and backend-neutral: hosts can
/// preview destructive fade/gain edits without rebuilding or re-uploading the
/// retained signal payload on each pointer update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSignalGainPreview {
    /// Normalized selection start.
    pub start: f32,
    /// Normalized selection end.
    pub end: f32,
    /// Gain applied inside the selection after fades.
    pub gain: f32,
    /// Fade-in length as a fraction of the selection width.
    pub fade_in_length: f32,
    /// Fade-in curve tension.
    pub fade_in_curve: f32,
    /// Fade-in outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_in_mute: f32,
    /// Fade-out length as a fraction of the selection width.
    pub fade_out_length: f32,
    /// Fade-out curve tension.
    pub fade_out_curve: f32,
    /// Fade-out outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_out_mute: f32,
}

/// Renderable shape resolved from a retained GPU signal payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSignalRenderShape {
    /// Effective source frame count available to the renderer.
    pub frames: usize,
    /// Number of interleaved bands per frame.
    pub band_count: usize,
    /// Visible frame range as start/end frame offsets.
    pub frame_range: [f32; 2],
    /// Number of source sample or summary bucket entries.
    pub sample_count: usize,
}
