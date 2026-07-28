//! Backend-neutral system appearance selection.

use crate::runtime::{ResolvedEnvironment, WindowColorScheme};

use super::ThemeTokens;

/// Select whether a surface follows its native environment or uses fixed
/// caller-provided tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AppearancePolicy {
    /// Resolve light/dark and contrast from the current window snapshot.
    #[default]
    FollowEnvironment,
    /// Use these tokens byte-for-byte regardless of environment preferences.
    Fixed(ThemeTokens),
}

impl AppearancePolicy {
    /// Resolve one immutable appearance snapshot for a paint pass.
    pub fn resolve(self, environment: ResolvedEnvironment) -> ResolvedAppearance {
        ResolvedAppearance::resolve(self, environment)
    }

    /// Build a fixed-token policy.
    pub const fn fixed(theme: ThemeTokens) -> Self {
        Self::Fixed(theme)
    }

    /// Build a policy that follows the native window snapshot.
    pub const fn follow_environment() -> Self {
        Self::FollowEnvironment
    }
}

/// Immutable, copyable appearance selected for one frame or paint pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedAppearance {
    theme: ThemeTokens,
    color_scheme: Option<WindowColorScheme>,
    contrast: bool,
}

impl ResolvedAppearance {
    /// Resolve a policy against one lossless environment snapshot.
    pub fn resolve(policy: AppearancePolicy, environment: ResolvedEnvironment) -> Self {
        let (theme, color_scheme, contrast) = match policy {
            AppearancePolicy::Fixed(theme) => (theme, None, false),
            AppearancePolicy::FollowEnvironment => {
                let theme = match (environment.color_scheme(), environment.contrast()) {
                    (Some(WindowColorScheme::Light), false) => ThemeTokens::light(),
                    (Some(WindowColorScheme::Light), true) => ThemeTokens::light_high_contrast(),
                    (Some(WindowColorScheme::Dark), false) => ThemeTokens::dark(),
                    (Some(WindowColorScheme::Dark), true) => ThemeTokens::dark_high_contrast(),
                    (None, false) => ThemeTokens::dark(),
                    (None, true) => ThemeTokens::dark_high_contrast(),
                };
                (theme, environment.color_scheme(), environment.contrast())
            }
        };
        Self {
            theme,
            color_scheme,
            contrast,
        }
    }

    /// Build an explicit fixed-token appearance with no environment metadata.
    pub const fn fixed(theme: ThemeTokens) -> Self {
        Self {
            theme,
            color_scheme: None,
            contrast: false,
        }
    }

    /// Return the selected theme tokens by value.
    pub const fn tokens(self) -> ThemeTokens {
        self.theme
    }

    /// Return the selected theme tokens by value.
    pub const fn theme(self) -> ThemeTokens {
        self.theme
    }

    /// Return the selected system scheme, preserving `None` when unknown.
    pub const fn color_scheme(self) -> Option<WindowColorScheme> {
        self.color_scheme
    }

    /// Return whether the selected appearance uses high contrast.
    pub const fn contrast(self) -> bool {
        self.contrast
    }

    /// Alias for [`Self::contrast`].
    pub const fn high_contrast(self) -> bool {
        self.contrast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::DpiScale;

    fn environment(
        scheme: Option<WindowColorScheme>,
        contrast: bool,
        scale: DpiScale,
        reduced_motion: bool,
    ) -> ResolvedEnvironment {
        crate::runtime::WindowEnvironment::new(scale, scheme, contrast, reduced_motion).resolved()
    }

    #[test]
    fn follow_environment_covers_the_full_scheme_and_contrast_matrix() {
        let cases = [
            (Some(WindowColorScheme::Light), false, ThemeTokens::light()),
            (
                Some(WindowColorScheme::Light),
                true,
                ThemeTokens::light_high_contrast(),
            ),
            (Some(WindowColorScheme::Dark), false, ThemeTokens::dark()),
            (
                Some(WindowColorScheme::Dark),
                true,
                ThemeTokens::dark_high_contrast(),
            ),
            (None, false, ThemeTokens::dark()),
            (None, true, ThemeTokens::dark_high_contrast()),
        ];
        for (scheme, contrast, expected) in cases {
            let resolved = AppearancePolicy::FollowEnvironment.resolve(environment(
                scheme,
                contrast,
                DpiScale::ONE,
                false,
            ));
            assert_eq!(resolved.tokens(), expected);
            assert_eq!(resolved.color_scheme(), scheme);
            assert_eq!(resolved.contrast(), contrast);
        }
    }

    #[test]
    fn fixed_theme_is_invariant_to_every_environment_preference() {
        let fixed = ThemeTokens::light_high_contrast();
        for scheme in [
            None,
            Some(WindowColorScheme::Light),
            Some(WindowColorScheme::Dark),
        ] {
            let resolved = AppearancePolicy::fixed(fixed).resolve(environment(
                scheme,
                true,
                DpiScale::new(2.0),
                true,
            ));
            assert_eq!(resolved.tokens(), fixed);
            assert_eq!(resolved.color_scheme(), None);
            assert!(!resolved.contrast());
            assert!(!resolved.high_contrast());
        }
    }

    #[test]
    fn scale_and_motion_do_not_change_followed_appearance() {
        let a = AppearancePolicy::FollowEnvironment.resolve(environment(
            Some(WindowColorScheme::Light),
            false,
            DpiScale::ONE,
            false,
        ));
        let b = AppearancePolicy::FollowEnvironment.resolve(environment(
            Some(WindowColorScheme::Light),
            false,
            DpiScale::new(3.0),
            true,
        ));
        assert_eq!(a, b);
    }
}
