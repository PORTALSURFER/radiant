use super::super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::repaint::RepaintSignal,
    runtime::{
        Command, RuntimeAnimationActivity, RuntimeAnimationHost, RuntimePlatformHost,
        RuntimeQueueHost, RuntimeTaskHost,
    },
};
use std::{sync::Arc, time::Duration};

impl<State, Message, Project, Update, View> RuntimeTaskHost<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.install_runtime_repaint_signal(signal);
    }

    fn schedule_timer(&mut self, delay: Duration, wake: crate::runtime::RuntimeTimerWake) -> bool {
        self.schedule_runtime_timer(delay, wake)
    }

    fn spawn_worker_task(
        &mut self,
        name: &'static str,
        priority: crate::runtime::TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() + Send + 'static>,
    ) -> bool {
        self.spawn_runtime_worker_task(name, priority, is_cancelled, work)
    }
}

impl<State, Message, Project, Update, View> RuntimePlatformHost<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn request_platform_service(
        &mut self,
        request: crate::runtime::PlatformRequest,
        on_completed: crate::runtime::PlatformCompletion<Message>,
    ) -> Result<(), crate::runtime::PlatformServiceFallback<Message>> {
        self.request_app_platform_service(request, on_completed)
    }
}

impl<State, Message, Project, Update, View> RuntimeQueueHost<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    State: 'static,
{
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        self.take_runtime_command_queue()
    }

    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        self.drain_runtime_command_queue_into(commands);
    }

    fn take_runtime_messages(&mut self) -> Vec<Message> {
        self.take_runtime_message_queue()
    }

    fn take_runtime_timer_wakes(&mut self) -> Vec<crate::runtime::RuntimeTimerWake> {
        self.take_runtime_timer_wake_queue()
    }

    fn map_runtime_timer_wake(
        &mut self,
        wake: crate::runtime::RuntimeTimerWake,
    ) -> Option<Message> {
        AppBridge::map_runtime_timer_wake(self, wake)
    }

    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        self.drain_runtime_message_queue_into(messages);
    }

    fn drain_runtime_message_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        self.drain_runtime_message_queue_batch_into(messages, max_messages)
    }
}

impl<State, Message, Project, Update, View> RuntimeAnimationHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn needs_animation(&mut self) -> bool {
        self.needs_runtime_animation()
    }

    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.runtime_animation_activity()
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queue_runtime_animation_frame()
    }
}
