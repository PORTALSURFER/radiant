/// Native text layout cache diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTextDiagnostics {
    /// Text layout and atom-cache activity observed while preparing this frame.
    pub cache: NativeTextCacheDiagnostics,
    /// Shaping and font coverage quality counters observed while preparing this frame.
    pub quality: NativeTextQualityDiagnostics,
}

/// Text cache diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTextCacheDiagnostics {
    /// Text-layout cache counters.
    pub layout: NativeTextCacheCounters,
    /// Interned text atom cache counters.
    pub atom: NativeTextCacheCounters,
}

/// Hit, miss, and eviction counters for one native text cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTextCacheCounters {
    /// Cache hits observed while preparing this frame.
    pub hits: u64,
    /// Cache misses observed while preparing this frame.
    pub misses: u64,
    /// Cache evictions observed while preparing this frame.
    pub evictions: u64,
}

/// Text shaping and font coverage quality counters for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTextQualityDiagnostics {
    /// Text runs that contain shaping-sensitive Unicode handled by the basic native layout path.
    pub unsupported_shaping_runs: u64,
    /// Rendered Unicode scalar values in runs that need a real shaping engine.
    pub unsupported_shaping_scalars: u64,
    /// Glyphs substituted with the renderer's fallback glyph this frame.
    pub fallback_glyphs: u64,
    /// Glyphs the active native font could not resolve even through fallback.
    pub missing_glyphs: u64,
}

impl NativeTextDiagnostics {
    /// Return whether this frame encountered text that needs more than the
    /// native renderer's current basic glyph mapping path.
    pub const fn has_shaping_limits(self) -> bool {
        self.quality.has_shaping_limits()
    }

    /// Return whether this frame substituted or missed glyphs with the active
    /// native font configuration.
    pub const fn has_font_coverage_gaps(self) -> bool {
        self.quality.has_font_coverage_gaps()
    }

    /// Return whether this frame exposed visible text-quality risk through
    /// shaping limits or font coverage gaps.
    pub const fn has_text_quality_warnings(self) -> bool {
        self.has_shaping_limits() || self.has_font_coverage_gaps()
    }

    /// Return the highest-level text quality status exposed by this frame.
    ///
    /// This keeps host overlays and telemetry from duplicating raw counter
    /// policy while Radiant's native text path still reports basic-layout
    /// shaping limits separately from active-font coverage gaps.
    pub const fn quality_status(self) -> NativeTextQualityStatus {
        match (self.has_shaping_limits(), self.has_font_coverage_gaps()) {
            (false, false) => NativeTextQualityStatus::Clean,
            (true, false) => NativeTextQualityStatus::ShapingLimited,
            (false, true) => NativeTextQualityStatus::FontCoverageLimited,
            (true, true) => NativeTextQualityStatus::ShapingAndFontCoverageLimited,
        }
    }
}

impl NativeTextQualityDiagnostics {
    /// Return whether this frame encountered text that needs more than the
    /// native renderer's current basic glyph mapping path.
    pub const fn has_shaping_limits(self) -> bool {
        self.unsupported_shaping_runs > 0 || self.unsupported_shaping_scalars > 0
    }

    /// Return whether this frame substituted or missed glyphs with the active
    /// native font configuration.
    pub const fn has_font_coverage_gaps(self) -> bool {
        self.fallback_glyphs > 0 || self.missing_glyphs > 0
    }
}

/// Text quality status for a native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeTextQualityStatus {
    /// No shaping-limit or font-coverage warning was observed.
    #[default]
    Clean,
    /// One or more text runs needed shaping beyond the current basic layout path.
    ShapingLimited,
    /// One or more glyphs needed fallback or remained missing in the active font.
    FontCoverageLimited,
    /// Both shaping limits and font coverage gaps were observed.
    ShapingAndFontCoverageLimited,
}
