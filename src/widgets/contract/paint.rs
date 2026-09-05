//! Widget paint-boundary vocabulary shared by primitives and runtimes.

use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::{ResolvedAppearance, ThemeTokens},
};

/// Shared layout, theme, bounds, output, and window-environment inputs for one
/// widget paint traversal.
///
/// The context borrows the caller-owned primitive buffer and existing layout
/// and theme values. It borrows one resolved environment projection and does
/// not allocate per widget.
pub struct WidgetPaintContext<'a> {
    primitives: &'a mut Vec<PaintPrimitive>,
    bounds: Rect,
    layout: &'a LayoutOutput,
    theme: &'a ThemeTokens,
    environment: &'a ResolvedEnvironment,
    appearance: ResolvedAppearance,
}

impl<'a> WidgetPaintContext<'a> {
    /// Build a paint context for one assigned widget rectangle and output buffer.
    pub fn new(
        primitives: &'a mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &'a LayoutOutput,
        theme: &'a ThemeTokens,
        environment: &'a ResolvedEnvironment,
    ) -> Self {
        Self {
            primitives,
            bounds,
            layout,
            theme,
            environment,
            appearance: ResolvedAppearance::fixed(*theme),
        }
    }

    /// Build a paint context with one appearance resolved for the whole pass.
    pub fn new_with_appearance(
        primitives: &'a mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &'a LayoutOutput,
        theme: &'a ThemeTokens,
        environment: &'a ResolvedEnvironment,
        appearance: ResolvedAppearance,
    ) -> Self {
        Self {
            primitives,
            bounds,
            layout,
            theme,
            environment,
            appearance,
        }
    }

    /// Return the caller-owned primitive output buffer.
    pub fn primitives(&mut self) -> &mut Vec<PaintPrimitive> {
        self.primitives
    }

    /// Alias for [`Self::primitives`].
    pub fn primitives_mut(&mut self) -> &mut Vec<PaintPrimitive> {
        self.primitives()
    }

    /// Return the widget's assigned logical bounds.
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Return the resolved layout metadata for this paint pass.
    pub const fn layout(&self) -> &'a LayoutOutput {
        self.layout
    }

    /// Return the active theme tokens for this paint pass.
    pub const fn theme(&self) -> &'a ThemeTokens {
        self.theme
    }

    /// Return the immutable appearance selected for this paint pass.
    pub const fn appearance(&self) -> ResolvedAppearance {
        self.appearance
    }

    /// Return the environment resolved from the current surface snapshot.
    pub const fn environment(&self) -> &'a ResolvedEnvironment {
        self.environment
    }

    /// Alias for [`Self::environment`].
    pub const fn resolved_environment(&self) -> &'a ResolvedEnvironment {
        self.environment
    }

    /// Return the resolved display scale for this paint pass.
    pub const fn scale(&self) -> crate::theme::DpiScale {
        self.environment.scale()
    }

    /// Alias for [`Self::scale`].
    pub const fn display_scale(&self) -> crate::theme::DpiScale {
        self.environment.display_scale()
    }

    /// Return the resolved color scheme, when known.
    pub const fn color_scheme(&self) -> Option<crate::runtime::WindowColorScheme> {
        self.environment.color_scheme()
    }

    /// Return whether higher contrast is enabled.
    pub const fn contrast(&self) -> bool {
        self.environment.contrast()
    }

    /// Return whether nonessential motion should be reduced.
    pub const fn reduced_motion(&self) -> bool {
        self.environment.reduced_motion()
    }
}

/// Shared paint clipping contract for widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintBounds {
    /// Runtime traversal clips widget paint to the assigned widget rectangle.
    ClipToRect,
    /// Paint may extend beyond the assigned rectangle when the parent allows it.
    AllowOverflow,
}

/// Shared paint responsibilities required from every widget primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaintContract {
    /// Whether paint is clipped to the assigned widget rectangle.
    pub bounds: PaintBounds,
    /// Whether focus state should be expressed visually by the widget itself.
    pub paints_focus: bool,
    /// Whether selection/active state should be expressed visually by the widget.
    pub paints_state_layers: bool,
    /// Whether this widget's own chrome should block hover chrome on parent containers.
    pub suppresses_container_hover: bool,
}

impl Default for PaintContract {
    fn default() -> Self {
        Self {
            bounds: PaintBounds::ClipToRect,
            paints_focus: true,
            paints_state_layers: true,
            suppresses_container_hover: false,
        }
    }
}
