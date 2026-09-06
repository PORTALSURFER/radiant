//! Bounded UI-local restoration evidence retired with widget ownership.
use super::{
    FocusTransferOutcome, SurfaceRuntime,
    focus::FocusTransition,
    interaction_state::{RuntimeFocusOwner, RuntimeSplitPaneSeparatorFocusOwner},
};
use crate::{
    gui::automation::{AutomationNodeId, AutomationRole},
    runtime::RuntimeBridge,
    widgets::WidgetId,
};
use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

/// UI-local bookmark for one continuous widget or separator incarnation in one runtime.
/// Compatible refreshes may preserve it; removal or replacement permanently retires it.
#[derive(Clone, Debug)]
pub struct FocusBookmark {
    runtime: u64,
    owner: BookmarkOwner,
}

#[derive(Clone, Debug)]
enum BookmarkOwner {
    Widget {
        widget: WidgetId,
        stamp: Rc<()>,
        path: Vec<AutomationNodeId>,
        role: AutomationRole,
    },
    Separator(RuntimeSplitPaneSeparatorFocusOwner),
}

/// Failure to record a bounded focus bookmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusBookmarkError {
    /// There is no current eligible focus owner.
    NoFocus,
    /// The runtime is closed.
    Unavailable,
    /// Bookmarks already retain 64 distinct live widget incarnations.
    Capacity,
}

#[derive(Default)]
pub(super) struct FocusRestorationState {
    stamps: HashMap<WidgetId, Weak<()>>,
}
impl FocusRestorationState {
    pub(super) fn retain_live(&mut self, mut live: impl FnMut(WidgetId) -> bool) {
        self.stamps
            .retain(|id, stamp| stamp.strong_count() > 0 && live(*id));
    }
    pub(super) fn retire(&mut self, widget: WidgetId) {
        self.stamps.remove(&widget);
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Record the current focus owner without moving focus or creating virtual demand.
    /// At most 64 distinct live widget bookmarks are retained; dropping the last clone
    /// releases its slot at the next capture. Separator bookmarks use existing mount identities.
    pub fn capture_focus(&mut self) -> Result<FocusBookmark, FocusBookmarkError> {
        if !self.lifecycle_accepts_work() {
            return Err(FocusBookmarkError::Unavailable);
        }
        if let Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) = self.interaction.focus.owner {
            if !self.separator_focus_owner_is_current(owner) {
                return Err(FocusBookmarkError::NoFocus);
            }
            return Ok(FocusBookmark {
                runtime: self.runtime_identity(),
                owner: BookmarkOwner::Separator(owner),
            });
        }
        let widget = self.focused_widget().ok_or(FocusBookmarkError::NoFocus)?;
        let target = self
            .focus_target(widget)
            .ok_or(FocusBookmarkError::NoFocus)?;
        let stamps = &mut self.interaction.focus_restoration.stamps;
        stamps.retain(|_, stamp| stamp.strong_count() > 0);
        let stamp = if let Some(stamp) = stamps.get(&widget).and_then(Weak::upgrade) {
            stamp
        } else {
            if stamps.len() == 64 {
                return Err(FocusBookmarkError::Capacity);
            }
            let stamp = Rc::new(());
            stamps.insert(widget, Rc::downgrade(&stamp));
            stamp
        };
        Ok(FocusBookmark {
            runtime: self.runtime_identity(),
            owner: BookmarkOwner::Widget {
                widget,
                stamp,
                path: target.target.path,
                role: target.target.role,
            },
        })
    }

    /// Restore one continuous widget or separator incarnation using fresh target authority.
    /// Stale bookmarks are inert and never select a replacement with the same ID.
    pub fn restore_focus(&mut self, bookmark: &FocusBookmark) -> FocusTransferOutcome {
        if !self.lifecycle_accepts_work() {
            return FocusTransferOutcome::Unavailable;
        }
        if bookmark.runtime != self.runtime_identity() {
            return FocusTransferOutcome::Stale;
        }
        match &bookmark.owner {
            BookmarkOwner::Widget {
                widget,
                stamp,
                path,
                role,
            } => {
                if !self
                    .interaction
                    .focus_restoration
                    .stamps
                    .get(widget)
                    .and_then(Weak::upgrade)
                    .is_some_and(|current| Rc::ptr_eq(&current, stamp))
                {
                    return FocusTransferOutcome::Stale;
                }
                let Some(target) = self.focus_target(*widget) else {
                    return FocusTransferOutcome::Unavailable;
                };
                if target.target.path != *path || target.target.role != *role {
                    return FocusTransferOutcome::Stale;
                }
                self.transfer_focus(&target)
            }
            BookmarkOwner::Separator(owner) => {
                if !self.separator_focus_owner_is_current(*owner) {
                    return FocusTransferOutcome::Stale;
                }
                let Some(projection) = self.current_split_pane_separator_projection(owner.target)
                else {
                    return FocusTransferOutcome::Stale;
                };
                match self.request_split_pane_separator_focus(projection) {
                    FocusTransition::Vetoed => FocusTransferOutcome::Vetoed,
                    FocusTransition::InvalidTarget => FocusTransferOutcome::Invalidated,
                    FocusTransition::Changed | FocusTransition::Unchanged => {
                        FocusTransferOutcome::AdmittedRuntimeOwned
                    }
                }
            }
        }
    }
}
