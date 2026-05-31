use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
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

    /// Emit a mapped host message when this progress bar emits output.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ProgressBarMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
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
        let mut node =
            view_node_from_widget(MappedWidget::new(progress, WidgetMessageMapper::typed(map)));
        node.style = self.style;
        node
    }
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
