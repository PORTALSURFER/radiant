use super::read_runtime_source;

#[test]
fn native_vello_present_diagnostics_stay_in_focused_module() {
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let diagnostics =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs");

    assert!(
        present.contains("mod diagnostics;")
            && present.contains("native_frame_diagnostics(")
            && !present.contains("fn native_frame_diagnostics"),
        "present driver should delegate structured frame diagnostics projection"
    );
    assert!(
        diagnostics.contains("fn native_frame_diagnostics")
            && diagnostics.contains("NativeSceneDiagnostics")
            && diagnostics.contains("NativeTextDiagnostics")
            && diagnostics.contains("NativeGpuSurfaceDiagnostics")
            && diagnostics.contains("NativeFrameTimingDiagnostics"),
        "native frame diagnostics projection should live in present/diagnostics.rs"
    );
    assert!(
        diagnostics.contains(
            "use super::super::{RenderFrameProfile, RetainedSurfaceEncodeStats, gpu_surface};"
        ) && diagnostics
            .contains("use crate::gui_runtime::native_vello::TextLayoutProfileCounters;")
            && diagnostics.contains("use std::time::Duration;")
            && !diagnostics.starts_with("use super::super::*;"),
        "native frame diagnostics should name frame profile, encode stats, GPU stats, text stats, and timing dependencies"
    );
}

#[test]
fn native_gpu_upload_byte_casts_stay_in_focused_module() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let upload =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/gpu_upload_bytes.rs");
    let encoding =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface/encoding.rs");
    let vertex = read_runtime_source(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/vertex.rs",
    );

    assert!(
        module.contains("mod gpu_upload_bytes;")
            && upload.contains("unsafe trait GpuUploadBytes")
            && upload.contains("from_raw_parts"),
        "generic runtime should own raw WGPU upload byte views in one explicit helper"
    );
    assert!(
        encoding.contains("upload_value_as_bytes")
            && encoding.contains("upload_slice_as_bytes")
            && vertex.contains("upload_slice_as_bytes")
            && !encoding.contains("from_raw_parts")
            && !vertex.contains("from_raw_parts"),
        "renderer upload structs should delegate byte casting instead of duplicating pointer logic"
    );
}
