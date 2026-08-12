# Virtual Layout Design

Status: normative design contract. The query-only keyed `VirtualLayoutPolicy`
capability and bounded query executor are shipped as qualified APIs; the
crate-private visible-window coordinator and crate-private accepted-window
materialization/recycling correctness kernel are shipped private slices. The
private retained-item adapter, private `SurfaceRuntime` registration/two-pass
bridge, the current-fence one-item semantic admission path, and its private
semantic projection boundary are also shipped as crate-private/private runtime
evidence. The crate-private semantic-demand owner/provider-attempt/retention
kernel and private atomic whole-surface publication/composition kernel in
[Semantic demand and refresh](#semantic-demand-and-refresh-generic-logical-consumer-shipped-native-contract-defined-custom-deferred)
are shipped and implemented. Their shipped scope includes explicit admission,
exact `SemanticProviderFence` and `SemanticPublicationFence` fields,
generation/attempt/cancellation, typed outcomes and validation, exact fallback
retention, retained-evidence reclassification, and all-or-nothing logical
composition. The next production consumer contract in
[Next production consumer: semantic automation session](#next-production-consumer-semantic-automation-session-normative-generic-logical-implementation-shipped)
is now shipped for the bounded generic logical consumer: its runtime-owned
session owner, selected-publication boundary, explicit refresh/retry/close
operations, and conservative fallback are live. Scheduler/cancellation
transport, native/product provider consumer implementations, and
custom-coordinate implementation remain deferred. The later macOS/AppKit native
semantic accessibility query contract below is normative for that consumer. The
public declarative Logical-only provider attachment contract below is normative
and shipped as the first public-API evidence point; it does not itself provide a
native/product integration.

This document freezes ownership, invariants, and observable behavior for a
future implementation. It does not freeze Rust names, trait signatures, module
boundaries, or storage choices. Any pseudocode in this document is explicitly
non-API. An implementation may use different names when it preserves this
contract.

## Reading and authority

This document specializes the target architecture in
[`DESIGN_DIRECTION.md`](DESIGN_DIRECTION.md). It is constrained by the broader
product target in [`TARGET.md`](TARGET.md), the subsystem boundaries in
[`ARCHITECTURE.md`](ARCHITECTURE.md), and the current public surface described
in [`API.md`](API.md). When a future virtualization implementation needs a
decision specific to keyed windows, this document is the normative source.

The current implementation references are deliberately separate from the
future contract:

- [`VirtualizationPolicy`](../src/gui/layout_core/model/virtualization.rs) is
  the current fixed-child, pixel-overscan policy.
- [`VirtualWindowInfo`](../src/gui/layout_core/engine/types/virtualization.rs)
  is the current index-based layout result for that policy.
- [`VirtualListWindow`](../src/gui/list/virtual_list/window.rs) and
  [`VirtualListController`](../src/gui/list/virtual_list/controller/state.rs)
  are current host-owned fixed-row window helpers.
- [`MaterializedVirtualListItem`](../src/gui/list/virtual_list/item.rs) is the
  current host-facing item geometry/state record.
- [`virtual_list_window`](../src/application/layout_builders/lists/virtual_window.rs)
  and [`VirtualListBuilder`](../src/application/layout_builders/lists/virtual_builder.rs)
  are current explicit projection helpers.
- [`layout_virtualized_child`](../src/gui/layout_core/engine/layout/scroll/virtualization.rs)
  is the current fixed-child culling path.

Those modules are compatibility context, not an implicit promise that the
future keyed design will reuse their names or internal representations.

## 1. Status, scope, and non-goals

### Status

The contract is approved as a design target, with its first two query-only
slices, private materialization/recycling correctness kernel, private
retained-item adapter, and private `SurfaceRuntime` registration/two-pass bridge
implemented and shipped as crate-private/private runtime evidence.
At this status:

1. `radiant::layout::VirtualLayoutPolicy` and its bounded query-only identity,
   input, sink, fence, result, diagnostic, and disposition types provide the
   first capability slice. The policy is not registered through
   `LayoutCapabilities` and receives no runtime or materialization handle.
2. A crate-private `VirtualLayoutWindowCoordinator` provides the second
   slice: one exact container/policy/mount scope, checked query tokens,
   coalesced invalidation evidence, bounded accepted keyed windows, key-based
   continuity, clipped previous-valid fallback, and query-only anchor
   correction evidence. It does not register with a runtime or expose a
   materialization callback.
3. A crate-private accepted-window materialization/recycling kernel validates
   exact scope, fence, owner, and revision evidence before projection or
   lifecycle work. It stages pure host projection separately from lifecycle
   callbacks, preserves stable key/kind continuity, unmounts and resets before
   reuse, bounds active/recyclable/staging state, and atomically publishes the
   complete state only after every lifecycle callback succeeds. Before the first
   lifecycle callback it pessimistically enters an indeterminate state; a
   callback error, reentry, or unwind terminally retires the kernel, clears its
   authority, and never replays or compensates callbacks. Admission, projection,
   and other pre-callback rejection remain recoverable. Runtime policy for that
   terminal state is deferred. The kernel itself has no runtime registration,
   concrete surface projection, focus/accessibility pin ownership, scheduling,
   or product consumer.
4. The runtime consumer boundary is specified in [Runtime consumer bridge evidence](#105-runtime-consumer-bridge-evidence)
   and is now exercised by the private retained-item adapter and private
   `SurfaceRuntime` registration/two-pass bridge as crate-private/private runtime
   evidence. These slices do not add public registration or API,
   scheduler/renderer policy, focus/capture traversal, full accessibility
   semantics, or a product consumer.
5. The private runtime bridge now carries at most one required item key through
   policy input and the exact query fence. A ready result must contain that
   stable key or it is rejected before coordinator commit; changing the key
   invalidates pending work and previous fallback. This is query/materialization
   admission evidence only, not focus traversal or offscreen promotion.
6. The private current-authority semantic admission path accepts only one live
   mounted container identity and one opaque stable item key. It constructs the
   exact request from current registration authority, invokes the immutable
   provider once, and retains one `Semantic` pin only for a valid `Found`
   result. Unstable, stale, malformed, and typed terminal outcomes clear or
   reject the pin before any provider result survives. This is semantic-only
   private evidence with no public consumer: it performs no automation
   traversal, offscreen materialization, focus/capture transfer, scrolling,
   paint, hit testing, scheduler/renderer work, or product integration. A
   crate-private `VirtualLayoutSemanticProjection` may be created only from a
   validated `Semantic` pin. It retains the opaque container/key identity,
   logical index, declared coordinate space, finite bounds,
   `AutomationNodeSemantics`, the exact provider-supplied serializable
   `AutomationNodeId` evidence, exact request/fence evidence, and explicit
   `Unmaterialized` authority. `VirtualLayoutItemKey` remains the lifecycle and
   authority identity; the automation ID is never synthesized from an index, key,
   pointer, slot, or bounds. There is no global ID admission, and the ID is not
   wired into `AutomationTarget` or `GuiAutomationSnapshot`.
7. The current fixed-child and host-projected fixed-row APIs retain their
   existing behavior and compatibility promises.
8. The crate-private semantic-demand owner/provider-attempt/retention kernel and
   private atomic whole-surface `SemanticPublicationFence`
   publication/composition kernel are shipped and implemented. They provide one
   owner per `SurfaceRuntime`, explicit admission, exact provider/publication
   fences, generation/attempt/cancellation, typed outcomes and validation,
   exact fallback retention, retained-evidence reclassification, and
   all-or-nothing logical composition. The generic logical semantic automation
   session and public selected-snapshot visibility are shipped through
   `SurfaceRuntime`; the public declarative Logical-only provider attachment
   contract below is normative and shipped. Native/product provider consumer
   implementations and custom transforms remain outside this contract; the later
   macOS/AppKit native semantic accessibility query contract is defined below.
9. A future slice must name the subset of this contract it implements and must
   not imply that later slices already exist.

### Scope

The future design covers data-backed containers whose logical children may be
large, sparse, reordered, filtered, variable-sized, or only partially known.
It defines:

- stable identity for a container, its policy, and each logical item;
- a bounded, read-only policy query for a visible window and its overscan;
- keyed range-to-bounds, total-extent, measurement, and semantic results;
- anchor preservation across data, geometry, and viewport changes;
- exact revision fences, cancellation, stale-result rejection, and fallback;
- explicit reconciliation, materialization, focus/accessibility pins, and
  recycling ownership; and
- the separate semantic-demand, provider-refresh, and whole-surface semantic
  publication contract for logical virtual ranges;
- the order and acceptance evidence for implementing the design incrementally.

The contract applies equally to lists, grids, trees, timelines, tables, and
other keyed collections when their policy can provide the same bounded
coordinates and identity guarantees. A policy may use one or more axes, but it
must declare the coordinate space and bounded query behavior.

### Non-goals

This contract does not:

- own application data, sorting, filtering, loading, persistence, or domain
  transactions;
- choose how an application extracts a key, except to require the key
  invariants at the boundary;
- make a policy a widget factory, renderer, scheduler, or accessibility-tree
  owner;
- require the application to construct an unbounded hidden widget tree;
- define a complete accessibility tree for every offscreen item;
- transfer domain ownership, reducer state, or widget lifecycle implicitly;
- define a renderer backend, GPU cache, thread model, or scheduler fairness
  algorithm beyond the ownership and fence requirements below;
- replace the current fixed-child `VirtualizationPolicy` path or current
  host-owned `VirtualListWindow` path; or
- define `split_pane` behavior, including its resize, collapse, persistence,
  interaction, or runtime consumer;
- implement custom-coordinate transformation or a production/native consumer
  implementation,
  scheduler/backoff/fairness policy, multiple active ranges per container, or a
  public imperative provider-registration API. The one public declarative
  Logical-only provider-attachment contract is defined below and is shipped.
  The generic logical session and selected-publication runtime
  implementation are shipped below; the
  crate-private owner/provider-attempt/retention and atomic
  publication/composition kernels remain the only provider authority. The
  logical-only target slice rejects `Custom` before provider invocation and has
  no identity-transform fallback.

## 2. Normative vocabulary and invariants

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative. A **container instance** is one mounted keyed collection at one
stable container identity. A **policy instance** is the layout query behavior
registered for that container identity. A **snapshot** is the app-owned data
view identified by one `data_revision`.

The following terms have precise meanings:

- **Logical item**: one app-owned data record identified by a stable key. It
  may have a logical index in the current snapshot, but the index is not its
  identity.
- **Viewport**: the current scroll-visible coordinate interval or rectangle.
- **Overscan**: a finite pre-materialization margin around the viewport. It is
  a work budget, not a request to paint every returned item.
- **Query bounds**: policy-provided candidate bounds for keyed items or keyed
  ranges. They may be estimated until measured.
- **Item bounds**: bounds accepted after the runtime reconciles and, when
  required, lays out or measures a materialized item.
- **Pin**: a bounded runtime request to keep a keyed item available for focus,
  capture, drag, semantic, IME, or another explicitly declared requirement.
- **Semantic bounds**: logical position and extent information used for
  accessibility or navigation. Semantic bounds do not by themselves paint or
  hit-test a widget.
- **Semantic-demand owner**: the one crate-private owner inside a
  `SurfaceRuntime` that records explicit semantic demand, invokes the provider
  under an exact attempt fence, stages/retains evidence, and owns the atomic
  whole-surface selected publication. Public snapshot selection is exposed only
  through the explicit session operations. It is distinct from the
  ordinary layout coordinator and from observational snapshot reads.
- **Range-demand slot**: one active contiguous logical half-open range demand
  for one mounted virtual container. It is not merged with another range and
  does not replace the independent one-item semantic pin.
- **Explicit demand**: only a semantic/accessibility-layer range request or an
  explicit required-item pin. Registration, viewport/overscan, paint,
  hit-testing, provider availability, item count, diagnostics, snapshot reads,
  or IME/native events are not demand. Only the explicit semantic-session
  refresh or retry operation may turn an admitted demand into a provider call.
- **Demand generation**: the monotonically advancing identity of one exact
  demand set. A changed demand or superseding live fence starts a new
  generation; an explicit retry advances the attempt within the unchanged exact
  demand.
- **Provider attempt**: one bounded provider invocation for one container and
  demand generation. At most one invocation is active or recorded for a
  container per attempt.
- **Eligible fallback**: a previously complete virtual composition whose exact
  demand, provider, registration, content, coordinate, and budget fence still
  matches the current demand. A merely recent or observational snapshot is not
  eligible fallback.
- **Complete surface demand-set generation**: the exact generation of all
  active range-demand slots and the independent one-item semantic pin set for a
  surface. A virtual publication is complete only when every active member is
  resolved or staged under the same publication fence.
- **Accepted window**: the last query result committed by the coordinator after
  all identity, revision, cancellation, and structural checks pass.
- **Previous-valid-window fallback**: an accepted window retained while a newer
  query is pending or deferred, subject to the fallback fence rules in
  [Revision fences and fallback](#7-revision-fences-cancellation-and-fallback).

Every implementation MUST preserve these invariants:

1. Application state is the source of truth for domain records and key
   extraction. Radiant never invents a key from a current index, frame number,
   pointer address, or allocation address.
2. One mounted container has exactly one visible-window coordinator and one
   policy identity. Ownership is not inferred from a callback or shared mutable
   object.
3. One `SurfaceRuntime` has exactly one crate-private semantic-demand owner.
   The owner has at most one active contiguous range-demand slot per mounted
   virtual container, plus the independent one-item semantic pin.
4. A policy query is bounded, read-only, deterministic for the supplied
   snapshot and inputs, and cannot create, mount, unmount, focus, paint,
   recycle a widget, create semantic demand, or invoke a semantic provider.
5. Only an accepted window may drive materialization, item bounds, hit testing,
   focus pins, semantic exposure, culling, or paint planning.
6. A result is accepted only against an exact current revision fence. A newer
   result is not allowed to overwrite an older result merely because it was
   delivered later or has a numerically larger individual revision.
7. A key-preserving reorder preserves logical item continuity. A key change,
   incompatible item kind, container identity change, or policy identity change
   does not transfer widget lifecycle or interaction state.
8. All query, item, pin, semantic, and deferred-work sets are explicitly
   bounded. A count, estimate, or semantic request never authorizes full-list
   enumeration or hidden widget materialization.
9. Invalidation is coalesced and reentrant-safe. Policy code, lifecycle code,
   and provider callbacks cannot synchronously re-enter the same coordinator's
   query or commit path; follow-up invalidation is coalesced.
10. Diagnostics are bounded and deterministic. An invalid result is rejected at
   the boundary; it is never repaired by choosing an arbitrary duplicate,
   silently dropping a missing key, or accepting non-finite geometry.

## 3. Ownership contract

The following table is normative. An implementation may combine modules, but it
must preserve the ownership boundaries.

| Concern | Owner | Required behavior and boundary |
| --- | --- | --- |
| Application data and key extraction | Host application/data source | Owns records, membership, ordering, sorting/filtering, loading, and stable key extraction. Supplies a snapshot and `data_revision`. It must not delegate domain identity to a visible index or to widget allocation. |
| UI-local policy queries | The registered policy adapter, invoked by Radiant | Reads only bounded query inputs and app snapshot access. Computes keyed range-to-bounds, extent estimates, and anchor resolution. It must not create semantic demand, invoke a semantic provider, mutate UI state, invoke a materializer, schedule recursive work, or make lifecycle decisions. |
| Virtual-layout registration | The mounted `SurfaceRuntime` | The public declarative capability declares provider capability only. `SurfaceRuntime` derives the live container/policy identity, registration and mount generations, content revisions, provider identity/generation, the fixed `Logical` coordinate, and budget. Registration is not demand, exposes no imperative register/remove API, and never accepts an application-owned mount generation. |
| Semantic demand and publication | One crate-private semantic-demand owner per `SurfaceRuntime`, driven by one explicit public session | Records and owns only explicit semantic/accessibility range requests and explicit required-item pins; owns one active contiguous range-demand slot per mounted virtual container plus the independent one-item semantic pin, provider attempts, exact fences, staging, fallback, and the atomic whole-surface selected publication. It does not grant materialization, scrolling, action, focus, paint, hit-test, scheduler, renderer, or provider authority to semantics. |
| Semantic provider | The registered immutable provider, called by the semantic-demand owner | Supplies only the bounded logical semantic evidence requested by the exact demand. It is called at most once per container/attempt, cannot recursively re-enter the owner, and cannot publish or mutate runtime state. Missing or unsupported capability is an explicit terminal outcome, not a demand source. |
| Radiant visible-window coordinator | Radiant, one instance per mounted container | Owns viewport/overscan state, query sequence, revision fences, cancellation, accepted-window fallback, anchor state, invalidation coalescing, and the desired keyed set. It is the only component that commits a window. |
| Materialization and reconciliation | Eventual `SurfaceRuntime` owner, one materialization record per mounted virtual-container generation, using the coordinator/runtime and an explicit host item projection boundary | `SurfaceRuntime` owns the retained record and chooses which accepted keys require runtime items, then reconciles slots by key. `AppBridge`, `RuntimeBridge`, the policy adapter, and product/application state do not own retained slots. The host supplies item data and an explicit item projection/materializer; querying never implicitly constructs a widget. |
| Measurement | Radiant layout/measurement path, using host-provided item content | Owns measurement requests, measurement cache validity, `measurement_revision`, and the promotion of measured bounds into the next accepted result. The host may provide intrinsic-size inputs but does not mutate the coordinator's cache. |
| Focus and accessibility | Radiant focus/semantic layer with app-supplied semantic data | Owns focus, keyboard traversal, pointer capture continuity, bounded pins, and explicit semantic/accessibility request intent. The host supplies labels, values, roles, actions, and domain focus policy; the semantic layer does not own provider invocation or force a permanent offscreen widget tree. |
| Culling, paint, and hit testing | Radiant layout/input/paint runtime | Consumes accepted item bounds and the scroll clip. Paints visible accepted items, hit-tests only eligible visible/pinned runtime geometry, and never asks the policy to create hidden hit targets during a pointer event. |
| Recycling | Radiant coordinator/runtime slot store | May reuse allocation/storage only after the old keyed item has been unmounted and reset. It must never transfer lifecycle, focus, capture, hover, semantic state, or item-local state from one key to another. |
| Scheduler and renderer | Radiant runtime scheduler/renderer | Schedules bounded deferred queries, measurement, reconciliation, and frame work; owns cancellation delivery and rendering of the accepted scene. It does not create semantic demand from ordinary work, change key identity, repair invalid provider output, or bypass a coordinator or semantic-demand fence. Backoff and fairness are deferred. |

The host may observe window changes or diagnostics, but observation does not
transfer ownership. A callback that sends an application message is an
observation channel, not a second coordinator.

## 4. Identity and admission

### 4.1 Container identity

Every keyed container instance MUST have a stable `container_identity` scoped to
its parent/root identity and its declared container key. The identity MUST
survive ordinary view reconstruction, data reorder, scrolling, and measurement
changes. It MUST NOT be based on a child index, a frame counter, a pointer, or a
temporary allocation.

A structural identity is permitted for one unambiguous static container when
the parent establishes a stable slot and no sibling can collide with it. A
dynamic collection, repeated container, or container moved among keyed siblings
MUST use an explicit stable identity. Equal identities in one parent identity
scope are an admission error.

Changing a container identity creates a new instance: the old coordinator is
unmounted, its work is cancelled, and no coordinator state is transferred to
the new instance.

### 4.2 Policy identity

The policy has a separate stable `policy_identity`. It identifies the declared
query semantics, coordinate model, contract version, and identity-bearing
configuration. It is stable across scroll, data revision, and ordinary
measurement updates. A policy parameter that changes the meaning of indices,
keys, axes, or bounds MUST either advance `policy_revision` without changing
identity when the same policy contract remains compatible, or replace the
policy identity when continuity is unsafe.

Policy identity MUST be explicit at the capability boundary. It MUST NOT be
derived from a closure address or an implementation detail that can change on
every projection. Two policies with the same container identity cannot both be
active. An absent, duplicated, or incompatible policy registration rejects the
mount rather than selecting one arbitrarily.

### 4.3 Item keys

The host extracts one stable key for every admitted logical item. A key MUST:

- be comparable for exact equality within the snapshot and coordinator scope;
- remain equal when its item moves because of insert, remove, sort, or filter;
- identify one logical item within one `(container_identity, data_revision)`;
- be independent of materialization, widget identity, and current index; and
- have deterministic equality and hashing/lookup behavior for the lifetime of
  the query and any retained item slot.

An index MAY be used as an input hint or a temporary pre-key lookup, but it MUST
be resolved to an exact key before the result is accepted as keyed state. A
policy that cannot provide stable keys is a fixed/index-based policy and does
not satisfy this contract.

The same key denotes continuity only when the item's semantic/item kind remains
compatible. If an application reuses a key for an incompatible item kind, the
coordinator MUST diagnose the replacement, unmount the old item, clear its
interaction/lifecycle state, and mount the new item without state transfer.

### 4.4 Duplicate, missing, and ambiguous identity

Identity errors are rejected before a new window is committed:

| Condition | Required result |
| --- | --- |
| Two records expose equal keys in one snapshot | Reject the affected query/data revision. Do not choose first, last, nearest, or visible occurrence. Keep an eligible previous-valid-window fallback only under the exact fence rules. |
| A record needed for an admitted range has no key | Reject the affected range/query. Do not synthesize an index key, omit the record silently, or reuse a prior slot by position. |
| Key extraction is non-deterministic or changes equality during one query | Reject the query and diagnose unstable identity. |
| Unequal keys cannot be distinguished by the selected lookup/fingerprint representation | Reject as ambiguous identity. The lookup must be upgraded or the host must supply a representation with exact equality. |
| A required key or index resolves to multiple records | Reject the request and return a bounded diagnostic; do not guess. |
| A required key is absent after a valid data revision | Treat the item as removed. Release its pin/focus/capture according to the focus policy; do not transfer by index implicitly. |
| Two container or policy registrations collide in one identity scope | Reject the later admission and retain the prior mounted instance if it is still valid. |
| The same key occurs in separate container identity scopes | Allowed. Keys are not globally unique; the scoped identity tuple is. |

Diagnostics SHOULD include the scoped container identity, policy identity,
revision, and a bounded sample of offending keys. Diagnostics MUST NOT retain an
unbounded snapshot or make rejection dependent on diagnostic allocation.

## 5. Conceptual bounded query

The policy boundary is a conceptual operation, not a prescribed public Rust
signature. The following pseudocode is **non-API** and illustrates the minimum
information that must be fenced and the bounded information that may cross the
boundary. Ordinary layout querying and semantic demand/refresh are separate
runtime turns. A query may carry already accepted semantic evidence for
projection, but it cannot create a semantic demand, invoke a semantic provider,
or publish a virtual semantic tree. The semantic-demand turn is specified in
[Semantic demand and refresh](#semantic-demand-and-refresh-generic-logical-consumer-shipped-native-contract-defined-custom-deferred).

The following pseudocode illustrates the ordinary query boundary:

```text
query(policy, QueryInput) -> QueryResult

QueryInput {
    container_identity,
    policy_identity,
    viewport,                 // finite logical rectangle/axis interval
    overscan,                 // finite leading/trailing distance or item budget
    required_item,            // optional exact key or pre-key index request
    anchor,                   // optional key + edge + local offset
    data_revision,
    policy_revision,
    measurement_revision,
    semantic_revision,
    viewport_revision,
    previous_valid_window,    // opaque summary; never a mutation channel
    bounded_budget,
    cancellation_token,
}

QueryResult {
    fence,
    viewport_range,
    overscan_range,
    keyed_range_to_bounds,
    total_extent,
    estimated_extent,
    measured_extent,
    resolved_anchor,
    retained_semantic_observation,
    deferred_work,
    diagnostics,
}
```

### 5.1 Query inputs

The coordinator MUST supply the following inputs, conceptually or through an
equivalent immutable query context:

| Input | Contract |
| --- | --- |
| Container and policy identity | Exact identities from the mounted instance. The policy cannot answer for another container. |
| Viewport | Current finite viewport rectangle or main-axis interval, cross-axis constraints, scroll origin, and coordinate-space declaration. A viewport change advances `viewport_revision`. |
| Overscan | Explicit finite leading/trailing pixel, logical-unit, or item budget, plus the coordinator's maximum. The policy MUST honor the smaller admitted bound. |
| Required key/index | At most the bounded number admitted by the coordinator. A key is authoritative; an index is a lookup hint that must resolve to one key before acceptance. An explicit required-item pin may be a semantic demand source, but a query's ordinary required-item input is never an unbounded fetch. |
| Anchor | Optional primary key, edge/offset rule, and current screen/local offset. An index-only anchor is provisional and loses to a resolved stable key. |
| Data revision | Exact app snapshot revision for membership, ordering, key extraction, and item data relevant to geometry. |
| Policy revision | Exact revision for policy parameters and query semantics that remain compatible with the same policy identity. |
| Measurement revision | Exact revision for accepted intrinsic/item measurements and extent measurements visible to this query. |
| Semantic revision | Exact revision for labels, roles, values, actions, semantic ordering, and on-demand semantic data. |
| Viewport revision | Exact revision for viewport/overscan/constraint/required-request changes. It prevents a scroll result from committing after another scroll. |
| Previous-valid-window summary | Read-only prior accepted keys/bounds/fence for conservative fallback and anchor continuity. It cannot be mutated by policy code. |
| Bounded budget | Maximum returned item/range entries, pins, semantic entries, measurement work, and deferred tasks. The coordinator owns the limits. |
| Cancellation token | Advisory cancellation for abandoned work. Cancellation never replaces the acceptance fence. |

All numeric coordinates and extents MUST be finite and in the declared
coordinate space. Negative sizes, inverted item bounds, overflowed arithmetic,
and unbounded conversions are invalid query output.

### 5.2 Query outputs

The result is accepted only if every output is structurally valid and its fence
matches the current mounted instance. Its conceptual outputs are:

| Output | Contract |
| --- | --- |
| Query fence | Echoes every identity and revision needed for exact acceptance, including the query sequence and viewport revision. Missing fence fields are not implicitly current. |
| Viewport range | The keyed or index-resolved items whose accepted bounds cover the visible viewport. It may be represented as bounded disjoint ranges plus keyed entries. |
| Overscan range | The finite leading/trailing materialization candidate around the viewport. It must not silently expand to the total collection. |
| Keyed range-to-bounds | Ordered entries or bounded ranges of `(key, logical index, bounds, confidence)`. Keys are unique within the result, indices are consistent with the snapshot, and bounds are finite. A range without a key-resolution rule is not a keyed result. |
| Total extent | The current logical content extent when exact. If exact extent is unavailable, the result must distinguish it from an estimate rather than presenting an estimate as measured truth. |
| Estimated extent | A finite extent used for scroll mapping while records or measurements are incomplete. The estimate carries its revision/fence and may be corrected later. |
| Measured extent | Extent contributed by accepted measurements and valid measured gaps. It never silently includes stale measurements from another data, policy, or measurement revision. |
| Resolved anchor | The retained key, resolved bounds/offset, and any bounded scroll adjustment needed to keep the anchor stable. |
| Retained semantic observation | Previously accepted item/range evidence read by ordinary projection. A new key/index/range answer, provider call, or semantic-demand state transition belongs to the separate mutating semantic-demand turn; it is not a complete semantic tree. |
| Deferred work | Finite tokens describing work that may be scheduled later, such as a bounded data fetch, measurement, or semantic lookup. Each token carries the fence needed to accept its completion. |
| Diagnostics | Bounded validation, ambiguity, estimate, cancellation, and budget records. Diagnostics do not turn an invalid result into a valid one. |

An accepted result MAY contain less overscan than requested when the policy or
budget cannot provide more, but it MUST state the resulting extent/window and
remain safe for culling and scroll clamping. It MUST NOT claim that omitted
items were materialized.

### 5.3 Query purity and boundedness

The query MUST be observational with respect to UI/runtime state. It MAY read a
snapshot, a bounded index/key service, and immutable policy configuration. It
MUST NOT:

- construct or mutate a widget/view node;
- mount, update, unmount, recycle, focus, capture, or semantic-register an
  item;
- create, replace, or clear a semantic-demand slot, invoke a semantic provider,
  or publish a virtual semantic composition;
- mutate the scroll offset or anchor directly;
- synchronously send an application message or invoke user code that re-enters
  the coordinator;
- enumerate the entire data set merely to discover a window; or
- return an unbounded vector, iterator, callback stream, or deferred queue.

If more information is required, the query returns a bounded deferred result or
an explicit rejection. The scheduler may retry with a new query fence.

## 6. Bounds, budgets, and deferred work

The four bound classes are intentionally distinct:

### Query bounds

Query bounds are the policy's candidate geometry for the viewport, overscan, and
admitted required items. They may be estimated and are valid only under their
fence. A query bound becomes usable runtime geometry only after the coordinator
accepts its result. The policy must not return a candidate outside the admitted
query budget merely because its total extent is large.

### Item bounds

Item bounds are the runtime's accepted geometry for a materialized keyed item.
They are the authority for ordinary paint and hit testing. They may initially
come from query bounds, then be replaced by a measured/layout result carrying
the same key and exact compatible fence. A measured item with a changed key,
container, policy, or data revision is not an update to the old item.

### Pin bounds

Pin bounds are temporary bounds for explicitly required keyed items that are
outside the ordinary viewport/overscan set. A pin may keep a focused editor,
captured drag source, active drop target, IME composition, or requested semantic
item available. Pins are de-duplicated by key, finite in count, have an owner
and reason, and expire on release, unmount, identity replacement, or the
declared interaction boundary. A pin cannot be used to retain an unbounded
collection.

An offscreen pin may require a scroll/anchor adjustment for focus or interaction,
but the adjustment is a coordinator decision, not a policy-side widget
materialization. A pin that cannot be resolved is reported as deferred,
not-found, or rejected according to its reason.

### Semantic bounds

Semantic bounds describe a logical item's position, extent, ordering, and
relationship for accessibility/navigation. They MAY be available without an
item widget. They MUST NOT create paint, pointer hit regions, pointer capture,
or widget lifecycle by themselves. Semantic evidence retains explicit
`Unmaterialized` authority and `materialized = false` when no ordinary runtime
item exists. Semantics cannot authorize materialization, scrolling, actions,
focus, paint, hit testing, scheduler work, renderer work, or another provider
call.

### Query, item, pin, and semantic budgets

The coordinator MUST impose independent finite budgets for:

- policy query entries/ranges and the number of policy calls per commit;
- materialized item slots, including viewport and overscan;
- simultaneous pins and the number of items one pin request can promote;
- semantic records returned for one request; and
- deferred queries, measurements, data lookups, and diagnostics retained for a
  mounted container.

The total desired materialization set is the bounded union of viewport,
overscan, pins, and explicitly promoted semantic items. Duplicate keys count
once. If a union exceeds a budget, the coordinator applies a deterministic
priority order: visible items, primary focus/capture, declared interaction
pins, leading/trailing overscan, then semantic prefetch. It records the
truncation and never silently materializes the remainder.

Deferred work MUST be:

1. represented by a finite token and reason;
2. associated with the exact query fence and container/policy identity;
3. cancellable on a newer query, revision change, policy replacement, or
   unmount;
4. accepted only through the same fence check as query results; and
5. scheduled outside the policy query and coordinator commit call stack.

The scheduler MAY prioritize visible work over overscan, pins, or semantics, but
it MUST preserve the fence and boundedness rules. A deferred result that misses
its fence is discarded even if it would otherwise be useful.

### Semantic-demand bounds

The approved semantic-demand target adds a separate, finite budget to the
ordinary query and materialization budgets:

- one crate-private semantic-demand owner exists per `SurfaceRuntime`;
- a surface admits at most `MAX_VIRTUAL_LAYOUT_REGISTRATIONS` virtual-layout
  registrations, and `MAX_VIRTUAL_LAYOUT_REGISTRATIONS` is 64;
- each mounted virtual container has at most one active contiguous logical
  range-demand slot, with no range merging, plus the existing independent
  one-item semantic pin;
- each range demand has a finite per-registration maximum and MUST be no larger
  than `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES`, and `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES`
  is 1024;
- the aggregate length of all active range-demand slots on one surface MUST be
  no greater than 1024; and
- the semantic provider is called at most once for one container and one
  provider attempt.

The planned declarative capability may declare that a provider exists, but it
does not select a demand, create a slot, or authorize provider work.
`SurfaceRuntime` derives the live identity, registration/mount generations,
revisions, provider identity/generation, fixed `Logical` coordinate, and budget
used by the demand fence. Provider availability, item count, viewport,
visibility, overscan, paint, hit testing, diagnostics, snapshot reads, and
IME/native events do not consume or create demand. Only an explicit semantic
session refresh or retry may execute the resulting source ticket.

## 7. Revision fences, cancellation, and fallback

### 7.1 Exact fence

Every query, measurement, semantic lookup, deferred task, and materialization
acceptance carries this conceptual fence:

```text
Fence {
    container_identity,
    policy_identity,
    mount_generation,
    query_sequence,
    viewport_revision,
    data_revision,
    policy_revision,
    measurement_revision,
    semantic_revision,
    query_input_digest,
}
```

This pseudocode is **non-API**. `query_input_digest` covers any additional
input whose change could alter the result, including viewport constraints,
overscan, required-item request, anchor request, coordinate scale, and budget
parameters. An implementation may represent it with structured fields instead
of a digest.

Acceptance requires exact equality for all applicable fence fields:

```text
accept(result) iff
    result.fence.container_identity == current.container_identity
    && result.fence.policy_identity == current.policy_identity
    && result.fence.mount_generation == current.mount_generation
    && result.fence.query_sequence == latest_query_sequence
    && result.fence.viewport_revision == current.viewport_revision
    && result.fence.data_revision == current.data_revision
    && result.fence.policy_revision == current.policy_revision
    && result.fence.measurement_revision == current.measurement_revision
    && result.fence.semantic_revision == current.semantic_revision
    && result.fence.query_input_digest == current.query_input_digest
    && !result.cancellation_token.is_cancelled()
    && structural_validation_passes(result)
```

This pseudocode is **non-API**. Equality is intentional: `>=`, “latest known
revision,” or “same data but newer measurement” substitution is not sufficient.
If a change is compatible and should be accepted, the coordinator issues a new
query with a new exact fence.

The semantic-demand turn has two conceptual exact fences. A per-slot provider
completion and retention use `SemanticProviderFence`; whole-surface publication
uses `SemanticPublicationFence`, which wraps the exact provider fence and adds
the current composition authorities. The names and storage are non-API:

```text
SemanticProviderFence {
    container_identity,
    policy_identity,
    registration_identity,
    registration_generation,
    mount_generation,
    data_revision,
    policy_revision,
    measurement_revision,
    semantic_revision,
    coordinate_space,
    budget,
    exact_demand,                       // one logical range or one pinned item
    provider_identity,
    provider_generation,
    demand_source,                      // semantic range request or required pin
    demand_generation,
    attempt,
    cancellation,
}

SemanticPublicationFence {
    provider_fence,                      // exact SemanticProviderFence
    materialization_authority,
    classification_authority,
    ordinary_projection_generation,
    complete_surface_demand_set_generation,
}
```

Provider completion and per-slot retention require exact equality of every
`SemanticProviderFence` field. They do not require
`complete_surface_demand_set_generation`. Whole-surface publication requires
exact equality of the wrapped provider fence and every
`SemanticPublicationFence` field against the live owner, materialization,
classification, ordinary-projection, and complete-surface authority. A missing
provider is represented by an exact `NoProvider` provider identity/generation,
not by a wildcard. A cancelled or superseded attempt is not accepted merely
because the provider returned a structurally valid result. No individual
revision may be compared independently, and no fence may be partially
matched. `registration_generation`, `mount_generation`, and each
source-qualified provider generation are runtime-owned; an application cannot
manufacture or advance any of them.

Adding, removing, or superseding another active slot advances only the
publication generation for unchanged slots. Exact unchanged provider evidence
may be restaged under the new `SemanticPublicationFence` without provider
reentry. Provider completion and per-slot retention still require exact equality
of the unchanged slot's `SemanticProviderFence`.

### 7.2 Cancellation and stale rejection

The coordinator MUST cancel or supersede an older query when any of the
following occurs:

- a newer query is issued for the same container;
- the viewport, overscan, required item, or anchor request changes;
- data, policy, measurement, or semantic revision changes;
- container or policy identity changes; or
- the container begins unmounting.

Cancellation is an efficiency and ownership signal. It is not proof that work
stopped. Every completion still performs the exact fence check. A stale,
cancelled, malformed, ambiguous, or over-budget result is rejected and cannot
replace the accepted window, item bounds, focus pins, semantic result, or
scroll offset. It MAY contribute one bounded diagnostic.

For semantic demand/refresh, a stale or superseded provider completion is
ignored entirely: it does not clear a slot, retain evidence, publish a
diagnostic-driven fallback, or trigger a retry. Follow-up invalidation is
coalesced for a later runtime turn under a new exact demand generation or
attempt.

### 7.3 Previous-valid-window fallback

The coordinator retains the last accepted window as a previous-valid-window
candidate while a new query is pending, deferred, cancelled, or rejected.

The candidate MAY remain the active fallback only when:

- container identity, policy identity, mount generation, data revision, policy
  revision, measurement revision, and semantic revision are all unchanged;
- the prior bounds are finite and structurally valid; and
- using the prior window cannot expose an item outside the current safe clip or
  required interaction boundary.

This permits a viewport-only query to be delayed without flashing or tearing
down valid items. A changed `viewport_revision` alone makes the candidate
non-authoritative for the new viewport, but it may be used conservatively until
the replacement is accepted; hit testing and focus must be clipped to the
current safe intersection.

If any content-bearing identity or revision differs, the old window MUST NOT be
used as current item geometry, hit-test state, focus state, or semantic truth.
The runtime MAY retain a last-painted inert frame or safe placeholder while a
fresh query is pending, but it must not expose stale data as accepted content.
If no safe fallback exists, the coordinator publishes a bounded empty or
placeholder window and a diagnostic, then retries through normal scheduling.

For a semantic publication, fallback eligibility is stricter than the ordinary
window rule: the retained evidence must match the exact demand, provider
identity/generation, registration identity, content revisions, coordinate, and
budget fence. Current materialization authority and ordinary projection
generation may reclassify or recompose that retained exact evidence without a
provider call. If the exact fallback is not eligible, the runtime withholds the
complete new virtual generation and retains either the prior eligible complete
composition or an ordinary-only baseline; it never exposes a mixed partial
virtual tree.

### 7.4 Invalidation and reentrancy

Invalidations are queued as flags/reasons and coalesced before the next query.
At minimum, an implementation distinguishes viewport, data, policy,
measurement, semantic, mount, and unmount invalidation. A policy query,
materializer, lifecycle callback, measurement callback, or semantic callback
MUST NOT synchronously query or commit the same coordinator.

The commit path is a non-reentrant critical section owned by the coordinator.
If a callback requests invalidation during commit, the coordinator records it,
finishes the current bounded commit, and schedules one subsequent query with a
new sequence/fence. It does not recurse. Semantic provider callbacks have the
same rule: they cannot reenter the semantic-demand owner or publication path;
one follow-up invalidation is coalesced for a later runtime turn. If the
bounded invalidation queue overflows, it coalesces to a conservative full
invalidation and records one diagnostic; it does not drop the invalidation
silently.

## 8. Anchor and scroll behavior

An anchor is a stable key plus an edge/offset rule. The default primary anchor
is the focused/captured item when it has a valid key; otherwise it is the
explicit scroll anchor; otherwise it is the leading visible keyed item. The
anchor is not an index. A missing anchor may be resolved only from authoritative
required-item evidence, not from the absence of a key in a bounded window.

The coordinator preserves the anchor's chosen screen position as far as finite
extent information permits. The policy supplies estimates; the coordinator
owns the actual scroll adjustment and later correction.

The query-only coordinator shipped in this slice is narrower. An explicit anchor
key remains authoritative across accepted and non-accepted query outcomes. It
produces same-key correction only when that key is present in both accepted
bounded windows. If the key is absent from a later bounded result, the anchor
remains unresolved and no correction is emitted; the key may become active when
it reappears. This bounded absence is not deletion evidence, so successor or
predecessor replacement waits for a later prerequisite that can report
authoritative required-key `found`/`not_found` evidence.

| Change | Required anchor behavior |
| --- | --- |
| Insert before the anchor | Keep the anchor key and local offset. Add the inserted extent to the scroll position when measured; use the current bounded estimate first and apply a fenced correction when measured. Do not move the anchor to the same index. |
| Insert after the anchor | Keep the anchor position. Do not adjust scroll solely because content was appended after it, except for a declared end-follow policy. |
| Remove before the anchor | Keep the anchor key and subtract the removed item's valid extent, or its bounded estimate, from the scroll position. Clamp to the new total extent. |
| Remove after the anchor | Keep the anchor and scroll position, then clamp to the new extent. |
| Reorder | Find the same anchor key in the new snapshot and recompute its bounds. Preserve key continuity regardless of its new index. A reorder must not be treated as remove-old/add-new when the key and item kind remain compatible. |
| Measurement changes before the anchor | Adjust scroll by the measured delta so the anchor remains at the same screen position. Corrections are fenced by the measurement revision and may be deferred. |
| Measurement changes on the anchor | Preserve the declared anchor edge and local offset. If the anchor has no edge rule, use the leading edge; do not let a new size arbitrarily center or jump the viewport. |
| Viewport resize or cross-axis constraint change | Keep the primary anchor key and edge/offset, recompute the window under a new viewport revision, and clamp. If the viewport becomes empty, retain the key as a pending required item only within the pin budget. |
| Scroll input | Update the viewport revision and requested scroll position. Keep the previous valid window as a conservative fallback while querying; do not synchronously materialize every newly crossed index. |
| Anchor removal | After authoritative required-key `not_found` evidence, do not transfer by index. A later runtime slice may choose the first surviving successor, then the nearest surviving predecessor, then the first item, then the empty state, with a bounded anchor-replaced diagnostic. A bounded-window absence alone does not authorize replacement. |
| Anchor key temporarily unavailable | Preserve the explicit key as unresolved and emit no correction from bounded absence. Once authoritative required-key evidence resolves the key, key identity wins over any provisional position. |

If the anchor and the primary focus key differ, focus continuity wins for
keyboard/capture behavior and the explicit scroll anchor wins for passive
scrolling only when focus remains within its guard policy. Multiple anchors are
not allowed to cause unbounded retention; one primary anchor and a bounded set
of required pins are sufficient.

An anchor adjustment is a runtime/layout result, not an implicit application
data mutation. The host may observe it or persist a deliberate settled scroll
position through its own API, but a policy query cannot mutate host scroll state.

## 9. Mount, query, reconciliation, and unmount state machine

The following state names are conceptual and non-API. They define observable
ordering rather than a required enum.

```text
Unmounted
  --mount(identity, policy)--> Mounted
  --mount query--> Querying
  --valid fenced result--> Accepted
  --invalid/deferred/stale result--> Rejected
  --retry after queued invalidation--> Querying
  --scroll--> Querying
  --measurement result/request--> MeasurementPending --> Querying
  --data revision--> DataRevisionPending --> Querying
  --policy/semantic revision--> Querying
  --unmount--> Unmounting --> Unmounted
```

This diagram is **non-API pseudocode**. The required transitions are:

| State/event | Required behavior |
| --- | --- |
| Mount | The eventual `SurfaceRuntime` owner admits one unique container/policy identity, discovers the future registration descriptor during the shell pass, creates a new `mount_generation`, resets or initializes the coordinator, and issues an initial bounded query. Mount does not materialize the entire data set. |
| Querying | Capture the exact fence and cancellation token. The policy may return accepted candidates, deferred work, or a rejection, but it cannot mutate runtime state. |
| Accept | Validate fence, keys, bounds, extent, budgets, anchor, and diagnostics. Commit the accepted window atomically, then reconcile the bounded desired item set. This is kernel evidence only; it does not expose a partial child collection. |
| Reject | Do not install any part of the result. Keep an eligible previous-valid-window fallback; otherwise publish a safe empty/placeholder result. Record a bounded reason and schedule retry only when useful. |
| Scroll | Advance `viewport_revision`, cancel/supersede the prior query, preserve the current safe fallback, and issue a new bounded query. Scroll does not itself transfer item lifecycle. |
| Measurement | Accept only a key-matching, exact-fenced measurement. Advance `measurement_revision`, update caches, recompute anchor correction, and issue a new query when geometry/extent changes. |
| Data revision | Invalidate old membership/order/item data, advance `data_revision`, reconcile the anchor by key, release removed-key pins, and issue a new query. An old result cannot be accepted against the new data. |
| Semantic revision | Advance `semantic_revision`, invalidate affected semantic results/pins, retain item geometry only where its content fence remains valid, and issue bounded semantic/query work. |
| Unmount | Advance `mount_generation`, cancel every query/deferred task/measurement, release pins, unmount retained items in deterministic order, and prevent all late completions from committing. |

### 9.1 Atomic acceptance

Acceptance is one coordinator commit. The coordinator must not expose a mixed
window containing new bounds with old extent, old keys with new data, or new
semantic pins with an old semantic revision. A renderer may continue showing a
previous accepted frame until the commit is complete, but the runtime's next
paint, hit-test, focus, and semantic snapshot must use one accepted fence.

### 9.2 Reconciliation ordering

For an accepted desired keyed set, the coordinator performs this logical order:

1. Validate all keys and the desired-set budget.
2. Mark removed keys and release their pins/capture/focus ownership as required.
3. Unmount removed or incompatible items before reusing any storage.
4. Reconcile retained compatible keys by key, including reorder and updated
   bounds.
5. Recycle only reset slots, if available, and mount new keys through the
   explicit item projection boundary.
6. Perform bounded layout/measurement work and publish item bounds.
7. Publish culling, paint, hit-test, focus, and semantic snapshots from the
   same committed window/fence.

An implementation may optimize allocation, but it must preserve the observable
remove-before-reuse and same-key continuity rules.

### 9.3 Semantic-demand state and attempt sequence

The semantic-demand state names below are conceptual and non-API. They define
the required sequence for one mounted virtual container:

1. Registration exposes capability only. It does not allocate a demand slot or
   invoke a provider. `SurfaceRuntime` derives the live registration, identity,
   revisions, provider identity/generation, coordinate, and budget for a later
   demand turn.
2. An explicit refresh may admit a semantic/accessibility-layer range request or
   required-item pin for its source. The owner records one contiguous range slot
   or the independent one-item semantic pin, advances the demand generation
   when the exact demand changes, and starts attempt one. It never merges ranges
   or treats ordinary runtime activity as demand.
3. An attempt captures the complete per-slot provider fence and calls the
   provider at most once for that container/attempt, outside any recursive
   owner/publication entry. A later explicit retry uses the same unchanged
   exact demand with a new attempt; a changed live fence or demand invalidates
   old work but does not call a provider until a new explicit refresh or retry.
4. `Found` is structurally and exactly validated, then staged under the
   provider and publication fences. No entry is published independently.
   `NotFound` is authoritative empty evidence for the exact demand and resolves
   that slot.
5. `Unavailable(NoProvider)`, `Unavailable(Unsupported)`, and
   `Unavailable(DataUnavailable)` retain their typed outcome behavior:
   `Unavailable(NoProvider)` and `Unavailable(Unsupported)` are terminal for
   the new virtual publication and clear the affected slot;
   `Unavailable(DataUnavailable)` and `Deferred` retain only an eligible
   exact-fence fallback; without one, the complete new virtual generation is
   withheld. `Rejected`, panic, malformed, and collision evidence use the
   conservative ordinary baseline without an automatic retry. A stale,
   cancelled, or superseded completion is ignored entirely.
6. Each slot is first validated under its exact provider fence and then staged
   under the current publication fence. Adding, removing, or superseding
   another active slot advances only the publication generation for unchanged
   slots; their exact provider evidence may be restaged without provider
   reentry. Only after every active demand member is resolved or staged under
   one exact publication generation may the owner publish the virtual semantic
   composition. Failure retains the prior eligible complete composition or an
   ordinary-only baseline, never a mixed partial virtual tree.
7. A materialization or ordinary-projection change may reclassify and recompose
   retained exact evidence with current materialization/classification
   authority and ordinary projection generation. It MUST NOT reenter the
   provider merely because a retained item became materialized, unmaterialized,
   replaced, or recomposed.

The owner clears a resolved or terminal slot only for the exact source and
demand that produced the outcome. The independent one-item semantic pin is not
silently replaced by a range result, and a range result cannot manufacture a
second pin. No state transition in this sequence grants semantic authority over
materialization, scrolling, actions, focus, paint, hit testing, scheduling,
rendering, or provider registration.

## 10. Materialization, lifecycle, and recycling

### 10.1 No implicit widget materialization

The policy returns keys, indices, bounds, extents, and semantic answers. It does
not return widgets and cannot call the host's item projection. The coordinator
decides whether an accepted key belongs to the bounded desired set; only then
does it invoke an explicitly registered item materializer/projection boundary.

The following are forbidden:

- constructing a widget because a policy query inspected a key;
- constructing hidden widgets to make scroll extent or semantics appear
  complete;
- materializing in a hit-test fallback when no accepted item bounds exist;
- retaining a widget solely because its old index is still in a window; or
- letting a semantic lookup silently transfer a widget lifecycle to a new key.

Only a separate explicit focus/interaction runtime consumer may cause the
coordinator to issue a bounded pin, scroll, or materialization transition for a
required key. Semantic evidence alone cannot authorize materialization,
scrolling, actions, or focus; a semantic request may create or refresh only
the exact semantic evidence allowed by the semantic-demand contract. Any
focus/interaction transition is an observable coordinator decision, not an
implicit side effect of a policy or semantic-provider callback.

### 10.2 Item lifecycle ownership

The host owns the data record and the projection function. Radiant owns the
runtime slot once the item is explicitly materialized within a mounted
container. A runtime slot has one active key and one container/policy scope at
a time. The slot's lifecycle is ordered and fence-aware:

- mount follows accepted admission and key validation;
- update/reconcile follows a same-key compatible accepted result;
- measure follows the item content and measurement fence; and
- unmount happens before removal, identity replacement, recycling, or
  container unmount.

Unmount cleanup MUST run while the old key is still known, so pointer capture,
focus, IME, drag, hover, semantic registration, and item-local subscriptions
can be released against the correct identity. A callback after unmount is
cancelled or ignored by mount generation and must not resurrect the slot.

The shipped private kernel's publication atomicity is success-only. It stages
the complete next active/recyclable/slot/fence/revision state, then installs it
only after all unmount, reset, reconcile, and mount callbacks succeed. Before
the first such callback it pessimistically hides the old authority. Any
callback error, lifecycle reentry, or callback unwind terminally retires the
kernel with no active/recyclable/fence/revision authority, without rollback,
compensation, callback replay, or recovery claim. Pre-callback admission and
pure-projection failures remain recoverable; runtime retry or replacement
policy is a later integration decision.

### 10.3 No implicit lifecycle transfer

Lifecycle and interaction state may continue across a reorder only for the same
key, same container identity, same compatible item kind, and a valid fence. A
different key, incompatible item kind, changed container identity, or changed
policy identity requires old-item unmount and new-item mount. The coordinator
must not copy focus, capture, hover, selection overlay, semantic node, local
state, or callback ownership from the old item to the new one.

Application state may intentionally preserve domain selection or editing data
by key; that is application continuity, not runtime lifecycle transfer.

### 10.4 Recycling

Recycling is an allocation optimization only. A slot may enter a recyclable
pool after its old item has been unmounted, its callbacks cancelled, its
interaction and semantic state reset, and its old fence retired. Reuse then
mounts a new item as a fresh lifecycle. A pool must never retain an active
focus/capture/IME/semantic registration or use a stale item key as a lookup
shortcut.

### 10.5 Runtime consumer bridge evidence

This section records the existing crate-private/private runtime evidence for
the runtime consumer boundary. The private retained-item adapter and private
`SurfaceRuntime` registration/two-pass bridge are shipped as bounded runtime
evidence; they do not claim public registration, a public API, or product
integration. Names, trait signatures, and storage types remain non-API, and the
bridge does not add scheduler/renderer policy, focus/capture traversal, full
accessibility semantics, or a product consumer.

For one mounted virtual container generation, the shipped private bridge keeps
exactly one materialization record owned by `SurfaceRuntime`. The record may contain the
coordinator evidence, retained item payloads, slot generations, and lifecycle
authority needed for that mounted generation, but it is one runtime-owned record
rather than one retained owner per callback or per policy result. `AppBridge`,
`RuntimeBridge`, the policy adapter, and product/application state MUST NOT own
retained slots. They may supply immutable application data, projection
descriptors, messages, or observations, but none of those boundaries may retain
the materialized slot set.

#### Registration evidence and the two-stage mount

The shipped crate-private registration descriptor is discoverable from the declarative
`UiSurface`/`SurfaceContainer` shell before any materialized item children exist.
The descriptor is shell evidence for the private runtime owner; it is not a new
public registration/API in this slice. No public export, public registration
method, or capability contract version is added now.

Initial mount is a synchronous two-stage pipeline on the owning UI runtime. Its
required order is:

```text
pull shell → project/layout shell → query → project complete item batch
→ identity admission → lifecycle commit → install children → final project/layout
```

The shell stage pulls the declarative shell, discovers the descriptor, and
projects/layouts only enough to obtain container viewport and coordinate-space
evidence for the exact query. It MUST NOT expose an incomplete collection,
placeholder child set, or partially installed item batch to paint, hit testing,
focus, semantics, or an observing host. After a query is accepted, the item
stage pure-projects the complete bounded active batch, admits its identities,
commits lifecycle, installs the complete child set, and performs the final
projection/layout. A rejected, deferred, stale, or otherwise non-accepted query
does not start the item stage.

Refresh MUST repeat the same shell-and-item sequence when registration evidence,
container/policy scope, data or measurement fence, coordinate evidence, or
relevant geometry changes. A viewport change is relevant geometry: a viewport
relayout alone MUST NOT leave an accepted virtual window silently stale. The
consumer must invalidate/requery the affected coordinator before treating that
window as authoritative, even when the general `SurfaceRuntime` refresh path
could otherwise reuse completed layout. A previous-valid fallback may remain
visible only under the exact fallback fence rules; it is not a new incomplete
collection. This ordinary shell/item refresh rule is separate from semantic
provider refresh: a viewport-only or ordinary-projection refresh does not call
the semantic provider unless it is part of explicit
`refresh_semantic_automation_session` or `retry_semantic_automation_session`
demand intent in
[Semantic demand and refresh](#semantic-demand-and-refresh-generic-logical-consumer-shipped-native-contract-defined-custom-deferred)
also changes.

#### Projection, identity, and retained payload

The host item projector consumes only exact accepted kernel evidence from the
committed window. Any immutable projection descriptor/data it uses is part of
that admitted evidence, not a mutable runtime lookup. It MUST NOT read the
scheduler, renderer, focus or interaction state, or any other mutable runtime
state while projecting. It MUST not query the policy again or use a newer
unaccepted result to fill missing fields.

Future `ViewNode` lowering occurs as a fallible, pure projection/preflight step
before any lifecycle callback. The retained payload passed to the materialization
store is an immutable `SurfaceNode` subtree. Runtime-mutated widget instances,
traversal indexes, focus/capture state, and other mutable runtime objects MUST
never enter that store as retained payload.

Each materialized item is lowered below a slot wrapper whose scoped identity is
the tuple `(container NodeId, mount generation, slot index, checked slot
generation)`. A compatible same-key refresh preserves the complete wrapper
identity and all descendant identity. Removal or incompatible replacement
unmounts the old item and advances the checked slot generation before the slot
can be reused; a new container generation supplies a new mount generation.
Descendants MUST be scoped below their slot wrapper, never directly beside the
container or in a shared scene/root scope, so a recycled slot cannot collide
with a sibling or transfer descendant state.

The first adapter MUST reject an item that supplies an explicit raw `NodeId` or
a direct/pre-retained `SurfaceNode` identity. It MUST also reject scene-level
presentation, shortcut, overlay, or other out-of-band effects from an item
subtree. Those forms remain unsupported unless a later contract defines
collision-safe remapping and ownership for them; the adapter MUST NOT guess a
remapping or promote an item effect to the containing scene.

Whole-shell and active-batch identity admission is one transaction and MUST
complete before any lifecycle side effect. The shell identity, registration
scope, slot wrappers, and every active descendant identity are admitted before
`unmount`, `reset`, `reconcile`, or `mount` can run. A projection, identity, or
capacity rejection before callbacks leaves the current authoritative kernel
state recoverable, preserving the shipped materialization-kernel behavior.

#### Unmount, failure, and callback boundaries

Descriptor removal, container-generation replacement, and runtime close each
perform one explicit unmount transition for the affected materialization record
before dropping that materialization owner/record from `SurfaceRuntime`. A
cleanly retired record ignores a duplicate close/removal without replaying an
unmount callback. If a lifecycle callback fails, unwinds, or re-enters, the
terminal lifecycle state suppresses the partial materialized tree; it does not
automatically retry, replay callbacks, or transfer state to a replacement.
Replacement and recovery policy are deferred to later runtime integration.

This private bridge is synchronous and has no scheduler or renderer callbacks.
It may request ordinary runtime work through an already-existing host contract,
but it does not make scheduler/renderer policy part of item projection or
lifecycle admission. Existing public APIs and all existing contract versions
remain unchanged.

## 11. Focus, accessibility, culling, paint, and hit testing

### Focus and interaction

Radiant owns runtime focus, keyboard traversal, pointer capture, and the bounded
required-key/pin set. The host owns domain selection and supplies explicit
messages or policy for domain-level focus changes. Focus follows a stable key
through reorder and measurement when the key remains present and compatible.

If the focused key is removed, focus is cleared or moved only by the declared
focus policy. It is never moved to the item that happens to occupy the old
index. A focus request for an offscreen key may request one bounded pin and
scroll-to-anchor operation. It must not expand the complete collection.

### Accessibility and semantics

The semantic layer exposes a bounded virtual collection model: count or extent
when known, stable keys, logical positions, semantic bounds, labels, values,
roles, relationships, and supported actions. Offscreen semantics are obtained
through bounded on-demand results. A request for one item may return one item,
a finite bounded range, `not_found`, `unsupported`, or `deferred`; it must not
force a permanent accessibility tree or complete widget materialization.

Semantic order is the policy's validated logical/semantic order, not an
accidental paint order. A semantic revision invalidates labels/roles/actions
without necessarily invalidating geometry, but any result still requires an
exact semantic fence at acceptance.

### Semantic demand and refresh (generic logical consumer shipped; native contract defined; custom deferred)

This is the approved contract and implementation boundary for provider-backed
virtual semantic demand, refresh, and private atomic publication. The
crate-private semantic-demand owner/provider-attempt/retention kernel and
private `SemanticPublicationFence` publication/composition kernel are shipped
and implemented: explicit admission, exact provider/publication fence fields,
generation/attempt/cancellation, typed outcomes and validation, exact fallback
retention, retained-evidence reclassification, and all-or-nothing logical
composition are current private runtime behavior. The generic logical runtime
session consumer and public selected-snapshot boundary are also shipped;
scheduler/cancellation transport, native/product provider consumer
implementations, and custom transforms remain unshipped. The later macOS/AppKit
native semantic accessibility query contract is defined below. The private kernel
does not add a public
imperative provider-registration surface; the shipped qualified surface is
defined in
[Public declarative logical provider attachment](#public-declarative-logical-provider-attachment-normative-shipped).

#### Owner, sources, and logical scope

Each `SurfaceRuntime` has one crate-private semantic-demand owner. For every
mounted virtual container, that owner has one active contiguous logical
range-demand slot and the existing independent one-item semantic pin. A new
range replaces/supersedes the old range slot for that container; ranges are not
merged, split, or accumulated. The aggregate active range length across the
surface is at most 1024, and the surface has at most 64 virtual-layout
registrations (`MAX_VIRTUAL_LAYOUT_REGISTRATIONS`). Each registration's finite
maximum and `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` (1024) both bound its range
length. The provider is called at most once per container and attempt.

Capability-only registration is admitted by `SurfaceRuntime` only within the
64-registration limit and only when its mounted container/registration scope
is not a duplicate. Admission binds the provider identity and generation,
container and mount identity, and live data/policy/measurement/semantic
revisions, coordinate, and budget to that mounted container. A registration
replacement or unmount retires the prior registration, marks its in-flight
attempts cancelled, and clears its range-demand slot, independent semantic pin,
and pending demand state before the old authority is dropped. Registration,
replacement, and unmount do not invoke a provider or create demand. The current
registration bridge is private; the qualified public declarative capability is
shipped at the boundary defined below.

A same-scope declaration update may retain the mounted container identity, but
the runtime advances the registration generation for changed capability or
revision evidence. Replacing only the item provider or only the range provider
advances that source's provider generation, cancels the source's in-flight
attempts, and makes the old source evidence ineligible. Removing a provider
produces runtime-synthesized `NoProvider` for a later explicit demand, not a
callback. None of these lifecycle changes calls a provider by itself; an
explicit refresh or retry is required to issue a new source ticket.

Only an explicit session operation creates active semantic demand:

1. `refresh_semantic_automation_session(session, demands)` may admit an
   explicit range request or required-item pin; or
2. `retry_semantic_automation_session(session)` may reattempt the unchanged
   existing demand set.

Registration declares capability only. A registration does not select a range,
create a pin, or initiate a provider call. Viewport, visibility, and overscan
changes, ordinary layout or paint, hit testing, provider availability reads,
item count, diagnostics, `automation_snapshot`/`automation_target_snapshot`
reads, and IME/native events are not demand. `SurfaceRuntime` derives the live
container/policy/registration identity, registration and mount generations,
data/policy/measurement/semantic revisions, provider identity/generation, fixed
`Logical` coordinate, and budget before a demand attempt.

The target provider path is logical-only. `Logical` coordinates may be
validated and staged. `Custom` coordinates are unavailable/rejected before
provider invocation; no identity-transform fallback is permitted. The public
parts have no custom-coordinate field. Custom-coordinate transformation and the
production/native provider consumer implementation remain deferred; the public
provider implementation is shipped.

#### Demand generations, attempts, and refresh

An explicit session refresh creates a demand generation and attempt one for
each admitted source ticket; an explicit retry of the unchanged exact demand
creates a new attempt within the same demand generation. A changed exact
demand or live identity, mount, data, policy, measurement, semantic, provider,
coordinate, or budget fence supersedes old work, but that change alone does not
create demand or call a provider. Every attempt carries the exact demand
source: a semantic range request or an explicit required-item pin.

Only `refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` may execute those source tickets.
A snapshot read, ordinary snapshot observation, ordinary repaint, paint-only
invalidation, unchanged refresh, registration, opening, enumeration, provider
availability read, item-count read, diagnostics, viewport/visibility/overscan
change, or IME/native event MUST NOT invoke the provider. A materialization or
ordinary-projection change may reclassify or recompose retained exact evidence
using current materialization/classification authority and ordinary projection
generation; it MUST NOT reenter the provider. A changed provider availability
or replacement invalidates the old provider fence; it is not demand by itself.

#### Cancellation and supersession

Before supersession, registration replacement, unmount, or owner retirement,
the semantic-demand owner marks the active attempt cancelled in its private
fence. Where the private runtime work boundary provides cancellation delivery,
the owner delivers that cancellation before dropping the old authority. A
provider return after cancellation is stale regardless of structural validity
or matching result fields: it is ignored entirely and cannot retain, clear,
diagnose, retry, classify, recompose, or publish evidence. Cancellation does
not automatically retry. Only one of the listed refresh triggers or an
explicit retry creates a new attempt under a new exact fence.

#### Provider outcomes and exact fences

Provider completion and per-slot retention use exact equality of the
`SemanticProviderFence`: container/policy/registration identity and generation,
mount generation, data/policy/measurement/semantic revisions, coordinate,
budget, exact demand, provider identity/generation, demand source, demand
generation, attempt, and cancellation. They do not require the complete
surface demand-set generation. Whole-surface publication uses exact equality of a
`SemanticPublicationFence`, which wraps that exact provider fence and adds
materialization authority, classification authority, ordinary projection
generation, and complete surface demand-set generation. No field is inferred
from registration or a snapshot read, and no `>=`, partial, or “latest known”
match is valid.

At most one provider call is made for a container and attempt. The callback is
read-only, cannot recursively reenter demand or publication, and cannot unwind
through the public runtime boundary. A provider panic is contained and mapped
to the conservative rejected/panic outcome. A reentry attempt is rejected and
coalesced for a later runtime turn; it never creates a nested provider call.
The outcome rules are:

| Provider or validation outcome | Required semantic-demand behavior |
| --- | --- |
| `Found` | Validate the exact count, contiguous logical demand, stable keys, provider semantic IDs, finite logical bounds, provider identity, and complete fence; stage the complete exact result and do not publish entries individually. |
| `NotFound` | Treat the exact demand as authoritative empty evidence and resolve that demand slot. It does not authorize an index-based substitute or successor. |
| `Unavailable(NoProvider)` or `Unavailable(Unsupported)` | Treat the outcome as terminal for the new virtual publication, clear the affected slot, and do not automatically retry or invoke another provider. |
| `Unavailable(DataUnavailable)` or `Deferred` | Retain only an exact eligible fallback. Without one, withhold the complete new virtual generation; do not expose a partial range. A later explicit retry or valid refresh may begin a new attempt. |
| `Rejected`, provider panic, or malformed evidence | Clear the affected slot, expose the conservative ordinary baseline, and fail complete publication. Do not automatically retry, repair, merge, or silently downgrade the result. |
| Stale or superseded completion | Ignore it entirely. It cannot clear, retain, diagnose into, retry, classify, recompose, or publish any semantic state. |

#### Retention, recomposition, and publication

Per-slot retention requires exact equality of the demand, provider
identity/generation, registration identity, content revisions, coordinate, and
budget in the `SemanticProviderFence`. It does not require equality of the
complete surface demand-set generation. A retained provider result is not
eligible merely because its key, item count, or semantic ID still looks useful.
Recomposition uses current materialization/classification authority and
ordinary projection generation only after the retained evidence passes that
exact provider fence.

Adding, removing, or superseding another active slot during an explicit refresh
advances only the publication generation for unchanged slots. Their exact
unchanged provider evidence may be restaged under the new publication fence
without provider reentry. For example: after publishing A, an explicit refresh
adding B restages unchanged A, invokes only B, and atomically publishes A+B
under one new publication generation.

Virtual semantic publication is atomic for the whole surface. Every active
range-demand slot and the independent one-item semantic pin must be resolved or
staged under one exact `SemanticPublicationFence` and complete surface
demand-set generation before a provider result can publish in the new virtual
composition. If any slot fails, the runtime retains the prior eligible complete
composition or an ordinary-only baseline. It never publishes a mixed partial
virtual tree, mixes old and new demand generations, or lets an ordinary
projection change manufacture missing provider evidence.

`Unmaterialized` and `materialized = false` remain authoritative for semantic
leaves without ordinary runtime items. Semantic evidence does not authorize
materialization, scrolling, actions, focus, paint, hit testing, scheduler work,
renderer work, or provider registration. The automation snapshot functions
remain pure observational reads; demand/refresh is a separate mutating runtime
turn with its own publication fence.

### Next production consumer: semantic automation session (normative; generic logical implementation shipped)

The private semantic-demand/publication contract above is shipped. The first
generic logical production consumer is now shipped as a bounded
`SurfaceRuntime` session with runtime-issued opaque handles, explicit refresh,
retry, close, atomic selected publication, and typed conservative status. It
adds no public provider-registration API, scheduler, native/product consumer,
or custom-coordinate transform.

#### Owner and authority

The first consumer MUST be a generic, backend-neutral semantic automation
session. It MUST NOT be a native adapter or a product integration. The caller or
host owns session intent and MUST explicitly open, refresh, retry, and close the
session. `SurfaceRuntime` owns the bounded session state, demand membership,
cancellation and supersession, selected publication, and publication lifetime.
Provider registration remains owned by the mounted virtual-layout runtime.
Callers MUST NOT infer demand from paint order, visibility, viewport or
overscan, item count, provider availability, diagnostics, or snapshot reads.

The session/container identity issued by the runtime MUST be opaque. A caller
MUST NOT fabricate provider identity or provider authority, and this contract
does not invent a public provider-registration API.

#### Conceptual API boundary

The ownership and invariant labels in this section are conceptual; the listed
semantic-session operations are concrete public Rust methods under
`radiant::runtime`. Ordinary `automation_snapshot(&self)` and
`automation_target_snapshot(&self)` MUST remain pure ordinary reads. Explicit
refresh and retry are the only operations allowed to call a provider: refresh
atomically replaces the complete demand set, while retry reattempts the
unchanged set. Opening and closing perform provider-free lifecycle mutation. A
separate pure selected semantic snapshot read returns either the last accepted
session publication or the conservative ordinary baseline together with a typed
status.

Public selection and visibility of that selected semantic snapshot is now the
shipped consumer boundary. The shipped operations are
`open_semantic_automation_session`, `semantic_automation_containers`,
`refresh_semantic_automation_session`, `retry_semantic_automation_session`,
`selected_semantic_automation_snapshot`, and
`close_semantic_automation_session`, with the corresponding opaque demand,
handle, result, status, and fallback types under `radiant::runtime`. No public
provider-registration surface is added.

#### Lifecycle and first-implementation bounds

Opening a session MUST establish one bounded empty session and an exact session
generation. The first explicit refresh MUST supply any initial demand members,
which MUST start at attempt one. Refresh MUST replace the entire session demand
set atomically and MUST supersede/cancel all prior work. Retrying an unchanged
demand MUST advance only the attempt. Closing a session MUST cancel before
retiring its generation and MUST clear its selected publication and demand.

The first implementation is bounded to one active semantic session per
`SurfaceRuntime`, one contiguous logical range per mounted container plus the
existing independent one-item pin, at most 64 registrations, the existing
per-registration and `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` (1024-entry) caps, an
aggregate active range length of 1024, and at most one provider call per
container and attempt. Automatic retry, backoff, and a scheduler are not part
of this slice. `Deferred` MUST return to the caller; only an explicit retry may
reattempt it.

#### Selection fences and provider isolation

Every selected result and publication MUST carry the exact session generation,
demand generation, attempt, request/range or pin, mount/container/policy
identity, registration identity and generation, data/policy/measurement/semantic
revisions, provider
identity/generation, coordinate, budget, cancellation, materialization and
classification authority, ordinary projection generation, and complete-demand-
set generation. A result is accepted only when every required field matches the
current fence exactly. A stale, superseded, or cancelled result is inert.

Provider attempts MUST be non-reentrant. A provider callback MUST NOT publish
or mutate runtime state directly; it returns bounded evidence to the owning
session turn, which alone validates, stages, and publishes it.

#### Atomic publication, retention, and conservative fallback

The consumer MUST stage the complete selected snapshot and typed status under
the exact fence. It MAY swap the staged publication only after every active
demand member resolves or has an eligible exact-fence fallback. It MUST never
publish a partial subset. `Found` and authoritative `NotFound` are complete
outcomes and MAY participate in an atomic publication.

For `DataUnavailable` or `Deferred`, the consumer MUST retain only an eligible
last-complete selection for an unchanged exact demand and fence; without that
exact fallback it MUST expose the ordinary baseline and a typed non-success
status. `NoProvider`/`Unsupported` are terminal and expose the ordinary
baseline. `Rejected`, provider panic, malformed evidence, and collision expose
the conservative ordinary baseline even when an older selection exists. A
stale, cancelled, or superseded completion is inert and MUST NOT mutate runtime
state; if reported to the explicit caller, its status is observational for
that attempt only. No failure may expose a mixed old/new or partial virtual
selection.

Changed demand, session close, mount/identity/provider changes, revision
changes, coordinate changes, and budget changes MUST invalidate the old
selection. Materialization or ordinary-projection changes MAY reclassify
retained exact provider evidence without provider reentry when the current
fences permit it. `Unmaterialized` and `materialized = false` remain
authoritative and MUST never authorize materialization, scrolling, focus, action,
paint, hit testing, scheduling, or renderer work.

#### Coordinate admission

The first consumer admits only `Logical` coordinates. `Custom` MUST be rejected
before provider invocation, with no identity fallback. A separate future
transform contract MUST define the owner, source and destination spaces, the
supported coordinate class, its revision, finite non-inverted conversion,
clipping and nesting behavior, and conservative behavior for singular, stale,
unsupported, or ambiguous transforms before any custom coordinate is admitted.

#### Contract definition of done

The four alignment documents MUST agree on this contract and MUST continue to
state that the private demand/publication kernel and the generic logical
consumer are shipped. The public declarative Logical-only provider contract is
normative and shipped as the first public-API evidence point; native/product
provider consumer implementations, scheduling, and custom transforms remain
deferred. The later macOS/AppKit native semantic accessibility query contract is
defined below. Numeric
estimates remain unchanged for this branch: generic ~97%, Declarative identity
71%, layout 97%, and broad coverage `902 / 11` (82.00%). Existing pure
public snapshot APIs and the existing non-goals remain explicit. Future slices
MUST preserve this ownership, lifecycle, fence, fallback, coordinate, and
authority contract.

### Native semantic accessibility query consumer (normative; later macOS/AppKit consumer)

This is the later macOS/AppKit consumer contract for the shipped generic logical
semantic automation session. It defines a private adapter boundary only; it does
not ship native code or add a public provider API. One private native-window
adapter MAY acquire one runtime-issued semantic-session lease. The adapter and
lease remain private: neither owns provider registration, mount identity, provider
generations, demand fences, cancellation, or publication.

The existing one-active-session bound remains. A native lease MUST NOT evict,
supersede, or silently reuse an externally active session. If an external session
is active, lease acquisition has exactly one typed unavailable outcome,
`Unavailable(SessionContended)`, and performs no provider call. Multi-consumer
arbitration is a later contract.

Accessibility enablement, native tree-root construction, accessibility-state
observation, ordinary native events, repaint, and ordinary property reads are
observation/capability only. Only an explicit bounded native item or child-range
query MAY become `SemanticAutomationDemand`. A query MUST translate to exactly
one current runtime-issued semantic-session lease and one current runtime-issued
container handle, plus either one stable required-item key or one finite
contiguous logical range. Missing, stale, ambiguous, duplicate, oversized, or
unrepresentable evidence is unavailable and MUST cause no provider call.

The adapter submits intent only through the existing explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` operations. It MUST NOT invoke a
provider directly or cause a second call for one container/attempt. Native
callbacks MUST NOT synchronously re-enter a provider or mutate `SurfaceRuntime`
through observational access. Native-to-runtime handoff enters one owned runtime
turn and is bounded transport only; it creates no scheduler, retry, or fairness
policy.

Native tree publication exposes only a complete selected snapshot under the
existing exact fence. It MUST NOT expose partial virtual subtrees, mix
generations, or repair malformed or colliding evidence. `DataUnavailable` and
`Deferred` MAY retain only an exact eligible complete selection. Missing provider,
unsupported, rejected, panic, malformed, collision, stale, or cancelled evidence
uses the existing typed conservative baseline behavior; stale and cancelled
completions are inert and MUST NOT mutate or publish native state.

The first consumer accepts provider semantics only in `Logical`. Native
conversion MUST identify source surface space, destination window/screen
accessibility space, DPI, window/display generation, orientation, clipping, and a
finite non-inverted conversion. Stale or unsupported conversion withholds native
bounds. `Custom` remains rejected; no affine or identity fallback is permitted.

Activation/opening is provider-free. Explicit native queries refresh, and an
explicit repeated query MAY retry. Deactivation, window retirement, recovery
replacement, and close MUST cancel and retire the lease before native objects
drop. `materialized = false` remains authoritative: native semantics cannot
materialize, scroll, focus, execute actions, paint, hit-test, schedule, render,
or claim provider authority.

Native tree publication and native action dispatch are separate contracts. This
consumer MUST NOT connect numeric accessibility dispatch. The adapter and lease
remain private and do not create a public imperative provider registry.
`automation_snapshot(&self)`, `automation_target_snapshot(&self)`, and
`selected_semantic_automation_snapshot(&self)` remain pure reads.

#### Provider-free semantic cardinality (qualified, shipped declaration foundation; native consumer deferred)

The shipped declaration foundation exposes the qualified public value
`radiant::application::virtual_layout::VirtualLayoutSemanticCardinality` on
`VirtualLayoutParts<Message>`. The value contains exactly an `usize` logical
item count and a separate `u64` cardinality revision. The corresponding
qualified builder is
`VirtualLayoutParts::with_semantic_cardinality(...)`. The field and builder ship
outside the common prelude, and the exact private registration/live-fence
invalidation foundation is shipped. Native child traversal, normalized
sidecar, native topology, and AppKit queries remain unshipped in the later
platform consumer.

Cardinality is immutable declaration evidence. It is not a callback, provider
availability signal, or demand. `None` means unknown/unsupported; exact zero is
supported. The exact count is not capped at `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES`
(1024) and must not allocate storage proportional to the count. Count reads,
declaration updates, mounting, and enumeration are provider-free and never
create demand.

The exact `(logical_item_count, cardinality_revision)` pair is fenced with
registration identity and generation, container identity, mount generation, the
existing data/policy/measurement/semantic revisions, coordinate space, budget,
and source-qualified provider generations. Fence equality is exact: no latest
ordering, `>=`, wildcard, or partial match is valid. Any count or cardinality
revision change invalidates affected semantic and native state provider-free.
Replacing a provider preserves the cardinality declaration but invalidates
provider publication. Unmount, native recovery, deactivation, and session close
retire all cardinality, token, and publication state.

The native adapter MUST NOT vend a virtual child container when cardinality is
unknown. A positive count without a range provider is unsupported for native
child traversal and MUST NOT be vended; an exact zero MAY be represented without
a provider. An AppKit count read returns the exact declared count. A bounded
range query normalizes `index` and `maxCount` by checked subtraction from the
count: zero count or zero maximum is empty, an out-of-range index is empty,
subtraction and end arithmetic must not overflow, and the normalized length must
fit the declared budget, `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES`, and the remaining
aggregate budget. The provider must supply the stable key; the adapter MUST NOT
synthesize a key from an index.

#### Compositor-owned normalized native sidecar and private topology

The compositor produces one crate-private normalized sidecar from the same
staged `entries_by_container` union that produces
`VirtualLayoutAutomationComposition`. Each sidecar member retains
container/mount/registration authority, the cardinality fence, logical index,
stable `VirtualLayoutItemKey`, provider `AutomationNodeId`, final normalized
node/path, materialization authority, and publication fence. Exact same-key,
same-index overlaps coalesce only under the existing full-evidence equality.
Raw range/pin members MUST NOT be reconstructed by the native adapter.

Conflicting, ambiguous, duplicate, unstable, colliding, ordinary-ID, and
aggregate-budget failures reject the whole publication. The sidecar is stored
atomically with `RuntimeSemanticAutomationSelection` composition, status,
ordinary projection, and native projection. Parallel reconstruction and mixed
native/public selection are forbidden.

The primary content view/window exposes one private root. Each accepted virtual
anchor has one private read-only virtual container, and each normalized logical
item is a direct child; duplicate placement elsewhere is suppressed. Container
identity and monotonic item tokens are private and runtime-issued. Tokens are
never derived from an index, pointer, provider ID, serialized ID, or bounds.
Token continuity requires exact lease, container, mount, cardinality-fence, and
stable-key equality; a cardinality change retires tokens. Foreign, stale,
retired, duplicate, ambiguous, or colliding tokens return `nil`/`NSNotFound`
without a provider call.

The root, virtual containers, and all non-text roles map to
`NSAccessibilityGroupRole`; only `Text` and `Readout` map to
`NSAccessibilityStaticTextRole`. The native surface exposes only role, exact
parent/children, finite frame, label, description/help, and static-text value.
Checked, selected, enabled, read-only, focusable, focused, tab, live, and action
metadata are omitted; focused is false, actions are empty/no-op, and buttons,
toggles, sliders, tables, and text inputs MUST NOT map to actionable roles.
Defunct objects return conservative empty/zero values.

AppKit callbacks MUST be non-blocking and MUST NOT call or mutate the runtime or
provider. A valid explicit item or range query enqueues/coalesces one owned
runtime turn. While pending, count remains exact; item/range reads return only
an exact eligible retained result under the same fence, otherwise empty/`nil`,
with no placeholders or mixed tree. Identical in-flight queries coalesce. An
explicit repeated query after `Deferred` MAY retry; ordinary reads are not
retries.

An accepted publication installs the complete normalized native projection
atomically. Retention requires exact equality of the semantic fence and native
coordinate/cardinality fence. `DataUnavailable`/`Deferred` without an exact
fallback expose only an empty/baseline result; terminal failures clear virtual
native publication; stale/cancelled results are inert. A changed visible state
posts exactly one `NSAccessibilityLayoutChangedNotification` only after the
complete state is queryable on the main thread. Unchanged, pending, stale,
cancelled, and rejected work posts no layout notification. Retired custom
objects follow the `UIElementDestroyed` notification lifecycle.

This extension preserves the one-session bound, opaque private handles, explicit
refresh/retry-only demand, one range plus one required-item slot, 64
registrations, 1024 per-query and aggregate caps, one provider call per
container/attempt, exact publication/fallback, `materialized = false`,
Logical-only conservative coordinates, and pure snapshots. It excludes focus,
actions, selection mutation, scroll/materialize, scheduler/retry policy, render,
product, custom, Wayland/Windows, auxiliary, multi-consumer, and public registry
behavior.

This contract is limited to the later macOS/AppKit consumer. Wayland, Windows,
native actions, focus, scrolling, product policy, custom transforms, scheduler,
and renderer behavior are excluded.

### Public declarative logical provider attachment (normative; shipped)

The existing generic semantic automation session and its explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` operations are shipped. The
public provider-attachment path defined here is normative and shipped. The
current crate-private registration bridge is evidence for the runtime ownership
rules underneath the qualified public API.

#### Qualified public vocabulary

There is exactly one public declarative capability for attaching
semantic providers to a virtual layout:

- `radiant::application::VirtualLayoutParts<Message>` carries the normal
  virtual-layout declaration and optional semantic item and range provider
  capabilities. It has no custom-coordinate field and does not carry a mount or
  registration generation.
- `virtual_layout_from_parts` constructs the declarative view from those parts.
  It is a declaration, not a registration side effect or a provider call.
- `radiant::runtime::VirtualLayoutRevisions` carries host-supplied data,
  policy, measurement, and semantic revision evidence as applicable. It never
  supplies the runtime-owned mount or registration generations.
- `VirtualLayoutSemanticProvider` supplies one required-item semantic result;
  `VirtualLayoutSemanticRangeProvider` supplies one bounded contiguous logical
  range result.
- Both callbacks receive read-only item/range request values. A request
  contains only the bounded logical demand and immutable evidence needed for
  validation; it provides no mutable runtime handle, registration/removal
  operation, materialization authority, focus/action authority, or scheduler
  handle.
- `VirtualLayoutSemanticEntry` carries the requested stable item key, logical
  index, finite non-inverted `Logical` bounds, provider semantic fields, and a
  provider-supplied stable serializable automation node ID. The runtime, not
  the provider, owns `Unmaterialized`/`materialized = false` authority.
- `VirtualLayoutSemanticProviderOutcome<T>` is the generic result shape:
  `Found(T)`, `NotFound`, `Unavailable(reason)`, `Deferred(reason)`, or
  `Rejected`. An item provider uses `T = VirtualLayoutSemanticEntry`; a range
  provider uses `T = Vec<VirtualLayoutSemanticEntry>` within the bounded query
  cap.
- The shipped declaration exposes the qualified public value
  `radiant::application::virtual_layout::VirtualLayoutSemanticCardinality` as
  an optional field on `VirtualLayoutParts<Message>`, plus the qualified builder
  `VirtualLayoutParts::with_semantic_cardinality(...)`. It contains the exact
  `usize` logical item count and a separate `u64` cardinality revision. The
  field and builder ship outside the common prelude, and the exact private
  registration/live-fence invalidation foundation is shipped. The value is
  immutable declaration evidence, not a callback or demand; `None` is
  unknown/unsupported, exact zero is supported, the count is independent of
  the 1024 per-query/aggregate caps, and it must not allocate proportional
  storage. The normalized sidecar, native cardinality query/topology, and
  platform consumer remain unshipped. It remains outside the prelude.
- Provider-supplied `Unavailable` reasons are only `DataUnavailable` and
  `Unsupported`. `NoProvider` is not provider-supplied: it is synthesized by
  the runtime when the relevant optional slot is absent. Provider `Deferred`
  reasons are bounded to `DataPending`,
  `SemanticPending`, and `Retry`.

These names are qualified exports, are not in the prelude, and are limited to
this shipped logical attachment boundary. The first boundary stores callbacks as synchronous
single-threaded `Rc` capabilities. It makes no `Send`, `Sync`, worker, or
scheduler promise and does not define asynchronous provider completion.

#### Runtime-owned mounted registration and lifecycle

`virtual_layout_from_parts` is the only public declarative attachment path.
During mounting, `SurfaceRuntime` discovers the declaration, validates the
logical-only capability, and creates the mounted registration. The runtime
owns the opaque container identity, registration identity and generation,
mount generation, source-specific provider generations, cancellation, and
lifetime. There is no public imperative `register`, `remove`, or equivalent
provider API, and an application cannot provide or advance a mount generation.

The runtime admits at most 64 registrations and rejects duplicate mounted
container/registration scope before a provider callback exists. A same-scope
declaration update may retain the mounted container identity while advancing
the registration generation for changed capability or revision evidence. A
scope replacement is remove-then-mount with a fresh mount generation. Removing
or unmounting a declaration cancels its active source tickets, retires the old
registration, clears its range slot, required-item slot, pending demand, and
selected publication, and makes every late return stale. None of these events
creates demand or calls a provider.

Replacing only the item provider or only the range provider advances that
source's provider generation and cancels that source's active ticket. Old
evidence is not eligible across the provider-generation change. A missing
provider is represented by the exact runtime-synthesized `NoProvider` outcome
when an explicit session operation executes the source; provider replacement
itself does not call the new provider. An explicit refresh or retry is required
to issue a new source ticket.

#### Explicit session intent, requests, and exact fences

The runtime has one semantic-demand owner per `SurfaceRuntime`, one active
contiguous range slot and one independent required-item slot per mounted
container, a 1024-entry cap per query, and a 1024-entry aggregate active-range
cap. Each explicit range or required-item demand is normalized to one exact,
runtime-issued source ticket. The ticket records whether the source is a
range or required-item pin; source identity cannot be changed or reused by an
application.

Only `refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` are provider-calling demand
intent. Refresh atomically replaces the complete session demand set and starts
attempt one for each admitted source. Retry reattempts the unchanged set and
advances only its attempt. Opening, enumeration, closing, registration,
replacement, removal, ordinary snapshot or target reads, repaint, viewport,
visibility, overscan, diagnostics, item count, provider availability reads,
and IME/native events do not create demand or call a provider. A changed live
revision, provider capability, registration, or mount fence invalidates old
work; it does not create a new demand outside an explicit refresh or retry.

The per-source `SemanticProviderFence` requires exact equality of container and
policy identity, registration identity and generation, mount generation,
`VirtualLayoutRevisions` evidence, fixed `Logical` coordinate, budget, exact
range or required-item demand, exact source ticket, provider identity and
source-specific provider generation, demand generation, attempt, and
cancellation. No field is inferred from a snapshot or compared with `>=`.
The whole-surface `SemanticPublicationFence` adds session generation,
materialization authority, classification authority, ordinary projection
generation, and complete session demand-set generation. A result that lacks
any required fence field is not current.

Provider callbacks are read-only and isolated from runtime mutation. They run
at most once per container and attempt. Reentry into demand, registration,
publication, or the same callback is rejected; the follow-up is coalesced for
a later runtime turn and never becomes a nested call. A provider panic is
contained at the callback boundary and maps to the conservative rejected/panic
outcome. The callback cannot publish, mutate runtime state, install widgets,
materialize an item, transfer focus, scroll, schedule work, or invoke another
provider.

#### Normative result, fallback, and authority rules

The runtime validates all results before staging them and publishes only a
complete whole-surface composition:

- `Found` is accepted only when an item entry matches the requested key and
  exact logical index, or a range result has the exact requested count,
  contiguous ordered indices, stable unique keys, distinct provider IDs,
  finite non-inverted logical bounds, valid semantic fields, and the complete
  exact fence. Any mismatch, malformed value, unstable equality, or ID/key
  collision rejects the entire result atomically.
- `NotFound` is authoritative empty evidence for the exact demand. It resolves
  that slot and never authorizes an index substitute, successor, predecessor,
  or synthesized entry.
- `Unavailable(NoProvider)` (runtime-synthesized) and
  `Unavailable(Unsupported)` are terminal for the requested virtual
  publication. They do not retry, invoke another provider, or retain an old
  semantic selection as current.
- `Unavailable(DataUnavailable)` and every bounded `Deferred` reason may
  retain only an eligible complete selection whose exact provider fence still
  matches. Without that exact fence, the runtime exposes the ordinary baseline
  and typed non-success status; it never exposes a partial range.
- `Rejected`, provider panic, malformed evidence, reentry rejection, and any
  key/provider-ID/cross-range collision use the conservative ordinary baseline.
  They do not repair, merge, silently downgrade, automatically retry, or use a
  stale selection as semantic truth.
- A stale, cancelled, or superseded result is inert. It cannot clear, retain,
  diagnose into, retry, classify, recompose, or publish any state.
- Publication is atomic across every active range slot and required-item slot.
  Unchanged exact evidence may be restaged under a new publication fence, but
  no mixed old/new or partial virtual tree is visible.
- `Unmaterialized` and `materialized = false` remain authoritative for a
  semantic leaf without an ordinary runtime item. Provider evidence never
  authorizes materialization, scrolling, focus, actions, paint, hit testing,
  scheduling, renderer work, or provider registration.

The complete allow/veto/terminal/fallback/stale/authority fixtures are in the
[semantic provider acceptance matrix](#semantic-provider-acceptance-matrix-normative-public-declarative-attachment-shipped),
which is part of this contract and includes provider panic and reentry.

#### Native boundary and non-goals

The later macOS/AppKit native semantic accessibility query contract is defined
in [Native semantic accessibility query consumer](#native-semantic-accessibility-query-consumer-normative-later-macosappkit-consumer).
It is not the hidden owner of provider registration, demand, or publication.
Native IME and other ordinary native events remain non-demand inputs. The public
declarative attachment contract does not implement native tree publication or
native action dispatch; those are separate contracts. Custom transforms, focus,
scrolling or materialization, scheduler/backoff/fairness, renderer, paint,
hit-testing, cache policy, product policy, multiple ranges per container, and a
prelude export remain excluded here. It also does not add a public imperative
registration API or promise any worker/threading model.

### Culling and paint

Radiant culls from accepted item bounds and the scroll clip. Ordinary paint is
limited to accepted visible items and explicitly painted runtime overlays.
Overscan may be laid out or retained for continuity, but overscan alone does
not require paint. A pinned item may remain available for focus/semantics while
remaining outside ordinary paint unless its interaction policy explicitly
requires a visible overlay.

The policy never paints and cannot bypass the clip. Total or estimated extent
contributes to scrollbars and clamping, not to paint work.

### Hit testing

Normal pointer hit testing uses accepted item bounds intersected with the
current viewport/clip and the runtime's overlay/capture rules. Hidden logical
items, semantic-only bounds, estimates, and rejected query results are not hit
targets. A captured pointer may continue to route to its valid pinned item
under the capture policy; capture is released when the key, fence, or mount
becomes invalid.

## 12. Interoperability with current virtualization APIs

The future keyed contract is additive and must not silently change the current
fixed-child or host-projection paths.

### Fixed-child layout virtualization

The current [`VirtualizationPolicy`](../src/gui/layout_core/model/virtualization.rs)
enables linear virtualization for an already-built child list. It uses axis and
pixel overscan; the current layout engine emits
[`VirtualWindowInfo`](../src/gui/layout_core/engine/types/virtualization.rs)
with child indices, viewport/window coordinates, cull counts, and total resolved
extent. It has no application item-key extraction or keyed materialization
contract.

Future keyed virtualization MUST NOT reinterpret `VirtualWindowInfo` indices as
stable item keys. An adapter may use its geometry as fixed-child evidence or
translate a fixed-row result into a keyed query, but the adapter must supply
keys and exact fences before keyed reconciliation. A container must have one
authoritative window owner; attaching both policies without an explicit adapter
is an identity/ownership error.

### Host-owned fixed-row windows

The current [`VirtualListWindow`](../src/gui/list/virtual_list/window.rs)
resolves a bounded index window, and
[`VirtualListController`](../src/gui/list/virtual_list/controller/state.rs)
stores host-owned viewport/focus-follow state. The current
[`MaterializedVirtualListItem`](../src/gui/list/virtual_list/item.rs) carries a
host key, index, rectangle, and overlay state. The application builders
explicitly project the returned range through
[`virtual_list_window`](../src/application/layout_builders/lists/virtual_window.rs)
or [`VirtualListBuilder`](../src/application/layout_builders/lists/virtual_builder.rs).

These APIs remain valid compatibility projections. They already establish
important boundaries: the host owns the logical collection, only the returned
range is projected, and hidden rows are not needed for ordinary hit testing.
They do not, by themselves, provide keyed variable-extent policy queries,
exact data/policy/measurement/semantic fences, coordinator-owned recycling, or
on-demand semantics. A future adapter MAY use a `VirtualListItemKey` as the
source key and a `VirtualListWindow` as a fixed-row window, but it must not claim
the full keyed contract until the relevant future slice is implemented.

The current `VirtualListController` remains host state. A future Radiant
visible-window coordinator must not silently mutate or replace it. Interop must
choose one direction explicitly:

1. host resolves a fixed-row `VirtualListWindow`, then Radiant projects it; or
2. Radiant resolves a future keyed window, then an explicit host adapter
   observes/materializes it.

There is no implicit bidirectional synchronization or lifecycle transfer between
the two owners.

## 13. Ordered future implementation slices

The slices are intentionally ordered by dependency. Each slice must preserve the
contract already stated and must identify any deliberate temporary limitation.

### Slice 1 — Query-only capability (shipped)

The qualified `radiant::layout::VirtualLayoutPolicy` capability and
`VirtualLayoutQueryExecutor` define the bounded query/diagnostic model. The
shipped slice implements identity admission, finite input/output validation,
atomic result acceptance, and exact fence construction without creating widgets
or changing public materialization APIs. Runtime registration and coordinator
ownership remain in Slice 2 and later.
Acceptance requires duplicate/missing/ambiguous-key rejection, bounded output,
finite geometry, pure-query tests, and stale-result tests.

### Slice 2 — Keyed window reconciliation

Add the per-container coordinator, accepted-window commit, keyed
range-to-bounds, extent estimates, measurement revision, exact viewport/data/
policy fences, and conservative same-key anchor behavior. Keep the result
query-only: it may expose a desired keyed window to tests or an internal adapter
but must not materialize widgets. Acceptance requires insert/remove/reorder/
measurement/viewport tests, same-key bounded anchor presence/absence evidence,
and previous-valid-window fallback evidence. Removal replacement waits for the
authoritative required-key prerequisite described above.

### Slice 3 — Private materialization and recycling correctness (shipped)

The crate-private materialization module provides an explicit host projection
boundary, keyed item slots, lifecycle ordering, compatibility checks, bounded
desired-set reconciliation, and reset-only recycling for an accepted
coordinator commit. It is a correctness kernel only: it does not register a
runtime surface, project concrete `SurfaceNode` or widgets, own
focus/accessibility pins, schedule work, or serve a product collection.
Successful publication is atomic after complete lifecycle staging; callback
failure, reentry, or unwind terminally retires the private kernel without
rollback or recovery, while pre-callback rejection remains recoverable.
Acceptance requires no implicit materialization, remove-before-reuse,
same-key continuity, incompatible replacement cleanup, fail-stop lifecycle
retirement, and unmount tests. Cancellation and those later runtime and product
consumers remain unshipped.

### Slice 4 — Private retained-item adapter (shipped)

The shipped **private retained-item adapter** provides
fallible, scoped `ViewNode` lowering for one complete accepted item batch, whole
shell-plus-batch identity admission, slot-wrapper/descendant identity evidence,
and an immutable `SurfaceNode` payload for the existing private kernel. It must
reject explicit raw `NodeId`/direct retained `SurfaceNode` identities and
scene-level presentation, shortcut, overlay, or out-of-band effects until a
collision-safe remapping contract exists. It must run entirely in pure
projection/preflight and must not register `SurfaceRuntime`, invoke lifecycle
callbacks, call a scheduler or renderer, or add a public API or contract
version.

Acceptance requires all-or-nothing pre-callback admission, compatible same-key
identity preservation, generation advancement for removal and incompatible
replacement, descendant scoping below the wrapper, and recoverable projection,
identity, and capacity rejection. This adapter is the prerequisite for the
private runtime registration bridge shipped in Slice 5; neither slice is public
registration or product integration.

### Slice 5 — `SurfaceRuntime` registration and two-pass bridge (shipped)

The shipped private `SurfaceRuntime` registration/two-pass bridge connects the
crate-private shell registration descriptor to one `SurfaceRuntime`
materialization record per mounted container generation. It implements the
synchronous shell/item pipeline, repeats it for registration or relevant
geometry changes (including viewport invalidation), and explicitly unmounts
exactly once before descriptor removal, container-generation replacement, or
runtime close drops the materialization owner/record from `SurfaceRuntime`.
Terminal lifecycle failure suppresses partial materialization without automatic
retry or state transfer; replacement and recovery remain a later
runtime-integration policy. This bridge does not move retained-slot ownership
into `AppBridge`, `RuntimeBridge`, policy, or product state, and does not add
scheduler/renderer callbacks, public APIs, or contract versions. Public
registration, scheduler/renderer policy, focus/capture traversal, full
accessibility semantics, and product wiring remain unshipped.

### Slice 6 — Focus and accessibility

A private bounded pin-owner prerequisite and the current-fence one-item
semantic admission path are shipped by this patch. Each mounted runtime record
owns exactly one optional pin, tagged with one of the private `Focus`,
`PointerCapture`, or `Semantic` reasons. The pin retains the exact immutable
request and validated provider entry. The request uses the exact applicable
container identity, policy identity, mount generation, and
data/policy/measurement/semantic revision fence; provider invocation remains
immutable and occurs only after that fence. The current-authority semantic
path accepts only the container identity and opaque key, validates stable
reflexive key equality before provider lookup, and invokes the provider at most
once. A valid `Found` result installs the `Semantic` pin; invalid key/bounds and
not-found/deferred/unavailable/rejected, stale, revision, and retirement
outcomes clear or reject it. A successful query replaces the one bounded pin
in deterministic query order.

This semantic admission and projection are private evidence only. The projection
is created only from a validated live `Semantic` pin and has explicit
`Unmaterialized` authority; it performs no automation traversal, offscreen
materialization, focus/capture transfer, scrolling, paint, hit testing,
scheduler/renderer work, or product integration. It does not wire
`AutomationNodeId`, `AutomationTarget`, or `GuiAutomationSnapshot`. The
evidence moves Declarative identity from 70% to 71% and broad coverage from
`900 / 11` to `901 / 11` (~81.91%); generic architecture remains ~97% and
layout remains 97%.

The private semantic range extension is an exact downstream query over the same
mounted registration authority. A request is the half-open interval
`[start_index, start_index + length)`. Construction rejects `length == 0` and
an unrepresentable checked-add end; the runtime rejects a length above either
the registration `VirtualLayoutBudget::max_entries()` or
`VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` before invoking the provider. The request
fences the live container identity, policy identity, mount generation, data,
policy, measurement, semantic revisions, declared coordinate space, and
registration budget. A missing provider is typed `Unavailable(NoProvider)`;
provider `NotFound`, `Unavailable`, `Deferred`, and `Rejected` outcomes pass
through without changing the existing one-item pin.

The range provider is invoked at most once. A `Found` vector is accepted only
when it has exactly the requested count, contains strictly increasing,
contiguous logical indices matching the requested interval, has stable
reflexive opaque keys with no duplicates, has distinct provider-supplied
serializable `AutomationNodeId` values for distinct keys, and has finite
non-inverted bounds. The live fence is checked again after provider return. Any
count, index, key, semantic-ID, geometry, provider, or fence failure rejects
the complete batch with no partial projection. A same-key, same-fence
provider-ID drift against an existing one-item pin is also rejected without
exposing or replacing that pin. Accepted entries become ordered crate-private
`VirtualLayoutSemanticProjection` values carrying the opaque key identity, the
unchanged provider ID, the declared coordinate space, logical index, bounds,
`AutomationNodeSemantics`, the exact range fence, and `Unmaterialized`
authority. The range path never replaces, clears, or creates a second retained
one-item pin and has no runtime, materialization, layout, refresh, focus,
capture, scroll, paint, hit-test, or automation-snapshot side effects. Path
construction, coordinate-space resolution, and cross-range ID deduplication
remain later boundaries.

The shipped downstream classification boundary is synchronous, crate-private,
and read-only. It consumes only a successfully validated
`VirtualLayoutSemanticProjectionBatch` and its matching live
`RuntimeVirtualLayoutRecord`/materialization store; it never invokes a semantic
provider again. Before reading active slots it requires the batch request to
match the live registration and the store's authoritative `VirtualLayoutQueryFence`
on container identity, stable policy identity, mount generation, data/policy/
measurement/semantic revisions, coordinate-space identity, and admitted budget.
Missing, retired, lifecycle-indeterminate, or authority-less materialization
evidence, unstable identity/equality, malformed batches, and any fence or
key/index mismatch reject the complete classification. Registration-only
evidence cannot classify an item.

Matching is bounded by the semantic range and active-slot caps and uses only
`VirtualLayoutItemKey::stable_equals`; an exact key at another logical index,
another key occupying an in-range active index, duplicate/ambiguous evidence,
or an unstable comparison is never downgraded to `Unmaterialized`. Every
in-range active slot must correspond exactly once to the ordered semantic
projection. The result preserves range order, opaque key identity, logical
index, bounds, coordinate declaration, semantics, provider `AutomationNodeId`,
and request fence. Its private origin vocabulary is distinct from projection
authority: `Materialized { slot identity, payload-root NodeId }` or
`Unmaterialized`; the retained `SurfaceNode` payload is not cloned and the
generated wrapper root is not substituted for the provider ID. The operation
does not mutate pins, providers, materialization, refresh, layout, traversal,
snapshots, focus, capture, lifecycle, or presentation state.

This classification boundary does not itself add path insertion,
coordinate-space resolution or custom transforms, final semantic ordering,
global ID/collision admission, cross-range deduplication, or semantic-tree
construction; those responsibilities belong to the private compositor below.

The private automation-tree compositor is now shipped as a staged, crate-private
consumer of these classification batches. It admits only `Logical` coordinates
and rejects `Custom` before insertion. It normalizes batches by exact container,
registration fence, and logical index so caller order cannot affect the result.
Only exact same-key/index overlaps with identical semantic, geometry,
provider-ID, origin, and fence evidence coalesce; conflicting overlap,
key/index drift, duplicate payload roots, unstable equality, aggregate budget
overflow, and the hard query cap reject the complete composition.

The compositor requires an exact unique ordinary anchor for each participating
container and an exact direct generated wrapper root for each materialized item.
It replaces each materialized wrapper in place while preserving its descendants,
inserts each unmaterialized provider root once as a leaf, and carries private
flattened authority marking those leaves `materialized = false`. Ordinary,
descendant, container, provider, and cross-range IDs are one global namespace;
only the exact generated wrapper being replaced may be displaced. A final ID
audit and clone-after-preflight staging preserve all-or-nothing behavior and
leave source/runtime state unchanged on failure. This slice performs no provider
invocation, scheduling/demand ownership, custom transform, focus/action/product
wiring, public API change, or serialized-schema change. Estimates remain
unchanged: generic architecture ~97%, Declarative identity 71%, layout 97%,
and broad coverage `901 / 11` (~81.91%).

The private runtime bridge also ships one bounded required-key admission path.
An in-crate registration may request one exact stable key; the immutable policy
input and query fence carry that key, and a ready result that omits it is
rejected before coordinator commit or materialization. A changed required key
supersedes pending work and disables a previous-valid fallback. This path does
not yet perform focus traversal, pointer-capture routing, offscreen promotion,
scroll-to-anchor, or accessibility/product wiring.

The full Slice 6 remains unshipped: full focus traversal, automation/accessibility
traversal, focus-follow/anchor, offscreen materialization, scheduler/renderer
policy, and product wiring remain unshipped. Full acceptance still requires
focus and capture continuity/removal, semantic requests beyond this one-item
path, semantic-only non-paint behavior, and no permanent full accessibility
tree.

The semantic-demand owner/provider-attempt/retention kernel and private atomic
`SemanticPublicationFence` publication/composition kernel are shipped and
implemented. They provide one crate-private owner per `SurfaceRuntime`, one
active contiguous range-demand slot per mounted virtual container plus the
independent one-item semantic pin, explicit-demand-only sources, the 64/1024
bounds, exact provider/publication fence fields, generation/attempt/cancellation,
typed outcomes and validation, non-reentrant provider attempts, exact-fence
fallback retention, retained-evidence reclassification, and all-or-nothing
logical composition. The generic logical production consumer above now ships
public snapshot selection/visibility and the runtime semantic session;
scheduler/cancellation transport, native/product provider consumer
implementations, custom transforms, and the other listed non-goals remain
outside the slice. The
shipped private kernel and session do not authorize materialization, scrolling,
actions, focus, paint, hit testing, scheduling, rendering, public provider
registration, or any other deferred authority.

### Slice 7 — Performance and deferred work

Add measured cache strategy, deferred query/measurement scheduling, bounded
invalidation coalescing, diagnostics/telemetry, and performance regression
coverage. Only after the correctness slices pass may the implementation optimize
allocation, cache reuse, batching, or parallel-safe work. Acceptance requires
stable bounds under the configured budgets, no reentrancy growth, cancellation
under load, and measured large-collection scenarios.

No slice may opportunistically implement `split_pane` or turn the current public
fixed-child/fixed-row APIs into the future keyed API without an explicit contract
update.

## 14. Acceptance and test matrix

The future implementation is not complete when one happy-path list scrolls. The
following matrix is the minimum evidence for each relevant slice.

| Area | Required scenarios | Required evidence |
| --- | --- | --- |
| Container/policy identity | Rebuild, sibling collision, moved container, policy replacement, duplicate registration | Stable continuity for equal identities; deterministic reject/unmount for collisions and replacements; no pointer/frame identity. |
| Key extraction | Stable reorder, insert/remove, index changes, missing key, equal duplicate, unequal fingerprint collision, unstable equality | Invalid snapshots never commit; no arbitrary occurrence or synthesized index key; bounded diagnostics. |
| Query boundedness | Huge total count, sparse ranges, finite overscan, required key/index, budget exhaustion | Query work and output stay within configured bounds; no widget/materializer/lifecycle calls from policy query. |
| Geometry | Fixed, estimated, measured, variable-size, non-finite/negative/inverted bounds | Finite validated keyed range-to-bounds; exact measured revision; invalid geometry rejected. |
| Extent | Exact total, estimated total, partial measurements, append/remove | Scroll extent and scrollbar mapping use the declared exact/estimated/measured distinction and are corrected only by fenced results. |
| Anchor insert/remove | Same-key bounded-window presence/absence; later authoritative required-key `found`/`not_found` evidence for removal replacement | Same key and screen offset retained when present in both accepted windows; bounded absence remains unresolved with no correction or index-based successor transfer. |
| Anchor reorder/measurement | Reorder anchor, size change before/at anchor, viewport resize | Key continuity and measurement delta correction; no arbitrary recentering. |
| Revision fences | Each data/policy/measurement/semantic/viewport revision changed independently and together; out-of-order completion | Exact mismatch rejection; old result cannot overwrite accepted state; only current fence publishes. |
| Cancellation/unmount | New query, scroll burst, policy replacement, unmount with in-flight query/measurement/semantic work | Cancellation is delivered or safely ignored; no late mount, commit, callback, or resurrection after unmount. |
| Runtime consumer boundary | Synchronous shell/item mount, descriptor removal, generation replacement, runtime close, viewport relayout, terminal lifecycle failure | One `SurfaceRuntime` record owns each mounted generation; no externally visible partial batch; exact-once unmount precedes owner drop; no automatic retry or state transfer. |
| Fallback/rejection | Deferred query, malformed result, transient unavailable data, content revision while querying | Prior valid window is used only under eligible fences; otherwise inert/empty fallback; rejection is bounded and retry is non-reentrant. |
| Reconciliation | Same-key update/reorder, removed key, new key, incompatible same-key kind | Remove-before-reuse; same-key compatible state continuity; incompatible replacement has no lifecycle/interaction transfer. |
| Recycling | Pool reuse under scroll churn and mixed item kinds | Reused storage is reset; active capture/focus/semantic state never leaks across keys. |
| Focus/pins | Offscreen focus, capture, drag target, pin budget exhaustion, focused-key removal | Bounded promotion and scroll; no hidden full-list materialization; deterministic focus release/fallback. |
| Accessibility | On-demand key/index/range semantic request, semantic revision, unavailable result | Bounded semantic result, exact fence, semantic-only bounds do not paint or hit-test, no permanent full tree. |
| Culling/paint/hit test | Overscan, clip edges, rejected/stale bounds, pinned offscreen item, captured pointer | Only accepted eligible item geometry is interactive/painted; estimates and semantic-only entries are excluded. |
| Deferred work | Coalesced invalidation, retry, cancellation, queue overflow, reentrant callback attempt | Work stays bounded, callback cannot recurse, overflow widens conservatively, stale completions are discarded. |
| Current API interop | Existing fixed-child layout tests and host `VirtualListWindow` tests alongside an adapter | Existing behavior and public signatures remain unchanged; no dual owner or accidental lifecycle synchronization. |
| Performance | Large keyed list, rapid scroll, variable measurement, repeated reorder | Measured bounds and scheduler work stay within declared budgets; optimization does not weaken fences or ownership. |
| Scope guard | `split_pane` and unrelated interaction/persistence features | No implementation or acceptance claim for `split_pane`; it remains an explicit non-goal. |

Tests should assert observable ownership, boundedness, identity, and revision
behavior. They should not assert the names or storage layout of this document's
non-API pseudocode.

### Semantic provider acceptance matrix (normative; public declarative attachment shipped)

The following direct rows preserve the approved acceptance contract. The
owner/provider-attempt/retention and private publication/composition rows
describe shipped runtime behavior, including the generic logical session,
public selected snapshot, and qualified declarative provider attachment.

| Decision | Direct fixture | Required evidence |
| --- | --- | --- |
| Declarative registration allow | One `SurfaceRuntime` with a valid planned `VirtualLayoutParts<Message>` declaration | Exactly one runtime-owned mounted registration and one crate-private demand owner; live identity/revisions/provider/budget are derived by the runtime; the declaration alone creates no demand or provider call. |
| Registration reject | More than 64 virtual-layout registrations or duplicate mounted registration scope | `MAX_VIRTUAL_LAYOUT_REGISTRATIONS` (64) is enforced before a slot or provider attempt exists; no public/application/native demand surface appears. |
| Registration lifecycle | Admit a declaration, replace its scope or provider, or unmount its container while an attempt is pending | The runtime owns registration/mount/provider generations; scope replacement gets a fresh mount generation, provider replacement advances only its source generation, removal cancels and clears old authority, late work is stale, and none of these events invokes a provider or creates demand. |
| Missing-provider synthesis | The selected item or range provider slot is absent when explicit session intent executes it | The runtime creates the exact `NoProvider` outcome for that source; registration/removal/provider replacement itself performs no callback and no automatic retry. |
| Declarative API veto | Application attempts an imperative register/remove call or supplies a mount generation/custom-coordinate field | No such public operation or field exists in the capability; runtime ownership is unchanged and custom coordinates are rejected before provider invocation. |
| Provider callback isolation | Synchronous `Rc` item/range callback receives its read-only request | Callback has no mutable runtime, materialization, focus/action, scheduler, or provider-registration authority; no `Send`/`Sync` or worker promise is made. |
| Demand-source allow | Explicit session refresh with a semantic range request or required-item pin | Only those exact source tickets can call a provider; viewport, overscan, paint, hit-test, provider availability, item count, diagnostics, and snapshot reads produce no demand. |
| Demand-source reject | Registration, opening, enumeration, closing, snapshot/target read, ordinary repaint, paint-only invalidation, unchanged refresh, provider-availability read, or IME/native event | No demand generation, attempt, slot mutation, provider invocation, or publication occurs. |
| Range-slot allow | A valid contiguous logical range for a mounted container | One active range slot exists for the container, the independent one-item pin remains independent, and no range is merged or split. |
| Range-bound reject | Zero length, overflow, per-registration maximum exceeded, length above `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES`, or aggregate active length above 1024 | Rejection occurs before provider invocation; no partial slot or publication is staged. |
| Coordinate allow/reject | `Logical` versus `Custom` demand | `Logical` may proceed; `Custom` is rejected/unavailable before provider invocation with no identity-transform fallback. |
| Generation/attempt allow | Explicit refresh versus explicit retry | Refresh atomically replaces the complete demand set and starts attempt one; explicit retry of unchanged demand advances only the attempt. A changed live fence invalidates old work but does not create demand outside either operation. |
| Cancellation before supersession | In-flight attempt is superseded, its registration is replaced, its container unmounts, or its owner retires | The owner marks the attempt cancelled before the transition; cancellation is delivered through the private runtime work boundary where available, and no automatic retry occurs. |
| Cancelled provider return | Structurally valid provider result returns after the attempt was cancelled | The result is stale regardless of structural validity or matching fields; it is ignored entirely and cannot retain, clear, diagnose, retry, classify, recompose, or publish. |
| Refresh allow | Explicit `refresh_semantic_automation_session(session, demands)` or `retry_semantic_automation_session(session)` | A new exact provider attempt is created only by the explicit operation and carries the current runtime-owned fence. |
| Refresh reject | Identity/mount/revision/provider/coordinate/budget change by itself; snapshot/read, ordinary repaint, paint-only change, unchanged refresh, or materialization/ordinary-projection change | No provider call or new demand; old evidence is invalidated or may only be reclassified/recomposed when its exact retention fence still permits it. |
| Exact fence allow/reject | Compare a returned source ticket with the live authority and publication state | Acceptance requires exact container/policy/registration identity and generation, mount generation, revision evidence, fixed `Logical` coordinate, budget, exact demand/source ticket, provider identity/generation, demand generation, attempt, cancellation, and publication authorities; missing, partial, `>=`, or latest-known matches are rejected. |
| Provider-call bound | Reentrant callback or a second call for one container/attempt | At most one provider call occurs; callback reentry is rejected/coalesced and follow-up invalidation waits for a later runtime turn. |
| Provider panic | Provider panics during the synchronous callback | The panic is contained and mapped to conservative rejection/panic; no unwind escapes, no partial result survives, and the ordinary baseline is selected. |
| Provider reentry | Provider calls demand, publication, registration, or the same provider recursively | Reentry is rejected; no nested provider call, mutation, retry, or partial publication is allowed. |
| Provider `Found` allow | Exact count, contiguous logical indices, stable unique keys/IDs, finite logical bounds, and exact fence | The complete result is validated and staged; no entry is published independently. |
| Provider `Found` reject | Count/index/key/ID/geometry/provider/fence mismatch or malformed vector | The whole result is rejected atomically; no partial projection or fallback from malformed evidence is accepted. |
| `NotFound` outcome | Provider reports no result for the exact demand | Exact demand becomes authoritative empty and its slot resolves; no index successor or synthesized item is substituted. |
| Terminal unavailable | `Unavailable(NoProvider)` or `Unavailable(Unsupported)` | The new virtual publication fails terminally, the affected slot clears, and no automatic retry or alternate provider call occurs. |
| Eligible fallback | `Unavailable(DataUnavailable)` or `Deferred` with exact demand/provider/registration/content/coordinate/budget fence | The prior complete virtual composition is retained; current materialization/ordinary projection may recompose it without provider reentry. |
| Fallback withheld | `Unavailable(DataUnavailable)` or `Deferred` without an exact eligible fallback | The complete new virtual generation is withheld and an ordinary-only baseline may remain; no mixed partial virtual tree is visible. |
| Rejected/malformed outcome | Provider rejection or post-return malformed evidence | The affected slot clears, complete publication fails to the conservative ordinary baseline, and no automatic retry, repair, merge, or silent downgrade occurs. |
| Collision veto | Duplicate key, duplicate provider automation ID, cross-range identity collision, or conflicting same-key/index evidence | The whole affected result/publication is rejected atomically; no partial entry, stale selection, or collision repair is exposed. |
| Stale/superseded outcome | Completion after cancellation, demand generation, attempt, identity, or fence supersession | Completion is ignored entirely with no clear, retain, diagnostic-driven retry, classify, recompose, or publish side effect. |
| Exact retention | Retained result with every demand/provider/registration/content/coordinate/budget/source-ticket field equal | Evidence remains eligible only for `DataUnavailable`/`Deferred`; recomposition reads current materialization/classification and ordinary-projection generations. |
| Retention reject | Any mismatch in the exact retention fence | Evidence is ineligible; the runtime withholds the new complete generation or retains only the ordinary-only baseline. |
| Publication restage allow | A is published; add B while A's provider fence is unchanged | Only the publication generation advances for A; A is restaged under the new publication fence, only B is invoked, and A+B becomes visible atomically under one publication generation. |
| Whole-surface publication allow | Every active range slot and independent pin resolved/staged under one complete surface demand-set generation | One atomic complete virtual composition becomes visible; no provider result publishes before every active member is staged/resolved under that publication generation. |
| Whole-surface publication reject | One active member fails, is unresolved, or has a mismatched publication fence | Prior eligible complete composition or ordinary-only baseline remains; no mixed old/new or partial virtual tree is published. |
| Semantic authority guard | Provider result contains an unmaterialized item or is read by automation snapshots | `Unmaterialized`/`materialized = false` remains authoritative; semantics cannot materialize, scroll, act, focus, paint, hit-test, schedule, render, or register a provider. |
| Snapshot purity/reentry | Snapshot read during a demand turn or provider callback requests follow-up work | `automation_snapshot` and `automation_target_snapshot` remain observational; the mutating demand turn is separate and follow-up invalidation is coalesced. |
| Native semantic accessibility boundary | A private later macOS/AppKit adapter translates one explicit bounded item or contiguous child-range query | One runtime-issued lease and current container handle plus one stable key or finite logical range are required; contention is one typed `Unavailable(SessionContended)` result; translation uses only the explicit session refresh/retry operations, never direct or duplicate provider calls. Native publication is complete-fence-only, retains only exact eligible fallback, withholds stale/unsupported converted bounds, preserves `materialized = false`, and leaves native actions separate. |
| Cardinality declaration | A `VirtualLayoutParts<Message>` declaration includes `Some(VirtualLayoutSemanticCardinality { logical_item_count, cardinality_revision })`, `None`, or exact zero | The value is immutable qualified declaration evidence, not a callback or demand; `None` is unknown/unsupported, zero is supported, the count is not capped at 1024, and no storage proportional to the count is allocated. The field and qualified builder, plus the exact private registration/live-fence invalidation foundation, are shipped outside the prelude. The native cardinality query/child traversal, normalized sidecar, native topology, and platform consumer remain unshipped. |
| Cardinality fence | Count or cardinality revision changes, provider replacement, unmount, recovery, native deactivation, or session close | Exact `(count, cardinality_revision)` equality is required together with registration identity/generation, container/mount, existing revisions, coordinate, budget, and provider generations. Count/revision changes invalidate semantic/native state provider-free; provider replacement preserves count but invalidates provider publication; lifecycle retirement clears all state. |
| Cardinality availability | Unknown count, positive count without a range provider, exact zero, or positive count with a range provider | Unknown does not vend a virtual child container; positive-without-range-provider is unsupported and not vended; exact zero is representable without a provider; positive-with-range-provider may vend the private container. Count reads, updates, mount, and enumeration never create demand. |
| Native cardinality query | AppKit asks for a count or a bounded range `(index, maxCount)` | Count returns the exact declared count. Range normalization uses checked subtraction from count, handles zero and out-of-range inputs, rejects overflow, applies declared budget, 1024 per-query cap, and remaining aggregate budget, and never synthesizes a key from the index. |
| Normalized sidecar authority | The compositor stages `entries_by_container` and produces `VirtualLayoutAutomationComposition` | One crate-private sidecar is produced from that same union and retains container/mount/registration authority, cardinality fence, logical index, stable key, provider `AutomationNodeId`, final normalized node/path, materialization authority, and publication fence. Raw range/pin members are not reconstructed by native code. |
| Sidecar publication veto | Conflicting/ambiguous/duplicate/unstable/colliding/ordinary-ID/aggregate evidence or an exact same-key/index overlap | Same-key/index overlap coalesces only under existing full-evidence equality; every listed failure rejects the whole publication. The sidecar is atomically stored with `RuntimeSemanticAutomationSelection` composition/status/ordinary/projection; no parallel reconstruction or mixed native/public selection is allowed. |
| Private native topology | One primary content view/window, accepted virtual anchors, normalized logical items, and repeated/stale native callbacks | One private root, one private read-only container per accepted anchor, and direct item children only; duplicate placement elsewhere is suppressed. Runtime-issued private container identities and monotonic item tokens are never derived from index, pointer, provider/serialized ID, or bounds. Exact lease/container/mount/cardinality-fence/key continuity is required; foreign/stale/retired/duplicate/ambiguous/colliding tokens return `nil`/`NSNotFound` with no provider call. |
| Conservative native mapping | Root/container/non-text item, `Text`/`Readout`, actionable provider role, or defunct object | Group role is used for root/container/non-text; only `Text`/`Readout` use static text. Expose only role, exact parent/children, finite frame, label, description/help, and static value. Omit state/action metadata, force focused false and empty/no-op actions, never map buttons/toggles/sliders/tables/text inputs to actionable roles, and return empty/zero for defunct objects. |
| Native pending query | Valid explicit item/range query while an identical request is in flight or a prior result is `Deferred` | Non-blocking callbacks enqueue/coalesce one owned runtime turn without provider/runtime reentry. Count stays exact; item/range reads return exact eligible same-fence retention or empty/`nil`, never placeholders/mixed trees. Identical in-flight queries coalesce; explicit repeat after `Deferred` may retry; ordinary reads do not retry. |
| Native publication and notification | Complete accepted projection, exact retention, terminal failure, pending/stale/cancelled/rejected work, or a retired custom object | Complete normalized projection installs atomically and retains only exact semantic/native coordinate/cardinality fences. DataUnavailable/Deferred without exact fallback is empty/baseline; terminal failures clear virtual publication; stale/cancelled are inert. A changed visible state posts exactly one `NSAccessibilityLayoutChangedNotification` after main-thread queryability; unchanged/pending/stale/cancelled/rejected post none; retired objects follow `UIElementDestroyed`. |

## 15. Explicit exclusion: `split_pane`

`split_pane` is excluded from this design. Virtualization does not define its
resize gesture, collapse/restore semantics, persistence, layout interaction,
focus behavior, or host/runtime consumer. Any future `split_pane` work must have
its own contract and must not be smuggled into a virtualization slice as
materialization, anchoring, or scheduler work.
