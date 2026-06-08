//! Native file drag/drop event routing.

use super::GenericNativeVelloRunner;
use crate::runtime::{NativeFileDrop, RuntimeBridge};
use crate::widgets::WidgetId;
use std::path::PathBuf;
use winit::event_loop::ActiveEventLoop;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_native_file_hover(&mut self, event_loop: &ActiveEventLoop, path: PathBuf) {
        tracing::info!(
            path = %path.display(),
            position = ?self.input.last_cursor,
            target_widget = ?self.native_file_drop_target(),
            "radiant generic native vello: native file hover"
        );
        self.dispatch_native_file_drop_event(
            event_loop,
            NativeFileDrop::hover(path, self.input.last_cursor, self.native_file_drop_target()),
        );
    }

    pub(super) fn handle_native_file_cancel(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!(
            position = ?self.input.last_cursor,
            target_widget = ?self.native_file_drop_target(),
            "radiant generic native vello: native file hover cancelled"
        );
        self.dispatch_native_file_drop_event(
            event_loop,
            NativeFileDrop::cancel(self.input.last_cursor, self.native_file_drop_target()),
        );
    }

    pub(super) fn handle_native_file_drop(&mut self, event_loop: &ActiveEventLoop, path: PathBuf) {
        tracing::info!(
            path = %path.display(),
            position = ?self.input.last_cursor,
            target_widget = ?self.native_file_drop_target(),
            "radiant generic native vello: native file dropped"
        );
        self.dispatch_native_file_drop_event(
            event_loop,
            NativeFileDrop::dropped(path, self.input.last_cursor, self.native_file_drop_target()),
        );
    }

    fn native_file_drop_target(&self) -> Option<WidgetId> {
        self.core
            .runtime
            .native_file_drop_target(self.input.last_cursor)
    }

    fn dispatch_native_file_drop_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        drop: NativeFileDrop,
    ) {
        let outcome = self.core.runtime.dispatch_native_file_drop(drop);
        let routed = self.core.route_command_outcome(outcome);
        self.handle_route_outcome(event_loop, routed);
    }
}
