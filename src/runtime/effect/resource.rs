//! Shared-resource worker construction on the existing qualified effect lane.

use super::{Effect, EffectOwner};
use crate::application::runtime::task::resource_operations::{
    ResourceOperationReplaceMode, ResourceOperationReservation, ResourceOperationReserve,
};
use crate::application::{
    CancellationToken, SharedResourceCompletion, SharedResourceTaskError, SharedResourceTaskMode,
    SharedResourceTasks,
};
use crate::runtime::{Command, ResourceKey, TaskPriority};
use std::sync::Arc;

impl<Message: 'static> Effect<Message> {
    /// Start or join application-owned work for a resource with live interests.
    ///
    /// `None` means existing work, ready state, or backoff was reused. Only the
    /// starter supplies a completion mapper; it should update shared application
    /// state. Consumer retirement never transfers worker ownership to a view.
    #[allow(clippy::too_many_arguments)]
    pub fn resource_worker<Output: Send + 'static>(
        tasks: &SharedResourceTasks,
        key: impl Into<ResourceKey>,
        mode: SharedResourceTaskMode,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(SharedResourceCompletion<Output>) -> Message + 'static,
    ) -> Result<Option<Self>, SharedResourceTaskError> {
        let mode = match mode {
            SharedResourceTaskMode::Join => ResourceOperationReplaceMode::Join,
            SharedResourceTaskMode::Refresh => ResourceOperationReplaceMode::Replace,
        };
        let outcome = tasks.operations.reserve(key.into(), mode)?;
        Ok(Self::from_resource_outcome(
            outcome, name, priority, work, map,
        ))
    }

    /// Take a due retry exactly once using the application's logical clock.
    ///
    /// This does not install an autonomous retry loop. Call from a clock-driven
    /// UI update; retired interests and superseded deadlines cannot start work.
    pub fn resource_retry<Output: Send + 'static>(
        tasks: &SharedResourceTasks,
        key: &ResourceKey,
        now: u64,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(SharedResourceCompletion<Output>) -> Message + 'static,
    ) -> Option<Self> {
        let outcome = tasks.operations.take_retry(key, now)?;
        Self::from_resource_outcome(outcome, name, priority, work, map)
    }

    fn from_resource_outcome<Output: Send + 'static>(
        outcome: ResourceOperationReserve,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(SharedResourceCompletion<Output>) -> Message + 'static,
    ) -> Option<Self> {
        let ResourceOperationReserve::Reserved(reservation) = outcome else {
            return None;
        };
        Some(Self::from_resource_reservation(
            reservation,
            name,
            priority,
            work,
            map,
        ))
    }

    fn from_resource_reservation<Output: Send + 'static>(
        reservation: ResourceOperationReservation,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(SharedResourceCompletion<Output>) -> Message + 'static,
    ) -> Self {
        let current = reservation.current();
        let currentness = reservation.currentness_probe();
        let ticket = reservation.ticket();
        let identity = crate::runtime::command::EffectId(reservation.effect_id());
        let token = CancellationToken::new();
        reservation.attach_cancellation(token.clone());
        let mut command = Command::perform_worker_effect_for_effect(
            identity,
            EffectOwner::Application,
            name,
            priority,
            ticket,
            reservation.into_transaction(),
            token.clone(),
            work,
            move |completion| {
                map(SharedResourceCompletion {
                    output: completion.output,
                    current,
                })
            },
        );
        if let Command::PerformWorker(effect) = &mut command
            && let Some(lifecycle) = &mut effect.lifecycle
        {
            let cancellation = Arc::clone(&lifecycle.cancellation);
            lifecycle.cancellation = Arc::new(move || cancellation() || !currentness());
        }
        Self {
            command,
            ticket,
            token,
        }
    }
}
