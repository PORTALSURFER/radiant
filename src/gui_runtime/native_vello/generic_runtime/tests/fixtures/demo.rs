use super::super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum DemoMessage {
    Increment,
    Rename(String),
}

#[derive(Default)]
pub(in super::super) struct DemoState {
    pub(in super::super) count: usize,
    pub(in super::super) name: String,
}

#[derive(Default)]
pub(in super::super) struct DemoBridge {
    pub(in super::super) state: DemoState,
}

impl RuntimeBridge<DemoMessage> for DemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn reduce_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::Increment => self.state.count += 1,
            DemoMessage::Rename(name) => self.state.name = name,
        }
    }
}

pub(in super::super) fn demo_bridge() -> DemoBridge {
    DemoBridge::default()
}

#[derive(Default)]
pub(in super::super) struct RepaintBridge {
    pub(in super::super) state: DemoState,
}

impl RuntimeBridge<DemoMessage> for RepaintBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        match message {
            DemoMessage::Increment => {
                self.state.count += 1;
                Command::request_repaint()
            }
            DemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }
}

pub(in super::super) fn demo_surface(state: &DemoState) -> Arc<UiSurface<DemoMessage>> {
    let button = ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    Arc::new(UiSurface::new(SurfaceNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    button,
                    WidgetMessageMapper::button(|_| DemoMessage::Increment),
                ),
            ),
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    input,
                    WidgetMessageMapper::text_input(|message| match message {
                        TextInputMessage::Changed { value }
                        | TextInputMessage::Submitted { value }
                        | TextInputMessage::CompletionRequested { value } => {
                            DemoMessage::Rename(value)
                        }
                    }),
                ),
            ),
        ],
    )))
}
