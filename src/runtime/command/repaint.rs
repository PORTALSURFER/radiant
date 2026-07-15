/// Scope of a runtime repaint request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RepaintScope {
    /// Repaint after refreshing the declarative surface projection and layout.
    Surface,
    /// Reproject the declarative surface and recompute layout while preserving
    /// stable widget identities where possible.
    Layout,
    /// Reproject the declarative surface while reusing geometry guarded by an
    /// unchanged structural/layout revision.
    Projection,
    /// Repaint the current paint plan without refreshing declarative state.
    PaintOnly,
}

impl RepaintScope {
    /// Merge two repaint scopes, preferring the scope that preserves correctness.
    ///
    /// Broader stages include narrower work, so any mixed batch selects the
    /// broadest correctness-preserving stage.
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Surface, _) | (_, Self::Surface) => Self::Surface,
            (Self::Layout, _) | (_, Self::Layout) => Self::Layout,
            (Self::Projection, _) | (_, Self::Projection) => Self::Projection,
            (Self::PaintOnly, Self::PaintOnly) => Self::PaintOnly,
        }
    }

    /// Return whether this repaint can reuse the current declarative surface.
    pub const fn is_paint_only(self) -> bool {
        matches!(self, Self::PaintOnly)
    }

    /// Return whether this scope must pull and project a fresh surface.
    pub const fn refreshes_projection(self) -> bool {
        !matches!(self, Self::PaintOnly)
    }

    /// Return whether this scope must recompute layout geometry.
    pub const fn refreshes_layout(self) -> bool {
        matches!(self, Self::Surface | Self::Layout)
    }
}

/// Stable host-owned revisions used to select a correctness-preserving repaint stage.
///
/// `structure` covers widget/container identity and layout-node topology. `layout`
/// covers geometry-affecting values within that topology. `projection` covers
/// paint/runtime projection values that can change while both invariants remain
/// stable. Hosts must bump the broadest affected revision; unknown changes should
/// continue to request [`RepaintScope::Surface`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SurfaceRevisions {
    /// Revision for widget/container identity and layout topology.
    pub structure: u64,
    /// Revision for geometry-affecting projected values.
    pub layout: u64,
    /// Revision for projected values that preserve structure and geometry.
    pub projection: u64,
}

impl SurfaceRevisions {
    /// Build a revision snapshot from explicit structural, layout, and projection keys.
    pub const fn new(structure: u64, layout: u64, projection: u64) -> Self {
        Self {
            structure,
            layout,
            projection,
        }
    }

    /// Resolve the narrowest safe repaint scope since `previous`.
    ///
    /// An unchanged snapshot still selects paint-only because this method is used
    /// after a repaint-producing frame message. The absence of a repaint request is
    /// represented by `None` at the command/invalidation boundary.
    pub const fn repaint_scope_since(self, previous: Self) -> RepaintScope {
        if self.structure != previous.structure {
            RepaintScope::Surface
        } else if self.layout != previous.layout {
            RepaintScope::Layout
        } else if self.projection != previous.projection {
            RepaintScope::Projection
        } else {
            RepaintScope::PaintOnly
        }
    }
}

/// Typed invalidation stage selected for one runtime presentation pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SurfaceInvalidation {
    /// No repaint or refresh work is required.
    #[default]
    None,
    /// Repaint the current projected surface.
    PaintOnly,
    /// Pull and project a fresh surface while reusing proven-stable layout.
    Projection,
    /// Pull and project a fresh surface and recompute layout.
    Layout,
    /// Use the correctness-first complete surface refresh fallback.
    Surface,
}

impl SurfaceInvalidation {
    /// Convert an optional repaint request into a typed invalidation stage.
    pub const fn from_repaint_scope(scope: Option<RepaintScope>) -> Self {
        match scope {
            None => Self::None,
            Some(RepaintScope::PaintOnly) => Self::PaintOnly,
            Some(RepaintScope::Projection) => Self::Projection,
            Some(RepaintScope::Layout) => Self::Layout,
            Some(RepaintScope::Surface) => Self::Surface,
        }
    }

    /// Return the corresponding repaint scope, if any.
    pub const fn repaint_scope(self) -> Option<RepaintScope> {
        match self {
            Self::None => None,
            Self::PaintOnly => Some(RepaintScope::PaintOnly),
            Self::Projection => Some(RepaintScope::Projection),
            Self::Layout => Some(RepaintScope::Layout),
            Self::Surface => Some(RepaintScope::Surface),
        }
    }

    /// Return a stable diagnostic label.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PaintOnly => "paint_only",
            Self::Projection => "projection",
            Self::Layout => "layout",
            Self::Surface => "surface",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RepaintScope, SurfaceInvalidation, SurfaceRevisions};

    #[test]
    fn revisions_choose_the_narrowest_safe_scope() {
        let base = SurfaceRevisions::new(1, 2, 3);

        assert_eq!(base.repaint_scope_since(base), RepaintScope::PaintOnly);
        assert_eq!(
            SurfaceRevisions::new(1, 2, 4).repaint_scope_since(base),
            RepaintScope::Projection
        );
        assert_eq!(
            SurfaceRevisions::new(1, 3, 4).repaint_scope_since(base),
            RepaintScope::Layout
        );
        assert_eq!(
            SurfaceRevisions::new(2, 3, 4).repaint_scope_since(base),
            RepaintScope::Surface
        );
    }

    #[test]
    fn invalidation_round_trips_optional_repaint_scope() {
        for scope in [
            None,
            Some(RepaintScope::PaintOnly),
            Some(RepaintScope::Projection),
            Some(RepaintScope::Layout),
            Some(RepaintScope::Surface),
        ] {
            assert_eq!(
                SurfaceInvalidation::from_repaint_scope(scope).repaint_scope(),
                scope
            );
        }
    }
}
