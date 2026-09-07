use super::*;
use crate::application::{IntoView, TextEditorDocument, TextEditorEdit, column, text_editor};
use crate::gui::input::InputTimestamp;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton},
};

struct Editors {
    documents: Vec<TextEditorDocument>,
}
impl RuntimeBridge<(usize, TextEditorEdit)> for Editors {
    fn project_surface(&mut self) -> Arc<UiSurface<(usize, TextEditorEdit)>> {
        column(self.documents.iter().enumerate().map(|(index, document)| {
            text_editor(document.snapshot())
                .id(10_000 + index as u64)
                .message(move |edit| (index, edit))
                .height(40.0)
                .fill_width()
        }))
        .into_surface()
        .into()
    }
    fn reduce_message(&mut self, (index, edit): (usize, TextEditorEdit)) {
        self.documents[index].apply(&edit).unwrap();
    }
}

#[test]
fn native_editor_pointer_promotes_65th_before_press_and_captured_drag() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        Editors {
            documents: (0..65)
                .map(|_| TextEditorDocument::new("AAAA").unwrap())
                .collect(),
        },
        Vector2::new(160.0, 4_000.0),
    );
    runner.rebuild_scene();
    let id = 10_064;
    let plan_ids = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::TextEditor(input) => Some(input.request.widget_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(plan_ids.len(), 65, "all editors must enter the current plan");
    assert!(plan_ids.contains(&id));
    assert_eq!(runner.frame.editor_geometry_receipts().len(), 64);
    assert!(
        !runner
            .frame
            .editor_geometry_receipts()
            .iter()
            .any(|receipt| receipt.request().widget_id == id)
    );
    let bounds = runner.core.runtime.layout().rects[&id];
    // Press in the padding: the text viewport alone must not exclude the target.
    runner.input.last_cursor = Some(Point::new(bounds.max.x - 1.0, bounds.min.y + 10.0));
    runner.retain_native_mouse_device(DeviceId::dummy(), None);
    let outcome = runner.route_native_mouse_input_with_timestamp(
        MouseButton::Left,
        ElementState::Pressed,
        None,
    );
    runner.apply_route_outcome(outcome.outcome);
    assert_eq!(runner.core.runtime.focused_widget(), Some(id));
    assert_eq!(
        runner.core.runtime.bridge().documents[64]
            .snapshot()
            .selection()
            .caret,
        4
    );
    runner.rebuild_scene();
    assert!(
        runner
            .frame
            .editor_geometry_receipts()
            .iter()
            .any(|receipt| receipt.request().widget_id == id)
    );
    assert_eq!(runner.core.runtime.pointer_capture(), Some(id));
    let route = runner.route_cursor_moved_with_timestamp(
        PhysicalPosition::new(
            f64::from(bounds.min.x + 6.0),
            f64::from(bounds.min.y + 10.0),
        ),
        InputTimestamp::capture(),
    );
    assert!(route.outcome.routed, "captured move must remain admitted");
    runner.apply_cursor_moved_route(route);
    let selection = runner.core.runtime.bridge().documents[64]
        .snapshot()
        .selection();
    assert_eq!(selection.anchor, 4);
    assert_eq!(selection.caret, 0);
}
