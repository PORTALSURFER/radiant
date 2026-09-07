//! Bounded presentation builders for application-owned feedback state.
//!
//! These builders only project supplied state. They do not retain work, poll,
//! schedule retries, or mutate application state. All supplied text is
//! truncated at Unicode scalar boundaries to [`MAX_FEEDBACK_TEXT_CHARS`].

#[path = "feedback/animation.rs"]
mod animation;

use std::rc::Rc;

use crate::{
    application::{IntoView, ViewNode, button, column, row, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use animation::{FeedbackChrome, FeedbackChromeKind};

/// Maximum number of Unicode scalar values retained by a feedback label or value.
///
/// Input beyond this bound is safely truncated without splitting a UTF-8 code
/// point. The bound keeps presentation-only feedback from retaining unbounded
/// domain diagnostics or labels.
pub const MAX_FEEDBACK_TEXT_CHARS: usize = 240;

const DEFAULT_SPINNER_LABEL: &str = "Loading";
const DEFAULT_SKELETON_LABEL: &str = "Loading content";

/// A compact semantic state for [`status_badge`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusSemantic {
    /// A neutral, informational state.
    #[default]
    Info,
    /// A positive or connected state.
    Success,
    /// A cautionary state that remains readable without motion.
    Warning,
    /// A failed, disconnected, or otherwise error state.
    Error,
    /// A deliberately unavailable or disconnected state.
    Offline,
}

impl StatusSemantic {
    fn style(self) -> WidgetStyle {
        let tone = match self {
            Self::Info | Self::Offline => WidgetTone::Neutral,
            Self::Success => WidgetTone::Success,
            Self::Warning => WidgetTone::Warning,
            Self::Error => WidgetTone::Danger,
        };
        WidgetStyle::new(tone, WidgetProminence::Normal)
    }
}

/// Build an indeterminate activity indicator driven by the shared runtime clock.
pub fn spinner() -> SpinnerBuilder {
    SpinnerBuilder {
        label: String::from(DEFAULT_SPINNER_LABEL),
    }
}

/// Builder for an indeterminate activity indicator.
pub struct SpinnerBuilder {
    label: String,
}

impl SpinnerBuilder {
    /// Set the visible and accessible activity label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = bounded_text(label.into());
        self
    }

    /// Build this indicator as a display-only view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        row([
            animation_anchor(FeedbackChromeKind::Spinner, 16.0, 16.0),
            text(self.label).muted_text(),
        ])
        .spacing(6.0)
    }
}

impl<Message: 'static> IntoView<Message> for SpinnerBuilder {
    fn into_projection(self) -> crate::application::ViewProjection<Message> {
        self.view().into_projection()
    }
}

/// Build a pending-content placeholder that reserves the expected layout.
pub fn skeleton() -> SkeletonBuilder {
    SkeletonBuilder {
        label: String::from(DEFAULT_SKELETON_LABEL),
        width: 160.0,
        height: 18.0,
    }
}

/// Builder for a pending-content layout placeholder.
pub struct SkeletonBuilder {
    label: String,
    width: f32,
    height: f32,
}

impl SkeletonBuilder {
    /// Set the visible and accessible pending-content label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = bounded_text(label.into());
        self
    }

    /// Reserve an explicit expected layout size.
    ///
    /// Non-finite and negative dimensions become zero so the builder cannot
    /// introduce invalid layout geometry.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = bounded_extent(width);
        self.height = bounded_extent(height);
        self
    }

    /// Build this placeholder as a display-only view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        column([
            animation_anchor(FeedbackChromeKind::Skeleton, self.width, self.height),
            text(self.label).muted_text(),
        ])
        .spacing(4.0)
    }
}

impl<Message: 'static> IntoView<Message> for SkeletonBuilder {
    fn into_projection(self) -> crate::application::ViewProjection<Message> {
        self.view().into_projection()
    }
}

/// Build a compact, static semantic status presentation.
pub fn status_badge(status: StatusSemantic) -> StatusBadgeBuilder {
    StatusBadgeBuilder {
        status,
        label: String::new(),
        value: None,
    }
}

/// Builder for a compact semantic status presentation.
pub struct StatusBadgeBuilder {
    status: StatusSemantic,
    label: String,
    value: Option<String>,
}

impl StatusBadgeBuilder {
    /// Set the status label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = bounded_text(label.into());
        self
    }

    /// Set an optional, readable status value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(bounded_text(value.into()));
        self
    }

    /// Build this status presentation as a display-only view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        let label = if self.label.is_empty() {
            String::from(status_fallback_label(self.status))
        } else {
            self.label
        };
        let mut children = vec![
            column([text(label)])
                .style(self.status.style())
                .padding(4.0),
        ];
        if let Some(value) = self.value {
            children.push(text(value).muted_text());
        }
        row(children).spacing(4.0)
    }
}

impl<Message: 'static> IntoView<Message> for StatusBadgeBuilder {
    fn into_projection(self) -> crate::application::ViewProjection<Message> {
        self.view().into_projection()
    }
}

/// Build a static error presentation for supplied, owned error text.
pub fn inline_error(error: impl Into<String>) -> InlineErrorBuilder {
    InlineErrorBuilder {
        error: bounded_text(error.into()),
    }
}

/// Builder for a static inline error with an optional normal retry action.
pub struct InlineErrorBuilder {
    error: String,
}

impl InlineErrorBuilder {
    /// Build a display-only error presentation without an action.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        inline_error_view(self.error, None)
    }

    /// Bind a normal retry action through the host's usual message model.
    pub fn retry<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + 'static,
    {
        inline_error_view(self.error, Some(button("Retry").message(message)))
    }
}

fn inline_error_view<Message: 'static>(
    error: String,
    retry: Option<ViewNode<Message>>,
) -> ViewNode<Message> {
    let mut children = vec![
        column([text("Error")]).danger().padding(4.0),
        text(error).wrap().fill_width(),
    ];
    if let Some(retry) = retry {
        children.push(retry);
    }
    row(children).spacing(6.0)
}

fn animation_anchor<Message: 'static>(
    kind: FeedbackChromeKind,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    // The animated container receives its accepted bounds from the normal
    // layout pass; the capability only contributes paint from runtime samples.
    row::<Message>([])
        .width(bounded_extent(width))
        .height(bounded_extent(height))
        .animatable(Rc::new(FeedbackChrome::new(kind)))
}

fn bounded_text(value: String) -> String {
    value.chars().take(MAX_FEEDBACK_TEXT_CHARS).collect()
}

fn bounded_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn status_fallback_label(status: StatusSemantic) -> &'static str {
    match status {
        StatusSemantic::Info => "Status",
        StatusSemantic::Success => "Success",
        StatusSemantic::Warning => "Warning",
        StatusSemantic::Error => "Error",
        StatusSemantic::Offline => "Offline",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{Animatable, AnimationPaintContext, AnimationValues},
        gui::types::{Point, Rect},
        runtime::{PaintPrimitive, WindowEnvironment},
        theme::{DpiScale, ThemeTokens},
    };

    #[test]
    fn feedback_text_is_unicode_safe_and_bounded() {
        let source = "🙂".repeat(MAX_FEEDBACK_TEXT_CHARS + 1);
        let bounded = bounded_text(source);

        assert_eq!(bounded.chars().count(), MAX_FEEDBACK_TEXT_CHARS);
        assert!(bounded.is_char_boundary(bounded.len()));
    }

    #[test]
    fn spinner_and_skeleton_declare_shared_runtime_feedback() {
        let spinner = FeedbackChrome::new(FeedbackChromeKind::Spinner);
        let skeleton = FeedbackChrome::new(FeedbackChromeKind::Skeleton);

        assert_eq!(spinner.targets(), &[]);
        assert_eq!(skeleton.targets(), &[]);
        assert_eq!(spinner.feedback().len(), 1);
        assert_eq!(skeleton.feedback().len(), 1);
        assert_eq!(
            spinner.feedback()[0].group(),
            skeleton.feedback()[0].group()
        );
    }

    #[test]
    fn spinner_paint_uses_runtime_samples_and_reduced_motion_static_fallback() {
        let chrome = FeedbackChrome::new(FeedbackChromeKind::Spinner);
        let bounds = Rect::from_min_size(
            Point::new(0.0, 0.0),
            crate::layout::Vector2::new(16.0, 16.0),
        );
        let theme = ThemeTokens::dark();
        let environment = WindowEnvironment::new(DpiScale::ONE, None, false, false).resolved();
        let reduced_environment =
            WindowEnvironment::new(DpiScale::ONE, None, false, true).resolved();
        let property = chrome.feedback()[0].property();
        let mut animated = Vec::new();
        let mut reduced = Vec::new();

        chrome.append_paint(
            AnimationValues::new(&[(property, 0.75)]),
            AnimationPaintContext {
                bounds,
                theme: &theme,
                environment: &environment,
            },
            &mut animated,
        );
        chrome.append_paint(
            AnimationValues::new(&[(property, 0.75)]),
            AnimationPaintContext {
                bounds,
                theme: &theme,
                environment: &reduced_environment,
            },
            &mut reduced,
        );

        let animated_rect = animated.iter().find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        });
        let reduced_rect = reduced.iter().find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        });
        assert_ne!(animated_rect, reduced_rect);
        assert_eq!(
            reduced_rect,
            Some(Rect::from_min_size(
                Point::new(0.0, 6.0),
                crate::layout::Vector2::new(16.0, 4.0)
            ))
        );
    }

    #[test]
    fn skeleton_paint_uses_the_runtime_phase_for_its_highlight() {
        let chrome = FeedbackChrome::new(FeedbackChromeKind::Skeleton);
        let bounds = Rect::from_min_size(
            Point::new(0.0, 0.0),
            crate::layout::Vector2::new(100.0, 18.0),
        );
        let theme = ThemeTokens::dark();
        let environment = WindowEnvironment::default().resolved();
        let property = chrome.feedback()[0].property();
        let mut early = Vec::new();
        let mut late = Vec::new();

        for (phase, output) in [(0.0, &mut early), (0.75, &mut late)] {
            chrome.append_paint(
                AnimationValues::new(&[(property, phase)]),
                AnimationPaintContext {
                    bounds,
                    theme: &theme,
                    environment: &environment,
                },
                output,
            );
        }

        let highlight_rect = |primitives: &[PaintPrimitive]| {
            primitives.iter().find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) if fill.color.a == 28 => Some(fill.rect),
                _ => None,
            })
        };
        assert_ne!(highlight_rect(&early), highlight_rect(&late));
    }

    #[test]
    fn static_status_and_error_presentations_paint_readable_labels_and_retry() {
        let status = status_badge(StatusSemantic::Warning)
            .label("Sync")
            .value("Waiting")
            .view::<()>();
        let error = inline_error("Connection failed").retry("retry");

        let status_frame =
            status.view_frame_at_size_with_default_theme(crate::layout::Vector2::new(240.0, 28.0));
        let error_frame =
            error.view_frame_at_size_with_default_theme(crate::layout::Vector2::new(320.0, 28.0));

        assert!(status_frame.paint_plan.contains_text("Sync"));
        assert!(status_frame.paint_plan.contains_text("Waiting"));
        assert!(error_frame.paint_plan.contains_text("Error"));
        assert!(error_frame.paint_plan.contains_text("Connection failed"));
        assert!(error_frame.paint_plan.contains_text("Retry"));
    }
}
