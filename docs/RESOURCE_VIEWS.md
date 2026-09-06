# Effect-backed resource views

`application::Resource<T, E>` stores application values, typed errors, progress,
and local revision/generation evidence. `resource(state.snapshot())` selects an
ordinary declarative branch. Neither snapshots nor branch projection invoke a
provider, poll a worker, or start a retry.

```rust,ignore
resource(state.waveform.snapshot())
    .pending(text("Loading waveform"))
    .ready(|waveform| waveform_view(waveform))
    .refreshing(|previous| waveform_view(previous))
    .failed(|error| error_view(error))
    .interest(&state.tasks, 1, ResourceInterestKind::Visible)
    .into_view()
    .key("waveform-consumer")
```

The example uses application-defined `waveform_view` and `error_view` factories.
`ready`, `refreshing`, and `failed` invoke only the selected factory immediately
with an owned `Arc` value. They retain no closure or state borrow. The finished
view contains only the selected node and demand metadata. Use `failed_with_ready`
when an error presentation should also show eligible previous content. An omitted
branch is an empty container; labels and actions remain application-supplied.
Selected widgets keep their ordinary accessibility semantics and message routes.
For prebuilt branches, pass `ResourceViewBranches` to `ResourceView::from_branches`;
this selects one owned node without closures.

## Accepted view demand

The interest builder stores a broker, resource key, consumer id, and interest
kind. Constructing, lowering, caching, or discarding a view acquires nothing.
Once its projection is accepted, the runtime owns a strong consumer lease until
that view disappears or changes identity. The existing explicit-interest registry
keeps only weak retirement guards. A dedicated owner namespace prevents projected
consumers from colliding with explicit application or declarative interests.

The stable wrapper provides ordinary structural continuity across resource phase
changes. Key consumers when they move within a changing sibling list. Accepted
identity includes source and ancestor compatibility, the resource key, broker,
and consumer id. Changing Visible/Prefetch/Persistent kind preserves the lease.
Persistent projected interest still ends when its view disappears; use an explicit
application-owned interest for demand that outlives the view.

Demand metadata survives cached surface cloning. Reconciliation compares its
exact identity, so a demand change cannot be hidden by interaction-only reuse.
Cached subtree flags skip ordinary trees without resource demand. Traversal is
bounded to 128 ancestor levels, 65,536 visited nodes, and 1,024 consumers. Invalid
source inventory releases old projected leases; a rejected new admission cannot
leave an obsolete view's lease alive. Broker and aggregate runtime limits from
[Shared resource tasks](SHARED_RESOURCE_TASKS.md) still apply. Read
`SurfaceRuntime::resource_view_interest_status()` for bounded admission counts.
Visible admission precedes persistent and prefetch demand, with stable projection
order breaking ties; worker scheduling remains the explicit effect priority.

## Explicit work and state updates

After accepted demand exists, reserve `Effect::resource_worker`, obtain
`tasks.operation(&key)`, and call `state.begin(operation, policy)` before returning
the effect command. Choose `RetainReady` or `DiscardReady` explicitly. The opaque
operation snapshot starts no work and retains no broker or interest. It fences
broker identity, demand generation, operation epoch, and latest ticket.

The worker mapper constructs a `SharedResourceCompletion<Result<T, E>>` message.
The reducer calls `state.apply_completion(&tasks, completion)`. Values/errors stay
in the application state; stale or foreign completions do not update it. Progress
uses the same operation snapshot plus a strictly increasing sequence and validated
indeterminate or completed/total counters.

Resource state retains at most one predecessor snapshot while a replacement is
pending. Rejection restores only the exact broker-restored predecessor, without
rolling local revision/generation backward. Accepted replacement never revives an
old operation. A pure snapshot presents cancelled work as retained ready content
when available, or Cancelled otherwise, so a dropped command cannot leave a
permanent loading presentation.

`rekey(new_key)` clears this state's old values and fences, without cancelling
shared work still needed elsewhere. View reconciliation releases the old demand.
Explicit `cancel` cancels the exact operation; it cannot cancel a newer replacement.

## Retry and cancel controls

A snapshot supplies an opaque retry or cancel intent only for applicable phases.
Bind it to an ordinary application message in the selected branch. In the reducer,
`take_retry(&intent)` consumes a matching retry intent once, then the application
explicitly reserves and begins work. `cancel_intent(&tasks, &intent)` cancels only
its matching current operation. Resource-instance identity, local revision, and
generation reject stale controls, same-key resource recreation, and duplicates.
Taking an intent starts no work by itself. Use `apply_completion_with_retry` to retain a typed failure while scheduling an
explicit broker deadline. A clock-driven update takes `Effect::resource_retry`
once due and begins that operation in the resource state. These logical-clock
retry hooks remain explicit; no view installs an autonomous timer.

Run `cargo run --example resource_view_lifecycle` for a deterministic headless
fixture covering shared consumers, retained-ready failure, typed retry/cancel
intents, late-result suppression, and final release.
