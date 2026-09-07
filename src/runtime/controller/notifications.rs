//! Accepted, bounded notice lifetime accounting on the central runtime clock.
use super::SurfaceRuntime;
use crate::{
    application::{NoticeDismissal, NoticeDismissalReason},
    runtime::{RuntimeBridge, surface::ProjectedNoticeDescriptor},
};
use std::time::{Duration, Instant};

struct Record<Message> {
    descriptor: ProjectedNoticeDescriptor<Message>,
    remaining: Option<Duration>,
    last: Instant,
    paused: bool,
    delivered: bool,
}
pub(super) struct Notifications<Message> {
    records: Vec<Record<Message>>,
    deadline: Option<Instant>,
    modal: bool,
}
impl<Message> Default for Notifications<Message> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            deadline: None,
            modal: false,
        }
    }
}
impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_notifications(&mut self) {
        if !self.lifecycle_accepts_work() {
            return;
        }
        let now = self.timed_repaint_now();
        self.update_notice_pause(now);
        let desired = self.surface.notice_descriptors().unwrap_or_default();
        self.notifications.modal = !desired.is_empty() && self.surface.has_notice_modal();
        let mut old = std::mem::take(&mut self.notifications.records);
        for descriptor in desired {
            if !descriptor.demand.token.is_live() {
                continue;
            }
            let previous = old.iter().position(|record| {
                record.descriptor.identity == descriptor.identity
                    && record
                        .descriptor
                        .demand
                        .token
                        .same(&descriptor.demand.token)
            });
            let record = if let Some(index) = previous {
                let mut record = old.swap_remove(index);
                record.descriptor = descriptor;
                record
            } else {
                Record {
                    remaining: descriptor.demand.timeout,
                    descriptor,
                    last: now,
                    paused: true,
                    delivered: false,
                }
            };
            self.notifications.records.push(record);
        }
        self.update_notice_pause(now);
    }
    pub(super) fn notice_deadline(&self) -> Option<Instant> {
        self.notifications.deadline
    }
    pub(super) fn clear_notifications(&mut self) {
        self.notifications = Default::default();
    }
    pub(super) fn update_notice_pause(&mut self, now: Instant) {
        let hidden = self.declarative_animation_status().hidden;
        let modal = self.notifications.modal;
        let pointer = self.interaction.pointer.current_position;
        let focus = self
            .interaction
            .focus
            .owner
            .and_then(|owner| owner.widget_id());
        let mut deadline = None;
        for record in &mut self.notifications.records {
            let now = now.max(record.last);
            if !record.paused
                && let Some(remaining) = &mut record.remaining
            {
                *remaining = remaining.saturating_sub(now.duration_since(record.last));
            }
            record.last = now;
            let node = record.descriptor.node_id;
            let bounds = self.layout.rects.get(&node);
            record.paused = hidden
                || modal
                || self.layout.is_omitted(node)
                || !bounds.is_some_and(|bounds| bounds.overlaps(self.viewport))
                || bounds.is_some_and(|bounds| pointer.is_some_and(|point| bounds.contains(point)))
                || focus.is_some_and(|id| record.descriptor.widgets.contains(&id));
            if !record.paused
                && !record.delivered
                && record.descriptor.demand.token.is_live()
                && let Some(next) = record
                    .remaining
                    .and_then(|remaining| now.checked_add(remaining))
            {
                deadline = Some(deadline.map_or(next, |old: Instant| old.min(next)));
            }
        }
        self.notifications.deadline = deadline;
    }
    pub(super) fn advance_notifications(&mut self, now: Instant) -> bool {
        self.update_notice_pause(now);
        // Each dispatch can synchronously replace the accepted projection. Retest
        // the live record and token before mapping the next event.
        let mut changed = false;
        for _ in 0..64 {
            let Some(index) = self.notifications.records.iter().position(|record| {
                !record.delivered
                    && !record.paused
                    && record.remaining == Some(Duration::ZERO)
                    && record.descriptor.demand.token.is_live()
            }) else {
                break;
            };
            let record = &mut self.notifications.records[index];
            record.delivered = true;
            let demand = record.descriptor.demand.clone();
            let message = (demand.on_dismiss)(NoticeDismissal {
                token: demand.token.clone(),
                reason: NoticeDismissalReason::Timeout,
            });
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
            changed = true;
            if !self.lifecycle_accepts_work() {
                break;
            }
            self.update_notice_pause(now);
        }
        self.update_notice_pause(now);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, Notice, NoticeQueue, NoticeSeverity, notifications, scene, text},
        gui::{
            pointer_ingress::{DeviceKind, InputDeviceId, PointerButtons, PointerContactId},
            types::{Point, Vector2},
        },
        runtime::{Command, WindowEnvironment},
        widgets::{PointerButton, PointerModifiers},
    };
    use std::sync::Arc;
    struct Model {
        queue: NoticeQueue,
        dismissals: usize,
    }
    impl RuntimeBridge<NoticeDismissal> for Model {
        #[allow(clippy::arc_with_non_send_sync)]
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<NoticeDismissal>> {
            Arc::new(
                scene(text("Base"))
                    .layer(
                        notifications(self.queue.snapshot())
                            .on_dismiss(|event| event)
                            .layer(),
                    )
                    .into_view()
                    .into_surface(),
            )
        }
        fn update(&mut self, event: NoticeDismissal) -> Command<NoticeDismissal> {
            if self.queue.dismiss(&event) {
                self.dismissals += 1;
            }
            Command::none()
        }
    }
    #[test]
    fn captured_ingress_updates_notice_hover_before_focus_is_released() {
        let origin = Instant::now();
        let mut queue = NoticeQueue::new();
        queue
            .push(
                Notice::new(1, NoticeSeverity::Info, "Ready")
                    .unwrap()
                    .timeout(Some(Duration::from_secs(1)))
                    .unwrap(),
            )
            .unwrap();
        let mut runtime = SurfaceRuntime::new_with_environment_and_clock(
            Model {
                queue,
                dismissals: 0,
            },
            Vector2::new(400.0, 300.0),
            WindowEnvironment::default(),
            Some(origin),
        );
        let widget = *runtime.notifications.records[0]
            .descriptor
            .widgets
            .last()
            .unwrap();
        let bounds = runtime.layout.rects[&widget];
        let inside = Point::new(
            (bounds.min.x + bounds.max.x) / 2.0,
            (bounds.min.y + bounds.max.y) / 2.0,
        );
        let device = InputDeviceId::new(1).unwrap();
        let contact = PointerContactId::new(1).unwrap();
        runtime.set_timed_repaint_clock(Some(origin + Duration::from_millis(400)));
        runtime.dispatch_pointer_start(
            DeviceKind::Mouse,
            device,
            contact,
            inside,
            PointerButton::Primary,
            PointerButtons::PRIMARY,
            PointerModifiers::default(),
        );
        assert_eq!(runtime.focused_widget(), Some(widget));
        runtime.set_timed_repaint_clock(Some(origin + Duration::from_secs(10)));
        runtime.dispatch_pointer_move(
            DeviceKind::Mouse,
            device,
            contact,
            Point::new(0.0, 0.0),
            PointerButtons::PRIMARY,
            PointerModifiers::default(),
        );
        assert_eq!(
            runtime.current_pointer_position(),
            Some(Point::new(0.0, 0.0))
        );
        runtime.clear_focus();
        runtime
            .advance_timed_repaints(origin + Duration::from_secs(10) + Duration::from_millis(599));
        assert_eq!(runtime.bridge().dismissals, 0);
        runtime.set_timed_repaint_clock(Some(
            origin + Duration::from_secs(10) + Duration::from_millis(600),
        ));
        runtime
            .advance_timed_repaints(origin + Duration::from_secs(10) + Duration::from_millis(600));
        assert_eq!(runtime.bridge().dismissals, 1);
    }
}
