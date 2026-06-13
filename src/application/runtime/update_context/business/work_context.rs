use crate::application::CancellationToken;

/// Context supplied to a business worker closure.
#[derive(Clone, Debug)]
pub struct BusinessWorkContext {
    cancellation: Option<CancellationToken>,
}

impl BusinessWorkContext {
    pub(super) fn new(cancellation: Option<CancellationToken>) -> Self {
        Self { cancellation }
    }

    /// Return whether cooperative cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
    }
}
