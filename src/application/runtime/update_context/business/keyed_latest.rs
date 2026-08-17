use crate::application::{
    CancellationToken, DeclarativeEffectOwner, KeyedTaskCompletion, LatestTaskTransaction,
    TaskTicket,
};

use super::{
    BusinessEventSink, BusinessWorkContext,
    admission::{AdmissionReceiptGuard, BusinessTaskAdmissionReceipt},
    request::BusinessRequest,
};

pub(super) enum KeyedLatestAdmission {
    Transaction {
        effect_id: u64,
        transaction: LatestTaskTransaction,
    },
}

/// Builder for one keyed-latest business request.
pub struct BusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
    pub(super) admission: KeyedLatestAdmission,
}

impl<Message, Key> BusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> BusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + Sync + 'static,
{
    /// Make this keyed latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
        CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: CancellationToken::new(),
            ticket: self.ticket,
            key: self.key,
            admission: self.admission,
        }
    }

    /// Run keyed latest work and tag the output with its key and task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        let Self {
            request,
            ticket,
            key,
            admission,
        } = self;
        let work = move |context| KeyedTaskCompletion {
            key,
            ticket,
            output: work(context),
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.run_with_latest_transaction(effect_id, transaction, None, work, map);
    }

    /// Run keyed latest worker-only work and map its output on the UI runtime.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.run(work, map);
    }

    /// Run keyed latest work that may emit intermediate events tagged with its key and task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let Self {
            request,
            ticket,
            key,
            admission,
        } = self;
        let event_key = key.clone();
        let event_map = move |event| {
            map_event(KeyedTaskCompletion {
                key: event_key.clone(),
                ticket,
                output: event,
            })
        };
        let final_map = move |output| {
            map_final(KeyedTaskCompletion {
                key,
                ticket,
                output,
            })
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.stream_with_latest_transaction(
            effect_id,
            transaction,
            None,
            work,
            event_map,
            final_map,
            false,
        );
    }

    /// Run keyed latest work with coalesced intermediate events tagged with its key and task ticket.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let Self {
            request,
            ticket,
            key,
            admission,
        } = self;
        let event_key = key.clone();
        let event_map = move |event| {
            map_event(KeyedTaskCompletion {
                key: event_key.clone(),
                ticket,
                output: event,
            })
        };
        let final_map = move |output| {
            map_final(KeyedTaskCompletion {
                key,
                ticket,
                output,
            })
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.stream_with_latest_transaction(
            effect_id,
            transaction,
            None,
            work,
            event_map,
            final_map,
            true,
        );
    }
}

/// Capability-qualified builder for one keyed-latest business request.
///
/// `BusinessRequest::latest_for` returns this wrapper so an explicit owner
/// route is available for application-owned keyed task registries. Resource
/// routes retain [`BusinessKeyedLatestRequest`] and therefore cannot select a
/// declarative owner or transfer ownership from `ResourceTasks`.
pub struct BusinessKeyedLatestOwnerRequest<'context, Message, Key> {
    pub(super) request: BusinessKeyedLatestRequest<'context, Message, Key>,
}

impl<Message, Key> BusinessKeyedLatestOwnerRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.request.ticket()
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        self.request.key()
    }
}

impl<'context, Message, Key> BusinessKeyedLatestOwnerRequest<'context, Message, Key>
where
    Key: Clone + Send + Sync + 'static,
{
    /// Make this keyed latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
        self.request.cancellable()
    }

    /// Run keyed latest work and tag the output with its key and task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.request.run(work, map);
    }

    /// Run keyed latest worker-only work and map its output on the UI runtime.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.request.run_on_ui(work, map);
    }

    /// Run keyed latest work only while `owner` resolves to one current,
    /// eligible keyed-node or overlay owner, and return its admission receipt.
    ///
    /// The exact keyed task ticket and replacement transaction are retained.
    /// The worker and UI completion are fenced by both keyed supersession and
    /// the resolved declarative owner generation.
    pub fn run_for_owner_with_receipt<Output>(
        self,
        owner: DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let BusinessKeyedLatestRequest {
            request,
            ticket,
            key,
            admission,
        } = self.request;
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
                crate::runtime::EffectId(effect_id),
                request.name,
                request.priority,
                None,
                ticket.id(),
                Some(transaction),
                Some(guard),
                Some(owner),
                move |cancellation_probe| KeyedTaskCompletion {
                    key,
                    ticket,
                    output: work(BusinessWorkContext::new_with_probe(
                        None,
                        cancellation_probe,
                    )),
                },
                map,
            ),
        );
        receipt
    }

    /// Run ordered keyed latest work only while `owner` resolves to one
    /// current, eligible keyed-node or overlay owner, and return its admission
    /// receipt.
    ///
    /// Intermediate events retain FIFO delivery and both event and final
    /// outputs carry this request's exact host key and keyed-latest ticket.
    /// Admission and mapping are fenced by both keyed supersession and the
    /// resolved declarative owner generation.
    pub fn stream_for_owner_with_receipt<Event, Output>(
        self,
        owner: DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let BusinessKeyedLatestRequest {
            request,
            ticket,
            key,
            admission,
        } = self.request;
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let event_key = key.clone();
        let event_map = move |event| {
            map_event(KeyedTaskCompletion {
                key: event_key.clone(),
                ticket,
                output: event,
            })
        };
        let final_map = move |output| {
            map_final(KeyedTaskCompletion {
                key,
                ticket,
                output,
            })
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
                crate::runtime::EffectId(effect_id),
                request.name,
                request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: ticket.id(),
                    latest: false,
                },
                Some(transaction),
                Some(guard),
                Some(owner),
                move |sink, cancellation_probe| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(
                        BusinessWorkContext::new_with_probe(None, cancellation_probe),
                        event_sink,
                    )
                },
                event_map,
                final_map,
            ),
        );
        receipt
    }

    /// Run keyed latest work that may emit intermediate events tagged with its key and task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        self.request.stream(work, map_event, map_final);
    }

    /// Run keyed latest work with coalesced intermediate events tagged with its key and task ticket.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        self.request.stream_latest(work, map_event, map_final);
    }
}

/// Cancellable builder for one keyed-latest business request.
pub struct CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
    pub(super) admission: KeyedLatestAdmission,
}

impl<Message, Key> CancellableBusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> CancellableBusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + Sync + 'static,
{
    /// Run cancellable keyed latest work and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        let Self {
            request,
            token: worker_token,
            ticket,
            key,
            admission,
        } = self;
        let work = move |context| KeyedTaskCompletion {
            key,
            ticket,
            output: work(context),
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.run_with_latest_transaction(effect_id, transaction, Some(worker_token), work, map);
        token
    }

    /// Run cancellable keyed latest worker-only work on the UI runtime.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        self.run(work, map)
    }

    /// Run cancellable keyed latest work that may emit intermediate events tagged with its key and task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        let Self {
            request,
            token: worker_token,
            ticket,
            key,
            admission,
        } = self;
        let event_key = key.clone();
        let event_map = move |event| {
            map_event(KeyedTaskCompletion {
                key: event_key.clone(),
                ticket,
                output: event,
            })
        };
        let final_map = move |output| {
            map_final(KeyedTaskCompletion {
                key,
                ticket,
                output,
            })
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.stream_with_latest_transaction(
            effect_id,
            transaction,
            Some(worker_token),
            work,
            event_map,
            final_map,
            false,
        );
        token
    }

    /// Run cancellable keyed latest work with coalesced intermediate events and return its cancellation token.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        let Self {
            request,
            token: worker_token,
            ticket,
            key,
            admission,
        } = self;
        let event_key = key.clone();
        let event_map = move |event| {
            map_event(KeyedTaskCompletion {
                key: event_key.clone(),
                ticket,
                output: event,
            })
        };
        let final_map = move |output| {
            map_final(KeyedTaskCompletion {
                key,
                ticket,
                output,
            })
        };
        let KeyedLatestAdmission::Transaction {
            effect_id,
            transaction,
        } = admission;
        request.stream_with_latest_transaction(
            effect_id,
            transaction,
            Some(worker_token),
            work,
            event_map,
            final_map,
            true,
        );
        token
    }
}

#[cfg(test)]
mod tests {
    use crate::application::KeyedLatestTasks;
    use crate::application::runtime::update_context::UiUpdateContext;
    use crate::runtime::{Command, ResourceKey, TaskPriority};

    #[test]
    fn keyed_latest_stream_tags_intermediate_and_final_outputs() {
        let mut context = UiUpdateContext::<String>::default();
        let mut latest = KeyedLatestTasks::new();
        context
            .business()
            .interactive("keyed-stream-test")
            .latest_for(&mut latest, ResourceKey::scoped("sample", "C:/kick.wav"))
            .stream(
                |_context, events| {
                    assert!(events.emit("preview"));
                    "done"
                },
                |event| format!("{}:{}:{}", event.key, event.ticket.id(), event.output),
                |output| format!("{}:{}:{}", output.key, output.ticket.id(), output.output),
            );

        let command = context.into_command();
        assert!(matches!(command, Command::PerformWorker(_)));
        assert_eq!(
            command.business_task_priority("keyed-stream-test"),
            Some(TaskPriority::Interactive)
        );
    }

    #[test]
    fn keyed_latest_stream_latest_uses_coalescing_command() {
        let mut context = UiUpdateContext::<String>::default();
        let mut latest = KeyedLatestTasks::new();
        context
            .business()
            .interactive("keyed-latest-stream-test")
            .latest_for(&mut latest, ResourceKey::scoped("sample", "C:/kick.wav"))
            .stream_latest(
                |_context, events| {
                    assert!(events.emit("preview"));
                    "done"
                },
                |event| format!("{}:{}:{}", event.key, event.ticket.id(), event.output),
                |output| format!("{}:{}:{}", output.key, output.ticket.id(), output.output),
            );

        let command = context.into_command();
        assert!(matches!(command, Command::PerformWorker(_)));
        assert_eq!(
            command.business_task_priority("keyed-latest-stream-test"),
            Some(TaskPriority::Interactive)
        );
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_publishes_owner_receipt_generation_and_transaction() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let key = ResourceKey::scoped("sample", "C:/kick.wav");
        let mut latest = KeyedLatestTasks::new();
        let predecessor = latest.begin(key.clone());
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("owner-keyed-stream")
            .latest_for(&mut latest, key.clone());
        let ticket = request.ticket();
        let receipt = request.stream_for_owner_with_receipt(
            owner,
            |_, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            |_| (),
            |_| (),
        );

        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("owner keyed latest stream should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("owner keyed stream should carry its transaction")
                .replacement(),
            ticket
        );
        assert_eq!(latest.active(&key), Some(ticket));
        assert_ne!(ticket, predecessor);
    }

    #[test]
    fn every_exclusive_builder_form_carries_transactional_admission() {
        let mut context = UiUpdateContext::<()>::default();
        let mut resources = crate::application::ResourceTasks::new();
        let key = |name| ResourceKey::scoped("sample", format!("C:/{name}.wav"));

        context
            .business()
            .background("exclusive-run")
            .exclusive_for(&mut resources, key("run"))
            .expect("run admission")
            .run(|_| (), |_| ());
        context
            .business()
            .background("exclusive-run-ui")
            .exclusive_for(&mut resources, key("run-ui"))
            .expect("run_on_ui admission")
            .run_on_ui(|_| (), |_| ());
        context
            .business()
            .background("exclusive-stream")
            .exclusive_for(&mut resources, key("stream"))
            .expect("stream admission")
            .stream(
                |_context, events| {
                    assert!(events.emit(()));
                },
                |_| (),
                |_| (),
            );
        context
            .business()
            .background("exclusive-stream-latest")
            .exclusive_for(&mut resources, key("stream-latest"))
            .expect("stream_latest admission")
            .stream_latest(
                |_context, events| {
                    assert!(events.emit(()));
                },
                |_| (),
                |_| (),
            );
        context
            .business()
            .background("exclusive-cancellable-run")
            .cancellable()
            .exclusive_for(&mut resources, key("cancellable-run"))
            .expect("cancellable run admission")
            .run(|_| (), |_| ());
        context
            .business()
            .background("exclusive-cancellable-run-ui")
            .cancellable()
            .exclusive_for(&mut resources, key("cancellable-run-ui"))
            .expect("cancellable run_on_ui admission")
            .run_on_ui(|_| (), |_| ());
        context
            .business()
            .background("exclusive-cancellable-stream")
            .cancellable()
            .exclusive_for(&mut resources, key("cancellable-stream"))
            .expect("cancellable stream admission")
            .stream(
                |_context, events| {
                    assert!(events.emit(()));
                },
                |_| (),
                |_| (),
            );
        context
            .business()
            .background("exclusive-cancellable-stream-latest")
            .cancellable()
            .exclusive_for(&mut resources, key("cancellable-stream-latest"))
            .expect("cancellable stream_latest admission")
            .stream_latest(
                |_context, events| {
                    assert!(events.emit(()));
                },
                |_| (),
                |_| (),
            );

        let Command::Batch(commands) = context.into_command() else {
            panic!("all exclusive forms should remain in a command batch");
        };
        assert_eq!(commands.len(), 8);
        for command in commands {
            let Command::PerformWorker(effect) = command else {
                panic!("exclusive forms should queue worker effects");
            };
            assert!(effect.transaction.is_some());
        }
    }

    #[test]
    fn keyed_latest_reuses_identity_per_key_and_keeps_keys_independent() {
        let key_a = ResourceKey::scoped("sample", "C:/a.wav");
        let key_b = ResourceKey::scoped("sample", "C:/b.wav");
        let mut latest = KeyedLatestTasks::new();
        let mut context = UiUpdateContext::<()>::default();
        context
            .business()
            .background("keyed-identity")
            .latest_for(&mut latest, key_a.clone())
            .run(|_| 1_u8, |_| ());
        context
            .business()
            .background("keyed-identity")
            .latest_for(&mut latest, key_b.clone())
            .run(|_| 2_u8, |_| ());
        context
            .business()
            .background("keyed-identity")
            .latest_for(&mut latest, key_a.clone())
            .run(|_| 3_u8, |_| ());

        let Command::Batch(commands) = context.into_command() else {
            panic!("three keyed requests should remain a command batch");
        };
        let effects = commands
            .into_iter()
            .map(|command| match command {
                Command::PerformWorker(effect) => effect,
                _ => panic!("keyed request should queue a worker effect"),
            })
            .collect::<Vec<_>>();
        assert_eq!(effects[0].id, effects[2].id);
        assert_ne!(effects[0].id, effects[1].id);
        assert_ne!(effects[0].generation, effects[2].generation);
        assert_eq!(
            effects[0]
                .transaction
                .as_ref()
                .expect("keyed effect transaction")
                .replacement()
                .id(),
            effects[0].generation.0
        );
        assert_eq!(
            latest.active(&key_b).map(|ticket| ticket.id()),
            Some(effects[1].generation.0)
        );
    }

    #[test]
    fn abandoned_keyed_and_resource_latest_builders_restore_predecessors() {
        let key = ResourceKey::scoped("sample", "C:/abandoned.wav");
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(key.clone());
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("abandoned-keyed")
            .latest_for(&mut keyed, key.clone());
        drop(request);
        assert_eq!(keyed.active(&key), Some(predecessor));
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("abandoned-keyed")
            .cancellable()
            .latest_for(&mut keyed, key.clone());
        drop(request);
        assert_eq!(keyed.active(&key), Some(predecessor));

        let mut resources = crate::application::ResourceTasks::new();
        let (predecessor, predecessor_transaction, _) = resources
            .begin_exclusive_transaction(key.clone())
            .expect("resource predecessor");
        predecessor_transaction.accept();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("abandoned-resource")
            .latest_for_resource(&mut resources, key.clone());
        drop(request);
        assert_eq!(resources.active(&key), Some(predecessor.ticket()));
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("abandoned-resource")
            .cancellable()
            .latest_for_resource(&mut resources, key.clone());
        drop(request);
        assert_eq!(resources.active(&key), Some(predecessor.ticket()));

        let exclusive_key = ResourceKey::scoped("sample", "C:/abandoned-exclusive.wav");
        let mut resources = crate::application::ResourceTasks::new();
        let request = context
            .business()
            .background("abandoned-exclusive")
            .exclusive_for(&mut resources, exclusive_key.clone());
        assert!(resources.active(&exclusive_key).is_some());
        drop(request);
        assert_eq!(resources.active(&exclusive_key), None);
        let request = context
            .business()
            .background("abandoned-exclusive")
            .cancellable()
            .exclusive_for(&mut resources, exclusive_key.clone());
        assert!(resources.active(&exclusive_key).is_some());
        drop(request);
        assert_eq!(resources.active(&exclusive_key), None);
    }
}
