use super::super::*;

#[derive(Default)]
pub(in super::super) struct CanvasBridge {
    pub(in super::super) text: String,
}

#[derive(Default)]
pub(in super::super) struct ScrollbarBridge {
    pub(in super::super) offset: f32,
}

#[derive(Default)]
pub(in super::super) struct WheelRefreshBridge {
    pub(in super::super) wheel_count: usize,
    pub(in super::super) project_count: usize,
}

impl RuntimeBridge<String> for CanvasBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            21,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Character(character),
                } => character.to_string(),
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                CanvasMessage::Input {
                    input: WidgetInput::PointerDoubleClick { .. },
                } => String::from("double"),
                _ => String::new(),
            },
        )))
    }

    fn update(&mut self, message: String) -> Command<String> {
        self.text.push_str(&message);
        Command::none()
    }
}

impl RuntimeBridge<f32> for ScrollbarBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<f32>> {
        let mut scrollbar = ScrollbarWidget::new(
            41,
            ScrollbarAxis::Horizontal,
            WidgetSizing::fixed(Vector2::new(220.0, 14.0)),
        );
        scrollbar.props.viewport_fraction = 0.25;
        scrollbar.state.offset_fraction = self.offset;
        Arc::new(UiSurface::new(SurfaceNode::widget(
            scrollbar,
            WidgetMessageMapper::scrollbar(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => offset_fraction,
            }),
        )))
    }

    fn reduce_message(&mut self, message: f32) {
        self.offset = message;
    }
}

impl RuntimeBridge<String> for WheelRefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            51,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                _ => String::new(),
            },
        )))
    }

    fn reduce_message(&mut self, message: String) {
        if message == "wheel" {
            self.wheel_count += 1;
        }
    }
}
