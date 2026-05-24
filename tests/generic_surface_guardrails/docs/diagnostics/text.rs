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
            && docs.contains("text layout-cache hits, misses,")
            && docs.contains("text atom-cache activity")
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
            && runtime_diagnostics.contains("pub enum NativeTextQualityStatus")
            && runtime_diagnostics.contains("layout_cache_hits")
            && runtime_diagnostics.contains("atom_cache_evictions")
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
            && native_diagnostics.contains("layout_cache_hits: parts.text_stats.layout_hits")
            && native_diagnostics.contains("atom_cache_evictions: parts.text_stats.atom_evictions")
            && native_diagnostics
                .contains("unsupported_shaping_runs: parts.text_stats.unsupported_shaping_runs")
            && native_diagnostics.contains(
                "unsupported_shaping_scalars: parts.text_stats.unsupported_shaping_scalars"
            )
            && native_diagnostics.contains("fallback_glyphs: parts.text_stats.fallback_glyphs")
            && native_diagnostics.contains("missing_glyphs: parts.text_stats.missing_glyphs")
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
