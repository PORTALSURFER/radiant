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
application lowering. Immutable snapshot identities now also let application
receipts skip cached descendants. An interaction-only leaf beside unchanged
components can use the existing atomic partial refresh, avoiding runtime
projection and layout. Interaction-only changes inside a component can also qualify by comparing that
result with its immediately preceding cache entry. Geometry and unsupported
changes retain the complete safe refresh path; OPT-1388 remains partially delivered.

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

Scene lifecycle bindings, custom layout policies/capabilities, overlays, effect
owners, scroll/virtual wrappers and
other unsupported application shapes use ordinary fresh projection. Only built-in text, button and text-input leaves under plain containers currently
qualify for retention. Custom and other widgets project freshly, preserving
arbitrary Clone behavior instead of assuming that cloning a cached declaration
is equivalent to constructing it again. Qualified snapshots use the existing
runtime surface path, preserving runtime-local state. Text-only subtrees can
share immutable child storage; button and text-input snapshots still clone.

## Reconciliation receipt contract

Only an admitted cache result receives a private shared snapshot identity. Cache
hits retain that identity; new results, eviction and remount allocate a new one.
When inputs change under the same function and input types, a bounded comparison
may retain an interaction-only transition from the preceding result. This proof
holds only one predecessor token and changed leaf paths, never a chain of old
subtrees. The committed and candidate receipts own both identities during comparison, so
allocation-address reuse cannot admit a replacement. Root path, slot, source,
identity and kind evidence is still compared normally. Raw `SurfaceNode` wrappers
receive no snapshot witness. A result without a direct proven transition from the committed snapshot selects
full refresh. Geometry, paint, source, slot, membership and unknown mapper changes
also select full refresh. Environment/type changes, eviction and remount never
inherit a predecessor proof.

The receipt is application-owned equality evidence. It does not acknowledge its
own publication, synchronize runtime state, or authorize an interaction. The
existing request/provider/generation fences and atomic runtime publication remain
authoritative. Unchanged retained widgets keep focus, capture and composition;
the partial operation changes only its admitted interaction leaves.

`ComponentProjectionContext::comparison_node_visits()` reports actual node visits
for changed-component comparison, including work before a rejected comparison.
Cache hits inspect no descendants. Comparison is bounded by the retained node
budget, 128 descendant levels, and the existing exact-change path/root limits.
A 32-component test changes one tooltip and visits 101 nodes, leaving the other
3,100 descendants unvisited; a skipped intermediate projection falls back.

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

`runtime_component_local_interaction_3200` changes one tooltip beside 32 cached
components of 100 text leaves. It reports actual projection/layout work and
verifies the updated tooltip after every refresh. A receipt test with 9,600 leaves
emits 34 node records and performs 66 comparisons: 32 snapshot identities plus
34 root/leaf records. Geometry and existing full-refresh/paint scenarios remain
performance controls, not evidence of native frame or GPU improvements.
