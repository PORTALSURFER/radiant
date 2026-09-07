//! Presentation-only notices and bounded application-owned data.
mod model;
pub use model::{
    Notice, NoticeAction, NoticeDismissal, NoticeDismissalReason, NoticeError, NoticeId,
    NoticeQueue, NoticeSeverity, NoticeSnapshot,
};

mod center;
pub(crate) use center::NoticeDemand;
pub use center::{NoticePlacement, NotificationCenter, notifications};
