//! Application-owned presentation environment.

use super::localization::{LocaleId, LocalizedText, TextCatalog, TextKey};
use crate::gui::shortcuts::ShortcutPlatform;
use crate::runtime::{RepaintScope, SurfaceInvalidation};
use std::sync::Arc;

/// Logical writing direction for layout and text presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WritingDirection {
    /// Left-to-right writing order.
    #[default]
    Ltr,
    /// Right-to-left writing order.
    Rtl,
}

/// A change to application-owned presentation inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ApplicationEnvironmentChange {
    /// Locale or catalog content changed, including visible labels.
    LocaleOrCatalog,
    /// Logical writing direction or text scale changed.
    DirectionOrTextScale,
    /// Platform shortcut display conventions changed.
    ShortcutPresentation,
}

impl ApplicationEnvironmentChange {
    /// Return the narrowest safe repaint scope for this change.
    pub const fn repaint_scope(self) -> RepaintScope {
        match self {
            Self::LocaleOrCatalog => RepaintScope::Surface,
            Self::DirectionOrTextScale => RepaintScope::Layout,
            Self::ShortcutPresentation => RepaintScope::Projection,
        }
    }

    /// Return the typed invalidation stage for this change.
    pub const fn surface_invalidation(self) -> SurfaceInvalidation {
        SurfaceInvalidation::from_repaint_scope(Some(self.repaint_scope()))
    }
}

/// Validated application text scale.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TextScale(f32);

impl TextScale {
    /// Smallest accepted text scale.
    pub const MIN: f32 = 0.5;
    /// Largest accepted text scale.
    pub const MAX: f32 = 4.0;

    /// Validate and construct a text scale.
    pub fn new(value: f32) -> Result<Self, TextScaleError> {
        if !value.is_finite() {
            return Err(TextScaleError::NotFinite);
        }
        if !(Self::MIN..=Self::MAX).contains(&value) || value <= 0.0 {
            return Err(TextScaleError::OutOfBounds { value });
        }
        Ok(Self(value))
    }

    /// Return the validated scale factor.
    pub const fn factor(self) -> f32 {
        self.0
    }
}

impl Default for TextScale {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Error returned when a text scale cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextScaleError {
    /// The input was NaN or infinite.
    NotFinite,
    /// The input was outside the bounded scale range.
    OutOfBounds {
        /// Rejected scale value.
        value: f32,
    },
}

impl std::fmt::Display for TextScaleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("text scale must be finite"),
            Self::OutOfBounds { value } => write!(
                formatter,
                "text scale {value} is outside [{}, {}]",
                TextScale::MIN,
                TextScale::MAX
            ),
        }
    }
}

impl std::error::Error for TextScaleError {}

/// Immutable application-level locale, direction, scale, and presentation
/// snapshot. The catalog is shared by value and never read from ambient host
/// state.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplicationEnvironment {
    fallback_chain: Vec<LocaleId>,
    writing_direction: WritingDirection,
    text_scale: TextScale,
    catalog: Arc<TextCatalog>,
    shortcut_platform: ShortcutPlatform,
    presentation_generation: u64,
}

impl Default for ApplicationEnvironment {
    fn default() -> Self {
        Self::new(LocaleId::english())
    }
}

impl ApplicationEnvironment {
    /// Construct a snapshot with one requested locale and the default catalog.
    pub fn new(locale: LocaleId) -> Self {
        Self {
            fallback_chain: vec![locale],
            writing_direction: WritingDirection::Ltr,
            text_scale: TextScale::default(),
            catalog: Arc::new(TextCatalog::default()),
            shortcut_platform: ShortcutPlatform::default(),
            presentation_generation: 0,
        }
    }

    /// Replace the complete ordered locale fallback chain.
    pub fn with_fallback_chain(
        mut self,
        fallback_chain: impl IntoIterator<Item = LocaleId>,
    ) -> Self {
        self.fallback_chain.clear();
        for locale in fallback_chain {
            if !self.fallback_chain.contains(&locale) {
                self.fallback_chain.push(locale);
            }
        }
        self
    }

    /// Set writing direction.
    pub const fn with_writing_direction(mut self, direction: WritingDirection) -> Self {
        self.writing_direction = direction;
        self
    }

    /// Set a validated text scale.
    pub const fn with_text_scale(mut self, scale: TextScale) -> Self {
        self.text_scale = scale;
        self
    }

    /// Set the immutable text catalog.
    pub fn with_catalog(mut self, catalog: Arc<TextCatalog>) -> Self {
        self.catalog = catalog;
        self
    }

    /// Set the platform family used by shortcut display.
    pub const fn with_shortcut_platform(mut self, platform: ShortcutPlatform) -> Self {
        self.shortcut_platform = platform;
        self
    }

    /// Set the generation used to invalidate presentation caches.
    pub const fn with_presentation_generation(mut self, generation: u64) -> Self {
        self.presentation_generation = generation;
        self
    }

    /// Return the ordered requested locale and explicit fallbacks.
    pub fn fallback_chain(&self) -> &[LocaleId] {
        &self.fallback_chain
    }

    /// Return the effective writing direction.
    pub const fn writing_direction(&self) -> WritingDirection {
        self.writing_direction
    }

    /// Return the validated text scale.
    pub const fn text_scale(&self) -> TextScale {
        self.text_scale
    }

    /// Return the immutable catalog generation.
    pub fn catalog_generation(&self) -> u64 {
        self.catalog.generation()
    }

    /// Return the shortcut platform family.
    pub const fn shortcut_platform(&self) -> ShortcutPlatform {
        self.shortcut_platform
    }

    /// Return the presentation generation.
    pub const fn presentation_generation(&self) -> u64 {
        self.presentation_generation
    }

    /// Resolve one text key using only the explicit snapshot chain.
    pub fn localized(&self, key: &TextKey) -> LocalizedText {
        self.catalog.resolve(key, &self.fallback_chain)
    }

    /// Return the strongest invalidation required when replacing `previous`.
    ///
    /// The comparison is value based and therefore remains valid when two
    /// snapshots happen to share their catalog allocation.
    pub fn repaint_scope_since(&self, previous: &Self) -> Option<RepaintScope> {
        let mut scope = None;
        if self.fallback_chain != previous.fallback_chain
            || (!Arc::ptr_eq(&self.catalog, &previous.catalog)
                && self.catalog.as_ref() != previous.catalog.as_ref())
        {
            scope = Some(RepaintScope::Surface);
        }
        if self.writing_direction != previous.writing_direction
            || self.text_scale != previous.text_scale
        {
            scope = Some(scope.map_or(RepaintScope::Layout, |current| {
                current.merge(RepaintScope::Layout)
            }));
        }
        if self.shortcut_platform != previous.shortcut_platform
            || self.presentation_generation != previous.presentation_generation
        {
            scope = Some(scope.map_or(RepaintScope::Projection, |current| {
                current.merge(RepaintScope::Projection)
            }));
        }
        scope
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationEnvironment, ShortcutPlatform, TextScale, TextScaleError, WritingDirection,
    };
    use crate::application::localization::{LocaleId, TextCatalog, TextKey};
    use std::sync::Arc;

    #[test]
    fn text_scale_rejects_non_finite_and_unbounded_values() {
        assert_eq!(TextScale::new(f32::NAN), Err(TextScaleError::NotFinite));
        assert!(matches!(
            TextScale::new(0.1),
            Err(TextScaleError::OutOfBounds { .. })
        ));
        assert_eq!(TextScale::new(1.5).expect("valid scale").factor(), 1.5);
    }

    #[test]
    fn application_environment_resolves_explicit_locale_fallbacks() {
        let en = LocaleId::english();
        let fr = LocaleId::new("fr").expect("valid locale");
        let key = TextKey::new("save", "Save");
        let catalog = TextCatalog::default().with_generation(7).insert(
            fr.clone(),
            key.clone(),
            "Enregistrer",
        );
        let environment = ApplicationEnvironment::new(en)
            .with_fallback_chain([LocaleId::new("de").expect("valid locale"), fr])
            .with_writing_direction(WritingDirection::Rtl)
            .with_catalog(Arc::new(catalog));

        let resolved = environment.localized(&key);
        assert_eq!(resolved.as_str(), "Enregistrer");
        assert_eq!(
            resolved.resolved_locale(),
            Some(&LocaleId::new("fr").unwrap())
        );
        assert_eq!(environment.catalog_generation(), 7);
        assert_eq!(environment.writing_direction(), WritingDirection::Rtl);
    }

    #[test]
    fn application_changes_select_targeted_invalidation() {
        use crate::runtime::{RepaintScope, SurfaceInvalidation};

        assert_eq!(
            super::ApplicationEnvironmentChange::LocaleOrCatalog.repaint_scope(),
            RepaintScope::Surface
        );
        assert_eq!(
            super::ApplicationEnvironmentChange::DirectionOrTextScale.surface_invalidation(),
            SurfaceInvalidation::Layout
        );
        assert_eq!(
            ApplicationEnvironment::default().repaint_scope_since(
                &ApplicationEnvironment::default()
                    .with_shortcut_platform(ShortcutPlatform::Windows)
            ),
            Some(RepaintScope::Projection)
        );
    }

    #[test]
    fn identical_catalog_pointer_is_unchanged_without_comparing_entries() {
        let catalog = Arc::new(TextCatalog::default());
        let previous = ApplicationEnvironment::default().with_catalog(Arc::clone(&catalog));
        let current = ApplicationEnvironment::default().with_catalog(catalog);

        assert_eq!(current.repaint_scope_since(&previous), None);
    }

    #[test]
    fn distinct_equal_catalog_values_are_unchanged() {
        let key = TextKey::new("save", "Save");
        let previous = ApplicationEnvironment::default().with_catalog(Arc::new(
            TextCatalog::default().with_generation(3).insert(
                LocaleId::english(),
                key.clone(),
                "Enregistrer",
            ),
        ));
        let current = ApplicationEnvironment::default().with_catalog(Arc::new(
            TextCatalog::default().with_generation(3).insert(
                LocaleId::english(),
                key,
                "Enregistrer",
            ),
        ));

        assert_eq!(current.repaint_scope_since(&previous), None);
    }

    #[test]
    fn distinct_same_generation_catalog_entries_require_surface_repaint() {
        let key = TextKey::new("save", "Save");
        let previous = ApplicationEnvironment::default().with_catalog(Arc::new(
            TextCatalog::default().with_generation(3).insert(
                LocaleId::english(),
                key.clone(),
                "Save",
            ),
        ));
        let current = ApplicationEnvironment::default().with_catalog(Arc::new(
            TextCatalog::default().with_generation(3).insert(
                LocaleId::english(),
                key,
                "Enregistrer",
            ),
        ));

        assert_eq!(
            current.repaint_scope_since(&previous),
            Some(crate::runtime::RepaintScope::Surface)
        );
    }
}
