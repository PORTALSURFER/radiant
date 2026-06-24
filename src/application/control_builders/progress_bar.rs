use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    gui::feedback::ProgressSnapshot,
    gui::types::Rgba8,
    runtime::WidgetMessageMapper,
    widgets::{
        ProgressBarMessage, ProgressBarMode, ProgressBarProps, ProgressBarWidget,
        ProgressBarWidgetParts, WidgetSizing, WidgetStyle,
    },
};

/// Builder for horizontal progress bars.
pub struct ProgressBarBuilder {
    mode: ProgressBarMode,
    style: Option<WidgetStyle>,
    track_color: Option<Rgba8>,
    fill_color: Option<Rgba8>,
    max_track_height: Option<f32>,
    activation: bool,
}

impl ProgressBarBuilder {
    /// Apply an explicit widget style before binding this progress bar.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set explicit track and fill colors.
    pub fn colors(mut self, track: Rgba8, fill: Rgba8) -> Self {
        self.track_color = Some(track);
        self.fill_color = Some(fill);
        self
    }

    /// Set the maximum painted track height.
    pub fn max_track_height(mut self, height: f32) -> Self {
        self.max_track_height = Some(height);
        self
    }

    /// Emit activation messages for primary pointer clicks.
    pub fn activatable(mut self) -> Self {
        self.activation = true;
        self
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.mapped(move |_| message.clone())
    }

    /// Build a display-only progress bar without host messages.
    pub fn passive<Message: 'static>(self) -> ViewNode<Message> {
        let (progress, style) = self.into_widget_and_style();
        view_node_with_style(progress, style)
    }

    /// Emit a mapped host message when this progress bar emits output.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ProgressBarMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let (progress, style) = self.into_widget_and_style();
        view_node_with_style(
            MappedWidget::new(progress, WidgetMessageMapper::typed(map)),
            style,
        )
    }

    fn into_widget_and_style(self) -> (ProgressBarWidget, Option<WidgetStyle>) {
        let mut progress = ProgressBarWidget::from_parts(ProgressBarWidgetParts {
            id: 0,
            sizing: WidgetSizing::fixed(crate::layout::Vector2::new(120.0, 10.0)),
            props: ProgressBarProps::new(self.mode),
        });
        if let (Some(track), Some(fill)) = (self.track_color, self.fill_color) {
            progress = progress.with_colors(track, fill);
        }
        if let Some(height) = self.max_track_height {
            progress = progress.with_max_track_height(height);
        }
        if self.activation {
            progress = progress.with_activation();
        }
        (progress, self.style)
    }
}

fn view_node_with_style<Message>(
    widget: impl crate::application::WidgetView<Message> + 'static,
    style: Option<WidgetStyle>,
) -> ViewNode<Message> {
    let mut node = view_node_from_widget(widget);
    node.style = style;
    node
}

/// Build a progress bar from an explicit mode.
pub fn progress_bar(mode: ProgressBarMode) -> ProgressBarBuilder {
    ProgressBarBuilder {
        mode,
        style: None,
        track_color: None,
        fill_color: None,
        max_track_height: None,
        activation: false,
    }
}

/// Build a determinate progress bar.
pub fn determinate_progress_bar(fraction: f32) -> ProgressBarBuilder {
    progress_bar(ProgressBarMode::Determinate(fraction))
}

/// Build an indeterminate activity progress bar.
pub fn indeterminate_progress_bar(position_fraction: f32) -> ProgressBarBuilder {
    progress_bar(ProgressBarMode::Indeterminate(position_fraction))
}

/// Build a progress bar from domain-neutral progress counters.
///
/// When the snapshot has a known total, this returns a determinate progress
/// bar. When the total is unknown, this returns an indeterminate activity bar
/// using `indeterminate_position_fraction` as the moving segment position.
pub fn progress_bar_for_snapshot(
    snapshot: ProgressSnapshot,
    indeterminate_position_fraction: f32,
) -> ProgressBarBuilder {
    if let Some(fraction) = snapshot.fraction() {
        determinate_progress_bar(fraction)
    } else {
        indeterminate_progress_bar(indeterminate_position_fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::IntoView;

    #[test]
    fn progress_bar_for_snapshot_uses_determinate_mode_when_total_is_known() {
        let builder = progress_bar_for_snapshot(ProgressSnapshot::new(3, 12), 0.75);

        assert_eq!(builder.mode, ProgressBarMode::Determinate(0.25));
    }

    #[test]
    fn progress_bar_for_snapshot_uses_indeterminate_mode_when_total_is_unknown() {
        let builder = progress_bar_for_snapshot(ProgressSnapshot::new(3, 0), 0.75);

        assert_eq!(builder.mode, ProgressBarMode::Indeterminate(0.75));
    }

    #[test]
    fn progress_bar_message_maps_activation_to_cloned_host_message() {
        let view = progress_bar(ProgressBarMode::Determinate(0.5))
            .activatable()
            .message("toggle")
            .id(42);

        assert_eq!(
            view.view_dispatch_widget_output(
                42,
                crate::widgets::WidgetOutput::typed(ProgressBarMessage::Activate,)
            ),
            Some("toggle")
        );
    }

    #[test]
    fn progress_bar_passive_paints_without_host_output_mapping() {
        fn passive_bar() -> ViewNode<&'static str> {
            progress_bar(ProgressBarMode::Determinate(0.5))
                .passive::<&'static str>()
                .id(43)
        }

        let frame = passive_bar()
            .view_frame_at_size_with_default_theme(crate::layout::Vector2::new(120.0, 10.0));

        assert!(
            frame.paint_plan.fill_rects().count() >= 2,
            "passive progress bars should still paint track and fill"
        );
        assert_eq!(
            passive_bar().view_dispatch_widget_output(
                43,
                crate::widgets::WidgetOutput::typed(ProgressBarMessage::Activate,)
            ),
            None
        );
    }
}
