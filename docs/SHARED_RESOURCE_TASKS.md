# Shared resource tasks

`application::SharedResourceTasks` coordinates application-owned work shared by
multiple consumers. Its clones share one broker. Existing `ResourceTasks`
latest/exclusive helpers retain their existing independent clone semantics.
The broker stores ownership and operation metadata; the application stores
resource values, errors, cache policy, and persistence.

## Interest admission

Call `tasks.interest(key, owner, interest_id, kind, on_completed)` to construct a
command. The runtime accepts the selected owner against its current projection
before returning a `ResourceInterest` through the UI-local callback. An absent,
ambiguous, or retired declarative owner fails without application-owner fallback.
Creating or dropping the command alone does not acquire an interest.

Choose a stable `interest_id` within a resource and owner. Repeated requests for
that exact identity share one handle; use `set_kind` to change its existing
`Visible`, `Prefetch`, or `Persistent` kind. A visibility transition preserves
the shared operation. Persistent interest still belongs to its selected owner;
choose `EffectOwner::Application` explicitly for work that outlives a view.

Keep the returned handle in application state. Clones share the same interest,
and dropping the final clone releases it. Explicit `release` retires every
clone. The runtime keeps a weak retirement guard, so it cannot keep a discarded
application handle alive. Accepted owner retirement releases that generation's
interests before later effect mapping. A removed and reinserted owner receives
a new generation; old handles cannot retire its new work.

## Worker and completion flow

`Effect::resource_worker` takes the shared broker, resource key, `Join` or
`Refresh` mode, worker name/priority, worker closure, and UI mapper. It returns a
new effect only when work was reserved. `Join` reuses running work, ready
metadata, or scheduled backoff. `Refresh` explicitly replaces accepted work.
Only one replacement per key may await admission; another refresh receives
`PendingAdmission`. Dropped or rejected effects roll back the exact reservation.

The worker uses the existing application-owned effect lane. Its first consumer
does not own the shared operation: another live consumer allows work to finish
after the starter disappears. Final interest release invalidates its demand
generation. Reacquiring the same key cannot make an old completion current.
Cancellation and quarantine use that demand generation, the exact broker,
operation identity, latest ticket, and existing runtime lifecycle checks.

The UI mapper should only construct a message containing
`SharedResourceCompletion<T>`. In the reducer, call `finish_ready(completion)`
or `finish_failed(completion)` before applying the returned value. These return
`None` for stale or foreign-broker completions. Finishing inside the mapper
invalidates its fence before the runtime's post-mapping check, so it is not the
supported application flow.

`tasks.cancel(&key)` cancels current work and retry state while preserving
interests. Cancelling an effect token fences that operation. A new join can
start after pending admission settles; cancellation cannot open another pending
replacement chain. Once a completion is accepted as ready or backoff, cancelling
its old effect token does not change that accepted state. Explicit key cancellation
also cancels a predecessor awaiting replacement admission.

`retain_ready(key, true)` explicitly permits
ready bookkeeping to survive interest removal. The value itself remains in
application state. Reacquisition can reuse that bookkeeping; `Refresh` requests
fresh work. Release of interest never grants permission for in-flight work to
publish into a retained cache.

## Retry and bounds

`schedule_retry(&completion, deadline)` accepts a failed operation into backoff.
The application supplies an absolute deadline in its own logical clock units.
`Effect::resource_retry` takes a due retry once. Ordinary joins do not bypass
backoff; an explicit refresh can. No observation, diagnostic query, semantic
demand, or resource handle autonomously invokes a provider or schedules a timer.

A broker permits 256 retained resource keys, 1,024 distinct interests, 64
interests per key, and 256 operation slots. A runtime also caps its aggregate
retirement registry at 1,024 interests across brokers. Retry storage is one
deadline per key. Replacement history is cleared after settlement, and identity
exhaustion fails closed. These limits bound metadata, not application data.

A broker binds to one runtime on successful admission. Runtime shutdown retires
its interests and fences work. Use a new broker for a different runtime.
`tasks.shutdown()` permanently closes the broker and cancels its work; native
surface/device recovery within the same runtime does not transfer ownership.
