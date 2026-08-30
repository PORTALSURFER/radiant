# Radiant Architecture Map

This map explains how the current Radiant codebase is organized against the
broader project direction in `docs/TARGET.md`. It is a contributor guide, not a
second public API reference: `docs/API.md` remains the application-facing
contract (and the current API reference), while `docs/API_STYLE.md` defines the
preferred application-facing API style and cleanup-ticket target shape.

`docs/DESIGN_DIRECTION.md` is the normative target-state architecture contract
for new design and API decisions. This map intentionally describes shipped
module ownership and the migration seams between it and that target; a boundary
described here is not evidence that every target-state node, scheduling, or
renderer contract is already implemented.

Only canonical merged source counts for shipped status; branch, draft,
acceptance-only, and unverified evidence do not. X11 and product-specific
behavior remain explicit non-goals for Radiant.

Radiant's architecture should keep one external mental model while allowing
focused internal modules. The main ownership rule is:

- Application code owns domain state, business logic, files, audio/plugin
  hosts, and product-specific naming.
- Radiant owns declarative view construction, stable widget identity, layout,
  input routing, focus, styling, invalidation, paint planning, diagnostics, and
  renderer-facing surface contracts.
- Radiant owns the scheduling boundary for UI-safe follow-up work. Normal app
  update handlers must not run business work directly; they schedule it through
  `UiUpdateContext::business()` or request typed Radiant platform services.
  Worker closures may receive `radiant::runtime::BusinessWorkContext` as an
  explicit runtime capability for cooperative cancellation, but the worker
  context is not part of the normal app prelude and is not app-constructible.
  The target design describes the same ownership boundary as an owned effect
  model; until that migration is complete, `UiUpdateContext` and its business
  lanes are the shipped spelling.
- Radiant additions must pass the primitive-boundary test in `docs/TARGET.md`:
  they should be generic UI primitives or reusable GUI building blocks, not
  product-shaped composite widgets or application workflows.
- Native or embedded hosts own the platform event loop and decide how to attach
  Radiant surfaces to windows, popup surfaces, or host-controlled render
  targets.

## Public Surface

Normal application code should start through `radiant::prelude`,
`radiant::window(...)`, or `radiant::app(...)`. These builders lower into the
same `UiSurface`, `SurfaceNode`, `WidgetId`, `Command`, minimal `RuntimeBridge`,
and cached `RuntimeHostCapabilities` contracts exposed through the explicit
runtime modules.

The normal app-facing surface is intentionally non-blocking. Update handlers
may mutate durable UI/application state, apply business or platform-service
results, emit messages, request repaint/focus/timers, request typed platform
services, and schedule business work. Filesystem, database, decode/load,
network/process work, sleeps, blocking waits or joins, thread creation, cache
hydration, and long CPU transforms belong behind the business runtime or a
platform adapter. Static app call sites should use the named business lanes;
host policy that already resolved a runtime `TaskPriority` should use
`context.business().priority(name, priority)` instead of duplicating lane
matches in reducers. Command-returning and command-injection paths are migration or
advanced surfaces, not the target ordinary application model.

The explicit runtime and widget modules are supported control surfaces, not a
competing framework. They exist for custom hosts, tests, advanced widgets,
diagnostics, and embedded integration where the application needs to drive a
surface without the native window runner.

Declarative view nodes, widget-view contexts, projected surfaces and nodes,
installed widgets, and the generic runtime lifecycle/controller are UI-affine.
They carry private zero-sized `Rc`-backed marker evidence, so the compiler
rejects moving those owners to a worker even when a particular visible payload
is otherwise transferable. `Message` and mapper storage keep their existing
local bounds, while worker and effect boundaries retain explicit `Send`
requirements for owned transfer payloads.

### Prelude Export Hygiene

`src/prelude.rs` is only the small facade for common application imports. It
must not accumulate direct subsystem export lists. The first-level prelude
modules are facades too: if a grouped file grows into a broad import block, split
it into smaller owning modules under a directory with the same name, as
`src/prelude/application/**`, `src/prelude/gui/**`, and
`src/prelude/runtime/**` do today. Add new prelude exports to the smallest
owning grouped file:

- `application.rs` for app builders, view builders, control builders, menu
  builders, panel builders, task/update types, and stateful app helpers.
- `gui.rs` for backend-neutral GUI models, layout helpers, selection,
  shortcuts, visualization, text-layout, paint geometry, and form/panel state
  helpers.
- `layout.rs` for direct layout result/types that are part of common app-facing
  signatures.
- `runtime.rs` for commands, runtime services, paint primitives, native run
  options, resources, drag/drop, windows, diagnostics, and retained surfaces.
- `theme.rs` for theme-token exports.
- `widgets.rs` for widget contracts, primitive widget parts, input/output
  messages, interaction helpers, and widget visual tokens.

Prelude export files should stay cohesive enough to scan without horizontal
ownership hunting. As a rule of thumb, split a file before it mixes unrelated
areas such as controls plus menus plus tasks, or before it needs a very large
`pub use crate::<subsystem>::{...}` block just to remain formatted. A facade may
`pub use` its focused child modules, but the child modules should encode the API
area being exported: controls, details lists, overlays, resources, paint,
visualization, and so on.

Keep exports explicit inside those grouped files. Do not replace them with
wildcard re-exports from owning subsystems, because that would let unrelated
future public items leak into `radiant::prelude::*` without an API decision.
When an item is useful but not common enough for normal app code, leave it on
its owning explicit module such as `radiant::runtime`, `radiant::widgets`,
`radiant::layout`, `radiant::theme`, or `radiant::gui`.

Import and export size is treated as design feedback, not as a formatting
problem. A prelude export leaf should stay below the source-quality guardrail of
32 lines. When a leaf approaches that limit, first decide whether the API area is
too broad, whether a new focused prelude child should own part of the surface, or
whether the item should stay on its explicit subsystem module instead of entering
the common prelude. Do not add catch-all prelude modules, wildcard subsystem
exports, or local application preludes to make a large list disappear.

New prelude exports should follow this checklist:

- Add the item to the smallest existing prelude leaf module that matches the
  owning API area.
- If no focused leaf exists, add one under the matching first-level prelude
  directory instead of lengthening a facade.
- Keep first-level split groups such as `application`, `gui`, and `runtime` as
  child-module facades. They should not contain direct `pub use crate::...`
  lists once they have been split.
- Treat a large import or export block as a module-boundary smell. First decide
  whether the code is mixing ownership areas or whether a reusable GUI primitive
  belongs in Radiant, then add the import only after the boundary is clear.
- If an application needs many low-level widget, paint, layout, and runtime
  types together, prefer introducing a generic Radiant primitive or splitting the
  app-side custom widget into smaller paint/input/state modules over expanding
  `radiant::prelude`.

The source-quality guardrails assert that the root prelude and split first-level
prelude groups remain facades. If those tests fail, the fix should normally be a
new focused export leaf or a module split, not a formatting workaround.

## Core Subsystems

- `src/application` owns the application-builder runtime: state projection,
  update callbacks, runtime messages, subscriptions, timers, and business-work
  delivery back into the UI-first runtime.
- `src/runtime` owns backend-neutral retained surfaces, runtime commands,
  widget traversal, input dispatch, focus, scroll state, resource slots,
  platform requests, paint plans, diagnostics, GPU-surface payload contracts,
  generic `SurfaceRuntime` lifecycle transition evidence, and the `RuntimeBridge`
  plus explicit host-capability boundary. Each generic controller keeps its
  lifecycle behind one private validated transition authority and exposes only
  bounded controller-owned evidence through `RuntimeDiagnostics`; this does not
  transfer native recovery, effect ownership, or scheduler policy to the
  generic runtime. `src/runtime/controller` also owns the bounded typed
  mounted-container state kernel and candidate-safe lifecycle reconciliation;
  the split-pane runtime/controlled ratio projection consumes that slot without
  broadening global capability semantics, while the runtime-owned divider
  capability and controller capture remain private to split-pane lowering. Its
  optional collapse policy is carried alongside the split layout/lowering
  interaction state rather than public `SplitPanePolicy`; the private mounted
  state schema includes bounded last-expanded-ratio evidence. An admitted
  divider double activation is a discrete runtime command that reuses the
  authoritative split resolver and quantization, mutates the mounted state
  before requesting existing layout work, and restores only a finite expanded
  ratio retained by the same compatible mounted identity. The resolver fails
  closed when its normalized minima are unsatisfied; ordinary split-layout
  undersized fallback does not authorize collapse. Policy/mode/schema
  incompatibility, unmount, stale evidence, and unavailable state retire that
  restore authority. The
  optional settled-ratio mapper is lowering-owned output: the controller
  completes mounted state mutation and terminal capture cleanup before host
  message reduction, and compatible same-identity reprojection retains the
  captured interaction and geometry authority. A crate-private
  `SplitPaneSeparatorProjection` is a post-commit read-only observation joining
  only the exact committed mounted state to one valid clipped divider target.
  The controller-owned pure automation compositor may consume it to publish one
  passive backend-neutral `AutomationRole::Separator` between the split's two
  direct content children, with the stable layout-target identity, exact clipped
  bounds, normalized ratio value, and logical-axis orientation. The projection
  itself carries no interaction, focus, key, paint, relayout, message, native,
  or public-API authority; malformed or ambiguous evidence leaves the ordinary
  snapshot unchanged. The private primary-window macOS/AppKit consumer now
  publishes each qualified separator as one `AXSplitter` between the two pane
  children, while retaining passive, non-focusable, actionless behavior.
  Separately, the runtime has a crate-private fixed-size focus owner and a
  committed mixed-order sidecar. The explicit backend-neutral sequential
  traversal consumer admits exact current mounted separator identity and
  behavior evidence as private stops between pane widget subtrees, including
  nested separators; invalid evidence uses the complete widget-only order. The
  crate-private traversal disposition distinguishes `NoDestination`,
  `AdmittedWidget`, `AdmittedPrivateSplitPaneSeparator`, `Vetoed`, and
  `Invalidated`. The generic native runtime consumes an unclaimed plain
  `Tab`/`Shift-Tab` using that committed sequential disposition:
  `AdmittedWidget` and `AdmittedPrivateSplitPaneSeparator` are consumed
  destinations; `Vetoed` and `Invalidated` are terminal with no fallback/retry;
  only `NoDestination` reaches the existing host-first/widget fallback exactly
  once. Focused-key/text input ownership and command/control/alt-modified `Tab`
  retain precedence. Repeat/release do not traverse again, and the per-window
  sequence latch is cleared on native focus loss/regain. This does not make the
  passive `AXSplitter` projection focusable or public; it remains non-focusable
  and actionless. Private separator ownership for traversal and pointer
  acquisition remains distinct from this native key-routing consumer,
  public/native separator focus, spatial traversal, keyboard/arrow-key resizing,
  semantic actions, and paint/cursor/renderer work, which remain future slices.
- `src/widgets` owns built-in widget contracts and named-part construction for
  primitive widgets.
- `src/gui` owns reusable backend-neutral GUI models: layout, forms, feedback,
  panels, lists, selection, shortcuts, text-line placement, visualization
  helpers, automation snapshots, and visual snapshots.
- The shipped custom layout extension keeps `LayoutPolicy` in the qualified
  `radiant::layout` facade and stores it as UI-local `Rc<dyn LayoutPolicy>`.
  The two-pass layout engine gives the policy normalized child measurement and
  bounded top-down placement contexts. Each child must be placed or explicitly
  omitted exactly once; malformed requests and unresolved children produce
  diagnostics and conservative output. This first slice is measure/place only:
  custom chrome, environment/appearance, interaction, semantics, alternate
  reading order, animation, virtualization attachment, exact policy revisions,
  and custom cache reuse remain outside the boundary. Built-in
  `ContainerPolicy`/`ContainerKind` dispatch is unchanged. OPT-1272 is Done;
  this measure/place-only boundary does not reopen that issue.
- `src/gui_runtime` owns native runtime integration and renderer adapters. The
  current native Vello runtime is the macOS implementation path; the target
  adds native Wayland and Windows host adapters behind the same Radiant-owned
  WGPU, Vello, font loading, scene caching, input, window-policy, and popup
  boundaries.
- Focused-key routing has an explicit three-part ownership boundary: the
  `Widget` contract opts a widget into metadata-aware participation and reports
  its current normalized captured key; `src/runtime/controller` owns the
  fixed-size capture record, host precedence, owner cancellation, stale/ignore
  decisions, and exact refresh reconciliation; and
  `src/gui_runtime/native_vello/generic_runtime` only translates native key
  evidence before delegating to the generic controller. The kernel is generic
  and numeric-policy-free, while widgets retain the existing key-only fallback
  unless they opt in.
- Pointer-press admission has the same generic ownership split: the qualified
  `radiant::widgets::PointerPressAdmission` hook selects Legacy, ManagedCapture,
  or Blocked after scrollbar/layout target precedence; `src/runtime/controller`
  owns one fixed-size exact-widget/exact-button managed record, continuation
  validation, cancellation, orphan-release suppression, and refresh
  reconciliation; native adapters continue to delegate normalized pointer
  events through the existing controller seam. Legacy widgets retain the old
  path and Blocked presses stop before focus, capture, widget dispatch, mapping,
  or host output. Complete-mode NumericInput owns its qualified
  `NumericScrubPolicy`, retained scrub consumer, typed output/failure mapping,
  and geometry/anchor lifecycle in the widget allowlist; it uses this kernel
  without changing generic controller or native production paths.
- Complete-mode NumericInput wheel consumption uses the same ownership split:
  `NumericWheelPolicy` is an explicit widget opt-in, exact line/pixel and phase
  evidence reaches the widget through `WheelSample`, and the widget owns typed
  adjustment/format output plus bounded continuity state. The scroll controller
  owns managed Idle/Active/Blocked authority, owner-only synthetic cancellation
  before a superseding start, and legacy metadata-preserving dispatch after
  metadata-neutral hit testing. Native unit/phase translation remains an
  adapter boundary and cannot be inferred from legacy vector input.
- `examples` owns maintained public-API sandboxes. Examples are validation
  surfaces as well as documentation.
- `benches/perf_harness` owns opt-in performance scenarios for layout,
  application projection, runtime surface work, command drainage, and
  GPU-surface data preparation.
- `tests` owns public API, behavior, source-quality, example, and documentation
  guardrails.

## Virtual Layout Semantic Provider Boundary

The current mounted `SurfaceRuntime` virtual-layout registration and semantic
demand/publication kernel are implementation evidence. The qualified public
declarative attachment is normative and shipped at the Logical boundary, with
the bounded custom-coordinate attachment defined fully in
[`VIRTUAL_LAYOUT_DESIGN.md`](VIRTUAL_LAYOUT_DESIGN.md).

Current shipped boundary: public declarative attachment and mounted runtime
registration/two-pass bridging are shipped. The first-class production
consumer/collection family remains future work, sequenced by OPT-1362 and then
OPT-1400, OPT-1398, OPT-1397, OPT-1399, and OPT-1401.

The only public declarative attachment capability is
`radiant::application::VirtualLayoutParts<Message>` with
`virtual_layout_from_parts`. It may attach optional
`VirtualLayoutSemanticProvider` and `VirtualLayoutSemanticRangeProvider`
capabilities through read-only item/range requests and
`VirtualLayoutSemanticEntry` results. `radiant::runtime::VirtualLayoutRevisions`
and generic `VirtualLayoutSemanticProviderOutcome<T>` with `Found`, `NotFound`,
`Unavailable`, `Deferred`, and `Rejected` are qualified shipped vocabulary:
they are not prelude entries. The separately qualified
`radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransform`
request/outcome vocabulary and
`VirtualLayoutParts::with_semantic_coordinate_transform(...)` attach a
synchronous `Rc` resolver for `Custom(identity)`; without the attachment the
existing declaration remains `Logical`.

The shipped qualified declaration foundation exposes the public value
`radiant::application::virtual_layout::VirtualLayoutSemanticCardinality` on
`VirtualLayoutParts<Message>`, with the qualified builder
`VirtualLayoutParts::with_semantic_cardinality(...)`. It contains exactly an
`usize` logical item count and a separate `u64` cardinality revision. The field,
builder, normalized sidecar, native child traversal/topology, bounded AppKit
queries, and private primary-window platform consumer are implemented outside
the common prelude. Automated AppKit boundary evidence remains shipped; exact
fresh-bundle activated Computer Use/AppKit evidence verifies discoverability and
numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible. The value is immutable
declaration evidence, not a callback or demand; `None` is unknown/unsupported,
exact zero is supported, the count is not capped at 1024, and it never allocates
proportional storage.

The custom resolver receives only a finite source rectangle, the unique
runtime-validated ordinary anchor, the complete effective logical destination
clip, host revision evidence, and the exact transform revision. It returns a
conservative finite AABB directly; no affine matrix, inverse, point mapping,
hit-test, or materialization assumption exists. `SurfaceRuntime` owns the
attachment lifetime, resolver generation/token, destination admission,
panic/reentry containment, clipping, exact private transform witness,
retention, publication, and invalidation. It invokes the resolver only during
explicit semantic refresh/retry after complete provider-output validation and
at most once per accepted provider entry.

`SurfaceRuntime` owns mounted registration, removal, replacement,
registration/mount/provider generations, lifetime cancellation, exact source
tickets, and whole-surface publication. Each mounted container has one active
contiguous range slot and one independent required-item slot; the surface is
bounded to 64 registrations, 1024 entries per query, and 1024 aggregate active
range entries, with at most one provider call per container and attempt. There
is no public imperative register/remove API or application-owned mount
generation. The first callback boundary is synchronous single-threaded `Rc`,
with no `Send`/`Sync`, worker, or scheduler promise. Only explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` call providers. Registration,
opening, enumeration, ordinary snapshot/target reads, repaint,
viewport/visibility/overscan, diagnostics, item count, provider availability,
and IME/native events do not create demand.

Scope replacement is remove-then-mount with a fresh mount generation. A same-
scope declaration update advances the registration generation when capability
or revision evidence changes; replacing only one provider advances only its
source generation. Removal, unmount, and provider replacement cancel the old
source tickets, retire old authority, and make late returns stale without
calling a provider. A missing optional provider becomes runtime-synthesized
`NoProvider` only when explicit session intent executes that source.

`NoProvider` is runtime-synthesized; provider unavailable reasons are
`DataUnavailable` and `Unsupported`, and bounded deferred reasons are
`DataPending`, `SemanticPending`, and `Retry`. Exact fences, read-only callback
isolation, reentry rejection, conservative panic mapping, validated
`Found`/authoritative `NotFound`, terminal missing/Unsupported, exact-fence
retention only for `DataUnavailable`/`Deferred`, conservative rejection
baseline, inert stale/cancelled/superseded results, atomic publication, and
preserved `Unmaterialized` authority are normative. The private primary-window
macOS/AppKit semantic accessibility consumer below translates explicit platform
queries only through the backend-neutral session model; it is not a hidden
provider owner. The contract's non-goals are direct native custom-resolver
invocation/reconstruction, native actions for virtual/provider targets, new
native focus setter/transfer or focus exposure beyond the ordinary materialized-target
contract,
scrolling/materialization, scheduler/backoff/fairness,
renderer/paint/hit-testing/cache policy, product policy, multiple ranges, and
prelude export. This bounded attachment is the public-API evidence point.
Automated AppKit boundary evidence remains shipped; exact fresh-bundle
activated Computer Use/AppKit evidence verifies discoverability and numeric
action, bounded set-value, and restart acceptance for this bounded primary-window
consumer. VoiceOver-specific acceptance remains unperformed; repeated
negative-geometry AppKit runtime diagnostics remain a separate unverified
follow-up if reproducible.

## Native semantic accessibility query consumer (normative; private primary-window macOS/AppKit consumer)

The first native semantic consumer is the private primary-window macOS/AppKit
production path over the shipped generic logical semantic automation session.
One private native-window
adapter MAY acquire one runtime-issued semantic-session lease. The adapter and
lease remain private: neither owns provider registration, mount identity, provider
generations, demand fences, cancellation, or publication. The existing bound of
one active semantic session per `SurfaceRuntime` remains. A native lease MUST NOT
evict, supersede, or silently reuse an externally active session; contention
returns the one private typed unavailable result `Unavailable(SessionContended)`.
Multi-consumer arbitration is a later contract.

Lease acquisition is lazy: passive root construction, ordinary native-tree
observation, exact count reads, registration/cardinality synchronization, and
ordinary property reads never acquire the lease or create demand. Only an
explicit item or child-range query reaching the owned runtime turn may acquire
it.

Accessibility enablement, native tree-root construction, accessibility-state
observation, ordinary native events, repaint, and ordinary property reads are
observation/capability only. Only an explicit bounded native item or child-range
query MAY become `SemanticAutomationDemand`. Each query MUST translate to exactly
one current runtime-issued semantic-session lease and one current runtime-issued
container handle, plus either one stable required-item key or one finite
contiguous logical range. Missing, stale, ambiguous, duplicate, oversized, or
unrepresentable evidence is unavailable and MUST cause no provider call.

The adapter submits intent only through the existing explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` operations. It never invokes a
provider directly or causes a second call for one container/attempt. Native
callbacks MUST NOT synchronously re-enter a provider or mutate `SurfaceRuntime`
through observational access. Native-to-runtime handoff enters one owned runtime
turn and is bounded transport only; it does not add scheduler, retry, or fairness
policy.

Native publication exposes only a complete selected snapshot under the existing
exact fence. It MUST NOT expose partial virtual subtrees, mix generations, or
repair malformed or colliding evidence. `DataUnavailable` and `Deferred` MAY
retain only an exact eligible complete selection. Missing provider, unsupported,
rejected, panic, malformed, collision, stale, or cancelled evidence uses the
existing typed conservative baseline behavior; stale and cancelled completions
are inert and MUST NOT mutate or publish native state.

The root is attached to the primary content view only through AppKit's supported
`accessibilityChildren`/`setAccessibilityChildren:` property, and installation
is accepted only after a bounded exact one-root identity readback. A nil or
wrong root/host, unsupported selector, Objective-C exception, or mismatched
readback leaves the adapter unattached, posts no layout/value notification, and
attempts to clear any attempted host state. Retirement releases pre-commit
allocations silently only after nil/empty clear readback; if that readback
cannot be verified, callback state and object ivars become inert but objects
remain quarantined, with no release or destruction notification while the host
may retain a stale root. Previously committed objects receive exactly one
destruction notification only after verified clear. Replacement, recovery,
close, and drop clear `accessibilityChildren` symmetrically before releasing
committed native objects; the root reports the same content view as `AXParent`.
Automated AppKit boundary evidence remains shipped. Exact fresh-bundle
activated Computer Use/AppKit evidence verifies discoverability for this
bounded primary-window consumer: the activated window exposed the Radiant
container and a settable stepper at `42.00`; Increment and Decrement produced
`43.00` and `42.00`, bounded `SetValueText` produced `55.50` and `57.25` with
fresh reads showing normal app-owned Begin/Update/Commit events, and a fresh
restarted instance exposed the same tree. VoiceOver-specific acceptance remains
unperformed. Repeated negative-geometry AppKit runtime diagnostics remain a
separate unverified follow-up if reproducible.

The first native consumer accepts `Logical` registrations unchanged and admits
`Custom(identity)` only with the matching current transform attachment, exact
cardinality/provider/anchor evidence, and runtime-owned transform
revision/generation/token. Native publication consumes only the compositor's
complete normalized logical-window bounds plus the matching sidecar witness and
publication fences. Native conversion MUST identify source surface space,
destination window/screen accessibility space, DPI, window/display generation,
orientation, clipping, and a finite non-inverted conversion. Stale, unsupported,
missing, or mismatched authority withholds the complete custom projection; no
resolver is invoked or reconstructed and no affine, corner-mapping, inversion,
or identity fallback is permitted.

The private primary-window macOS/AppKit consumer also consumes the passive
split-pane projection. It admits a separator only when one unique current
target with materialized authority matches the exact node ID, semantic path,
`AutomationRole::Separator`, normalized ratio value, finite bounds, orientation,
and actionless/non-focusable state. A valid separator publishes one `AXSplitter`
between the two pane children with current bounds, ratio value, and
`AXOrientation`; stable native token/object identity is retained across an
admitted ratio update. Static/controlled, stale, unmaterialized,
duplicate/mismatched, malformed, focused, or actionful evidence leaves the
ordinary native tree in place without provider calls, runtime refresh, partial
topology, or interaction authority. Manual macOS/VoiceOver acceptance remains
unverified. A separate crate-private runtime focus-order sidecar may retain
source candidates and a committed mixed-order evidence sequence after exact
projection/lifecycle reconciliation. The explicit backend-neutral sequential
traversal consumer is the only consumer of that sidecar: it admits each exact
current runtime-owned separator as one private stop between its pane widget
subtrees, including nested separators, and falls back to the complete
widget-only order for invalid evidence. Its crate-private disposition keeps
`NoDestination`, admitted widget/separator, `Vetoed`, and `Invalidated`
separate; only `NoDestination` may feed a future key-routing fallback, while
veto and invalidation are terminal. It is separate from this passive native
path and private pointer ownership for divider acquisition. The crate-private
generic native plain `Tab`/`Shift-Tab` consumer is shipped: focused-key/text
input gets first refusal; modified `Tab` is unchanged; repeats/releases do not
retraverse; focus loss/regain clears the latch; and only `NoDestination` feeds
the existing host-first/widget fallback. Public/native focus, spatial traversal,
keyboard/arrow-key resizing, semantic accessibility actions, and
paint/cursor/renderer work remain future slices.

Activation/opening is provider-free. Explicit native queries refresh, and an
explicit repeated query MAY retry. Deactivation, window retirement, recovery
replacement, and close cancel and retire the lease before native objects drop.
`materialized = false` remains authoritative: native semantics cannot materialize,
scroll, focus, execute actions, paint, hit-test, schedule, render, or claim
provider authority. Virtual/provider nodes therefore never acquire native
numeric action authority. For an ordinary runtime node only, the private
adapter pairs the pure ordinary `automation_target_snapshot` with the native
semantic tree and admits exactly an enabled, editable, focusable,
`AutomationRole::TextInput` target with current `value_text`, materialized
authority, and both neutral increment/decrement actions; it need not already be
runtime-focused. The exact ordinary
ID/path/role/authority is captured with one native token; geometry is never an
authority fence. A qualified node publishes `AXIncrementor`, the exact label when present
and NSString value, `AXDescription`/`AXHelp` when present, enabled true,
`AXFocused` and modern `isAccessibilityFocused` agree for the current
ordinary focus contract; a value is settable only for an eligible current
ordinary materialized target, and exactly `AXIncrement` and `AXDecrement` are
accepted. Modern
increment/decrement selectors use `BOOL c@:` and the
deprecated action selector uses `void v@:@`; only those exact action names are
accepted.
The modern `accessibilityValue` getter uses `id @@:`, while modern and legacy AXValue setters use `v@:@` and `v@:@@`; only a bounded `NSString` (at most 1,024 UTF-16 units and 4,096 UTF-8 bytes) is accepted and translated to the existing neutral `SetValueText` path. Runtime fences, codec validation, atomic publication, and inert stale/failure behavior remain authoritative; no native focus or virtual/provider mutation is added. Each native action enqueues one bounded primary-window,
adapter-generation, token, target, and neutral-action event. The running
event-loop validates current window/generation/token/identity/authority and
delegates to existing `SurfaceRuntime` admission once; that admission may
perform the ordinary runtime focus transition. Non-focusable, focus-vetoed,
blocked, stale, unsupported, disabled, read-only, recovery, close, borrow,
panic, and transport failures are inert and never retarget or mutate. Foundation
exceptions from callback-supplied native objects are caught inside the private
Objective-C boundary and map to the same inert no-event result; they never cross
into Rust. A stable
value-only
change retains the native object, installs the new queryable value before one
`AXValueChanged`, and posts no layout notification; unchanged, no-change,
typed-failure, stale, and enqueue-failure paths post none. Native requests are
not provider demand, materialization, virtual/provider mutation, slider,
range, orientation, percentage, or scalar conversion. The adapter and lease are
not public imperative provider-registration APIs. `automation_snapshot(&self)`, `automation_target_snapshot(&self)`, and
`selected_semantic_automation_snapshot(&self)` remain pure reads.

Read-only ordinary focus exposure is shipped in this slice under a separate
ownership contract. `SurfaceRuntime` and its controller remain the sole
logical-focus authority; the current focused state of the primary Winit window
is the platform eligibility fence; and the AppKit adapter is only a consumer.
At most one ordinary, enabled, focusable, materialized node in the current
runtime/window generation is focused when exactly one controller-owned target
matches the current ID, path, role, and focus evidence. Missing, stale,
ambiguous, mismatched, provider/virtual, unmaterialized, or inactive-window
evidence exposes no native focus. The root's legacy `AXFocusedUIElement` and
modern `accessibilityFocusedUIElement` return the same object or nil. Stable
A-to-B changes retain both native objects and post one
`AXFocusedUIElementChanged` notification on B after it is queryable; unchanged
and clearing transitions post no gained-focus notification, and focus-only
changes post no layout, value, or destruction notification. Focus publication
and update are provider-free and invoke no providers. The adapter never
implements `setAccessibilityFocused`, transfers native focus, or publishes
auxiliary-window/virtual focus. Explicit virtual item/range queries retain the
provider-demand exception described above.

This focus slice has automated Rust/Objective-C boundary coverage only. No live
VoiceOver or live AppKit focus acceptance was performed for it; prior numeric
action and host-attachment evidence is a separate acceptance boundary.

The native adapter uses provider-free semantic cardinality as its only child
count authority. It does not vend a virtual child container for unknown
cardinality. A positive count without a range provider is unsupported for native
child traversal and is not vended; exact zero is representable without a
provider. Count reads, declaration updates, mounting, and enumeration never
create demand. AppKit count returns the exact declared `usize` count. Range
normalization checks zero, out-of-range, checked subtraction/end arithmetic,
declared budget, `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` (1024), and remaining
aggregate budget; it never synthesizes a key from an index.

The exact `(count, cardinality_revision)` pair is fenced with registration
identity/generation, container and mount identity/generation, the existing
data/policy/measurement/semantic revisions, coordinate space, budget, and
source-qualified provider generations. Equality is exact, never latest-ordering
or partial matching. Any count or cardinality-revision change invalidates
affected semantic/native state provider-free. Provider replacement preserves the
count but invalidates provider publication; unmount, recovery, deactivation, and
close retire all state.

The compositor emits one crate-private normalized sidecar from the same staged
`entries_by_container` union that emits `VirtualLayoutAutomationComposition`.
Each member retains container/mount/registration authority, the cardinality
fence, logical index, stable `VirtualLayoutItemKey`, provider `AutomationNodeId`,
final normalized node/path, materialization authority, and publication fence.
Same-key/index overlap coalesces only under existing full-evidence equality.
Raw range/pin members are not reconstructed by the native adapter. Conflicting,
ambiguous, duplicate, unstable, colliding, ordinary-ID, or aggregate failures
reject the whole publication. The sidecar is stored atomically with
`RuntimeSemanticAutomationSelection` composition/status/ordinary/projection;
parallel reconstruction and mixed native/public selection are forbidden.

The primary content view/window exposes one private root; each accepted virtual
anchor has one private read-only virtual container; each normalized logical item
is a direct child, with duplicate placement suppressed elsewhere. Container
identity and monotonic item tokens are private and runtime-issued. Tokens are
never derived from index, pointer, provider ID, serialized ID, or bounds, and
continuity requires exact lease/container/mount/cardinality-fence/key equality.
Cardinality change retires tokens. Foreign, stale, retired, duplicate, ambiguous,
or colliding tokens return `nil`/`NSNotFound` without a provider call.

The existing dynamic native class registers exactly
`accessibilityIndexOfChild:` with Objective-C encoding `Q@:@`; its callback ABI
returns `usize`/`NSUInteger`. It returns `NSNotFound`, defined as
`isize::MAX as usize` (never `usize::MAX`), for any index equal to that sentinel.
The callback resolves receiver and child uniquely by opaque object identity in
one current immutable native projection and requires exact direct-parent token
equality. For `Root`, `Ordinary`, and `Item`, it searches only the receiver's
ordered direct `children` vector and returns the unique compact zero-based
position. For `Container`, it searches only `logical_children` against
`logical_count` and returns the retained logical index itself, including
sparse/nonzero indices such as 100; a child present only in ordinary `children`
is absent. Nil, foreign, stale, retired, indirect, sibling, ancestor, self,
wrong-parent, wrong-kind, out-of-count, missing, malformed, duplicate,
ambiguous, or sentinel-colliding evidence, borrow conflict, and panic all return
`NSNotFound`. The callback does not message or dereference the supplied child,
allocates no storage proportional to declared count, and performs no
provider/query/runtime/action/lease/notification mutation.

The same dynamic native class registers exactly
`accessibilityNotifiesWhenDestroyed` with Objective-C encoding `c@:`. Its
crate-private Rust callback ABI returns `ObjcBool` and returns `YES`
unconditionally for every instance, including when the callback-state ivar has
already been cleared during retirement. It is contained by the existing
`ffi_boundary`/unwind boundary but does not consult or borrow callback state,
inspect projection/tokens/view/window, allocate, lock, access provider/runtime,
enqueue events or notifications, or mutate anything. It does not post
notifications. Existing retirement order clears callback state and each state
ivar before posting exactly one `AXUIElementDestroyed` notification and
releasing each object; repeated retirement is idempotent for already-retired
objects.

Root, container, and non-text roles map to `NSAccessibilityGroupRole`; only
`Text` and `Readout` map to `NSAccessibilityStaticTextRole`, while
`AutomationRole::Separator` maps to `AXSplitter` with its finite frame,
normalized ratio value, and horizontal/vertical `AXOrientation`. The adapter
exposes only role, exact parent/children, finite frame, label, description/help,
and value. Checked/selected/enabled/read-only/focusable/focused/tab/live/action
metadata is omitted for passive nodes; separators remain non-focusable and
actionless, and buttons/toggles/sliders/tables/text inputs never become
actionable roles through this consumer. The runtime may hold private separator
ownership for pointer acquisition or explicit sequential traversal, but that
ownership does not expose separators as native/public focus targets. Defunct
objects return conservative empty/zero values.

AppKit callbacks are non-blocking and never call or mutate the runtime/provider.
A valid explicit item/range query enqueues and coalesces one owned runtime turn.
While pending, count remains exact; item/range reads return only an exact
eligible retained same-fence result, otherwise empty/`nil`, with no placeholders
or mixed tree. Identical in-flight queries coalesce. An explicit repeat after
`Deferred` may retry; ordinary reads are not retries.

Accepted publication installs the complete normalized native projection
atomically and retains it only under an exact semantic plus native
coordinate/cardinality fence. `DataUnavailable`/`Deferred` without exact
fallback expose empty/baseline; terminal failures clear virtual native
publication; stale/cancelled results are inert. A changed visible state posts
exactly one `NSAccessibilityLayoutChangedNotification` after complete state is
queryable on the main thread. Unchanged, pending, stale, cancelled, and rejected
work posts no layout notification. Retired custom objects follow the
`UIElementDestroyed` notification lifecycle.

This extension preserves the one-session bound, opaque private handles, explicit
refresh/retry-only demand, one range plus one required-item slot, 64
registrations, 1024 per-query and aggregate caps, one provider call per
container/attempt, exact publication/fallback, `materialized = false`,
normalized logical conservative coordinates, and pure snapshots. It excludes
native focus transfer and virtual/provider focus exposure, native actions for
virtual/provider targets, selection mutation, scroll/materialize, scheduler/retry policy, render,
product, direct native custom-resolver invocation/reconstruction, Wayland/Windows,
auxiliary, multi-consumer, and public registry behavior.

This contract is limited to the private primary-window macOS/AppKit consumer.
Automated AppKit boundary evidence remains shipped, covering projection
construction, supported host attachment, exact-root readback, failure cleanup,
and symmetric retirement. Exact fresh-bundle activated Computer Use/AppKit
evidence verifies discoverability and numeric action, bounded set-value, and
restart acceptance for this bounded primary-window consumer. VoiceOver-specific
acceptance remains unperformed. Repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible. Wayland,
Windows, non-qualified/virtual native actions, new native AX
native focus setter/transfer or focus exposure beyond the ordinary materialized-target
contract, scrolling, product policy,
direct native custom-resolver invocation/reconstruction, scheduler, and renderer
behavior remain excluded.

## Declarative Effect Ownership And Lifecycle Seam

The current effect-ownership seam is intentionally narrower than the target
model. Declarative lowering and traversal under `src/application` construct
`ViewNode`/`SurfaceNode` projections, preserve eligible overlay and keyed-node
source provenance, and route application messages. `src/runtime/controller/commands/dispatch.rs`
therefore enters ordinary work with the private `Application` origin unless an
existing auxiliary path or an explicit declarative owner consumer supplies a
live declarative origin. The private `EffectOrigin` model in
`src/runtime/controller/owner.rs` now distinguishes `Application`, `Auxiliary`
generations, and live declarative tokens.

Current shipped ownership is narrower than this target model: the private
`EffectOrigin` boundary and application-owned `ResourceTasks` are the current
ownership split; `runtime/effects` is not complete. The remaining
effect-ownership boundaries are future work tracked by OPT-1387, OPT-1390,
and OPT-1370.

That private auxiliary generation is already carried through the existing
worker, timer, and platform-completion registries in
`src/runtime/controller/effects.rs`, `timers.rs`, `platform.rs`, and `host.rs`,
including chained commands. Retirement fences only the matching generation;
it does not transfer ownership to the declarative tree or split the shared
ingress. The declarative timer, one-shot worker, cancellable ordinary owner
one-shot, application-owned
`KeyedLatestTasks` owner one-shot and ordered stream, ordinary ordered and
coalesced owner-scoped stream consumers, the cancellable ordinary ordered and
coalesced owner streams, the cancellable latest-task one-shot and
ordered/coalesced owner streams, ordered/coalesced latest-task owner streams,
and coalesced
keyed-latest owner streams reuse this same
registry/lifecycle seam; broader platform and shared-resource ownership remain
deferred.

The private declarative seam has five dependency-ordered stages. Generic
matching registry retirement is now shipped at the accepted projection
boundary; the bounded explicit timer, one-shot owner-worker, cancellable ordinary
owner one-shot, application-owned
`KeyedLatestTasks` owner one-shot and ordered stream, owner-scoped latest
one-shot worker, ordinary ordered and coalesced owner-scoped stream consumers,
the cancellable ordinary ordered and coalesced owner streams, the cancellable
latest-task one-shot and ordered/coalesced owner streams,
ordered/coalesced latest-task owner streams, and coalesced keyed-latest owner
streams are shipped; `ResourceTasks` ownership,
platform ownership, and broader product-facing
demand/refresh/provider ownership remain deferred:

1. Declarative lowering and traversal preserve crate-private source metadata
   alongside stable identity. The metadata may record independent eligible
   overlay and keyed-node candidates and compatibility context. The bounded
   public `DeclarativeEffectOwner` marker, `UiUpdateContext` owner-timer
   methods, and qualified one-shot/`KeyedLatestTasks`/ordinary
   ordered/coalesced/cancellable-owner-one-shot/cancellable-ordered/cancellable-coalesced/ordered-latest/coalesced-latest/keyed-ordered/keyed-coalesced owner-worker
   methods are now
   exposed, while runtime origin and effect
   payloads remain private. A dynamic
   unkeyed node cannot supply durable owner identity and therefore cannot be an
   implicit cancellation target.
2. The accepted declarative projection projects those candidates to the
   controller. A source location remains only an eligible context; explicit
   owner selection is required. Overlay and keyed-node candidates have no
   implicit precedence, and ordinary primary-surface work remains
   application-owned by default. Shared `ResourceTasks` remain application
   ownership even when an overlay or keyed node consumes their interest.
3. The controller owns the private owner-generation ledger and reconciles exact
   identity/kind/generation continuity. Compatible reprojection and keyed
   reorder preserve a generation. Removal and incompatible replacement retire
   the exact old generation; reinsertion receives a fresh one; sibling owners
   remain isolated. If removal and effect emission occur in one accepted update,
   owner-scoped work is rejected before registration, while explicitly
   application-owned/outlive work may continue.

Owner-scoped timer and one-shot worker admission refresh the accepted surface
before registration and reject absent, ambiguous, ineligible, stale, retired,
or incompatible handles without fallback. Latest worker admission, including
the application-owned `KeyedLatestTasks` owner route, additionally restores its
eligible predecessor ticket on any failed owner or host admission; keyed rollback
is isolated to the affected key.

The shipped coalesced keyed-latest owner stream retains the exact host key,
keyed ticket and replacement transaction, declarative owner generation, and
admission receipt. Its bounded ingress retains only the newest pending
intermediate payload before UI drain; the final remains uncoalesced and maps
exactly once after that retained event. Event and final mappers receive exact
`KeyedTaskCompletion<Key, Event>` or `KeyedTaskCompletion<Key, Output>` values
and remain UI-local/non-`Send`. Keyed supersession and owner retirement
independently fence worker execution, mapping, and reduction. Invalid, removed,
ambiguous, unkeyed, incompatible, stale, host, capacity, closing, and
same-update admissions fail closed without `Application` fallback and restore
only the affected key's eligible predecessor; sibling keys remain unchanged.

The shipped cancellable ordinary ordered and coalesced owner streams use the same
accepted-surface owner generation, worker registry, and admission receipt as the
ordinary stream routes. The coalesced route retains one pending intermediate
payload and one queued marker before UI drain, replaces older pending events,
records the existing coalescing diagnostic, and delivers the uncoalesced final
exactly once after the retained event; events separated by a UI drain map
separately. The explicit token and declarative owner probes are OR-composed
fences for cooperative work, event/final delivery, mapping, and reduction, and
the admission receipt remains admission-only. Event and final mappers remain
UI-local/non-`Send`. Invalid, removed, ambiguous, unkeyed, incompatible, stale,
same-update, host, capacity, and closing admissions fail closed without spawn,
mapping, retry, or `Application` fallback.

The shipped cancellable ordinary owner one-shot uses the same accepted-surface
owner generation, worker registry, and admission receipt as the ordinary
one-shot. Its explicit token and declarative owner probes are OR-composed fences
for cooperative work, deferred mapping, and reduction. Only the
token-cancellable owner one-shot defers mapping until UI drain; application-owned
and non-cancellable owner one-shots remain eager. The receipt is admission-only,
and its UI-local mapper need not be `Send` or `Sync`. Invalid, removed,
ambiguous, unkeyed, incompatible, stale, same-update, host, capacity, and
closing admissions fail closed without spawn, mapping, retry, or `Application`
fallback.

The cancellable latest-task owner routes reuse the existing latest ticket and
replacement transaction and compose the explicit token with the declarative
owner-generation fence for cooperative work, delivery, mapping, and reduction.
The one-shot maps one completion; the ordered stream preserves FIFO events and
final delivery; the coalesced stream keeps only the newest pending intermediate
event before UI drain and delivers the final uncoalesced. Each receipt is
admission-only and the UI-local mappers remain non-`Send` where permitted.
Invalid, removed, ambiguous, unkeyed, incompatible, stale, same-update, host,
capacity, and closing admissions fail closed without spawn, mapping, retry, or
`Application` fallback; failed latest admission restores the predecessor ticket.

4. The existing timer and worker registries carry the explicitly selected owner
   origin for the bounded owner-timer, one-shot owner-worker, cancellable
   ordinary owner one-shot, application-owned
   `KeyedLatestTasks` owner one-shot and ordered stream, owner-scoped latest
   one-shot worker, ordinary ordered/coalesced stream consumers,
   ordered/coalesced latest-task owner streams, coalesced keyed-latest owner
   streams, the cancellable latest-task one-shot and ordered/coalesced owner
   streams, and the cancellable ordinary ordered and coalesced owner streams. Its explicit token
   probe is OR-composed with transaction and declarative-owner probes; token
   cancellation and owner retirement independently fence cooperative work and
   queued delivery. `ResourceTasks` ownership, platform
   ownership, and product wiring remain deferred. The existing registries
   remain the admission
   and mapping points; they do not acquire separate per-owner queues or a second
   lifecycle authority. Recovery and cached hiding preserve a retained live
   generation unless an explicit close/removal or incompatible replacement
   retires it.
5. Matching registrations are retired at their owning registry, and every late
   completion, wake, result, or chained command is rejected before its mapper
   runs and before message reduction. Exact retirement must not cancel sibling,
   application-owned, or later same-identity generations.

`AppBridge`/the shared ingress and `RuntimeLifecycleController` remain the
global admission and lifecycle authorities throughout this seam. Owner
metadata, reconciliation, and registry retirement must respect the existing
`Accepting -> Closing -> Stopped` boundary and the current recovery path; they
must not add a per-owner event loop or bypass lifecycle vetoes. The seam defines
ownership identity, admission, and retirement only. It does not implement or
override the separately normative scheduler contract in
`docs/DESIGN_DIRECTION.md` (`Next scheduler policy contract`), including queue
capacity, budgets, fairness, priority, wake ordering, and stage ordering.
Overlay/keyed-node cancellation is an implementation-sequencing dependency for
completing this seam, not permission to define scheduler policy later.

Widget-local interaction retirement follows the same ownership boundary without
adding another runtime queue. During surface reconciliation, the installed
stateful widgets are visited in prior traversal order and receive an exact
compatible successor only when the identity/path/revision evidence is unique.
The retiring widget owns local teardown and can return a UI-local terminal
`WidgetOutput`; the old `SurfaceWidget` mapper translates it before the old
surface is discarded. The bounded mapped batch is delivered through deferred
controller dispatch after installation, while compatible unchanged widgets use
the existing state synchronization hook. Missing or ambiguous evidence is
conservative and never transfers authority, and successor objects are not
retained.

## Rendering Boundary

Radiant uses Vello for normal UI primitives and direct WGPU paths for retained
GPU surfaces where dense realtime rendering benefits from custom GPU resources.
The public application model should not split into separate "Vello apps" and
"WGPU apps". A custom GPU surface is still a Radiant widget/surface: it
participates in layout, input routing, paint planning, diagnostics, and normal
runtime invalidation.

Current shipped boundary: `RenderCanvas` is a compatibility alias over
`GpuSurface` vocabulary and emits `PaintPrimitive::GpuSurface`.
`CanvasProgram`/`CanvasGraph` remains future work: OPT-1407 owns the
compatibility decision and OPT-1408 owns the implementation. This boundary
does not choose a new RenderCanvas compatibility or deprecation policy.

Built-in GPU-surface payloads cover atlas and signal rendering. Advanced shader
surfaces use a backend-neutral custom shader descriptor for stable shader
identity, optional WGSL source, explicit vertex/fragment entry points, and
opaque payload bytes. The native WGPU adapter executes the source-backed subset
with Radiant's surface-uniform ABI plus optional app uniform and read-only
storage payload bindings; descriptors missing the required shader handoff
report that through frame diagnostics instead of introducing a parallel
application-facing WGPU API. The same diagnostics separate rendered custom
shader surfaces, pipeline rebuilds, bind-group rebuilds, and bind-group cache
hits so native shader setup is visible without exposing raw WGPU handles.
Validation errors from shader modules, render pipelines, and bind groups are
counted separately from unsupported descriptors and logged through tracing with
the backend error message.

Backend-neutral paint plans live under `src/runtime/paint`. Native Vello scene
construction, retained scene caching, post-GPU overlays, GPU-surface pipelines,
and frame presentation live under
`src/gui_runtime/native_vello/generic_runtime`. WGPU-specific details should
stay there or behind explicit GPU-surface contracts, not leak into normal
application-builder code.

Frame profiling and GPU timing remain separate observability contracts. The
public `on_frame_gpu_timing` callback carries a correlated
`FrameGpuTimingSample` whose target aggregate interval runs from the first
frame-owned GPU command through final composition, excluding CPU present and
display/scanout. The existing `FrameProfile` callback remains one delivery per
successful present, with its existing semantics unchanged. The generic native
native runner implements the producer privately per native window: it negotiates
the paired timestamp features, submits a standalone start marker before
frame-owned GPU work, resolves/copies the end marker asynchronously, and harvests
an independent fixed four-slot pool on the event loop before capability delivery.
This path is enabled only by that window's frame profiling plus the opted-in
observer. Auxiliary completion routes carry the exact window key/identity,
adapter generation, resource identity, slot, and token fences; callbacks remain
wake-only, while the parent event loop performs delivery and recycling through the
existing auxiliary diagnostics/profile handoff and ordering boundary. Mismatched,
stale, duplicate, or lifecycle-invalid completions publish nothing. Device-loss/
recovery or shutdown may conservatively cancel pending timing under the existing
bounded lifecycle behavior.

The generic native Vello runtime has one event-loop-confined adapter owner per
application run. The primary window selects the shared WGPU context, device,
queue, and device-loss callback witness; the owner publishes crate-private
monotonic adapter-generation evidence with that selection, while
`NativeTargetGeneration` remains per-window. Auxiliary windows borrow that
owner and create only compatible surfaces and per-window renderers. Auxiliary
`NativeGpuBackend::Auto` inherits the selected primary backend, while an
explicit auxiliary policy must be compatible with it and the selected adapter
must support the child surface. Auxiliary child runners route and present their
own projected surface, but the parent runner owns auxiliary projection and
synchronization. An accepted current-generation device loss moves the private
native lifecycle through `Recovering`; one fresh adapter/device candidate is
prepared off the event loop from an empty render context and committed only as
a complete primary bundle against the existing window. Old bundles remain in
the bounded completion-witness quarantine, while visible auxiliaries rebuild
lazily one per event-loop opportunity and cached hidden auxiliaries rebuild
before show. Any recovery publication or reconstruction veto returns through
the existing bounded `Closing` policy.

The parent `AboutToWait` owner observes the primary and each eligible visible
auxiliary runner's existing timed-repaint, caret, animation, and pending-redraw
deadlines. It admits at most one newly due window per scheduler turn using
stable window keys, then composes the selected cadence with the existing
activation and maintenance deadlines through Winit's `Wait`/`WaitUntil`
control flow; hard `Recovering` and `Closing` precedence retains their
existing deadlines. Hidden or cached, retiring, recovering,
resource-incomplete, stale-generation, and stopped auxiliaries remain dormant;
the scheduler does not render, create GPU work, or introduce a second wake
mechanism.

A `FrameRender` returned by the narrow Vello `render_to_texture` boundary has a
separate bounded per-window reconstruction path. After the failed redraw has
returned, the path preflights the current adapter generation, window identity,
lifecycle, and complete-bundle publication capacity, then constructs a fresh
surface, renderer, GPU state, and exact-generation completion witness against
the shared selected device. Publication moves the old complete bundle into the
bounded quarantine before target/frame/scene invalidation and one fresh redraw.
The same contract applies to auxiliary windows; successful reconstruction is
internal, while a veto, candidate failure, or repeated same-generation failure
enters `Closing` with the original `FrameRender` cause.

Environment-aware widget paint is additive and remains in the core contract:
`ResolvedEnvironment` is a lossless copyable projection of the current
`WindowEnvironment`, and `WidgetPaintContext` borrows the existing bounds,
layout, and theme inputs while carrying that value. Surface traversal derives
one projection per plan and preserves it through clipped descendants and
runtime overlays. Context-aware hooks default to one-call delegation to the
legacy hooks, preserving object safety and existing callers.

Appearance policy is layered above that environment projection. The
backend-neutral `AppearancePolicy` either follows the snapshot or fixes an
explicit `ThemeTokens` value; `ResolvedAppearance` is immutable frame data
carried additively beside the unchanged theme reference. Native rendering
resolves it once per paint pass and reuses it for clear color, base traversal,
clips, and runtime overlays. Unknown scheme values use dark tokens only for
appearance selection, while `ResolvedEnvironment::color_scheme()` remains
`None`; scale and reduced-motion values never affect token selection.

The native compositor preprocesses each paint plan's clip state and opaque
suffix coverage once into a reusable spatial index. GPU rendering,
interaction-region projection, post-GPU overlay visibility, and embedded
unsupported-surface validation query that shared plan instead of reconstructing
clip stacks or rescanning primitive suffixes per surface. The planner and query
scratch buffers persist across frames so steady-state planning does not require
recurring heap allocation.

Frame cadence, invalidation, and render reuse are separate responsibilities.
Native runtimes should be able to maintain a steady 60Hz presentation cadence
while rebuilding only the work invalidated by host state, layout, paint, text,
retained-surface revision, GPU payload, or transient-overlay changes.
`src/runtime` owns the backend-neutral invalidation and repaint-scope contract;
`src/gui_runtime/native_vello/generic_runtime` owns the native scheduling,
cached base-scene presentation, retained GPU-surface reuse, and frame-diagnostic
counters that prove stable frames avoid unnecessary reprojection, scene
encoding, GPU upload, or text/layout cache churn.

### Private staged-refresh consumer boundary (normative evidence contract)

Only one externally visible complete frame exists: the
`CommittedFrameState`/last-complete frame. An invisible private
`PreparedSurfaceRefresh` candidate may contain candidate surface, traversal,
source projection, layout root, view-delta decision, candidate layout,
candidate paint plan, damage, and timing evidence. Candidate preparation may
mutate candidate-owned storage only. It must not mutate active
focus/capture/composition/wheel ownership, the declarative owner,
accessibility/automation projection, active layout, retiring-widget ownership,
or the last-complete frame.

Immediately before irreversible replacement cleanup, the consumer revalidates
runtime identity, lifecycle-transition generation, active-surface generation,
layout-state generation, viewport, window environment, requested refresh
revision, and the existing native window, adapter, target, stage, owner, and
revision fences. Any mismatch, stale generation, lifecycle transition,
resize/recovery, newer visual work, unsupported/ambiguous/incomplete evidence,
or failure before commit drops the candidate with no active mutation, callback,
terminal message, or presentation and retains the combined correctness-first
fallback. After validation, irreversible replacement cleanup happens once,
complete candidate state is atomically published, and terminal messages are
dispatched only afterward. No scheduler yield occurs after cleanup begins. A
panic after that point is terminal recovery/shutdown, not rollback.

The prepared refresh path is the first private production Projection-stage
consumer of the safe-boundary owner. After candidate-local preparation passes
currentness, it completes the Projection ticket, admits/checks one exact
private Layout ticket, and completes Layout before admitting/checking one exact
private PaintPlan ticket immediately before irreversible runtime publication;
the PaintPlan ticket completes after the candidate publication call returns.
This remains one synchronous candidate transaction with no scheduler yield: it
does not claim independently scheduled Reconciliation/Layout/Paint, a public
API, a second event loop, or cross-window policy. Projection remains the
no-replay boundary; a later Layout or PaintPlan veto drops the inert candidate
without re-entering the combined path. The existing combined path remains
authoritative for virtual materialization and unsupported paths.
`WindowStageOwner` now admits private Deadline work plus this synchronous
Projection-to-Layout-to-PaintPlan handoff. Diagnostics and timing remain
observational and non-authoritative.

Current shipped boundary: prepared refresh constructs the Projection, layout,
and paint-plan candidate synchronously, then uses a later no-yield publication
gate. Independently schedulable Reconciliation, Layout, and Paint stages remain
future work under OPT-1389.

### Native visual request packet handoff (private native-window contract)

The native event loop has one crate-private `NativeVisualRequestPacket` handoff
per window. The packet is deliberately non-`Clone` and carries only the exact
Winit `WindowId`, a checked non-wrapping `NativeVisualOwnerGeneration`, a
checked non-wrapping `NativeVisualRevision`, and a private observational origin:
`ScheduledOrRuntime` (which records `FrameWork`) or
`NativeInvalidationFallback`. It is deliberately target-unbound and
adapter-unbound: current adapter and target generations are eligibility
evidence at `RedrawRequested`, not packet identity. `FrameWork` remains
diagnostic evidence; it does not authorize rendering, scene reuse, partial
work, or presentation.

Each window owns a fixed mailbox with `requested`, `consuming`, and `pending`
state, with at most two retained packet owners. `requested` and `consuming` are
mutually exclusive. The requested state is the packet for the outstanding
Winit redraw; an offer while `requested` is occupied replaces it with the
newest revision and emits no additional Winit wakeup. `RedrawRequested` moves
that packet to the consuming owner; only an offer while `consuming` is occupied
uses or replaces the one newest `pending` successor. The private
`NativeVisualRequestAdapter` is the only caller of raw
`Window::request_redraw`. All other runtime paths record `FrameWork` and enqueue
or reissue a packet. A reversible `suspended` state is distinct from retire: it
rejects enqueue and unsolicited fallback, clears packet ownership and wake
timing, advances the owner generation, and survives invalidation or native
resource recovery until an explicit resume. Auxiliary hide/cache dormancy
suspends the mailbox; inactive recovery rebuilds state without enqueue, an
inactive `RedrawRequested` reasserts dormancy, and show resumes the mailbox and
issues exactly one fresh latest-state packet.

Only a `WindowEvent::RedrawRequested` boundary may begin the existing redraw
kernel and reach presentation. Eligibility is a crate-private logical
presentation-capability predicate: an initialized primary is eligible even
while its host visibility is initially hidden or unknown, while an auxiliary
also requires `active` and `admitted`. Neither scheduling nor admission may
call or infer authority from Winit `is_visible()`. Every path still requires
the exact live `WindowId`/owner, a complete current adapter resource bundle,
running lifecycle, no recovery/closing/stopped state, and an ordinary known,
unfenced target. Runtime and scene dirty state remains recorded even when no
packet is admissible. Ordinary offers and scheduler demand use only that local
predicate; exact current adapter-resource validation remains at the redraw
callback. A requested packet whose begin eligibility fails returns the private
`RequestedVetoed` outcome, clears the requested packet and associated pending
packet, advances the owner generation, clears wake timing/stale state and the
recovery exception, and performs no redraw, completion, or fallback. A
`WrongWindow` result preserves the current packet; with no requested packet the
result is `Ineligible`. Missing adapter generation follows `RequestedVetoed`
when a requested packet exists rather than returning early. Requested packets
may additionally cross a temporary target fence only for a validated nonzero
pending resize that can restore the target or the existing one-shot `Other`
recovery permit; an unsolicited fallback is restricted to ordinary eligibility.
Scheduler demand exists only for ordinary eligibility or an outstanding
requested packet with a valid recovery exception. Exhausted fenced state has no
cadence/repaint demand, retry deadline, or `frame_wait` reinsertion until an
explicit rearm. For the primary scheduler, retry deadlines, and `frame_wait`,
the stored adapter owner must exist and expose the exact generation in the
active resource bundle; missing or mismatched evidence is quiescent and does
not reinsert work. Packet offering remains adapter-unbound so initialization
and recovery may enqueue while holding the adapter externally; auxiliary
scheduler admission remains parent-generation-authoritative. A primary
`RedrawRequested` callback with no stored adapter owner is a hard
`RequestedVetoed` transition: it clears requested and pending ownership,
stale wake timing, and the recovery exception, advances the owner when packet
work existed, and performs no redraw, finish, fallback, or diagnostics. With
no packet it only clears inconsistent wake/recovery state. After
resize/acquisition and before scene work, the current target must again be
known and unfenced. Hide/cache dormancy, close/retire, owner replacement,
recovery/loss, and resource isolation retain their explicit invalidation
fences.

Radiant stores desired visibility in crate-private
`logical_window_visible`, separate from physical Winit application. An explicit
policy update changes that desired state and applies it only while the local
lifecycle is running. Recovery, loss, and closing physically conceal the
window without changing desired state; successful renderer/device publication
reapplies the latest desired state, while failed recovery remains physically
hidden. Initial hidden and explicitly hidden windows remain hidden. Visibility
intent received during recovery updates desired state and is applied only after
successful publication. No path reads Winit visibility back.

A stale owner generation or revision drops the packet and does not fall back
into rendering. The redraw kernel returns one private typed disposition:
`Presented`, `RetrySamePacket`, or `DropPacket` (with the existing renderer
failure result kept separate). `Timeout` may return `RetrySamePacket` only
within its bounded nonzero-size permit; exhausted/zero-size timeout drops.
Nonzero `Lost`/`Outdated` invalidates and reconfigures, then may enqueue a fresh
policy-authorized requested packet; zero-size variants drop/defer. `Other` may
use at most one fresh requested recovery packet, never an unsolicited
fallback; out-of-memory, renderer failure, pre/post-acquire veto, missing
device, and no-submission paths drop. A completion promotes the newest pending
successor; when a retry sees a pending successor, that newer packet wins, and
only an empty pending slot retries the exact consuming identity. Offers allocate
their newer revision before any stale-wake reissue, and route-end flushing
rechecks current mailbox state and timestamp. Primary and auxiliary windows use
the same typed begin/redraw/finish kernel and parent-owned observational frame
evidence.

Hide/cache, close/retire, `WindowId` replacement, adapter/device recovery or
loss, and native resource isolation/replacement clear every mailbox state and
advance the owner generation. Ordinary deferred resize and its
target-generation advancement do not clear the mailbox: the packet claimed
for that redraw survives its own target transition. Owner and request
revisions are checked, start at one, and fail closed at exhaustion; neither
wraps or is reused. Primary and auxiliary windows use the same begin/finish
kernel and the same parent-owned observational frame evidence.

### Native encode/present snapshot ticket (private native-window contract)

After deferred Deadline drain, route/input coalescing, deferred resize,
scene admission/reuse, prepared-refresh terminal dispatch, auxiliary sync, and
transient overlay work complete, the native kernel stages volatile GPU
presentation updates into the existing fixed 32-slot mailbox. Staging is
two-phase: an acquisition or lifecycle/renderer veto aborts the snapshot and
retains every selected update; successful presentation commits only the exact
selected presentation revisions, so a newer update admitted while the snapshot
is in flight remains pending for the next frame. The steady-state path uses
the existing preallocated storage and never makes the update mailbox a dirty
bit or render-policy authority.

Immediately before `get_current_texture`, the window's `WindowStageOwner`
admits one crate-private non-`Clone` `NativeEncodePresentTicket`. It binds the
exact stage-owner identity and revision, consuming `NativeVisualRequestPacket`
identity, current adapter/resource generation, post-resize target generation,
running lifecycle, finalized direct-resize or composited path, and a checked
non-wrapping `NativeFrameSnapshotRevision`. No pending visual successor vetoes
this complete snapshot unless lifecycle, resource, or target evidence changes.
Every `get_current_texture`, surface-view creation, scene render, GPU encode,
submit, and present operation is behind that exact current ticket. Each
post-admission veto or failure consumes the exact ticket once without replay;
a wrong ticket preserves the real owner. Direct-resize and composited paths
share the same kernel and success-only completion gate.

Only exact successful completion advances frame sequence and publishes frame
diagnostics/profile, input-to-present latency, last-present timing, and first
reveal. Surface loss, timeout, out-of-memory, renderer failure, lifecycle
transition, resource loss, and target transition preserve the existing
recovery matrix and abort the volatile snapshot. A same-packet retry prepares a
fresh snapshot and fresh ticket. This remains crate-private and does not add a
public API/schema, dirty-bit suppression, renderer retention, render thread,
budget, timestamp, or platform claim.

### Native submission maintenance ticket (private native-window contract)

For normal `Running` native submission maintenance, after the parent scheduler
selects one exact stable window key, that window's `WindowStageOwner` admits
one crate-private non-`Clone` `MaintenanceStageTicket`. The ticket binds the
exact stable window key, the current window-owner adapter and target
generations, the exact active or quarantine resource slot, and the exact
`NativeSubmissionCompletionIdentity` witness. The owner adapter/target
generations fence the `FrameStageIdentity`; the bound resource generation is
independent and may be older for a quarantine slot. Admission requires a
known/valid owner fence, a known resource generation, and a completion witness
with that same resource generation; it does not require resource generation to
equal the current adapter generation. An inactive cached auxiliary may maintain
exact resources but stays visually dormant and cannot present. For one key with
jointly due visual and maintenance work, the first opportunity selects visual;
only after a visual `Deadline` bundle completes successfully while maintenance
remains due does the next same-key fairness-eligible opportunity select bounded
`Maintenance`, even if visual work remains due. An owed same-key maintenance
opportunity remains below another key's `Deadline`.

The ticket executes one bounded nonblocking unit with at most one device
`PollType::Poll`, one completion callback observation/rearm, and one exact
quarantined-resource removal. `NeverSubmitted` quarantine retires only its
exact slot; submitted retirement requires the exact callback; coalesced
completion requires the rearm and second callback; indeterminate completion
remains fenced until exact callback evidence. If work remains, scheduling
records only a bounded future maintenance deadline. Missing, unknown, wrong,
stale, exhausted, fenced, non-running, conflicting, or ambiguous key,
generation, target, slot, or witness evidence is inert: it performs no poll,
rearm, removal, redraw, present, or broad fallback scan, and preserves the
resources and real owner while demand is recomputed. Completion user events
are wake-only and do not perform maintenance or request unrelated `FrameWork`.
This private ticket path is only for normal `Running` submission maintenance;
startup/preflight, recovery, `Closing`, and already-retiring auxiliary cleanup
retain lifecycle-authoritative bounded paths and reuse the same witness
transition kernel.

### Native lifecycle admission ticket (private native-window contract)

An accepted current-generation device-loss callback is a per-window
`SchedulerStage::Lifecycle` operation before it is a recovery-resource
operation. Before any native or controller lifecycle phase changes from
`Running` to `Recovering`, the primary window and every admitted auxiliary
window stage one exact non-`Clone` `LifecycleStageTicket`. The shared
`WindowStageOwner` precomputes checked owner-generation and revision evidence,
atomically retires all lower-stage pending, in-flight, and completion state,
then installs one exact `Lifecycle` identity and owner token. Lower stages are
blocked until that exact ticket completes or is vetoed, and all older
lower-stage tickets are stale.

The private native ticket binds `BeginDeviceRecovery`, source phase `Running`,
the exact stable key and Winit `WindowId` presence, the accepted shared adapter
generation, the exact active-resource generation or absence, the exact
`NativeTargetGeneration` state (including unknown), the target-fenced state,
and the underlying lifecycle ticket. Unknown or absent evidence is exact
evidence; admission does not use ordinary `prepare_fence` or require a usable
post-transition adapter or target. A wrong completion or veto preserves the
real owner; an exact veto preserves the advanced fence.

The existing current registration/generation and publication-capacity
preflight remains authoritative. The callback registration and generation are
rechecked immediately before staging, then the complete primary-plus-
auxiliary ticket set is revalidated synchronously without a yield. Only after
that validation do the existing native/controller recovery hooks apply their
presentation, mailbox, target, diagnostic, and fairness fences; every exact
ticket completes before the existing one-candidate recovery worker starts.
Any staging, currentness, transition, or completion failure vetoes the staged
set, starts no recovery candidate, and converges once through the existing
bounded `Closing` path with the original loss cause. A stale registration or
generation remains inert, visibility and dormancy intent are unchanged, and
recovery/closing maintenance retains its existing bounded authority. This is
crate-private and does not add a public API, schema, queue, worker, or event
loop.

The terminal-only `BeginClosing` admission uses the same private lifecycle
stage owner for whole-run shutdown. The primary and every resident auxiliary
child whose native phase is `Running` or `Recovering` are staged in stable
primary-then-resident-vector order, including active, cached, unmaterialized,
and recovery-pending children whose wrapper is `Admitted` or `Retiring`.
Children already in native `Closing` or `Stopped` are skipped, and a repeated
primary `Closing` or `Stopped` request is inert. Each exact non-`Clone` ticket
binds the stable key, source phase, optional Winit identity, optional adapter
generation, optional active-resource generation, exact target generation
(including unknown), and target-fenced state. Absent adapter evidence is
represented by `None`; `Some(unknown)` is invalid. The terminal owner admission
encodes absent adapter evidence as an unknown lifecycle identity while
retiring all lower-stage state and advancing the owner once; existing recovery
admissions still require a known adapter, and lower-stage admissions retain
their known adapter and target requirements.

The complete ticket set is revalidated without a yield before any native,
controller, presentation, wrapper, cause, recovery, mailbox, or resource
mutation. After validation, existing per-window Closing preparation applies:
recovery-cause precedence and the first terminal cause are preserved,
recovery is cancelled, native and controller phases enter `Closing`, native
accessibility closes, presentation and mailboxes are fenced, fairness,
reopen, and wake state is cleared, and auxiliary wrappers retire without
dispatching close messages. Each exact ticket completes only after its
window's Closing fences. Only then does unchanged bounded resource retirement,
scheduling, and event-loop exit run. Any staging/currentness fault vetoes its
staged tickets once; before terminal intent, failure to admit the primary or an
eligible auxiliary, complete-set currentness failure, or primary Closing
preparation rejection returns inert without cause, recovery, controller,
native, resource, wrapper, visibility, budget, or event-loop mutation, without
ticket-free convergence or automatic retry. A later independent event may
make a fresh admission. Primary native Closing admission occurs before
cause/recovery mutation, preserving recovering-cause precedence even when
preparation is rejected. After primary Closing preparation succeeds, any
auxiliary-preparation, stage-owner, or completion fault converges one-way
through bounded Closing without a cause argument; it requires the primary
already Closing and preserves the first/original cause, without ticket
retry/replay, redraw, visibility restore, or lower-stage execution.
That fallback first invalidates each resident stage owner once, clearing any
surviving lower or lifecycle work and making its tickets stale before bounded
Closing continues.

An independently requested destructive close of a non-cached auxiliary is a
separate child-local `BeginClosing` transaction. Cached `CloseRequested` remains
a hide/reuse path and consumes no lifecycle ticket. For a non-cached admitted
child, the event boundary stages exactly one non-`Clone` ticket carrying the
child key, source phase (`Running` or `Recovering`), optional Winit identity,
optional adapter and active-resource generations, exact target generation
(including unknown), and target-fenced state. The ticket and exact live
`AuxiliaryWindowOwner` are carried to the parent lifecycle route; the parent
revalidates both without a yield before any child, owner, message, wrapper,
recovery, presentation, mailbox, lower-stage, or resource mutation.

Only after that preflight does the child cancel its own recovery while
preserving the original child recovery cause, enter native and controller
`Closing`, fence accessibility, presentation, mailboxes, fairness, wake, and
lower-stage work, and transition its wrapper to `Retiring`. The parent retires
that exact owner generation before dispatching the app-owned close message,
then completes the exact child ticket after all local fences. A missing close
message follows the same accepted path and still receives bounded resource
retirement. A pre-terminal admission, evidence, owner, currentness, or
preparation failure vetoes the staged ticket once, retains the message, and is
inert; a post-terminal owner or completion fault invalidates only the child
stage owner, converges locally, and emits the already-committed close message
exactly once without replay, parent or sibling mutation, or whole-run
shutdown. Retiring removal suppresses same-key recreation through that sync
turn; a later independent sync may recreate the projection. This private
transaction does not widen the whole-run, `DiscreteInput`, or
`ImmediateTransient` contracts and adds no public API.

The parent owns a distinct retiring-auxiliary maintenance deadline on the
canonical 16 ms native-resource cadence. An accepted destructive close arms
that deadline due immediately, even when there is no close message or event
proxy. In `Running` `AboutToWait`, one shared `NativeResourceMaintenanceTurn`
may advance retiring auxiliary children only, preserving the existing one-drop
budget; it rearms to `now + 16 ms` while any child remains and clears the
deadline when none remain. The exact retirement deadline is composed into the
parent `FrameScheduleDeadlines` and `WaitUntil` plan even with zero window
demand. When that retirement opportunity is due, a normal maintenance-stage
ticket is skipped for that opportunity but remains due for the next one.
`NativeResourceMaintenanceRequested` remains wake-only: it may accelerate an
existing retiring deadline to now, but performs no poll, sync, removal, or
message dispatch. `Recovering` and `Closing` retain their existing lifecycle
authorities; a return to `Running` arms due-now cleanup when a retiring child
remains, while `Stopped` clears the deadline. A removal marks deferred sync,
and same-key recreation waits for a later independent sync boundary.

### Native DiscreteInput stage contract

The native event-loop owner admits exactly one non-`Clone` `DiscreteInput`
ticket for each `MouseInput`, `KeyboardInput`, `ModifiersChanged`, or `Ime`
event. Primary and auxiliary windows use the same private stage owner and the
same route boundary. The input timestamp is captured at event arrival before
any forced exact deferred-`Deadline` terminal drainage. If a deferred
`Deadline` ticket is present, that exact owner is drained and completed before
input admission; input is never bypassed, coalesced, replayed, or queued as an
alternative.

Admission binds the stable window key, live native `WindowId`, active native
resource generation matching the adapter, known unfenced target, running
nonterminal native-window eligibility, event kind, and captured timestamp.
Auxiliary admission also binds caller-supplied active, admitted, materialized
wrapper eligibility; inactive, cached, retiring, or unmaterialized children
remain inert. Exact owner and native evidence currentness are revalidated
immediately before routing; a pre-route veto is inert and does not route or
apply lower-stage work. The ticket remains live through synchronous native
routing and message reduction, and completes before lower-stage route-outcome
work such as timed drainage, refresh, redraw, auxiliary synchronization, or
wakeup. A post-route completion mismatch never reroutes, replays, or applies a
lower-stage fallback. A Lifecycle admission may stale the input ticket; the
stale route remains terminal and is not applied.

This slice covers only `MouseInput`, `KeyboardInput`, `ModifiersChanged`, and
`Ime`. Cursor, wheel, focus and cursor-boundary, file/platform/lifecycle,
resize, redraw, and `ImmediateTransient` behavior remain outside this
contract.

### Native ImmediateTransient stage contract

The native event-loop owner admits exactly one non-`Clone` `ImmediateTransient`
ticket for each `Focused(false)`, `Focused(true)`, `CursorEntered`,
`CursorMoved`, `CursorLeft`, or `MouseWheel` event. Primary and auxiliary
windows use the same private stage owner and route boundary; a wheel ticket
retains the exact native `TouchPhase`. The event timestamp is captured at
native arrival before any forced exact deferred-`Deadline` terminal drainage.
If a deferred `Deadline` ticket is present, that exact owner is drained and
completed before transient admission, never bypassed or replayed.

Admission binds the stable window key, live native `WindowId`, active native
resource generation matching the adapter, known unfenced target, running
nonterminal native-window eligibility, exact event kind/phase, and captured
timestamp. Auxiliary admission also binds caller-supplied active, admitted,
materialized wrapper eligibility; inactive, cached, retiring, or unmaterialized
children remain inert. Exact owner and native evidence currentness are
revalidated immediately before routing. An existing `DiscreteInput` owner is
never drained or replaced: transient admission fails inertly and leaves that
owner current. A pre-route veto is fully inert with no retry, replay, fallback,
or lower-stage mutation.

The ticket remains live through synchronous runtime-local mutation, native
routing, message reduction, and the current semantic flush. Exact completion
occurs before timed-frame merge or `Deadline`, refresh/projection, redraw
publication, auxiliary synchronization, wakeup, or presentation. A post-route
completion mismatch never reroutes, replays, or applies a lower-stage outcome;
a wrong ticket never clears the real owner. Focus and cursor boundaries, plus
wheel `Started`, `Ended`, and `Cancelled`, remain non-coalesced. `CursorMoved`
and wheel `Moved` preserve the existing direct/coalesced policy, accumulated
delta, newest metadata, sequence range, axis-change flush, focus-loss clearing,
and admitted queued-sample completion behavior. This private slice adds no
pointer-motion coalescer, fairness consumer, or public API.

`DiscreteInput` and `ImmediateTransient` are authoritative native
`input_transient` soft-budget consumers. Every admitted `DiscreteInput` and
`ImmediateTransient` kind binds the current effective-FPS `input_transient`
budget independently of diagnostics and frame observation. This includes every
`ImmediateTransient` kind: `Focused`, `CursorEntered`, `CursorMoved`,
`CursorLeft`, and `MouseWheel`, including every wheel phase. The typed exact
completion result is either `Completed` with `NotBudgeted`, `Within`, or
`Exceeded`, or `Mismatch`. `Completed(NotBudgeted)` and `Completed(Within)` map
to `ContinueNow`; `Completed(Exceeded)` maps to `DeferLowerPriority`.
`Mismatch` authorizes no policy, fallback, publication, or replay. Stale,
wrong, repeated, vetoed, or lifecycle-invalidated tickets publish nothing and
never clear the real owner.

`Exceeded` completes runtime-local state, semantic routing, coalescer updates,
and message reduction exactly once, then defers only due `Deadline` and
lower-priority visual/publication work through the existing bounded state.
`CursorMoved` and wheel `Moved` remain latest/coalesced; focus and cursor
boundaries, plus wheel `Started`, `Ended`, and `Cancelled`, remain
non-coalesced. Primary and auxiliary routes share the same disposition;
`Exceeded` on an auxiliary route defers sibling synchronization. External-drag
launch occurs only after exact successful completion: `Exceeded` still launches
immediately, and only its visual follow-up defers; `Mismatch` never launches.
`Exit`, lifecycle/terminal intent, discrete input, semantic effects, external
launch, and the event itself are never deferred, replayed, or rolled back.

Every successful native `Recovering`-to-`Running` transition consumes an exact
`FinishDeviceRecovery` Lifecycle ticket. After the fresh primary bundle and
target transition are published, the primary and every admitted auxiliary that
has no rebuild pending (including an unmaterialized auxiliary) are staged as
one no-yield set and synchronously revalidated before any finish phase mutates.
The finish ticket binds source phase `Recovering`, the exact Winit identity and
stable key, the accepted new shared adapter generation, and the exact active
resource generation or absence, target generation, and target-fenced state.
Materialized evidence requires a present Winit identity, an active resource
generation equal to the accepted adapter generation, and a known unfenced
target; unmaterialized evidence is admitted only for an auxiliary key with
absent Winit/resource state, a retained target generation (possibly unknown),
and a fenced target.
Materialized auxiliaries retain the existing lazy one-window rebuild order;
after fresh bundle publication and target transition they stage and revalidate
the exact ticket, apply the local native/controller finish hook, then complete
that exact ticket before rebuild, visibility, or redraw. Retiring auxiliaries
are excluded. Any finish staging, currentness, transition, or
completion failure consumes or vetoes only its exact ticket, performs no retry,
replay, redraw, or visibility restoration, and converges through bounded
`Closing` with the original `RenderDeviceLost` cause.

## Text Boundary

Radiant treats text as a first-class GUI concern but keeps the responsibilities
separate:

- `src/gui/text_layout` owns deterministic text-line placement helpers and a
  small owned cache for renderer-side reuse.
- `src/gui_runtime/native_vello/text_renderer` owns native text rendering,
  font fallback, glyph/cache behavior, and scene text encoding.
- `src/gui_runtime/native_vello/text_edit` owns native text-edit state,
  cursor stops, selection, and single-line edit layout.
- `src/runtime/paint/primitives/text.rs` owns backend-neutral text paint
  primitives and shared text storage for paint plans.

Application code should configure portable font policy through
`NativeTextOptions` and `EmbeddedFont` instead of depending on installed fonts
or renderer internals.

Current shipped boundary: the environment exposes only display scale, color
scheme, contrast, and reduced-motion preference, and Unicode-scalar editing is
shipped. Locale and writing-direction services remain future work under
OPT-1386; bidi and complex shaping remain future renderer/text-layout work
under OPT-1402.

## Platform Boundary

Radiant is macOS-first today as an implementation path, with a cross-platform
design goal; the target is modern macOS, Windows, and Linux/Wayland systems.
Core GUI, runtime, widget, layout, and paint-plan code stays platform-neutral.
X11 is an explicit
non-goal.
Platform-specific integration belongs in native runtime/windowing modules or
explicitly named platform adapters. Platform services such as file dialogs and
URL opening flow through typed `PlatformRequest` commands and the opt-in
`RuntimePlatformHost` capability. Application update handlers request those
services through Radiant context helpers instead of calling platform APIs
directly. The portable library boundary must compile for all three targets.
Native macOS behavior is validated on the M5 Pro development host. Current
Linux/Windows repository CI is limited to portable/build/compile/check
evidence. The target GitHub Actions lanes must eventually add integration and
Linux headless Wayland plus Linux/Windows native-host smoke coverage where
runners permit; no current Linux/Windows host, IME, accessibility,
presentation, latency, GPU, or performance acceptance is established.

Current target-specific seams are intentionally narrow:

- `src/application/runtime/threading/platform.rs` owns native thread-priority
  hints for background business workers. The application runtime keeps a
  platform-neutral worker-pool contract, while unsupported targets use the same
  worker loop without priority changes.
- `src/application/runtime/bridge/adapter/platform_services.rs` owns app-runtime
  platform service dispatch for file dialogs, reveal/open, clipboard text and
  file-list reads/writes, and confirmation prompts. The bridge exposes typed
  `PlatformRequest` values while target-specific reveal and clipboard behavior
  stays inside this adapter.
- `src/gui_runtime/native_vello/generic_runtime/window/platform.rs` owns native
  window attribute extensions such as Windows drag/drop and popup taskbar
  policy. Non-Windows targets keep the same runtime options and no-op the
  unsupported window hints.
- `src/gui_runtime/native_vello/generic_runtime/native_file_open.rs` owns native
  open-document callbacks. macOS delegates ApplicationServices open-document
  events into Radiant's backend-neutral file-open command route, while other
  targets keep the same runtime contract with an explicit no-op registration.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs` owns
  external drag-out platform selection. Windows delegates to the native OLE
  implementation, macOS delegates to the macOS AppKit implementation, and
  other targets report an explicit unsupported result through the normal
  runtime command path. macOS admits the native dragging session first and
  publishes its terminal copy-or-cancel result asynchronously through the
  runtime event loop.
- `src/gui_runtime/native_vello/generic_runtime/activation/platform.rs` and
  `src/gui_runtime/native_vello/generic_runtime/activation/reopen.rs` own
  macOS activation and application-reopen integration. The activation policy
  remains backend-neutral while unsupported targets use explicit no-op hooks.
- `examples/macos_frame_profile_acceptance.rs` is a macOS-only native acceptance
  harness for the public Off/Frame profile contract. Its non-macOS checked
  fallback keeps the registered example buildable without claiming native
  acceptance on other platforms.
- `examples/macos_devtools_acceptance.rs` is a macOS-only native acceptance
  harness for the existing runtime-local devtools overlay through the primary
  application builder. Its non-macOS checked fallback keeps the registered
  example buildable without claiming native overlay acceptance on other
  platforms. The inspector remains observational and uses normal hit testing
  and focus; the runtime overlay does not own interaction.
- `examples/macos_external_drag_acceptance.rs` is a macOS-only native
  acceptance harness for outgoing file drags through the public drag APIs. Its
  live path owns one disposable temporary source, while its non-macOS fallback
  and tests remain buildable without creating a temporary source or claiming
  native Finder acceptance.
- `examples/macos_numeric_accessibility_acceptance.rs` is a macOS-only native
  acceptance harness for ordinary materialized NumericInput increment and
  decrement actions through the public application builder. Its non-macOS
  fallback and tests remain buildable without claiming live AppKit or VoiceOver
  evidence.
- `examples/macos_text_input_ime_acceptance.rs` is a macOS-only native
  acceptance harness for the shipped primary-window single-line TextInput IME
  path. Its live instructions require actual Japanese IME preedit,
  candidate-panel, caret, commit, cancel, and focus-loss observation; its
  deterministic tests inspect only the production runtime projection and do
  not claim live AppKit evidence.
- `src/gui_runtime/native_vello/generic_runtime/input/platform.rs` owns the
  small target-specific modifier and control-click projection differences used
  by native pointer and keyboard mapping.
- `src/gui_runtime/native_vello/generic_runtime/accessibility.rs`,
  `src/gui_runtime/native_vello/generic_runtime/adapter.rs`,
  `src/gui_runtime/native_vello/generic_runtime/auxiliary.rs`,
  `src/gui_runtime/native_vello/generic_runtime/ime.rs`,
  `src/gui_runtime/native_vello/generic_runtime/lifecycle.rs`,
  `src/gui_runtime/native_vello/generic_runtime/window_environment.rs`, and
  `src/gui_runtime/native_vello/runtime_event.rs` own the native window
  environment event boundary. The runtime keeps monitor, accessibility, and
  theme changes behind the backend-neutral invalidation contract, while
  unsupported targets retain the same no-op or fallback behavior.
  The shared IME module normalizes Winit byte ranges to Unicode-scalar
  composition evidence for both primary and auxiliary windows. Public
  `CompositionSample` remains the four-variant `Start`/`Update`/`Commit`/`Cancel`
  vocabulary; explicitly hidden preedit selection travels through the
  additive defaulted `Widget::handle_hidden_composition_update` hook and the
  existing fixed composition owner. Hidden built-in preedits keep actual focus
  while zeroing the existing caret/selection colors, and the native encoder
  skips zero-alpha adornment geometry.
  `SurfaceRuntime` now owns the immutable per-window `WindowEnvironment`
  snapshot and updates it before deferred projection; custom bridges may opt in
  through the default-no-op `RuntimeBridge::set_window_environment` hook.
- `src/gui_runtime/native_vello/generic_runtime/native_semantic_accessibility.rs`
  owns the macOS AppKit accessibility consumer. Native callbacks remain
  callback-local, while the event-loop turn uses the private runtime bridge for
  exact deferred range retries and ordinary-authoritative fallback.
  `src/gui_runtime/native_vello/generic_runtime/runner.rs` owns its native
  runner lifecycle wiring; `src/runtime/automation.rs`,
  `src/runtime/controller.rs`, and
  `src/runtime/controller/virtual_layout.rs` own the crate-private bridge and
  whole-surface semantic publication path without changing the public
  `RuntimeBridge` or automation snapshot contract.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/macos.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/macos/bridge.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/macos/payload.rs`,
  and
  `src/gui_runtime/native_vello/generic_runtime/external_drag/macos/source.rs`
  are the macOS-only AppKit file-drag implementation behind that selector.
  The source callback reports the terminal operation only after AppKit ends
  the dragging session, and native ownership is released on both completion
  and startup failure.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/windows.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/drop_source.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/payload.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/preview.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/formats.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/medium.rs`,
  and
  `src/gui_runtime/native_vello/generic_runtime/external_drag/payload/dropfiles.rs`
  are the Windows-only OLE file-drag implementation behind that selector. These
  modules must stay reachable only through the cfg-gated platform adapter.
- `src/gui_runtime/native_vello/text_renderer/font.rs` owns native fallback
  font discovery after application-provided embedded fonts and font paths.
  Platform-specific font candidates stay inside that renderer adapter rather
  than leaking installed-font assumptions into widgets or layout.
- `examples/popup_window/platform.rs`,
  `examples/popup_window/platform/readiness.rs`,
  `examples/popup_window/host/child.rs`,
  `examples/popup_window/host/prewarm.rs`, and
  `examples/popup_window/host/process.rs` own the popup example's optional
  Windows window-control proof. The public popup and multi-window APIs remain
  platform-neutral, and non-Windows example paths degrade through local no-op
  or unsupported host behavior.

New target-specific code should either fit one of these seams or introduce a
similarly named adapter with a neutral public contract and an explicit
non-target fallback. Do not add raw Windows imports, `target_os` branches, or
installed-font/path assumptions to core widget, layout, styling, runtime
surface, or paint-plan modules.

## Validation Map

Use the smallest validation slice that proves the edited boundary, then run the
normal quality lane before merging meaningful changes.

- Public API/runtime behavior: `cargo test --test runtime_surface_public_api`,
  `cargo test --test runtime_bridge_public_api`, or the specific public API
  integration test touched by the change.
- Source-quality and documentation guardrails:
  `cargo test --test generic_surface_guardrails`.
- Reusable host-app non-blocking scans:
  `radiant::guardrails::NonBlockingGuardrail::app_update_paths()` from a
  consumer test. Applications should scan app-facing update/action/view roots,
  add host-domain forbidden tokens such as decode or database entry points, and
  allowlist only explicit worker or platform-adapter modules.
- Runtime slow-handler backstop:
  configure `SurfaceRuntime::set_update_handler_diagnostics_policy(...)` with
  `UiUpdateHandlerDiagnosticsPolicy::panic_at(...)` in deterministic
  test/development harnesses when a slow UI update handler should fail the run.
  Keep this as a supplement to API lockdown and static guardrails; business
  worker durations are reported separately under `RuntimeDiagnostics::business`.
- Examples: `cargo test --examples`, or the focused example target when a
  change is local to one sandbox.
- Documentation: `cargo doc --no-deps` to verify the generated rustdoc build and
  source-level intra-doc references. The current setup does not establish
  `rustdoc with broken intra-doc links denied`, and this command does not
  validate Markdown API-reference snippets.
- Doctests: `cargo test --doc` to keep doctests for public documentation examples
  in Rust doc comments compiling against the real crate API. It does not execute
  Markdown API-reference snippets.
- Formatting and linting: `cargo fmt -- --check` and
  `cargo clippy --all-targets --all-features -- -D warnings`.
- Broad regression lane: `cargo test --lib --tests`, matching CI. Use
  `cargo test -j 1 --lib --tests` when diagnosing order-sensitive failures or
  reducing concurrent resource pressure locally.
- Example compile checks: `cargo test --examples`.
- Portable library boundary: after installing the Unix targets with
  `rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin` and the
  Windows target with `rustup target add x86_64-pc-windows-msvc`, run
  `cargo check --lib --no-default-features --target x86_64-unknown-linux-gnu`,
  `cargo check --lib --no-default-features --target x86_64-pc-windows-msvc`,
  and `cargo check --lib --no-default-features --target x86_64-apple-darwin`.
  These checks do not prove native host, presentation, latency, GPU, IME,
  accessibility, or performance behavior, but they catch target-specific
  dependency leakage and public/core API drift across the in-scope platforms.
- Native-host CI (future target): the Linux lane should use a headless Wayland
  compositor and the Windows lane should run native-host smoke tests where the
  runner permits. These lanes are not present in current repository CI, which
  provides only portable/build/compile/check evidence for Linux/Windows; until
  they exist, no Linux/Windows host, IME, accessibility, presentation, latency,
  GPU, or performance acceptance is established.
- Performance smoke: `cargo bench --bench perf_harness -- --list`, then a
  focused JSONL baseline round trip such as
  `cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl .\target\perf-baseline.jsonl`
  followed by
  `cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl .\target\perf-baseline.jsonl --fail-on-missing-baseline`.
- Performance investigation: run `cargo bench --bench perf_harness <scenario>`
  or a filtered `--category` pass for focused trend work.

The maintained `examples/arrangement_shell` is the direct source for the
`standalone_gui` workload lanes. They cover combined frame refresh and paint-plan
materialization, browser/inspector structural toggle with full refresh and
relayout, and existing hover movement with paint-only output whose application
projection, runtime projection, widget-state synchronization, and layout deltas
are zero. Exact counter deltas and repeated-run identity are asserted. The
harness samples bounded batches and reports finite nearest-rank `p50_us`,
`p95_us`, and `p99_us` in text/JSONL/baseline JSONL while retaining
average-based baseline comparison and legacy baseline compatibility. This is
consumer evidence only and does not establish production staged execution.

Performance benchmarks are trend and profiling tools, not portable timing
pass/fail gates. They should still cover hot paths that matter to the target:
large layout trees, virtualized lists, paint-plan generation, command drainage,
runtime refreshes, pointer overlays, text-line layout caching, and GPU-surface
data preparation.

## Current Non-Goals

Radiant should not own VST SDK integration, audio-domain host behavior,
application-specific asset models, product-specific state, or an X11 backend.
Accessibility is a required platform-adapter capability, while its semantic
model remains backend-neutral; hardware-backed accessibility systems in the
current phase remain unverified on Linux and Windows. Host-owned state,
platform services, custom widgets, business-runtime requests, and embedded-host
surfaces must not become
product state in Radiant core.

Avoid new architecture that creates parallel application models, leaks renderer
internals into normal app code, creates accidental Windows-only imports, or
couples core modules to Windows-only behavior,
or makes examples the only proof of a public feature. A feature is aligned when
it has a coherent API, clean module ownership, tests or guardrails where
practical, and an example or documentation path that shows how application code
is expected to use it.
