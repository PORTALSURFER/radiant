use radiant::{runtime::UiSurface, widgets::Widget};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DemoMessage {
    Increment,
}

#[derive(Default)]
pub(crate) struct DemoState {
    pub(crate) count: usize,
    pub(crate) name: String,
}

pub(crate) fn widget_ref<'a, T, Message>(
    surface: &'a UiSurface<Message>,
    id: u64,
    expected: &str,
) -> &'a T
where
    T: Widget + 'static,
{
    surface
        .find_widget(id)
        .unwrap_or_else(|| panic!("expected {expected} widget {id} to exist"))
        .widget()
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("expected widget {id} to be {expected}"))
}
