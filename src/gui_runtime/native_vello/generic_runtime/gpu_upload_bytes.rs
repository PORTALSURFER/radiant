//! Shared byte views for renderer-owned WGPU upload structs.

/// Marker for renderer structs whose in-memory bytes are the WGPU upload ABI.
///
/// # Safety
///
/// Implementors must be plain data with a stable representation for the shader
/// or vertex buffer that consumes the byte view. They must not contain pointers,
/// references, or other Rust-managed fields.
pub(super) unsafe trait GpuUploadBytes: Copy {}

pub(super) fn upload_value_as_bytes<T: GpuUploadBytes>(value: &T) -> &[u8] {
    upload_slice_as_bytes(std::slice::from_ref(value))
}

pub(super) fn upload_slice_as_bytes<T: GpuUploadBytes>(values: &[T]) -> &[u8] {
    let len = std::mem::size_of_val(values);
    let ptr = values.as_ptr().cast::<u8>();
    // SAFETY: `GpuUploadBytes` is implemented only for renderer ABI structs
    // whose memory representation is intentionally uploaded to WGPU buffers.
    // The returned view is tied to `values`, so it cannot outlive the source.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
