//! Host-message mapping for surface widget leaves.

use crate::{
    runtime::{NativeFileDrop, ScrollUpdate},
    widgets::WidgetOutput,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

type DynamicOutputMapper<Message> = EventMapper<WidgetOutput, Option<Message>>;

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
            Self::Dynamic(map) => Self::Dynamic(map.clone()),
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

/// A UI-local event mapper with optional exact, typed equality evidence.
///
/// `EventMapper::new` is conservative: its callback is intentionally opaque to
/// reconciliation. `EventMapper::with_revision` is an explicit opt-in for a
/// caller that can prove that the mapper's captured behavior is represented by
/// an `Eq` value. The mapper remains UI-local and is deliberately not
/// `Send`/`Sync`.
pub struct EventMapper<Input, Message> {
    map: MapperCallback<Input, Message>,
    evidence: MapperEvidence,
}

impl<Input, Message> Clone for EventMapper<Input, Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            evidence: self.evidence.clone(),
        }
    }
}

impl<Input, Message> EventMapper<Input, Message> {
    pub(crate) fn from_rc(map: Rc<dyn Fn(Input) -> Message>) -> Self {
        Self {
            map: MapperCallback::Rc(map),
            evidence: MapperEvidence::Conservative,
        }
    }

    pub(crate) fn from_arc(map: Arc<dyn Fn(Input) -> Message + Send + Sync>) -> Self {
        Self {
            map: MapperCallback::Arc(map),
            evidence: MapperEvidence::Conservative,
        }
    }

    /// Build a conservative mapper from an ordinary UI-local callback.
    pub fn new(map: impl Fn(Input) -> Message + 'static) -> Self {
        Self {
            map: MapperCallback::Rc(Rc::new(map)),
            evidence: MapperEvidence::Conservative,
        }
    }

    /// Build a mapper with exact equality evidence for its captured behavior.
    pub fn with_revision<Revision>(
        revision: Revision,
        map: impl Fn(Input) -> Message + 'static,
    ) -> Self
    where
        Revision: Eq + 'static,
    {
        Self {
            map: MapperCallback::Rc(Rc::new(map)),
            evidence: MapperEvidence::Exact(Rc::new(RevisionEvidenceValue(revision))),
        }
    }

    /// Adapt a typed mapper to widget-output routing without inspecting or
    /// invoking its callback during reconciliation.
    pub fn typed_mapped(self) -> EventMapper<WidgetOutput, Option<Message>>
    where
        Input: Clone + 'static,
        Message: 'static,
    {
        let EventMapper { map, evidence } = self;
        EventMapper {
            map: MapperCallback::Rc(Rc::new(move |output| {
                output
                    .typed_cloned::<Input>()
                    .map(|input| map.invoke(input))
            })),
            evidence,
        }
    }

    /// Invoke the mapper with one event payload.
    pub fn invoke(&self, input: Input) -> Message {
        self.map.invoke(input)
    }

    /// Alias for [`EventMapper::invoke`].
    pub fn map(&self, input: Input) -> Message {
        self.invoke(input)
    }

    pub(crate) fn descriptor(&self) -> MapperDescriptor {
        match &self.evidence {
            MapperEvidence::Conservative => MapperDescriptor::Conservative,
            MapperEvidence::Exact(revision) => MapperDescriptor::Exact(Rc::clone(revision)),
        }
    }
}

enum MapperCallback<Input, Message> {
    Rc(Rc<dyn Fn(Input) -> Message>),
    Arc(Arc<dyn Fn(Input) -> Message + Send + Sync>),
}

impl<Input, Message> Clone for MapperCallback<Input, Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Rc(map) => Self::Rc(Rc::clone(map)),
            Self::Arc(map) => Self::Arc(Arc::clone(map)),
        }
    }
}

impl<Input, Message> MapperCallback<Input, Message> {
    fn invoke(&self, input: Input) -> Message {
        match self {
            Self::Rc(map) => map(input),
            Self::Arc(map) => map(input),
        }
    }
}

pub(crate) trait RevisionEvidence: Any {
    fn equals(&self, other: &dyn RevisionEvidence) -> bool;
}

struct RevisionEvidenceValue<Revision>(Revision);

impl<Revision> RevisionEvidence for RevisionEvidenceValue<Revision>
where
    Revision: Eq + 'static,
{
    fn equals(&self, other: &dyn RevisionEvidence) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0 == other.0)
    }
}

impl dyn RevisionEvidence {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
enum MapperEvidence {
    Conservative,
    Exact(Rc<dyn RevisionEvidence>),
}

#[derive(Clone)]
pub(crate) enum MapperDescriptor {
    Absent,
    Conservative,
    Exact(Rc<dyn RevisionEvidence>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapperRelation {
    Unchanged,
    Interaction,
    Structural,
}

impl MapperDescriptor {
    pub(crate) fn relation(&self, other: &Self) -> MapperRelation {
        match (self, other) {
            (Self::Absent, Self::Absent) => MapperRelation::Unchanged,
            (Self::Exact(left), Self::Exact(right)) if left.equals(right.as_ref()) => {
                MapperRelation::Unchanged
            }
            (Self::Exact(_), Self::Exact(_))
            | (Self::Absent, Self::Exact(_))
            | (Self::Exact(_), Self::Absent) => MapperRelation::Interaction,
            _ => MapperRelation::Structural,
        }
    }
}

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
    native_file_drop: Option<EventMapper<NativeFileDrop, Message>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            native_file_drop: self.native_file_drop.clone(),
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
        Self::dynamic_mapped(EventMapper::new(map))
    }

    /// Build a dynamic output mapper with optional exact equality evidence.
    pub fn dynamic_mapped(map: EventMapper<WidgetOutput, Option<Message>>) -> Self {
        Self {
            map: Some(OutputMapper::Dynamic(map)),
            native_file_drop: None,
        }
    }

    /// Build a button-output mapper while preserving typed equality evidence.
    pub fn button_mapped(map: EventMapper<crate::widgets::ButtonMessage, Message>) -> Self
    where
        Message: 'static,
    {
        Self::dynamic_mapped(map.typed_mapped())
    }

    /// Build a toggle-output mapper while preserving typed equality evidence.
    pub fn toggle_mapped(map: EventMapper<crate::widgets::ToggleMessage, Message>) -> Self
    where
        Message: 'static,
    {
        Self::dynamic_mapped(map.typed_mapped())
    }

    /// Return this mapper with native file-drop events mapped to host messages.
    pub fn with_native_file_drop(
        mut self,
        map: impl Fn(NativeFileDrop) -> Message + 'static,
    ) -> Self {
        self.native_file_drop = Some(EventMapper::new(map));
        self
    }

    /// Return this mapper with a native file-drop mapper carrying optional
    /// exact equality evidence.
    pub(super) fn with_native_file_drop_mapped(
        mut self,
        map: EventMapper<NativeFileDrop, Message>,
    ) -> Self {
        self.native_file_drop = Some(map);
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
    pub(in crate::runtime::surface) fn output_mapper_descriptor(&self) -> MapperDescriptor {
        match self.map.as_ref() {
            Some(OutputMapper::Dynamic(map)) => map.descriptor(),
            Some(OutputMapper::Constant(_)) => MapperDescriptor::Conservative,
            None => MapperDescriptor::Absent,
        }
    }

    pub(in crate::runtime::surface) fn native_file_drop_mapper_descriptor(
        &self,
    ) -> MapperDescriptor {
        self.native_file_drop
            .as_ref()
            .map(EventMapper::descriptor)
            .unwrap_or(MapperDescriptor::Absent)
    }

    pub(super) fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        match self.map.as_ref()? {
            OutputMapper::Dynamic(map) => map.invoke(output),
            OutputMapper::Constant(map) => map.map_output(&output),
        }
    }

    pub(super) fn map_native_file_drop(&self, drop: NativeFileDrop) -> Option<Message> {
        self.native_file_drop.as_ref().map(|map| map.invoke(drop))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{
        ButtonMessage, InteractionProvenance, PointerModifiers, TextInputMessage,
    };
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
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
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
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
                .is_some()
        );
        assert!(
            cloned
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
                .is_some()
        );
        assert!(
            cloned_again
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
                .is_some()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn dynamic_and_filter_mapped_callbacks_remain_available() {
        let mapped = WidgetMessageMapper::button(|message| message.is_activate());
        assert_eq!(
            mapped.map_output(WidgetOutput::typed(ButtonMessage::Activate {
                provenance: crate::widgets::InteractionProvenance::Programmatic,
            })),
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
            filtered.map_output(WidgetOutput::typed(ButtonMessage::Activate {
                provenance: crate::widgets::InteractionProvenance::Programmatic,
            })),
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
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
                .is_some()
        );
        assert!(
            clone
                .map_output(WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }))
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
            mapper.map_output(WidgetOutput::typed(ButtonMessage::Activate {
                provenance: crate::widgets::InteractionProvenance::Programmatic,
            })),
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

    #[derive(PartialEq, Eq)]
    struct Revision(u32);

    #[test]
    fn event_mapper_equality_is_typed_conservative_and_cloneable() {
        let exact = EventMapper::with_revision(Revision(7), |value: u32| value + 1);
        let equal = EventMapper::with_revision(Revision(7), |value: u32| value + 2);
        let changed = EventMapper::with_revision(Revision(8), |value: u32| value + 3);
        let different_type = EventMapper::with_revision(7u32, |value: u32| value + 4);
        let conservative = EventMapper::new(|value: u32| value + 5);

        assert_eq!(
            exact.descriptor().relation(&equal.descriptor()),
            MapperRelation::Unchanged
        );
        assert_eq!(
            exact.descriptor().relation(&changed.descriptor()),
            MapperRelation::Interaction
        );
        assert_eq!(
            exact.descriptor().relation(&different_type.descriptor()),
            MapperRelation::Interaction
        );
        assert_eq!(
            exact.descriptor().relation(&conservative.descriptor()),
            MapperRelation::Structural
        );
        assert_eq!(
            exact.descriptor().relation(&MapperDescriptor::Absent),
            MapperRelation::Interaction
        );

        let cloned = exact.clone();
        assert_eq!(cloned.invoke(4), 5);
    }

    #[test]
    fn event_mapper_does_not_execute_during_equality() {
        let calls = Rc::new(RefCell::new(0usize));
        let calls_for_mapper = Rc::clone(&calls);
        let mapper = EventMapper::with_revision(Revision(1), move |value: u32| {
            *calls_for_mapper.borrow_mut() += 1;
            value + 1
        });
        let same = EventMapper::with_revision(Revision(1), |value: u32| value + 9);

        assert_eq!(
            mapper.descriptor().relation(&same.descriptor()),
            MapperRelation::Unchanged
        );
        assert_eq!(*calls.borrow(), 0);
        assert_eq!(mapper.invoke(1), 2);
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn typed_mapped_preserves_evidence_and_invokes_only_on_output() {
        let calls = Rc::new(RefCell::new(0usize));
        let expected = ButtonMessage::ActivateWithModifiers {
            provenance: InteractionProvenance::Pointer {
                modifiers: PointerModifiers {
                    command: true,
                    shift: false,
                    alt: true,
                },
                timestamp: None,
                sequence_range: None,
            },
        };
        let expected_for_mapper = expected;
        let calls_for_mapper = Rc::clone(&calls);
        let mapper = EventMapper::with_revision(Revision(4), move |message: ButtonMessage| {
            *calls_for_mapper.borrow_mut() += 1;
            assert_eq!(message, expected_for_mapper);
        })
        .typed_mapped();
        let equal =
            EventMapper::with_revision(Revision(4), |_message: ButtonMessage| {}).typed_mapped();

        assert_eq!(
            mapper.descriptor().relation(&equal.descriptor()),
            MapperRelation::Unchanged
        );
        assert_eq!(*calls.borrow(), 0);
        assert_eq!(mapper.invoke(WidgetOutput::typed(expected)), Some(()));
        assert_eq!(*calls.borrow(), 1);
        assert_eq!(
            mapper.invoke(WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("ignored"),
            })),
            None
        );
        assert_eq!(*calls.borrow(), 1);
    }

    struct UiDropProbe(Rc<RefCell<bool>>);

    impl Drop for UiDropProbe {
        fn drop(&mut self) {
            *self.0.borrow_mut() = true;
        }
    }
}
