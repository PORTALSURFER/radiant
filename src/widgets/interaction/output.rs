use std::{any::Any, rc::Rc};

/// Type-erased widget output payload stored on the owning UI runtime.
///
/// Widget outputs are synchronous UI-local values. They may contain `Rc`,
/// `RefCell`, or other non-thread-safe state and must not be transferred to a
/// worker or subscription lane.
#[derive(Clone)]
pub struct CustomWidgetOutput {
    payload: Rc<dyn Any>,
}

impl CustomWidgetOutput {
    /// Build a custom widget output from any `'static` UI-local payload.
    pub fn new<T>(payload: T) -> Self
    where
        T: 'static,
    {
        Self {
            payload: Rc::new(payload),
        }
    }

    /// Downcast this output payload to the requested type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }
}

impl std::fmt::Debug for CustomWidgetOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomWidgetOutput").finish_non_exhaustive()
    }
}

impl PartialEq for CustomWidgetOutput {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.payload, &other.payload)
    }
}

/// Typed widget output payload owned by the synchronous UI runtime.
///
/// Outputs are intentionally open: a widget emits its own message type with
/// [`WidgetOutput::typed`], and message mappers downcast only the payload types
/// they understand. Adding a widget does not require adding a central enum
/// variant.
#[derive(Clone, PartialEq)]
pub struct WidgetOutput {
    payload: CustomWidgetOutput,
}

impl WidgetOutput {
    /// Build a typed widget output payload.
    pub fn typed<T>(payload: T) -> Self
    where
        T: 'static,
    {
        Self {
            payload: CustomWidgetOutput::new(payload),
        }
    }

    /// Downcast this widget output to the requested payload type.
    pub fn typed_ref<T: 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }

    /// Downcast and clone this widget output payload.
    pub fn typed_cloned<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.typed_ref::<T>().cloned()
    }

    /// Downcast and copy this widget output payload.
    pub fn typed_copied<T>(&self) -> Option<T>
    where
        T: Copy + 'static,
    {
        self.typed_ref::<T>().copied()
    }

    /// Build a user-defined widget output payload.
    pub fn custom<T>(payload: T) -> Self
    where
        T: 'static,
    {
        Self::typed(payload)
    }

    /// Downcast this widget output to the requested custom payload type.
    pub fn custom_ref<T: 'static>(&self) -> Option<&T> {
        self.typed_ref()
    }

    /// Downcast and clone this user-defined widget output payload.
    pub fn custom_cloned<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.typed_cloned()
    }

    /// Downcast and copy this user-defined widget output payload.
    pub fn custom_copied<T>(&self) -> Option<T>
    where
        T: Copy + 'static,
    {
        self.typed_copied()
    }
}

impl std::fmt::Debug for WidgetOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetOutput").finish_non_exhaustive()
    }
}
