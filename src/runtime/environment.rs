use super::command::{RepaintScope, SurfaceInvalidation};
use crate::theme::DpiScale;

/// The system color scheme resolved for one native presentation surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowColorScheme {
    /// A light system appearance.
    Light,
    /// A dark system appearance.
    Dark,
}

/// Immutable, backend-neutral native environment for one window.
///
/// The snapshot is deliberately small and copyable. `display_scale` is the
/// effective [`DpiScale`] used by Radiant after any host override; an unknown
/// native color scheme remains `None`. Contrast and reduced motion default to
/// `false` when the platform does not report an accessibility preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowEnvironment {
    display_scale: DpiScale,
    color_scheme: Option<WindowColorScheme>,
    contrast: bool,
    reduced_motion: bool,
}

impl WindowEnvironment {
    /// Construct a window environment from backend-neutral values.
    pub const fn new(
        display_scale: DpiScale,
        color_scheme: Option<WindowColorScheme>,
        contrast: bool,
        reduced_motion: bool,
    ) -> Self {
        Self {
            display_scale,
            color_scheme,
            contrast,
            reduced_motion,
        }
    }

    /// Return the effective display scale used by the runtime.
    pub const fn display_scale(self) -> DpiScale {
        self.display_scale
    }

    /// Alias for [`Self::display_scale`] for callers that use scale wording.
    pub const fn scale(self) -> DpiScale {
        self.display_scale
    }

    /// Return the resolved system color scheme, when known.
    pub const fn color_scheme(self) -> Option<WindowColorScheme> {
        self.color_scheme
    }

    /// Return whether the window is using a higher-contrast accessibility mode.
    pub const fn contrast(self) -> bool {
        self.contrast
    }

    /// Alias for [`Self::contrast`].
    pub const fn high_contrast(self) -> bool {
        self.contrast
    }

    /// Return whether nonessential motion should be reduced or disabled.
    pub const fn reduced_motion(self) -> bool {
        self.reduced_motion
    }
}

impl Default for WindowEnvironment {
    fn default() -> Self {
        Self::new(DpiScale::ONE, None, false, false)
    }
}

/// A change to the native environment observed by one window.
///
/// The enum is intentionally closed so that every environment change has an
/// explicit invalidation policy. Platform adapters can translate their native
/// notifications into these semantic changes without exposing platform types
/// to the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowEnvironmentChange {
    /// Display scale or monitor placement/color characteristics changed.
    DisplayScaleOrMonitor,
    /// Locale, writing direction, text scale, or font fallback changed.
    TextMetrics,
    /// System color scheme or contrast preference changed.
    ColorSchemeOrContrast,
    /// Reduced-motion preference or another motion policy changed.
    MotionPreference,
    /// Pointer or keyboard conventions changed.
    InputConventions,
}

impl WindowEnvironmentChange {
    /// Select the conservative repaint stage for this environment change.
    pub const fn repaint_scope(self) -> RepaintScope {
        match self {
            // The existing surface scope is the safest stand-in until the
            // runtime has dedicated target and paint-resource invalidation.
            Self::DisplayScaleOrMonitor => RepaintScope::Surface,
            Self::TextMetrics => RepaintScope::Layout,
            Self::ColorSchemeOrContrast | Self::MotionPreference | Self::InputConventions => {
                RepaintScope::Projection
            }
        }
    }

    /// Convert this change to the corresponding typed invalidation stage.
    pub const fn surface_invalidation(self) -> SurfaceInvalidation {
        SurfaceInvalidation::from_repaint_scope(Some(self.repaint_scope()))
    }

    /// Combine changes into the strongest required repaint scope.
    ///
    /// An empty iterator returns `None`, matching the command boundary's
    /// representation of no repaint request. The result is independent of
    /// iteration order because [`RepaintScope::merge`] is commutative.
    pub fn repaint_scope_for<I>(changes: I) -> Option<RepaintScope>
    where
        I: IntoIterator<Item = Self>,
    {
        changes
            .into_iter()
            .map(Self::repaint_scope)
            .reduce(RepaintScope::merge)
    }

    /// Combine changes into the strongest required typed invalidation stage.
    pub fn surface_invalidation_for<I>(changes: I) -> SurfaceInvalidation
    where
        I: IntoIterator<Item = Self>,
    {
        SurfaceInvalidation::from_repaint_scope(Self::repaint_scope_for(changes))
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowColorScheme, WindowEnvironment, WindowEnvironmentChange};
    use crate::{
        runtime::{RepaintScope, SurfaceInvalidation},
        theme::DpiScale,
    };

    #[test]
    fn window_environment_defaults_are_backend_neutral() {
        let environment = WindowEnvironment::default();

        assert_eq!(environment.display_scale(), DpiScale::ONE);
        assert_eq!(environment.color_scheme(), None);
        assert!(!environment.contrast());
        assert!(!environment.reduced_motion());
    }

    #[test]
    fn window_environment_preserves_values_and_equality() {
        let environment = WindowEnvironment::new(
            DpiScale::new(2.0),
            Some(WindowColorScheme::Dark),
            true,
            true,
        );

        assert_eq!(environment.display_scale().factor(), 2.0);
        assert_eq!(environment.color_scheme(), Some(WindowColorScheme::Dark));
        assert!(environment.high_contrast());
        assert!(environment.reduced_motion());
        assert_eq!(environment, environment);
    }

    const MAPPINGS: &[(WindowEnvironmentChange, RepaintScope, SurfaceInvalidation)] = &[
        (
            WindowEnvironmentChange::DisplayScaleOrMonitor,
            RepaintScope::Surface,
            SurfaceInvalidation::Surface,
        ),
        (
            WindowEnvironmentChange::TextMetrics,
            RepaintScope::Layout,
            SurfaceInvalidation::Layout,
        ),
        (
            WindowEnvironmentChange::ColorSchemeOrContrast,
            RepaintScope::Projection,
            SurfaceInvalidation::Projection,
        ),
        (
            WindowEnvironmentChange::MotionPreference,
            RepaintScope::Projection,
            SurfaceInvalidation::Projection,
        ),
        (
            WindowEnvironmentChange::InputConventions,
            RepaintScope::Projection,
            SurfaceInvalidation::Projection,
        ),
    ];

    #[test]
    fn every_environment_change_maps_to_its_typed_scope() {
        for &(change, repaint_scope, invalidation) in MAPPINGS {
            assert_eq!(change.repaint_scope(), repaint_scope);
            assert_eq!(change.surface_invalidation(), invalidation);
        }
    }

    #[test]
    fn combining_no_changes_requests_no_invalidation() {
        let changes: [WindowEnvironmentChange; 0] = [];

        assert_eq!(WindowEnvironmentChange::repaint_scope_for(changes), None);
        assert_eq!(
            WindowEnvironmentChange::surface_invalidation_for(changes),
            SurfaceInvalidation::None
        );
    }

    #[test]
    fn combining_changes_is_order_independent_and_prefers_strongest_scope() {
        use WindowEnvironmentChange::{
            ColorSchemeOrContrast, DisplayScaleOrMonitor, InputConventions, MotionPreference,
            TextMetrics,
        };

        let permutations = [
            [ColorSchemeOrContrast, TextMetrics, DisplayScaleOrMonitor],
            [DisplayScaleOrMonitor, ColorSchemeOrContrast, TextMetrics],
            [TextMetrics, DisplayScaleOrMonitor, ColorSchemeOrContrast],
            [InputConventions, MotionPreference, ColorSchemeOrContrast],
            [MotionPreference, ColorSchemeOrContrast, InputConventions],
        ];

        for changes in permutations[..3].iter().copied() {
            assert_eq!(
                WindowEnvironmentChange::repaint_scope_for(changes),
                Some(RepaintScope::Surface)
            );
        }

        for changes in permutations[3..].iter().copied() {
            assert_eq!(
                WindowEnvironmentChange::repaint_scope_for(changes),
                Some(RepaintScope::Projection)
            );
        }

        let layout_mixed = [
            [TextMetrics, ColorSchemeOrContrast],
            [ColorSchemeOrContrast, TextMetrics],
        ];
        for changes in layout_mixed {
            assert_eq!(
                WindowEnvironmentChange::repaint_scope_for(changes),
                Some(RepaintScope::Layout)
            );
            assert_eq!(
                WindowEnvironmentChange::surface_invalidation_for(changes),
                SurfaceInvalidation::Layout
            );
        }

        let projection_only = [InputConventions, ColorSchemeOrContrast, MotionPreference];
        assert_eq!(
            WindowEnvironmentChange::surface_invalidation_for(projection_only),
            SurfaceInvalidation::Projection
        );
    }
}
