//! Persistent layout-core runtime caches used by native shell layout projection.
//!
//! This module keeps `LayoutEngine` instances alive across redraws so
//! measure/virtualization caches can be reused and subtree invalidation APIs can
//! target specific layout trees on hot interaction paths.

use super::layout_adapter::{
    BrowserBandSections, SHELL_ROOT_ID, ShellSectionRects, SidebarBandSections,
    build_browser_bands_tree, build_shell_sections_tree, build_sidebar_bands_tree,
    compute_browser_band_sections_with_layout_engine, compute_shell_sections_with_layout_engine,
    compute_sidebar_band_sections_with_layout_engine,
};
use super::style::{SizingTokens, StyleTokens};
use crate::gui::layout_core::{LayoutEngine, LayoutNode, LayoutState};
use crate::gui::types::{Rect, Vector2};

/// Native-shell layout tree families with independent persistent caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellLayoutTreeKind {
    /// Top-level shell sections (`top_bar`, `sidebar`, `content`, `status`).
    ShellSections,
    /// Browser panel tabs/toolbar/header/rows/footer bands.
    BrowserBands,
    /// Sidebar header/rows/footer bands.
    SidebarBands,
}

/// Dirty mark class applied to a cached shell layout tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellLayoutDirtyKind {
    /// Geometry-only dirty marker.
    #[allow(dead_code)]
    Layout,
    /// Intrinsic measure dirty marker.
    Measure,
}

#[derive(Default)]
struct TreeRuntime {
    engine: LayoutEngine,
    state: LayoutState,
    last_tree: Option<LayoutNode>,
}

impl TreeRuntime {
    fn remember_tree(&mut self, tree: LayoutNode) {
        self.last_tree = Some(tree);
    }

    fn mark_subtree_dirty(&mut self, node_id: u64, kind: ShellLayoutDirtyKind) {
        if let Some(tree) = self.last_tree.as_ref() {
            match kind {
                ShellLayoutDirtyKind::Layout => {
                    self.engine.mark_layout_dirty_subtree(tree, node_id)
                }
                ShellLayoutDirtyKind::Measure => {
                    self.engine.mark_measure_dirty_subtree(tree, node_id)
                }
            }
            return;
        }
        match kind {
            ShellLayoutDirtyKind::Layout => self.engine.mark_layout_dirty(node_id),
            ShellLayoutDirtyKind::Measure => self.engine.mark_measure_dirty(node_id),
        }
    }

    fn reset(&mut self) {
        self.engine = LayoutEngine::default();
        self.state = LayoutState::default();
        self.last_tree = None;
    }
}

/// Persistent runtime state for native shell layout-core projections.
#[derive(Default)]
pub(crate) struct ShellLayoutRuntime {
    shell_sections: TreeRuntime,
    browser_bands: TreeRuntime,
    sidebar_bands: TreeRuntime,
}

impl ShellLayoutRuntime {
    /// Resolve top-level shell sections with persistent engine/state caches.
    pub(crate) fn compute_shell_sections(
        &mut self,
        viewport: Vector2,
        style: &StyleTokens,
    ) -> ShellSectionRects {
        let viewport_width = viewport.x.max(style.sizing.min_viewport_width);
        self.shell_sections
            .remember_tree(build_shell_sections_tree(style, viewport_width));
        compute_shell_sections_with_layout_engine(
            viewport,
            style,
            &mut self.shell_sections.engine,
            &self.shell_sections.state,
        )
    }

    /// Resolve browser panel bands with persistent engine/state caches.
    pub(crate) fn compute_browser_band_sections(
        &mut self,
        browser_panel: Rect,
        sizing: SizingTokens,
    ) -> BrowserBandSections {
        self.browser_bands
            .remember_tree(build_browser_bands_tree(browser_panel, sizing));
        compute_browser_band_sections_with_layout_engine(
            browser_panel,
            sizing,
            &mut self.browser_bands.engine,
            &self.browser_bands.state,
        )
    }

    /// Resolve sidebar bands with persistent engine/state caches.
    pub(crate) fn compute_sidebar_band_sections(
        &mut self,
        sidebar: Rect,
        sizing: SizingTokens,
    ) -> SidebarBandSections {
        self.sidebar_bands
            .remember_tree(build_sidebar_bands_tree(sidebar, sizing));
        compute_sidebar_band_sections_with_layout_engine(
            sidebar,
            sizing,
            &mut self.sidebar_bands.engine,
            &self.sidebar_bands.state,
        )
    }

    /// Mark a cached layout tree subtree dirty using layout-core subtree APIs.
    pub(crate) fn mark_subtree_dirty(
        &mut self,
        tree_kind: ShellLayoutTreeKind,
        node_id: u64,
        dirty_kind: ShellLayoutDirtyKind,
    ) {
        match tree_kind {
            ShellLayoutTreeKind::ShellSections => {
                self.shell_sections.mark_subtree_dirty(node_id, dirty_kind);
            }
            ShellLayoutTreeKind::BrowserBands => {
                self.browser_bands.mark_subtree_dirty(node_id, dirty_kind);
            }
            ShellLayoutTreeKind::SidebarBands => {
                self.sidebar_bands.mark_subtree_dirty(node_id, dirty_kind);
            }
        }
    }

    /// Mark all tracked trees dirty with the provided dirty class.
    pub(crate) fn mark_all_dirty(&mut self, dirty_kind: ShellLayoutDirtyKind) {
        self.mark_subtree_dirty(
            ShellLayoutTreeKind::ShellSections,
            SHELL_ROOT_ID,
            dirty_kind,
        );
        self.mark_subtree_dirty(
            ShellLayoutTreeKind::BrowserBands,
            super::layout_adapter::BROWSER_BANDS_ROOT_ID,
            dirty_kind,
        );
        self.mark_subtree_dirty(
            ShellLayoutTreeKind::SidebarBands,
            super::layout_adapter::SIDEBAR_BANDS_ROOT_ID,
            dirty_kind,
        );
    }

    /// Drop all cached tree state, for example on viewport scale changes.
    pub(crate) fn reset(&mut self) {
        self.shell_sections.reset();
        self.browser_bands.reset();
        self.sidebar_bands.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellLayoutDirtyKind, ShellLayoutRuntime};
    use crate::gui::native_shell::{ShellLayout, StyleTokens};
    use crate::gui::types::Vector2;

    #[test]
    fn runtime_layout_build_is_deterministic_across_layout_dirty_marks() {
        let viewport = Vector2::new(1280.0, 720.0);
        let style = StyleTokens::for_viewport_width(viewport.x);
        let mut runtime = ShellLayoutRuntime::default();

        let first = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
        runtime.mark_all_dirty(ShellLayoutDirtyKind::Layout);
        let second = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);

        assert_eq!(first, second);
    }

    #[test]
    fn runtime_layout_build_is_stable_after_measure_dirty_and_reset() {
        let viewport = Vector2::new(1440.0, 810.0);
        let style = StyleTokens::for_viewport_width(viewport.x);
        let mut runtime = ShellLayoutRuntime::default();

        let first = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
        runtime.mark_all_dirty(ShellLayoutDirtyKind::Measure);
        let second = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
        runtime.reset();
        let third = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);

        assert_eq!(first, second);
        assert_eq!(second, third);
    }
}
