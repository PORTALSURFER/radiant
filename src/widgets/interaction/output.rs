use std::{any::Any, sync::Arc};

/// Type-erased widget output payload.
#[derive(Clone)]
pub struct CustomWidgetOutput {
    payload: Arc<dyn Any + Send + Sync>,
}

impl CustomWidgetOutput {
    /// Build a custom widget output from any cloneable, thread-safe payload.
    pub fn new<T>(payload: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            payload: Arc::new(payload),
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
        Arc::ptr_eq(&self.payload, &other.payload)
    }
}

/// Typed widget output payload.
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
        T: Send + Sync + 'static,
    {
        Self {
            payload: CustomWidgetOutput::new(payload),
        }
    }

    /// Downcast this widget output to the requested payload type.
    pub fn typed_ref<T: 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }

    /// Build a user-defined widget output payload.
    pub fn custom<T>(payload: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self::typed(payload)
    }

    /// Downcast this widget output to the requested custom payload type.
    pub fn custom_ref<T: 'static>(&self) -> Option<&T> {
        self.typed_ref()
    }
}

impl std::fmt::Debug for WidgetOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetOutput").finish_non_exhaustive()
    }
}
