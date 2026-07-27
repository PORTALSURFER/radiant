use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext, runtime::queue::SharedRuntimeDelivery},
    gui::repaint::RepaintSignal,
    runtime::{Command, RuntimeQueueDelivery, RuntimeQueueItem, TaskPriority},
};
use std::{sync::Arc, time::Duration};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
{
    pub(super) fn install_runtime_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.install_repaint(signal);
        self.run_startup_once();
        self.start_subscriptions_once();
    }

    pub(super) fn schedule_runtime_timer(
        &mut self,
        delay: Duration,
        wake: crate::runtime::RuntimeTimerWake,
    ) -> bool {
        self.runtime.schedule_timer_wake(delay, wake)
    }

    pub(super) fn spawn_runtime_worker_task(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(self.runtime.shared());
        self.runtime
            .spawn_business_task(name, priority, is_cancelled, move || {
                work();
                if let Some(runtime) = runtime.upgrade() {
                    runtime.request_repaint();
                }
            })
    }
}

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn take_runtime_command_queue(&mut self) -> Vec<Command<Message>> {
        std::mem::take(&mut self.commands)
    }

    pub(super) fn drain_runtime_command_queue_into(
        &mut self,
        commands: &mut Vec<Command<Message>>,
    ) {
        commands.append(&mut self.commands);
    }

    pub(super) fn take_runtime_message_queue(&mut self) -> Vec<Message> {
        let worker_registry = &mut self.worker_registry;
        let platform_registry = &mut self.platform_registry;
        let timer_registry = &mut self.timer_registry;
        self.runtime.take_pending_with_mappers(
            |delivery| worker_registry.map_delivery(delivery),
            |delivery| platform_registry.map_delivery(delivery),
            |wake| timer_registry.map_wake(wake),
        )
    }

    pub(super) fn drain_runtime_queue_item_batch_into(
        &mut self,
        items: &mut Vec<RuntimeQueueItem<Message>>,
        max_items: usize,
    ) -> bool {
        self.runtime.drain_pending_item_batch_into(items, max_items)
    }

    pub(super) fn map_runtime_queue_delivery(
        &mut self,
        delivery: RuntimeQueueDelivery,
    ) -> Option<Message> {
        let Ok(delivery) = delivery.downcast::<SharedRuntimeDelivery>() else {
            return None;
        };
        match delivery {
            SharedRuntimeDelivery::Worker(delivery) => self.worker_registry.map_delivery(delivery),
            SharedRuntimeDelivery::Platform(delivery) => {
                self.platform_registry.map_delivery(delivery)
            }
            SharedRuntimeDelivery::Timer(wake) => self.timer_registry.map_wake(wake),
        }
    }

    pub(super) fn map_runtime_timer_wake(
        &mut self,
        wake: crate::runtime::RuntimeTimerWake,
    ) -> Option<Message> {
        self.timer_registry.map_wake(wake)
    }

    pub(super) fn drain_runtime_message_queue_into(&mut self, messages: &mut Vec<Message>) {
        let worker_registry = &mut self.worker_registry;
        let platform_registry = &mut self.platform_registry;
        let timer_registry = &mut self.timer_registry;
        self.runtime.drain_pending_into_with_mappers(
            messages,
            |delivery| worker_registry.map_delivery(delivery),
            |delivery| platform_registry.map_delivery(delivery),
            |wake| timer_registry.map_wake(wake),
        );
    }

    pub(super) fn drain_runtime_message_queue_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        let worker_registry = &mut self.worker_registry;
        let platform_registry = &mut self.platform_registry;
        let timer_registry = &mut self.timer_registry;
        self.runtime.drain_pending_batch_into_with_mappers(
            messages,
            max_messages,
            |delivery| worker_registry.map_delivery(delivery),
            |delivery| platform_registry.map_delivery(delivery),
            |wake| timer_registry.map_wake(wake),
        )
    }
}
