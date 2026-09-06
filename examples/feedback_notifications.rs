//! Application-owned notices with runtime-owned visible timeout policy.
use radiant::{
    application::{
        IntoView, Layer, Notice, NoticeAction, NoticeDismissal, NoticeQueue, NoticeSeverity,
        StatusSemantic, column, inline_error, notifications, scene, spinner, status_badge, text,
    },
    gui::types::{Point, Vector2},
    runtime::{Command, Event, RuntimeBridge, UiSurface, testing::DeterministicHost},
};
use std::{sync::Arc, time::Duration};
struct Model {
    queue: NoticeQueue,
    visible: bool,
    modal: bool,
    dismissals: usize,
    actions: usize,
    ignore: bool,
}
enum Message {
    Dismiss(NoticeDismissal),
    Action(NoticeAction),
    Modal(bool),
    Visible(bool),
    Replace,
}
impl RuntimeBridge<Message> for Model {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        let notices = self.visible.then(|| {
            notifications(self.queue.snapshot())
                .max_visible(1)
                .on_dismiss(Message::Dismiss)
                .on_action(Message::Action)
                .layer()
        });
        Arc::new(
            scene(column([
                text("Application"),
                spinner().label("Syncing").view(),
                inline_error("Previous sync failed").view(),
                status_badge(StatusSemantic::Success).label("Saved").view(),
            ]))
            .layer_opt(notices)
            .layer_opt(
                self.modal
                    .then(|| Layer::modal(text("Modal")).block_input()),
            )
            .into_view()
            .into_surface(),
        )
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dismiss(event) => {
                self.dismissals += 1;
                if !self.ignore {
                    self.queue.dismiss(&event);
                }
            }
            Message::Action(event) => {
                if self.queue.take_action(&event) == Some(7) {
                    self.actions += 1;
                }
            }
            Message::Modal(value) => self.modal = value,
            Message::Visible(value) => self.visible = value,
            Message::Replace => {
                self.queue.push(notice(1)).unwrap();
            }
        }
        Command::none()
    }
}
fn notice(id: u64) -> Notice {
    Notice::new(id, NoticeSeverity::Info, format!("Notice {id}"))
        .unwrap()
        .action("Open", 7)
        .unwrap()
        .timeout(Some(Duration::from_secs(1)))
        .unwrap()
}
fn host() -> DeterministicHost<Model, Message> {
    let mut queue = NoticeQueue::new();
    queue.push(notice(1)).unwrap();
    DeterministicHost::with_default_config(
        Model {
            queue,
            visible: true,
            modal: false,
            dismissals: 0,
            actions: 0,
            ignore: false,
        },
        Vector2::new(400.0, 300.0),
    )
    .unwrap()
}
fn main() {
    let mut host = host();
    host.dispatch_message(Message::Modal(true)).unwrap();
    host.dispatch_message(Message::Modal(false)).unwrap();
    host.dispatch_message(Message::Visible(false)).unwrap();
    host.dispatch_message(Message::Visible(true)).unwrap();
    host.dispatch_message(Message::Replace).unwrap();
    host.advance_time(Duration::from_secs(1)).unwrap();
    assert!(host.bridge().queue.is_empty());
    println!(
        "{{\"phase\":\"expired\",\"dismissals\":{}}}",
        host.bridge().dismissals
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_timeout_delivers_one_ordinary_update() {
        main();
    }
    #[test]
    fn ignored_dismissal_does_not_repeat() {
        let mut h = host();
        h.bridge_mut().ignore = true;
        h.advance_time(Duration::from_secs(1)).unwrap();
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }
    #[test]
    fn hidden_time_does_not_consume_remaining_budget() {
        let mut h = host();
        h.advance_time(Duration::from_millis(400)).unwrap();
        h.set_animation_hidden(true).unwrap();
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.set_animation_hidden(false).unwrap();
        h.advance_time(Duration::from_millis(599)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.advance_time(Duration::from_millis(1)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }
    #[test]
    fn modal_time_is_paused() {
        let mut h = host();
        h.advance_time(Duration::from_millis(400)).unwrap();
        h.dispatch_message(Message::Modal(true)).unwrap();
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.dispatch_message(Message::Modal(false)).unwrap();
        h.advance_time(Duration::from_millis(600)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }
    #[test]
    fn replacement_restarts_and_removed_projection_retires() {
        let mut h = host();
        h.advance_time(Duration::from_millis(800)).unwrap();
        h.dispatch_message(Message::Replace).unwrap();
        h.advance_time(Duration::from_millis(800)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.dispatch_message(Message::Visible(false)).unwrap();
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.dispatch_message(Message::Visible(true)).unwrap();
        h.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }
    #[test]
    fn queued_notice_gets_full_budget_when_admitted() {
        let mut h = host();
        h.bridge_mut().queue.push(notice(2)).unwrap();
        h.dispatch_message(Message::Visible(true)).unwrap();
        h.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(h.bridge().queue.len(), 1);
        h.advance_time(Duration::from_millis(999)).unwrap();
        assert_eq!(h.bridge().queue.len(), 1);
        h.advance_time(Duration::from_millis(1)).unwrap();
        assert!(h.bridge().queue.is_empty());
    }
    #[test]
    fn pointer_outside_does_not_pause() {
        let mut h = host();
        h.dispatch_event(Event::pointer_move(Point::new(0.0, 0.0)))
            .unwrap();
        h.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }

    fn find_label<'a>(
        node: &'a radiant::gui::automation::AutomationNodeSnapshot,
        label: &str,
    ) -> Option<&'a radiant::gui::automation::AutomationNodeSnapshot> {
        if node.label.as_deref() == Some(label) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_label(child, label))
    }
    #[test]
    fn semantic_labels_match_visible_feedback_and_notice() {
        let h = host();
        let snapshot = h.snapshot().unwrap();
        for label in [
            "Syncing",
            "Notice 1",
            "Dismiss",
            "Saved",
            "Error",
            "Previous sync failed",
        ] {
            assert!(
                find_label(&snapshot.automation.root, label).is_some(),
                "{label}"
            );
            assert!(h.paint_plan().contains_text(label));
        }
    }
    #[test]
    fn hover_preserves_remaining_budget() {
        let mut h = host();
        let snapshot = h.snapshot().unwrap();
        let point = find_label(&snapshot.automation.root, "Dismiss")
            .unwrap()
            .bounds
            .center();
        h.advance_time(Duration::from_millis(400)).unwrap();
        h.dispatch_event(Event::pointer_move(Point::new(point.x, point.y)))
            .unwrap();
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.dispatch_event(Event::pointer_move(Point::new(0.0, 0.0)))
            .unwrap();
        h.advance_time(Duration::from_millis(600)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }
    #[test]
    fn keyboard_focus_preserves_actionable_notice() {
        let mut h = host();
        h.advance_time(Duration::from_millis(400)).unwrap();
        h.dispatch_event(Event::traverse_focus(
            radiant::runtime::FocusTraversal::Forward,
        ))
        .unwrap();
        assert!(h.runtime().focused_widget().is_some());
        h.advance_time(Duration::from_secs(10)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
        h.dispatch_event(Event::clear_focus()).unwrap();
        h.advance_time(Duration::from_millis(600)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }

    #[test]
    fn critical_notice_is_persistent_by_default() {
        let mut h = host();
        h.bridge_mut()
            .queue
            .push(Notice::new(1, NoticeSeverity::Critical, "Attention").unwrap())
            .unwrap();
        h.dispatch_message(Message::Visible(true)).unwrap();
        h.advance_time(Duration::from_secs(60)).unwrap();
        assert_eq!(h.bridge().dismissals, 0);
    }
    #[test]
    fn reduced_motion_stops_feedback_but_keeps_timeout_policy() {
        let mut h = host();
        let old = h.runtime().window_environment();
        h.set_window_environment(radiant::runtime::WindowEnvironment::new(
            old.display_scale(),
            old.color_scheme(),
            old.contrast(),
            true,
        ))
        .unwrap();
        assert_eq!(
            h.runtime()
                .declarative_animation_status()
                .feedback_instances,
            0
        );
        h.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(h.bridge().dismissals, 1);
    }

    #[test]
    fn notice_action_uses_normal_update_and_arrivals_preserve_focus() {
        let mut h = host();
        let snapshot = h.snapshot().unwrap();
        let center = find_label(&snapshot.automation.root, "Open")
            .unwrap()
            .bounds
            .center();
        let point = Point::new(center.x, center.y);
        h.dispatch_event(Event::pointer_press(
            point,
            radiant::widgets::PointerButton::Primary,
            Default::default(),
        ))
        .unwrap();
        h.dispatch_event(Event::pointer_release(
            point,
            radiant::widgets::PointerButton::Primary,
            Default::default(),
        ))
        .unwrap();
        assert_eq!(h.bridge().actions, 1);
        let focused = h.runtime().focused_widget();
        assert!(focused.is_some());
        h.bridge_mut()
            .queue
            .push(Notice::new(2, NoticeSeverity::Critical, "New critical notice").unwrap())
            .unwrap();
        h.dispatch_message(Message::Visible(true)).unwrap();
        assert_eq!(h.runtime().focused_widget(), focused);
        assert!(h.paint_plan().contains_text("Notice 1"));
        assert!(!h.paint_plan().contains_text("New critical notice"));
    }
}
