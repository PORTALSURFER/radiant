//! Host-message mapping for surface widget leaves.

use crate::{
    runtime::{NativeFileDrop, ScrollUpdate},
    widgets::WidgetOutput,
};
use std::sync::{Arc, Mutex, MutexGuard};

type DynamicOutputMapper<Message> = Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>;

enum OutputMapper<Message> {
    Dynamic(DynamicOutputMapper<Message>),
    Constant(ConstantOutputMapper<Message>),
}

/// Constant binding that stays inline until it must be shared by a clone.
struct ConstantOutputMapper<Message> {
    message: Mutex<ConstantMessage<Message>>,
    matches: fn(&WidgetOutput) -> bool,
    clone_message: fn(&Message) -> Message,
}

/// Storage state for a constant host message.
enum ConstantMessage<Message> {
    /// Message owned inline by a freshly projected mapper.
    Inline(Message),
    /// Message shared after the mapper or its enclosing surface is cloned.
    Shared(Arc<Message>),
    /// Temporary sentinel used only while moving storage under the mutex.
    Transitioning,
}

// SAFETY: Dynamic mappers store only a `Send + Sync` callback and do not retain
// a `Message`. The sole constructor for the constant variant requires `Message`
// to be `Send + Sync`, so every value actually retained by that variant is safe
// to move and share with the mapper. Keeping this invariant here preserves
// dynamic mappers for host message types that do not themselves need to be
// `Send + Sync`.
unsafe impl<Message> Send for OutputMapper<Message> {}
// SAFETY: See the `Send` implementation above; both auto-trait guarantees are
// enforced by the private variant and its bounded constructor.
unsafe impl<Message> Sync for OutputMapper<Message> {}

impl<Message> Clone for OutputMapper<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Dynamic(map) => Self::Dynamic(Arc::clone(map)),
            Self::Constant(map) => Self::Constant(map.clone()),
        }
    }
}

impl<Message> Clone for ConstantOutputMapper<Message> {
    fn clone(&self) -> Self {
        let cloned = self
            .shared_message()
            .map_or(ConstantMessage::Transitioning, ConstantMessage::Shared);
        Self {
            message: Mutex::new(cloned),
            matches: self.matches,
            clone_message: self.clone_message,
        }
    }
}

impl<Message> ConstantOutputMapper<Message> {
    fn shared_message(&self) -> Option<Arc<Message>> {
        let mut message = lock_constant_message(&self.message);
        let current = std::mem::replace(&mut *message, ConstantMessage::Transitioning);
        match current {
            ConstantMessage::Inline(current) => {
                let current = Arc::new(current);
                let shared = Arc::clone(&current);
                *message = ConstantMessage::Shared(current);
                Some(shared)
            }
            ConstantMessage::Shared(current) => {
                let shared = Arc::clone(&current);
                *message = ConstantMessage::Shared(current);
                Some(shared)
            }
            ConstantMessage::Transitioning => None,
        }
    }

    fn map_output(&self, output: &WidgetOutput) -> Option<Message> {
        if !(self.matches)(output) {
            return None;
        }
        let message = self.shared_message()?;
        Some((self.clone_message)(message.as_ref()))
    }
}

fn lock_constant_message<Message>(
    message: &Mutex<ConstantMessage<Message>>,
) -> MutexGuard<'_, ConstantMessage<Message>> {
    match message.lock() {
        Ok(message) => message,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Shared mapper type that turns scroll movement into optional host-defined messages.
///
/// Scroll containers may update local runtime offset for sub-row or otherwise
/// unchanged movement without asking the host to reproject the surface.
pub type ScrollMessageMapper<Message> = Arc<dyn Fn(ScrollUpdate) -> Option<Message> + Send + Sync>;

/// Shared mapper type that turns native file-drop events into host-defined messages.
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
            native_file_drop: self.native_file_drop.as_ref().map(Arc::clone),
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

    /// Build a mapper for any typed widget output payload.
    pub fn typed<Output>(map: impl Fn(Output) -> Message + Send + Sync + 'static) -> Self
    where
        Output: Clone + Send + Sync + 'static,
    {
        Self::dynamic(move |output| output.typed_cloned::<Output>().map(&map))
    }

    /// Build an allocation-free binding that clones one message for matching outputs.
    ///
    /// The matcher must be non-capturing so the binding can store it as a function
    /// pointer alongside the message instead of allocating a dynamic callback.
    pub(crate) fn constant(message: Message, matches: fn(&WidgetOutput) -> bool) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self {
            map: Some(OutputMapper::Constant(ConstantOutputMapper {
                message: Mutex::new(ConstantMessage::Inline(message)),
                matches,
                clone_message: Message::clone,
            })),
            native_file_drop: None,
        }
    }

    /// Build a dynamic output mapper for custom widgets.
    pub fn dynamic(map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static) -> Self {
        Self {
            map: Some(OutputMapper::Dynamic(Arc::new(map))),
            native_file_drop: None,
        }
    }

    /// Return this mapper with native file-drop events mapped to host messages.
    pub fn with_native_file_drop(
        mut self,
        map: impl Fn(NativeFileDrop) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.native_file_drop = Some(Arc::new(map));
        self
    }

    pub(super) fn maps_any_output(&self) -> bool {
        self.map.is_some()
    }

    pub(super) fn uses_dynamic_output_callback(&self) -> bool {
        matches!(self.map, Some(OutputMapper::Dynamic(_)))
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
}
