use super::super::{
    auxiliary_focus::AuxiliaryFocusCommand, effects::WorkerEffectMappingMode, owner::EffectOrigin,
};
use super::{CommandOutcome, SurfaceRuntime};
use crate::application::runtime::update_context::business::admission::{
    BusinessTaskAdmission, resolve as resolve_admission,
};
use crate::runtime::EffectOwner;
use crate::runtime::RepaintScope;
use crate::runtime::RuntimeUpdateSnapshot;
use crate::runtime::UiUpdateHandlerDiagnosticsMode;
use crate::runtime::command::{EffectMappingPolicy, WorkerEffectMapper};
use crate::{
    gui::types::Vector2,
    runtime::{Command, DragSession, RuntimeBridge},
};
use std::{any::type_name, panic::panic_any, time::Instant};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn effect_origin_is_active(
        &self,
        origin: &EffectOrigin,
    ) -> bool {
        if !origin.is_live() {
            return false;
        }
        match origin {
            EffectOrigin::Application => true,
            EffectOrigin::Auxiliary(owner) => self.auxiliary_effect_owner_is_active(owner),
            EffectOrigin::Declarative(token) => self.declarative_owner_ledger.is_live(token),
        }
    }

    fn schedule_timer_effect(
        &mut self,
        mut effect: crate::runtime::command::TimerEffect<Message>,
        origin: EffectOrigin,
    ) -> bool {
        let origin = match self.resolve_effect_origin(
            effect.lifecycle.as_ref().map(|lifecycle| lifecycle.owner),
            effect.owner.take(),
            origin,
            false,
        ) {
            Some(origin) => origin,
            None => {
                if let Some(transaction) = effect.transaction.as_ref() {
                    transaction.reject();
                }
                return false;
            }
        };
        let capability = self.host_capabilities.tasks.as_ref();
        let bridge = &mut self.bridge;
        self.timer_effects.schedule(effect, origin, |delay, wake| {
            capability.is_some_and(|capability| (capability.schedule_timer)(bridge, delay, wake))
        })
    }

    fn submit_worker_effect(
        &mut self,
        mut effect: crate::runtime::command::WorkerEffect<Message>,
        origin: EffectOrigin,
    ) -> bool {
        let owned = effect.owner.is_some() || effect.lifecycle.is_some();
        let cancellable = effect.is_cancelled.is_some() || effect.lifecycle.is_some();
        let mapping_policy = effect
            .lifecycle
            .as_ref()
            .map_or(EffectMappingPolicy::Eager, |lifecycle| lifecycle.mapping);
        let mapping_mode = match (mapping_policy, &effect.mapper, owned, cancellable) {
            (EffectMappingPolicy::Deferred, WorkerEffectMapper::Stream { .. }, _, _)
            | (EffectMappingPolicy::Deferred, WorkerEffectMapper::Once(_), _, _) => {
                match &effect.mapper {
                    WorkerEffectMapper::Stream { .. } => {
                        WorkerEffectMappingMode::DeferredOwnerStream
                    }
                    WorkerEffectMapper::Once(_) => WorkerEffectMappingMode::DeferredOwnerOneShot,
                }
            }
            (_, WorkerEffectMapper::Stream { latest: true, .. }, true, _)
            | (_, WorkerEffectMapper::Stream { latest: false, .. }, true, true) => {
                WorkerEffectMappingMode::DeferredOwnerStream
            }
            (_, WorkerEffectMapper::Once(_), true, true) => {
                WorkerEffectMappingMode::DeferredOwnerOneShot
            }
            _ => WorkerEffectMappingMode::Eager,
        };
        let origin = match self.resolve_effect_origin(
            effect.lifecycle.as_ref().map(|lifecycle| lifecycle.owner),
            effect.owner.take(),
            origin,
            true,
        ) {
            Some(origin) => origin,
            None => {
                if let Some(transaction) = effect.transaction.as_ref() {
                    transaction.reject();
                }
                if let Some(receipt) = effect.admission_receipt.as_ref() {
                    resolve_admission(&receipt.0, BusinessTaskAdmission::Rejected);
                }
                return false;
            }
        };
        self.submit_worker_effect_with_origin(effect, origin, mapping_mode)
    }

    fn resolve_effect_origin(
        &self,
        selection: Option<EffectOwner>,
        legacy_owner: Option<crate::application::DeclarativeEffectOwner>,
        origin: EffectOrigin,
        preserve_legacy_origin: bool,
    ) -> Option<EffectOrigin> {
        if let Some(selection) = selection {
            return match selection {
                EffectOwner::Application => Some(EffectOrigin::Application),
                EffectOwner::Declarative(handle) => {
                    self.declarative_owner_origin_for_handle(handle)
                }
            };
        }
        if let Some(handle) = legacy_owner {
            return self.declarative_owner_origin_for_handle(handle);
        }
        if preserve_legacy_origin {
            return Some(origin);
        }
        Some(match origin {
            // An unmarked timer keeps an auxiliary dispatch origin so its
            // wake and any chained command remain fenced by that generation.
            // Declarative dispatch does not implicitly own an unmarked effect.
            EffectOrigin::Auxiliary(owner) => EffectOrigin::Auxiliary(owner),
            EffectOrigin::Application | EffectOrigin::Declarative(_) => EffectOrigin::Application,
        })
    }

    pub(in crate::runtime::controller) fn dispatch_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        self.dispatch_message_inner_with_origin(message, outcome, EffectOrigin::Application);
    }

    pub(in crate::runtime::controller) fn dispatch_message_inner_with_origin(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        origin: EffectOrigin,
    ) {
        self.dispatch_message_inner_with_refresh(message, outcome, true, origin);
    }

    pub(in crate::runtime::controller) fn dispatch_message_inner_deferred_refresh(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        self.dispatch_message_inner_with_refresh(
            message,
            outcome,
            false,
            EffectOrigin::Application,
        );
    }

    pub(in crate::runtime::controller) fn dispatch_deferred_surface_messages(
        &mut self,
        messages: Vec<Message>,
    ) {
        if messages.is_empty() || !self.lifecycle_accepts_work() {
            return;
        }
        let mut outcome = CommandOutcome::default();
        // Keep this batch refresh-local: every mapped terminal message must be
        // reduced before a command can alter lifecycle, projection, or work
        // admission.
        let origin = EffectOrigin::Application;
        let mut commands = Vec::with_capacity(messages.len());
        for message in messages {
            let Some(command) = self.reduce_message_inner(message, &mut outcome, &origin) else {
                break;
            };
            commands.push(command);
        }
        let mut deferred_surface_is_fresh = false;
        for command in commands {
            self.dispatch_command_inner_with_refresh_state(
                command,
                &mut outcome,
                false,
                &mut deferred_surface_is_fresh,
                origin.clone(),
            );
        }
        self.finish_command_outcome(outcome);
    }

    fn dispatch_message_inner_with_refresh(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        origin: EffectOrigin,
    ) {
        let mut deferred_surface_is_fresh = refresh_surface;
        self.dispatch_message_inner_with_refresh_state(
            message,
            outcome,
            refresh_surface,
            &mut deferred_surface_is_fresh,
            origin,
        );
    }

    fn dispatch_message_inner_with_refresh_state(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        deferred_surface_is_fresh: &mut bool,
        origin: EffectOrigin,
    ) {
        let Some(command) = self.reduce_message_inner(message, outcome, &origin) else {
            return;
        };
        self.dispatch_command_inner_with_refresh_state(
            command,
            outcome,
            refresh_surface,
            deferred_surface_is_fresh,
            origin,
        );
    }

    fn reduce_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        origin: &EffectOrigin,
    ) -> Option<Command<Message>> {
        if !self.lifecycle_accepts_work() || !self.effect_origin_is_active(origin) {
            return None;
        }
        outcome.messages_dispatched += 1;
        Some(self.run_update_handler(message))
    }

    fn dispatch_command_inner_with_refresh_state(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        deferred_surface_is_fresh: &mut bool,
        origin: EffectOrigin,
    ) {
        if !self.lifecycle_accepts_work() || !self.effect_origin_is_active(&origin) {
            return;
        }
        let refresh_before = outcome.surface_refresh_requested;
        if !refresh_surface {
            *deferred_surface_is_fresh = false;
            outcome.surface_refresh_applied = false;
        }
        let repaint_scope = command.repaint_scope().unwrap_or(RepaintScope::Surface);
        let auxiliary_focus_only = matches!(&origin, EffectOrigin::Auxiliary(_))
            && command_contains_only_auxiliary_focus(&command);
        let requires_fresh_surface =
            command.requires_fresh_surface_before_dispatch() && !auxiliary_focus_only;
        let effective_scope = if requires_fresh_surface {
            RepaintScope::Surface
        } else {
            repaint_scope
        };
        let paint_only = effective_scope.is_paint_only();
        if paint_only {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        if (refresh_surface && !paint_only) || requires_fresh_surface {
            self.refresh_with_scope(effective_scope);
            *deferred_surface_is_fresh = true;
            outcome.record_applied_surface_refresh(effective_scope);
        }
        let messages_before_command = outcome.messages_dispatched;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            refresh_surface,
            deferred_surface_is_fresh,
            origin,
        );
        let command_dispatched_messages = outcome.messages_dispatched > messages_before_command;
        if !paint_only || command_dispatched_messages {
            outcome.surface_refresh_requested = true;
            outcome.surface_refresh_scope = Some(
                outcome
                    .surface_refresh_scope
                    .map_or(effective_scope, |current| current.merge(effective_scope)),
            );
        } else {
            outcome.surface_refresh_requested = refresh_before;
        }
    }

    fn run_update_handler(&mut self, message: Message) -> Command<Message> {
        let policy = self.update_handler_diagnostics_policy;
        let Some(threshold) = policy.threshold() else {
            return self.run_update_handler_with_snapshot(message);
        };
        let update_started = Instant::now();
        let command = self.run_update_handler_with_snapshot(message);
        let slow = self.diagnostics.record_update_handler(
            update_started.elapsed(),
            threshold,
            type_name::<Bridge>(),
            type_name::<Message>(),
        );
        if let (UiUpdateHandlerDiagnosticsMode::Panic, Some(diagnostic)) = (policy.mode(), slow) {
            panic_any(diagnostic.failure_message());
        }
        command
    }

    fn run_update_handler_with_snapshot(&mut self, message: Message) -> Command<Message> {
        let snapshot =
            RuntimeUpdateSnapshot::with_current_pointer_position(self.current_pointer_position())
                .with_window_environment(self.window_environment());
        self.bridge.update_with_runtime(message, snapshot)
    }

    pub(in crate::runtime::controller) fn execute_command_inner(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
    ) {
        if !self.lifecycle_accepts_work() {
            return;
        }
        if command.requests_paint_only() {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        let mut deferred_surface_is_fresh = true;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            true,
            &mut deferred_surface_is_fresh,
            EffectOrigin::Application,
        );
    }

    pub(in crate::runtime::controller) fn execute_command_inner_deferred_refresh(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
    ) {
        if !self.lifecycle_accepts_work() {
            return;
        }
        if command.requests_paint_only() {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        let mut deferred_surface_is_fresh = false;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            false,
            &mut deferred_surface_is_fresh,
            EffectOrigin::Application,
        );
    }

    fn execute_command_inner_with_refresh_state(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        deferred_surface_is_fresh: &mut bool,
        origin: EffectOrigin,
    ) {
        if !self.lifecycle_accepts_work() {
            return;
        }
        if !self.effect_origin_is_active(&origin) {
            return;
        }
        if let EffectOrigin::Auxiliary(owner) = &origin {
            let focus_command = match &command {
                Command::Focus(widget_id) => Some(AuxiliaryFocusCommand::Focus(*widget_id)),
                Command::ClearFocus => Some(AuxiliaryFocusCommand::Clear),
                _ => None,
            };
            if let Some(focus_command) = focus_command {
                self.enqueue_auxiliary_focus_request(owner.clone(), focus_command);
                return;
            }
        }
        if !refresh_surface
            && outcome.surface_refresh_requested
            && !*deferred_surface_is_fresh
            && command.requires_fresh_surface_before_dispatch()
        {
            self.refresh_with_scope(RepaintScope::Surface);
            *deferred_surface_is_fresh = true;
            outcome.record_applied_surface_refresh(RepaintScope::Surface);
        }
        match command {
            Command::None => {}
            Command::Message(message) => {
                self.dispatch_message_inner_with_refresh_state(
                    message,
                    outcome,
                    refresh_surface,
                    deferred_surface_is_fresh,
                    origin,
                );
            }
            Command::Batch(commands) => {
                for command in commands {
                    self.execute_command_inner_with_refresh_state(
                        command,
                        outcome,
                        refresh_surface,
                        deferred_surface_is_fresh,
                        origin.clone(),
                    );
                }
            }
            Command::RequestRepaint => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::RequestPaintOnly => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.paint_only_requested = true;
            }
            Command::UpdateGpuShaderPresentationUniform(update) => {
                if self.gpu_shader_presentation_uniform_mailbox.admit(update) {
                    self.repaint_requested = true;
                    outcome.repaint_requested = true;
                    outcome.paint_only_requested = true;
                }
            }
            Command::RequestProjectionRefresh => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Projection);
            }
            Command::RequestLayoutRefresh => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Layout);
            }
            Command::SetDpiScale(scale) => {
                self.repaint_requested = true;
                self.external_layout_dirty = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Surface);
                outcome.dpi_scale_override = Some(scale);
            }
            Command::SetWindowLogicalSize(size) => {
                self.repaint_requested = true;
                self.external_layout_dirty = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Surface);
                outcome.window_logical_size = Some(size);
            }
            Command::Timer(effect) => {
                if self.schedule_timer_effect(effect, origin) {
                    outcome.repaint_requested = true;
                }
            }
            Command::PerformWorker(effect) => {
                if self.submit_worker_effect(effect, origin) {
                    outcome.repaint_requested = true;
                }
            }
            Command::PlatformEffect(effect) => {
                let owner = effect.lifecycle.owner;
                let Some(effect_origin) =
                    self.resolve_effect_origin(Some(owner), None, origin, false)
                else {
                    effect.transaction.reject();
                    return;
                };
                if self.host_request_platform_effect(effect, &effect_origin) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Focus(widget_id) => {
                let focused = self.focus_widget(widget_id);
                outcome.repaint_requested |= focused;
                outcome.surface_repaint_requested |= focused;
            }
            Command::ClearFocus => {
                let had_focus = self.focused_widget().is_some();
                self.clear_focus();
                outcome.repaint_requested |= had_focus;
                outcome.surface_repaint_requested |= had_focus;
            }
            Command::ScrollTo { node_id, offset } => {
                let offset = Vector2::new(offset.x.max(0.0), offset.y.max(0.0));
                self.scroll_to_offset(node_id, offset);
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollIntoView {
                node_id,
                target_y,
                target_height,
                margin_top,
                margin_bottom,
                snap_y,
            } => {
                if let Some(offset) = self.scroll_into_view_offset(
                    node_id,
                    target_y,
                    target_height,
                    margin_top,
                    margin_bottom,
                    snap_y,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollFixedRowIntoView {
                node_id,
                row_index,
                row_stride,
                leading_context_rows,
                trailing_context_rows,
                direction,
            } => {
                if let Some(offset) = self.scroll_fixed_row_into_view_offset(
                    node_id,
                    row_index,
                    row_stride,
                    leading_context_rows,
                    trailing_context_rows,
                    direction,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::BeginExternalDrag {
                request,
                on_completed,
            } => {
                self.begin_external_drag_session(request, on_completed);
            }
            Command::BeginDrag { request } => {
                self.interaction.drag.session = Some(DragSession::new(request));
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::PlatformRequest {
                request,
                on_completed,
            } => {
                if let Err(fallback) =
                    self.host_request_platform_service(request, on_completed, &origin)
                {
                    let (request, on_completed) = *fallback;
                    let identity = self.platform_registry.register_legacy_for_request(
                        on_completed,
                        &request,
                        &origin,
                    );
                    let result = if request.validate().is_err() {
                        Err(crate::runtime::PlatformFailure::InvalidRequest)
                    } else {
                        Err(crate::runtime::PlatformFailure::Unsupported(
                            request.service(),
                        ))
                    };
                    if let Some(reservation) =
                        crate::runtime::controller::platform::PlatformResultIngress::reserve(
                            &self.platform_results,
                        )
                    {
                        let _ =
                            reservation.commit(crate::runtime::PlatformResultDelivery::Completed {
                                identity,
                                result,
                            });
                    } else {
                        let accepted = self
                            .platform_results
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .enqueue_overflow(crate::runtime::PlatformResultDelivery::Completed {
                                identity,
                                result: Err(crate::runtime::PlatformFailure::Capacity),
                            });
                        if !accepted {
                            let _ = self.platform_registry.remove(identity);
                        }
                    }
                }
            }
            Command::EndExternalDrag => {
                self.invalidate_external_drag();
            }
            Command::EndDrag => {
                self.interaction.drag.session = None;
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::Exit => {
                if self.begin_closing() {
                    outcome.exit_requested = true;
                    self.exit_requested = true;
                }
            }
        }
    }
}

fn command_contains_only_auxiliary_focus<Message>(command: &Command<Message>) -> bool {
    match command {
        Command::None | Command::Focus(_) | Command::ClearFocus => true,
        Command::Batch(commands) => commands.iter().all(command_contains_only_auxiliary_focus),
        _ => false,
    }
}
