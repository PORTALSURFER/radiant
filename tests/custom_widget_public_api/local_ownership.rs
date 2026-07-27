use radiant::{
    gui::automation::{AutomationNodeSnapshot, AutomationRole},
    layout::{Point, Rect, Vector2},
    prelude::IntoView,
    runtime::{PaintPrimitive, SurfaceRuntime, UiSurface, declarative_owned_runtime_bridge},
    theme::ThemeTokens,
    widgets::{
        EmbeddedInteractiveRowWidget, InteractiveRowMessage, InteractiveRowWidget, PointerButton,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSemantics, WidgetSizing,
    },
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalWidgetOutput {
    Activated,
}

#[derive(Clone)]
struct LocalState {
    activations: usize,
}

#[derive(Clone)]
struct LocalWidget {
    common: WidgetCommon,
    local: Rc<RefCell<LocalState>>,
    transient_activations: usize,
}

impl LocalWidget {
    fn new(id: u64, local: Rc<RefCell<LocalState>>) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 28.0)))
                .with_keyboard_focus(),
            local,
            transient_activations: 0,
        }
    }
}

impl WidgetSemantics for LocalWidget {
    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Readout
    }

    fn automation_label(&self) -> Option<String> {
        Some("UI-local widget".to_owned())
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(self.local.borrow().activations.to_string())
    }
}

impl Widget for LocalWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.transient_activations += 1;
                self.local.borrow_mut().activations += 1;
                Some(WidgetOutput::custom(LocalWidgetOutput::Activated))
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.transient_activations = previous.transient_activations;
        }
    }

    fn capabilities(&self) -> radiant::widgets::WidgetCapabilities<'_> {
        radiant::widgets::WidgetCapabilities::new().semantics(self)
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Clone)]
struct LocalEmbeddedRow {
    row: InteractiveRowWidget,
    local: Rc<RefCell<usize>>,
}

impl EmbeddedInteractiveRowWidget for LocalEmbeddedRow {
    type Message = InteractiveRowMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        *self.local.borrow_mut() += 1;
        Some(message)
    }

    fn append_interactive_row_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

fn automation_node<'a>(
    node: &'a AutomationNodeSnapshot,
    id: &str,
) -> Option<&'a AutomationNodeSnapshot> {
    if node.id.0 == id {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| automation_node(child, id))
}

#[test]
fn ui_local_widget_builder_dispatches_ui_local_output_to_local_message() {
    use radiant::prelude as ui;

    let local = Rc::new(RefCell::new(LocalState { activations: 0 }));
    let host_events = Rc::new(RefCell::new(0usize));
    let mapper_events = Rc::clone(&host_events);
    let mut surface: UiSurface<Rc<RefCell<usize>>> =
        ui::custom_widget(LocalWidget::new(30, Rc::clone(&local)), move |output| {
            output
                .custom_ref::<LocalWidgetOutput>()
                .map(|_| Rc::clone(&mapper_events))
        })
        .id(30)
        .into_surface();

    let output = surface
        .dispatch_widget_input(
            30,
            Rect::from_size(120.0, 28.0),
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 12.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("local widget should emit UI-local output");
    let message = surface
        .dispatch_widget_output(30, output)
        .expect("local widget output should map to host message");
    *message.borrow_mut() += 1;

    assert_eq!(*host_events.borrow(), 1);
    assert_eq!(local.borrow().activations, 1);
}

#[test]
fn ui_local_widget_output_payload_round_trips_through_clone_and_dispatch() {
    use radiant::prelude as ui;

    let local = Rc::new(RefCell::new(LocalState { activations: 0 }));
    let surface: UiSurface<Rc<RefCell<usize>>> =
        ui::custom_widget(LocalWidget::new(30, local), |output| {
            output.custom_ref::<Rc<RefCell<usize>>>().cloned()
        })
        .id(30)
        .into_surface();
    let payload = Rc::new(RefCell::new(5usize));
    let output = WidgetOutput::typed(Rc::clone(&payload));

    assert_eq!(output, output.clone());
    let message = surface
        .dispatch_widget_output(30, output.clone())
        .expect("local output should map to a local message");
    assert!(Rc::ptr_eq(&message, &payload));
    *message.borrow_mut() += 1;
    assert_eq!(*payload.borrow(), 6);
}

#[test]
fn ui_local_widget_semantics_and_state_survive_runtime_reprojection() {
    use radiant::prelude as ui;

    let local = Rc::new(RefCell::new(LocalState { activations: 0 }));
    let host_events = Rc::new(RefCell::new(0usize));
    let project_local = Rc::clone(&local);
    let project_events = Rc::clone(&host_events);
    let bridge = declarative_owned_runtime_bridge(
        (),
        move |_| {
            let map_events = Rc::clone(&project_events);
            ui::custom_widget(
                LocalWidget::new(30, Rc::clone(&project_local)),
                move |output| {
                    output
                        .custom_ref::<LocalWidgetOutput>()
                        .map(|_| Rc::clone(&map_events))
                },
            )
            .id(30)
            .into_surface()
        },
        |_state, message: Rc<RefCell<usize>>| {
            *message.borrow_mut() += 1;
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 28.0));

    assert!(runtime.dispatch_input(
        30,
        WidgetInput::PointerRelease {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    ));

    let widget = runtime
        .surface()
        .find_widget(30)
        .expect("local widget should remain projected")
        .widget_object()
        .as_any()
        .downcast_ref::<LocalWidget>()
        .expect("projected widget should retain its concrete type");
    assert_eq!(widget.transient_activations, 1);
    assert_eq!(local.borrow().activations, 1);
    assert_eq!(*host_events.borrow(), 1);
    let semantics = widget.automation_semantics();
    assert_eq!(semantics.role, AutomationRole::Readout);
    assert_eq!(semantics.label.as_deref(), Some("UI-local widget"));
    assert_eq!(semantics.value_text.as_deref(), Some("1"));

    let snapshot = runtime.automation_snapshot();
    let node = automation_node(&snapshot.root, "30").expect("local widget automation node");
    assert_eq!(node.role, AutomationRole::Readout);
    assert_eq!(node.label.as_deref(), Some("UI-local widget"));
    assert_eq!(node.value.as_deref(), Some("1"));
}

#[test]
fn ui_local_widget_and_embedded_row_values_can_be_boxed_without_thread_bounds() {
    let local = Rc::new(RefCell::new(LocalState { activations: 0 }));
    let widget = LocalWidget::new(31, Rc::clone(&local));
    let boxed: Box<dyn Widget> = Box::new(widget.clone());
    drop(boxed);
    drop(widget);
    assert_eq!(Rc::strong_count(&local), 1);

    let row_local = Rc::new(RefCell::new(0usize));
    let row = LocalEmbeddedRow {
        row: InteractiveRowWidget::new(32, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
        local: Rc::clone(&row_local),
    };
    let boxed_row: Box<dyn Widget> = Box::new(row);
    drop(boxed_row);
    assert_eq!(Rc::strong_count(&row_local), 1);
}
