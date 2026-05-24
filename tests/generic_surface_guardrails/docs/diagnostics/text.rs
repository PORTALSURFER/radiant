use super::read_project_file;

#[test]
fn api_docs_describe_text_cache_frame_diagnostics() {
    let docs = read_project_file("docs/API.md");
    let runtime_diagnostics = read_project_file("src/runtime/diagnostics/text.rs");
    let native_diagnostics =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs");
    let render_profile =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/render_profile.rs");

    assert!(
        docs.contains("NativeFrameDiagnostics::text")
            && docs.contains("cache.layout")
            && docs.contains("cache.atom")
            && docs.contains("quality")
            && docs.contains("layout-cache and text atom-cache hits, misses, and evictions")
            && docs.contains("shaping-sensitive run/scalar counts")
            && docs.contains("fallback/missing glyph counts")
            && docs.contains("NativeTextDiagnostics::has_shaping_limits()")
            && docs.contains("has_font_coverage_gaps()")
            && docs.contains("has_text_quality_warnings()")
            && docs.contains("NativeTextDiagnostics::quality_status()")
            && docs.contains("NativeTextQualityStatus")
            && docs.contains("text_quality_status"),
        "API docs should describe native text cache diagnostics"
    );
    assert!(
        runtime_diagnostics.contains("pub struct NativeTextDiagnostics")
            && runtime_diagnostics.contains("pub struct NativeTextCacheDiagnostics")
            && runtime_diagnostics.contains("pub struct NativeTextCacheCounters")
            && runtime_diagnostics.contains("pub struct NativeTextQualityDiagnostics")
            && runtime_diagnostics.contains("pub enum NativeTextQualityStatus")
            && runtime_diagnostics.contains("pub cache: NativeTextCacheDiagnostics")
            && runtime_diagnostics.contains("pub quality: NativeTextQualityDiagnostics")
            && runtime_diagnostics.contains("pub layout: NativeTextCacheCounters")
            && runtime_diagnostics.contains("pub atom: NativeTextCacheCounters")
            && runtime_diagnostics.contains("unsupported_shaping_runs")
            && runtime_diagnostics.contains("unsupported_shaping_scalars")
            && runtime_diagnostics.contains("fallback_glyphs")
            && runtime_diagnostics.contains("missing_glyphs")
            && runtime_diagnostics.contains("pub const fn has_shaping_limits")
            && runtime_diagnostics.contains("pub const fn has_font_coverage_gaps")
            && runtime_diagnostics.contains("pub const fn has_text_quality_warnings")
            && runtime_diagnostics.contains("pub const fn quality_status"),
        "runtime text diagnostics should expose structured native text cache counters"
    );
    assert!(
        native_diagnostics.contains("text: crate::runtime::NativeTextDiagnostics")
            && native_diagnostics.contains("cache: crate::runtime::NativeTextCacheDiagnostics")
            && native_diagnostics.contains("layout: crate::runtime::NativeTextCacheCounters")
            && native_diagnostics.contains("hits: parts.text_stats.layout.hits")
            && native_diagnostics.contains("evictions: parts.text_stats.atom.evictions")
            && native_diagnostics.contains("quality: crate::runtime::NativeTextQualityDiagnostics")
            && native_diagnostics.contains(
                "unsupported_shaping_runs: parts.text_stats.quality.unsupported_shaping_runs"
            )
            && native_diagnostics.contains(
                "unsupported_shaping_scalars: parts.text_stats.quality.unsupported_shaping_scalars"
            )
            && native_diagnostics
                .contains("fallback_glyphs: parts.text_stats.quality.fallback_glyphs")
            && native_diagnostics
                .contains("missing_glyphs: parts.text_stats.quality.missing_glyphs")
            && native_diagnostics.contains("pub(super) struct NativeFrameDiagnosticsParts"),
        "native frame diagnostics should project text renderer cache counters"
    );
    assert!(
        render_profile.contains("text_unsupported_shaping_runs")
            && render_profile.contains("text_unsupported_shaping_scalars")
            && render_profile.contains("text_quality_status"),
        "native render profile should include shaping-limit text counters"
    );
}
