use super::{model::NoticeToken, *};
use crate::{
    application::{Layer, ViewNode, button, column, row, text},
    layout::{CrossAlign, MainAlign},
};
use std::{rc::Rc, time::Duration};

pub(crate) struct NoticeDemand<Message> {
    pub token: NoticeToken,
    pub timeout: Option<Duration>,
    pub on_dismiss: Rc<dyn Fn(NoticeDismissal) -> Message>,
}

/// Logical corner used for the ordinary floating notification layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoticePlacement {
    /// Top at the reading-order start edge.
    TopStart,
    /// Top at the reading-order end edge.
    TopEnd,
    /// Bottom at the reading-order start edge.
    BottomStart,
    /// Bottom at the reading-order end edge.
    #[default]
    BottomEnd,
}
/// Project an immutable queue snapshot. Timers start only after runtime admission.
pub fn notifications<Message>(snapshot: NoticeSnapshot) -> NotificationCenter<Message> {
    NotificationCenter {
        snapshot,
        placement: NoticePlacement::default(),
        visible: 4,
        on_dismiss: None,
        on_action: None,
    }
}
/// Bounded notification presentation using existing floating-layer and button semantics.
pub struct NotificationCenter<Message> {
    snapshot: NoticeSnapshot,
    placement: NoticePlacement,
    visible: usize,
    on_dismiss: Option<Rc<dyn Fn(NoticeDismissal) -> Message>>,
    on_action: Option<Rc<dyn Fn(NoticeAction) -> Message>>,
}
impl<Message: 'static> NotificationCenter<Message> {
    /// Place notices at a logical corner without moving keyboard focus.
    pub fn placement(mut self, placement: NoticePlacement) -> Self {
        self.placement = placement;
        self
    }
    /// Bound visible notices to one through eight. Remaining notices stay queued.
    pub fn max_visible(mut self, count: usize) -> Self {
        self.visible = count.clamp(1, 8);
        self
    }
    /// Map manual and elapsed-time dismissal requests to ordinary application updates.
    /// Without this callback notices do not expire or offer a dismiss control.
    pub fn on_dismiss(mut self, callback: impl Fn(NoticeDismissal) -> Message + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(callback));
        self
    }
    /// Map semantic action tokens to ordinary updates; consume with `NoticeQueue::take_action`.
    pub fn on_action(mut self, callback: impl Fn(NoticeAction) -> Message + 'static) -> Self {
        self.on_action = Some(Rc::new(callback));
        self
    }
    /// Build a pass-through floating layer below modal content.
    pub fn layer(self) -> Layer<Message> {
        let entries = self.snapshot.into_entries_with_tokens();
        let cards = entries
            .into_iter()
            .take(self.visible)
            .map(|(entry, token)| {
                let mut content = vec![
                    text(format!("{:?}", entry.notice.severity)),
                    text(entry.notice.message.to_string()).wrap(),
                ];
                let mut actions = Vec::new();
                if let (Some((label, _)), Some(mapper)) = (&entry.notice.action, &self.on_action) {
                    let token = token.clone();
                    let mapper = mapper.clone();
                    actions.push(button(label.to_string()).filter_mapped(move |message| {
                        (message.is_activate() && token.is_live()).then(|| {
                            mapper(NoticeAction {
                                token: token.clone(),
                            })
                        })
                    }));
                }
                if let Some(mapper) = &self.on_dismiss {
                    let token = token.clone();
                    let mapper = mapper.clone();
                    actions.push(button("Dismiss").filter_mapped(move |message| {
                        (message.is_activate() && token.is_live()).then(|| {
                            mapper(NoticeDismissal {
                                token: token.clone(),
                                reason: NoticeDismissalReason::User,
                            })
                        })
                    }));
                }
                if !actions.is_empty() {
                    content.push(row(actions).spacing(6.0));
                }
                let mut card = column(content)
                    .width(320.0)
                    .style(crate::widgets::WidgetStyle::default())
                    .padding(8.0)
                    .spacing(4.0)
                    .key(format!("notice-{}", entry.notice.id.0));
                if let Some(mapper) = &self.on_dismiss {
                    card.notice_demand = Some(Rc::new(NoticeDemand {
                        token,
                        timeout: entry.notice.timeout,
                        on_dismiss: mapper.clone(),
                    }));
                }
                card
            })
            .collect::<Vec<ViewNode<Message>>>();
        let main = match self.placement {
            NoticePlacement::TopStart | NoticePlacement::TopEnd => MainAlign::Start,
            _ => MainAlign::End,
        };
        let cross = match self.placement {
            NoticePlacement::TopStart | NoticePlacement::BottomStart => CrossAlign::Start,
            _ => CrossAlign::End,
        };
        Layer::floating(
            column(cards)
                .fill()
                .padding(12.0)
                .spacing(8.0)
                .align_main(main)
                .align_cross(cross),
        )
        .pass_through()
    }
}
