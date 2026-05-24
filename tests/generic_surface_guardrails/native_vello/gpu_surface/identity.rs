use std::{fs, path::PathBuf};

#[test]
fn gpu_signal_surface_cache_keys_stay_in_focused_identity_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let signal = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal.rs",
    ))
    .expect("GPU signal type module should be readable");
    let cache_key = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal/cache_key.rs",
    ))
    .expect("GPU signal cache-key module should be readable");

    assert!(
        signal.contains("mod cache_key;")
            && signal.contains("SignalBodyTexture")
            && signal.contains("signal_body_matches_key(")
            && !signal.contains("struct SignalGainPreviewKey")
            && !signal.contains("const GPU_SIGNAL_STYLE_REVISION"),
        "GPU signal resource DTOs should delegate cache identity details"
    );
    assert!(
        cache_key.contains("struct SignalBufferCacheKey")
            && cache_key.contains("struct SignalBodyCacheKey")
            && cache_key.contains("struct SignalGainPreviewKey")
            && cache_key.contains("const GPU_SIGNAL_STYLE_REVISION")
            && cache_key.contains("fn signal_body_matches_key"),
        "GPU signal cache-key identity rules should live in signal/cache_key.rs"
    );
}
