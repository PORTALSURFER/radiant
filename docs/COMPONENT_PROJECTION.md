# Explicit component projection reuse

`StatefulAppBuilder::view_with_components(environment_source, view)` adds a
bounded projection context to ordinary application view functions. Components
remain pure Rust functions returning owned views; they acquire no lifecycle,
renderer ownership, hooks, or private mutable application state.

```rust
use radiant::application::{app, column, text, View};
use radiant::runtime::ResolvedEnvironment;

fn count(value: &u32, _: &ResolvedEnvironment) -> View<()> {
    column([text(format!("Count: {value}"))])
}

let bridge = app(0u32)
    .view_with_components(|_| Default::default(), |value, components| {
        components.project("counter", *value, count)
    })
    .into_bridge();
```

Run `cargo run --example component_projection` for the message-driven example
and actual callback/cache-hit counters. The enclosing view still executes on
application projection. An unchanged component skips its function and its
application lowering; runtime projection and geometry still use the existing
safe refresh path. This is partial delivery of OPT-1388, not bounded geometry or
complete incremental reconciliation.

## Dependency and identity contract

`project(key, input, function)` compares the exact input value and function-item
type, together with the complete window/application environment. Inputs must
include every state, theme, resource revision, and message-mapping dependency.
Equality is not inferred from hashes. Opaque mutable resource contents need an
explicit input revision. A component must not read mutable globals or hidden
state: those dependencies cannot be validated by memoization.

Function items and capture-free closures qualify for reuse. Capturing closures
and function pointers always project freshly because their type does not prove
behavioral identity. A change of input type or function-item type also projects
freshly. No `Message: Clone`, `Send`, or `Sync` bound is added. Inputs are owned by
the cache; prefer compact immutable revision records or shared immutable data.

The environment source must be a pure function of application state. It drives
both component environment and runtime presentation. Later calls to
`application_environment(...)` replace that shared source. Window scale,
appearance, contrast or motion changes and application locale, catalog,
direction or text-scale changes invalidate reuse. Theme choices used during
projection must be explicit component inputs; ordinary theme-dependent paint
remains runtime-owned.

Each key is unique within the enclosing projection and becomes the component
root's continuity identity, replacing an id/key returned on that root. Child
explicit ids retain their normal application-wide uniqueness requirement.
Root slot sizing is preserved. Removed keys retire at the end of projection;
reintroducing a removed key invokes its function again. Duplicate keys fail
before publication. A panicking component clears the cache; retained snapshots
never become a second focus, capture, IME, semantic or layout authority.

## Capacity and fallback

The cache retains at most 64 results and 32,768 container/widget nodes in total.
Keys longer than 256 bytes and results exceeding remaining capacity project
normally without retention. Unvisited entries retire at the end of the call.
These bounds constrain retained entry/node counts, not memory hidden inside
application-owned input values or widget resources.

Scene lifecycle bindings, overlays, effect owners, scroll/virtual wrappers and
other unsupported application shapes use ordinary fresh projection. Compatible
plain container/widget snapshots use the existing runtime surface path, which
preserves runtime-local widget state. Text-only subtree snapshots can share
immutable child storage; arbitrary custom widgets keep their existing deep clone
semantics. A cache hit does not imply zero cloning for custom widgets.

## Evidence

Focused tests check callback non-visitation, exact input changes, captured
projector fallback, retirement, capacity, duplicate keys, panic cleanup and
application-environment override. Production differential tests compare cached
and fresh projection during pointer capture and active IME composition, including
layout, paint, focus, semantics and committed input. The public example drives
an actual button click through `SurfaceRuntime`.

The performance harness includes `app_component_projection_cached_9600` and
`app_component_projection_fresh_9600`: 32 components of 300 text leaves, editing
only the first component. Both run the enclosing application projection once per
operation. Separate component callback and hit counters expose the work rather
than inferring it from wall-clock time. These are application projection
controls, not native frame, GPU or display-latency measurements.
