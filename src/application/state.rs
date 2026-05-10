use crate::widgets::WidgetSizing;
use std::sync::Arc;

/// A state mutation emitted by application builders with direct callbacks.
pub struct StateAction<State> {
    apply: Arc<dyn Fn(&mut State) + Send + Sync>,
}

impl<State> Clone for StateAction<State> {
    fn clone(&self) -> Self {
        Self {
            apply: Arc::clone(&self.apply),
        }
    }
}

pub(in crate::application) trait OptionalBaseline {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self;
}

impl OptionalBaseline for WidgetSizing {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self {
        if let Some(baseline) = baseline {
            self.with_baseline(baseline)
        } else {
            self
        }
    }
}

impl<State> StateAction<State> {
    pub(in crate::application) fn new(apply: impl Fn(&mut State) + Send + Sync + 'static) -> Self {
        Self {
            apply: Arc::new(apply),
        }
    }

    pub(in crate::application) fn run(&self, state: &mut State) {
        (self.apply)(state);
    }
}
