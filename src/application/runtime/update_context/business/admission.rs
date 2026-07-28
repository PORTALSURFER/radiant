use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU8, Ordering},
    sync::{Arc, Weak},
};

/// State of a business-task admission receipt.
///
/// A receipt starts as [`Pending`](Self::Pending) and is resolved by the UI
/// runtime controller once the host has actually accepted or rejected the
/// worker. `Closed` is used when the command is fenced, the controller closes,
/// or the latest-task transaction is otherwise unable to reach the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusinessTaskAdmission {
    /// The controller has not yet attempted host admission.
    Pending,
    /// The host accepted the worker.
    Accepted,
    /// The bounded controller or host lane rejected the worker.
    Rejected,
    /// The command/controller closed before admission could resolve.
    Closed,
}

/// Explicit name for the state returned by a [`BusinessTaskAdmissionReceipt`].
pub type BusinessTaskAdmissionReceiptState = BusinessTaskAdmission;

impl BusinessTaskAdmission {
    const PENDING: u8 = 0;
    const ACCEPTED: u8 = 1;
    const REJECTED: u8 = 2;
    const CLOSED: u8 = 3;

    const fn code(self) -> u8 {
        match self {
            Self::Pending => Self::PENDING,
            Self::Accepted => Self::ACCEPTED,
            Self::Rejected => Self::REJECTED,
            Self::Closed => Self::CLOSED,
        }
    }

    const fn from_code(code: u8) -> Self {
        match code {
            Self::ACCEPTED => Self::Accepted,
            Self::REJECTED => Self::Rejected,
            Self::CLOSED => Self::Closed,
            _ => Self::Pending,
        }
    }
}

/// Pollable, UI-local result for one business-task admission attempt.
///
/// Only this handle is retained by the UI owner. The runtime command carries
/// a weak reference, so dropping the receipt releases the shared state and
/// never leaves a runtime-owned callback or queue behind.
#[derive(Clone, Debug)]
pub struct BusinessTaskAdmissionReceipt {
    state: Arc<AtomicU8>,
    // Keep the receipt UI-local. The controller only receives a weak Arc and
    // therefore remains Send without making this handle cross-thread data.
    _ui_local: PhantomData<Rc<()>>,
}

impl BusinessTaskAdmissionReceipt {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(BusinessTaskAdmission::Pending.code())),
            _ui_local: PhantomData,
        }
    }

    /// Return the latest admission state without blocking.
    pub fn poll(&self) -> BusinessTaskAdmission {
        BusinessTaskAdmission::from_code(self.state.load(Ordering::Acquire))
    }

    /// Alias for [`Self::poll`].
    pub fn status(&self) -> BusinessTaskAdmission {
        self.poll()
    }

    /// Alias for [`Self::poll`].
    pub fn state(&self) -> BusinessTaskAdmissionReceiptState {
        self.poll()
    }

    pub(crate) fn weak(&self) -> Weak<AtomicU8> {
        Arc::downgrade(&self.state)
    }
}

pub(crate) fn resolve(weak: &Weak<AtomicU8>, state: BusinessTaskAdmission) {
    let Some(state_ref) = weak.upgrade() else {
        return;
    };
    // Resolution is monotonic. A late command drop or controller close must
    // not overwrite an already observed Accepted/Rejected result.
    let _ = state_ref.compare_exchange(
        BusinessTaskAdmission::Pending.code(),
        state.code(),
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

pub(crate) struct AdmissionReceiptGuard(pub(crate) Weak<AtomicU8>);

impl Drop for AdmissionReceiptGuard {
    fn drop(&mut self) {
        resolve(&self.0, BusinessTaskAdmission::Closed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_starts_pending_and_resolves_once() {
        let receipt = BusinessTaskAdmissionReceipt::new();
        assert_eq!(receipt.poll(), BusinessTaskAdmission::Pending);
        let weak = receipt.weak();
        resolve(&weak, BusinessTaskAdmission::Accepted);
        resolve(&weak, BusinessTaskAdmission::Rejected);
        assert_eq!(receipt.status(), BusinessTaskAdmission::Accepted);
    }

    #[test]
    fn dropping_receipt_releases_weak_state() {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let weak = receipt.weak();
        drop(receipt);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn guard_closes_pending_receipt() {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let weak = receipt.weak();
        {
            let _guard = AdmissionReceiptGuard(weak);
        }
        assert_eq!(receipt.poll(), BusinessTaskAdmission::Closed);
    }
}
