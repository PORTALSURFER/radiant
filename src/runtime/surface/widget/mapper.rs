//! Host-message mapping for surface widget leaves.

use crate::{
    runtime::{NativeFileDrop, ScrollUpdate},
    widgets::WidgetOutput,
};
use std::cell::RefCell;
use std::rc::Rc;

type DynamicOutputMapper<Message> = Rc<dyn Fn(WidgetOutput) -> Option<Message>>;

enum OutputMapper<Message> {
    Dynamic(DynamicOutputMapper<Message>),
    Constant(ConstantOutputMapper<Message>),
}

/// Constant binding that stays inline until it must be shared by a clone.
struct ConstantOutputMapper<Message> {
    message: RefCell<ConstantMessage<Message>>,
    matches: fn(&WidgetOutput) -> bool,
    clone_message: fn(&Message) -> Message,
}

/// Storage state for a constant host message.
enum ConstantMessage<Message> {
    /// Message owned inline by a freshly projected mapper.
    Inline(Message),
    /// Message shared after the mapper or its enclosing surface is cloned.
    Shared(Rc<Message>),
    /// Temporary sentinel used while moving storage out of the cell.
    Empty,
}

impl<Message> Clone for OutputMapper<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Dynamic(map) => Self::Dynamic(Rc::clone(map)),
            Self::Constant(map) => Self::Constant(map.clone()),
        }
    }
}

impl<Message> Clone for ConstantOutputMapper<Message> {
    fn clone(&self) -> Self {
        let cloned = ConstantMessage::Shared(self.shared_message());
        Self {
            message: RefCell::new(cloned),
            matches: self.matches,
            clone_message: self.clone_message,
        }
    }
}

impl<Message> ConstantOutputMapper<Message> {
    fn shared_message(&self) -> Rc<Message> {
        let mut message = self.message.borrow_mut();
        let current = std::mem::replace(&mut *message, ConstantMessage::Empty);
        match current {
            ConstantMessage::Inline(current) => {
                let current = Rc::new(current);
                let shared = Rc::clone(&current);
                *message = ConstantMessage::Shared(current);
                shared
            }
            ConstantMessage::Shared(current) => {
                let shared = Rc::clone(&current);
                *message = ConstantMessage::Shared(current);
                shared
            }
            ConstantMessage::Empty => unreachable!("constant mapper storage is not reentrant"),
        }
    }

    fn map_output(&self, output: &WidgetOutput) -> Option<Message> {
        if !(self.matches)(output) {
            return None;
        }
        let message = self.shared_message();
        Some((self.clone_message)(message.as_ref()))
    }
}

/// UI-local mapper type that turns widget-specific payloads into host-defined messages.
///
/// Mappers are invoked and dropped on the UI runtime; they are not `Send` or `Sync`.
pub type MessageMapper<Input, Message> = Rc<dyn Fn(Input) -> Message>;

/// UI-local mapper type that turns scroll movement into optional host-defined messages.
///
/// Scroll containers may update local runtime offset for sub-row or otherwise
/// unchanged movement without asking the host to reproject the surface.
/// The mapper remains owned by the UI runtime and is not `Send` or `Sync`.
pub type ScrollMessageMapper<Message> = Rc<dyn Fn(ScrollUpdate) -> Option<Message>>;

/// UI-local mapper type that turns native file-drop events into host-defined messages.
pub type NativeFileDropMessageMapper<Message> = MessageMapper<NativeFileDrop, Message>;

/// Message bindings that turn widget output payloads into host-defined messages.
#[derive(Default)]
pub struct WidgetMessageMapper<Message> {
    map: Option<OutputMapper<Message>>,
    native_file_drop: Option<NativeFileDropMessageMapper<Message>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            native_file_drop: self.native_file_drop.as_ref().map(Rc::clone),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper that does not emit host-defined messages.
    pub fn none() -> Self {
        Self {
            map: None,
            native_file_drop: None,
        }
    }

    /// Build a mapper for any typed, UI-local widget output payload.
    ///
    /// The payload is cloned and downcast synchronously on the owning UI
    /// runtime, so it may contain non-thread-safe state such as `Rc` or
    /// `RefCell`.
    pub fn typed<Output>(map: impl Fn(Output) -> Message + 'static) -> Self
    where
        Output: Clone + 'static,
    {
        Self::dynamic(move |output| output.typed_cloned::<Output>().map(&map))
    }

    /// Build an allocation-free binding that clones one message for matching outputs.
    ///
    /// The matcher must be non-capturing so the binding can store it as a function
    /// pointer alongside the message instead of allocating a dynamic callback.
    pub(crate) fn constant(message: Message, matches: fn(&WidgetOutput) -> bool) -> Self
    where
        Message: Clone + 'static,
    {
        Self {
            map: Some(OutputMapper::Constant(ConstantOutputMapper {
                message: RefCell::new(ConstantMessage::Inline(message)),
                matches,
                clone_message: Message::clone,
            })),
            native_file_drop: None,
        }
    }

    /// Build a dynamic output mapper for custom widgets.
    pub fn dynamic(map: impl Fn(WidgetOutput) -> Option<Message> + 'static) -> Self {
        Self {
            map: Some(OutputMapper::Dynamic(Rc::new(map))),
            native_file_drop: None,
        }
    }

    /// Return this mapper with native file-drop events mapped to host messages.
    pub fn with_native_file_drop(
        mut self,
        map: impl Fn(NativeFileDrop) -> Message + 'static,
    ) -> Self {
        self.native_file_drop = Some(Rc::new(map));
        self
    }

    pub(super) fn maps_any_output(&self) -> bool {
        self.map.is_some()
    }

    pub(super) fn uses_dynamic_output_callback(&self) -> bool {
        matches!(self.map, Some(OutputMapper::Dynamic(_)))
    }

    /// Return whether this mapper carries opaque host or native-drop behavior.
    ///
    /// Reconciliation cannot compare callback identity or captured state, so
    /// any message binding is conservatively treated as structural.
    pub(in crate::runtime::surface) fn has_opaque_behavior(&self) -> bool {
        self.map.is_some() || self.native_file_drop.is_some()
    }

    pub(super) fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        match self.map.as_ref()? {
            OutputMapper::Dynamic(map) => map(output),
            OutputMapper::Constant(map) => map.map_output(&output),
        }
    }

    pub(super) fn map_native_file_drop(&self, drop: NativeFileDrop) -> Option<Message> {
        self.native_file_drop.as_ref().map(|map| map(drop))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{ButtonMessage, TextInputMessage};
    use std::cell::RefCell;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct CountedMessage {
        clone_count: Arc<AtomicUsize>,
    }

    impl Clone for CountedMessage {
        fn clone(&self) -> Self {
            self.clone_count.fetch_add(1, Ordering::Relaxed);
            Self {
                clone_count: Arc::clone(&self.clone_count),
            }
        }
    }

    #[test]
    fn constant_mapper_stores_message_without_dynamic_callback() {
        let mapper = WidgetMessageMapper::button_message(());

        assert!(matches!(mapper.map, Some(OutputMapper::Constant(_))));
    }

    #[test]
    fn constant_button_mapper_clones_for_typed_button_outputs_only() {
        let clone_count = Arc::new(AtomicUsize::new(0));
        let mapper = WidgetMessageMapper::button_message(CountedMessage {
            clone_count: Arc::clone(&clone_count),
        });

        assert!(
            mapper
                .map_output(WidgetOutput::typed(TextInputMessage::Changed {
                    value: String::from("ignored"),
                }))
                .is_none()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 0);

        assert!(
            mapper
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 1);

        assert!(
            mapper
                .map_output(WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                    position: crate::gui::types::Point::new(1.0, 2.0),
                }))
                .is_some()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn cloning_constant_mapper_shares_message_without_cloning_it() {
        let clone_count = Arc::new(AtomicUsize::new(0));
        let mapper = WidgetMessageMapper::button_message(CountedMessage {
            clone_count: Arc::clone(&clone_count),
        });

        let cloned = mapper.clone();
        let cloned_again = cloned.clone();
        assert_eq!(clone_count.load(Ordering::Relaxed), 0);

        assert!(
            mapper
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert!(
            cloned
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert!(
            cloned_again
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn dynamic_and_filter_mapped_callbacks_remain_available() {
        let mapped = WidgetMessageMapper::button(|message| message.is_activate());
        assert_eq!(
            mapped.map_output(WidgetOutput::typed(ButtonMessage::Activate)),
            Some(true)
        );
        assert_eq!(
            mapped.map_output(WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: crate::gui::types::Point::new(1.0, 2.0),
            })),
            Some(false)
        );

        let filtered = WidgetMessageMapper::dynamic(|output| {
            output
                .typed_copied::<ButtonMessage>()
                .filter(|message| message.is_activate())
                .map(|_| "activated")
        });
        assert_eq!(
            filtered.map_output(WidgetOutput::typed(ButtonMessage::Activate)),
            Some("activated")
        );
        assert_eq!(
            filtered.map_output(WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: crate::gui::types::Point::new(1.0, 2.0),
            })),
            None
        );
    }

    #[test]
    fn dynamic_mapper_keeps_ui_local_capture_and_drops_on_ui_runtime() {
        let calls = Rc::new(RefCell::new(0usize));
        let captured = Rc::clone(&calls);
        let mapper = WidgetMessageMapper::dynamic(move |_| {
            *captured.borrow_mut() += 1;
            Some(Rc::new(RefCell::new(())))
        });
        let clone = mapper.clone();

        assert!(
            mapper
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert!(
            clone
                .map_output(WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert_eq!(*calls.borrow(), 2);

        drop(clone);
        drop(mapper);
        assert_eq!(Rc::strong_count(&calls), 1);
    }

    #[test]
    fn dynamic_mapper_invokes_and_drops_ui_local_capture() {
        let calls = Rc::new(RefCell::new(0usize));
        let dropped = Rc::new(RefCell::new(false));
        let probe = UiDropProbe(Rc::clone(&dropped));
        let calls_for_mapper = Rc::clone(&calls);
        let mapper = WidgetMessageMapper::dynamic(move |_| {
            let _probe = &probe;
            *calls_for_mapper.borrow_mut() += 1;
            Some(())
        });

        assert_eq!(
            mapper.map_output(WidgetOutput::typed(ButtonMessage::Activate)),
            Some(())
        );
        assert_eq!(*calls.borrow(), 1);
        drop(mapper);
        assert!(
            *dropped.borrow(),
            "local mapper capture should drop on the UI runtime"
        );
    }

    #[test]
    fn typed_mapper_round_trips_ui_local_payloads_and_preserves_output_identity() {
        let payload = Rc::new(RefCell::new(7usize));
        let output = WidgetOutput::typed(Rc::clone(&payload));
        let cloned = output.clone();

        assert_eq!(output, cloned);
        assert_ne!(output, WidgetOutput::typed(Rc::clone(&payload)));
        assert_eq!(output.typed_ref::<Rc<RefCell<usize>>>(), Some(&payload));

        let mapper = WidgetMessageMapper::typed(|value: Rc<RefCell<usize>>| {
            *value.borrow_mut() += 1;
            value
        });
        let mapped = mapper.map_output(cloned).expect("local payload should map");
        assert!(Rc::ptr_eq(&mapped, &payload));
        assert_eq!(*payload.borrow(), 8);
    }

    struct UiDropProbe(Rc<RefCell<bool>>);

    impl Drop for UiDropProbe {
        fn drop(&mut self) {
            *self.0.borrow_mut() = true;
        }
    }
}
