# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~82.09% (903 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 85% |
| Declarative model, identity, reconciliation | 71% |
| Input, provenance, and edit lifecycle | 97% |
| Layout, composition, virtualization | 97% |
| Text, focus, and selection | 74% |
| Numeric controls | 92% |
| Runtime, effects, and scheduling | 96% |
| Rendering, invalidation, retained GPU surfaces | 78% |
| Platform, windowing, and host boundaries | 71% |
| Diagnostics, profiling, and performance validation | 66% |
| Examples, documentation, and CI guardrails | 76% |

2026-08-13 cycle update: Validation is complete for the macOS view-to-screen correction. Estimates remain unchanged because this is a host-stability/correctness repair, and live semantic acceptance is not estimate credit: generic architecture ~97%; broad 903 / 11 (~82.09%); Public 85; Declarative 71; Input 97; Layout 97; Text 74; Numeric 92; Runtime 96; Rendering 78; Platform 71; Diagnostics 66; Examples/docs/CI 76. The next dependency-correct slice is the private crate-private `accessibilityIndexOfChild:` topology callback.

2026-08-13 implementation cycle: The private crate-private `accessibilityIndexOfChild:` callback now uses the current immutable native projection, exact direct-parent topology, compact ordinary positions, and retained sparse container logical indices, with focused selector/ABI and fallback coverage. All estimates remain unchanged: generic architecture ~97%; broad 903 / 11 (~82.09%); Public 85; Declarative 71; Input 97; Layout 97; Text 74; Numeric 92; Runtime 96; Rendering 78; Platform 71; Diagnostics 66; Examples/docs/CI 76. No credit is awarded for this private callback, its documentation, or its tests; live AppKit/VoiceOver acceptance remains separate.

2026-08-13 implementation cycle: The private `accessibilityNotifiesWhenDestroyed` selector is registered with Objective-C encoding `c@:` and a crate-private `ObjcBool` callback that returns `YES` without state or notification side effects, including after the callback-state ivar is cleared during retirement. Estimates remain unchanged: generic ~97%; broad 903/11 (~82.09%); Public 85; Declarative 71; Input 97; Layout 97; Text 74; Numeric 92; Runtime 96; Rendering 78; Platform 71; Diagnostics 66; Examples/docs/CI 76. No credit is awarded for this private callback, its documentation, or its tests; live AppKit/VoiceOver acceptance remains separate.

2026-08-14 implementation cycle: The private primary-window host boundary now attaches the constructed root only through supported AppKit `accessibilityChildren`/`setAccessibilityChildren:` with bounded exact-root identity readback. Nil/wrong roots or hosts, unsupported selectors, Objective-C exceptions, and mismatched readback are inert: no attachment or layout/value notification is committed, attempted host state is cleared when verified, and pre-commit allocations never receive destruction notifications. Retirement verifies nil/empty clear readback before release; if verification fails, callback state and object ivars become inert while objects remain quarantined and no release/destruction notification follows a possibly stale host root. Previously committed objects retain exactly-once destruction notification after verified clear; root-to-content-view `AXParent` symmetry and unchanged/value-only object continuity remain. Automated AppKit boundary evidence remains shipped. Exact fresh-bundle activated Computer Use/AppKit evidence verifies discoverability and numeric action, bounded set-value, and restart acceptance for this bounded primary-window consumer: the activated window exposed the Radiant container and a settable stepper at `42.00`; Increment and Decrement produced `43.00` and `42.00`, bounded `SetValueText` produced `55.50` and `57.25` with fresh reads showing normal app-owned Begin/Update/Commit events, and a fresh restarted instance exposed the same tree. VoiceOver-specific acceptance remains unperformed. Repeated negative-geometry AppKit runtime diagnostics remain a separate unverified follow-up if reproducible. Estimates remain unchanged and no estimate credit, including Platform credit, is awarded.

2026-08-14 implementation cycle: The private parent-event-loop fairness ledger now retains current-demand stable-key state in reusable `Vec` storage, prunes absent keys at the existing `remove_absent` boundary, and admits every current fairness-eligible live window beyond the former 16-key capacity. Direct policy and `NativeFrameScheduler` tests cover more than 16 current keys, permutation-independent stable ordering, priority/deadline selection, two-epoch promotion, lifecycle/discrete vetoes, retirement/reinsertion, and empty/ineligible fallback. Validation: `cargo test --locked --lib gui_runtime::native_vello::generic_runtime::frame_scheduler_policy::tests` (15 passed) and `cargo test --locked --lib gui_runtime::native_vello::generic_runtime::frame_scheduler::tests` (13 passed). Estimates remain unchanged: generic ~97%; broad 903/11 (~82.09%); Public 85, Declarative 71, Input 97, Layout 97, Text 74, Numeric 92, Runtime 96, Rendering 78, Platform 71, Diagnostics 66, Examples/docs/CI 76. No estimate credit is awarded for this correction.

2026-08-14 implementation cycle: The crate-private native Winit candidate-area publisher now scans the authoritative `SurfacePaintPlan` for exactly one focused `PaintTextInput`, reuses the existing text-field layout and selection/caret projection, rejects malformed, zero, ambiguous, or fallback geometry, and publishes finite logical caret areas through the actual per-runner `Window::set_ime_cursor_area` before retained-scene reuse in both primary and auxiliary Vello loops. `NativeImeCursorAreaCache` suppression is fenced by `WindowId`, `NativeTargetGeneration` scale/DPI conversion generation, and uninterrupted valid-candidate evidence; it records only after the Winit call, and invalid evidence forces a later identical valid area to republish. Focused coverage includes empty, Unicode, selected, hidden-alpha, long/clamped, malformed, duplicate-focus, unchanged, moved, invalid-to-valid, scale-transition, and window-replacement cases. Native Japanese/Chinese IME acceptance is unperformed. Estimates remain unchanged and no estimate credit is awarded.

The broad estimate is the unweighted mean of the category rows:
`(85 + 71 + 97 + 97 + 74 + 92 + 96 + 78 + 71 + 66 + 76) / 11 = 82.09%`,
reported as approximately `82.09%`.
The generic architecture-sequence estimate remains about 97%; the private
primary-window macOS/AppKit semantic accessibility consumer below is now
implemented without changing the estimates. Automated AppKit boundary evidence
remains shipped; exact fresh-bundle activated Computer Use/AppKit evidence
verifies discoverability and numeric action, bounded set-value, and restart
acceptance for this bounded primary-window consumer. VoiceOver-specific
acceptance remains unperformed; repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible. Estimates
remain unchanged and no estimate credit, including Platform credit, is awarded.
The pre-admission stale or mismatched explicit-retry correction is also covered:
it clears only the in-flight transport key and leaves semantic selection,
projection, runtime demand, and native notification state unchanged. This is a
correctness correction with no estimate credit; authoritative runtime-returned
baseline or terminal outcomes continue to replace semantic projection with
ordinary.
The broad estimate is now `903 / 11` (~82.09%) and Public API is 85% for the
bounded custom-coordinate attachment evidence; no design-only credit is
awarded for unvalidated work. The private native consumer now has a bounded
normalized-Custom path. Automated AppKit boundary evidence remains shipped; exact
fresh-bundle activated Computer Use/AppKit evidence verifies discoverability and
numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible. Estimates remain unchanged and no estimate
credit, including Platform credit, is awarded. Other native adapters, scheduler-,
renderer-, and product-policy boundaries remain separate.
Current values remain exactly: Public API 85; Declarative 71; Input 97; Layout
97; Text 74; Numeric 92; Runtime 96; Rendering 78; Platform 71; Diagnostics
66; Examples/docs 76; generic ~97%; broad `903 / 11` = 82.09%. The
provider-free exact semantic cardinality declaration and its exact private
registration/live-fence invalidation foundation are now shipped and remain
provider-free; this production slice does not change any estimate. The
compositor-owned complete normalized sidecar is now shipped from the exact
staged union, with final paths, materialization origin, and source-qualified
publication fences retained atomically by the selected composition. Estimates
remain unchanged. Native cardinality queries/topology and the bounded normalized
custom consumer are implemented. Automated AppKit boundary evidence remains
shipped; exact fresh-bundle activated Computer Use/AppKit evidence verifies
discoverability and numeric action, bounded set-value, and restart acceptance
for this bounded primary-window consumer. VoiceOver-specific acceptance remains
unperformed; repeated negative-geometry AppKit runtime diagnostics remain a
separate unverified follow-up if reproducible. No estimate credit, including
Platform credit, is awarded.
The
partial native implementation in stash
`radiant-native-semantic-consumer-partial-sol-audit` and the incomplete
cardinality/sidecar attempt in stash
`backend-neutral-semantic-sidecar-partial-luna-audit` are non-evidence and earn
no estimate credit.
The generic widget interaction teardown seam is now executable: an additive
defaulted `Widget` hook can terminate retiring local state, old-surface mappers
collect ordered UI-local output before discard, and the existing deferred
command path reduces it after installation. Conservative removal, incompatible
replacement, authority/disabled/read-only loss, compatible preservation, old
mapper ownership, unmapped cleanup, ordering, exactly-once behavior, and
non-reentrant projection are covered by private runtime fixtures. These results
do not change the estimates above.

The current numeric evidence is a public generic `numeric_input(value, codec,
adjustment)` builder with typed construction failures and the shipped
fixed-capacity `NumericInputEditBatch<T>` bounded incremental carrier. The
carrier accepts exactly `[Update]`, `[Commit]`, `[Cancel]`, `[Begin, Update]`,
`[Begin, Commit]`, `[Begin, Cancel]`, and `[Begin, Update, Commit]` in private
inline capacity-three storage. The shipped text-first widget emits only `[Begin, Commit]` and
`[Begin, Cancel]`, including `[Begin, Cancel]` when replacement teardown retires
an active edit; this carrier is storage and shape validation foundation only. The
consumer formats the initial value through the application codec,
validates the adjustment inverse, preserves verbatim drafts, caches draft
classification for the synchronous allocation-free focus-loss veto seam,
commits valid Enter/focus-loss edits, cancels active Escape edits, and retains
draft/caret/session state only for an actually active edit during same-ID
reprojection. Qualified exports remain outside the common prelude.
The qualified `NumericStepAttempt`, `NumericInputInteraction<T, StepError,
FormatError>`, and `NumericInputInteractionBatch<T, StepError, FormatError>`
are now shipped as a fixed-capacity keyboard envelope and complete TextEdit
output contract. The batch validates successful keyboard edit fragments,
TextEdit terminal fragments, and typed initial or rollback-before-repeat
failures. Complete mode consumes an explicitly attached step policy for
effective ArrowUp/ArrowDown transactions, typed failures, rollback, capture,
and teardown through the selected complete mapper; compatibility mode remains
inert.
The crate-private shared numeric interaction gate is now shipped for TextEdit
admission, no-op cleanup, terminal cleanup, replacement teardown, and compatible
active reprojection. `NumericInputWidget` consumes the generic
`Widget::prepare_replacement` seam: an exact same-ID, same-value, enabled,
non-read-only successor preserves the active session for normal synchronization;
every other replacement boundary publishes one rollback through the retiring
widget's selected mapper mode, restores the value/draft/caret/selection
snapshot, and releases TextEdit ownership. A mode change cannot inherit the
active session. Invalid, incomplete, and out-of-range drafts use the existing
cancel path without consulting codec or adjustment policy.
Normalized `KeyRelease` plumbing is now shipped across native input, runtime
events, and focused widget dispatch; complete-mode semantic keyboard
adjustment is now shipped for explicitly configured numeric step policies. The
qualified backend-neutral IME/composition foundation, the single-line
`TextInputWidget` consumer, and the `NumericInputWidget` consumer are now
shipped: validated Unicode-scalar `CompositionRange`/`CompositionSample`
values, default-compatible object-safe widget hooks, erased surface dispatch,
a private fixed-size focused `Idle`/`Active`/`Blocked` ownership kernel, and
widget-local start/update/commit/cancel state with revision-aware
reprojection. Numeric preedit remains visible local text without parse or
publication; a valid commit reuses text sanitization and the numeric codec for
one terminal `[Begin, Commit]` batch, while invalid commits remain correctable
and cancel/focus-loss restores the captured edit. The shared native Winit
normalizer/router is now wired to both primary and auxiliary Vello loops:
`Ime::Enabled` is capability-only, valid byte endpoints become scalar ranges,
`Preedit(..., None)` uses the additive defaulted hidden-update hook rather than
changing the four-variant public `CompositionSample` vocabulary, and malformed
evidence cancels conservatively. Built-in hidden preedits retain actual focus,
clear stale visible selection/caret adornments with zero-alpha existing
colors, and the native encoder skips that geometry; the legacy hook fallback
conservatively cancels. The bounded native Winit candidate-area publication is
shipped for primary and auxiliary Vello loops; native Japanese/Chinese IME
acceptance is unperformed. Matching-key suppression, other native IME adapters,
and product behavior remain unshipped. Complete-mode
NumericInput
PointerScrub consumption is now shipped for the explicitly configured
primary-plus-Alt/Option path, including managed capture, bounded output, typed
failures, and exact rollback/teardown. Complete-mode NumericInput wheel
consumption is also shipped through the explicit `NumericWheelPolicy`
attachment: exact samples preserve line/pixel units and phaseful provenance,
use the fixed 40-pixel line equivalence, retain pending/active ownership, emit
bounded `[Begin, Update, Commit]` or incremental fragments, report typed
adjustment and format failures, roll back before update diagnostics, and cancel
the incumbent owner directly before a superseding `Started` sample is routed.
Phase-less and `Discrete` samples are bounded atomic gestures; orphan phaseful
`Changed`, `Ended`, and `Cancelled` samples are ignored and remain available to
fallback routing.
Legacy vector dispatch remains metadata-preserving after metadata-neutral hit
testing. The typed, widget-local NumericInput accessibility policy now ships
the neutral Increment/Decrement/SetValueText vocabulary, Base-step and
complete-text semantics, atomic Accessibility provenance, typed failures, and
incumbent-owner blocking. The generic runtime dispatch boundary now ships
authority-bearing target evidence, current identity/path/role and materialized
target checks, pre- and post-focus owner checks, focus admission, and a
type-erased output/mapper seam for the typed local outcome. Native adapters,
virtual-target materialization, scheduler work, and product policy remain
separate unshipped boundaries.
The private virtual-layout pin-owner prerequisite is shipped as a bounded
one-item query path. Each mounted runtime record owns exactly one optional pin
tagged `Focus`, `PointerCapture`, or `Semantic`; it retains the exact request
and validated provider entry, uses the applicable container identity, policy
identity, mount generation, and data/policy/measurement/semantic revision
fence, and clears/rejects terminal outcomes before any stale pin survives.
The private runtime bridge now also forwards at most one exact required item key
through policy input and the query fence; a ready result omitting that key is
invalid before coordinator commit, and a changed key invalidates pending work
and previous fallback. Regression coverage includes the required-key query and
runtime path within the focused virtual-layout suite, plus the bounded format,
diff, check, and Clippy gates. No public semantic-tree consumer or full
focus/automation traversal, offscreen materialization, focus/capture transfer,
scrolling, scheduler/renderer policy, or product wiring shipped. A private
current-fence one-item semantic admission path is now shipped: current
registration authority constructs the exact request for one live mounted
container and opaque stable key, and a valid provider result retains one
`Semantic` pin. The crate-private `VirtualLayoutSemanticProjection` boundary
then retains only validated semantic evidence with the declared coordinate
space, finite bounds, the exact provider-supplied serializable `AutomationNodeId`,
exact request/fence, and explicit `Unmaterialized` authority. The opaque
`VirtualLayoutItemKey` remains lifecycle/authority identity; IDs are not
synthesized from indices, keys, pointers, slots, or bounds. There is no global
ID admission; this private evidence is not wired into `AutomationTarget` or
`GuiAutomationSnapshot`, and no automation traversal, offscreen materialization,
focus/capture transfer,
scrolling, paint, hit testing, scheduler/renderer work, public consumer, or
product wiring shipped. The evidence moves Declarative identity from 70% to
71% and broad coverage from `900 / 11` to `901 / 11` (~81.91%); generic
architecture remains ~97% and layout remains 97%.
The same private boundary now has an exact range query over
`[start_index, start_index + length)`: zero length, checked-add overflow, and
length above either the live registration budget or
`VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` are rejected before provider invocation.
At most one range-provider call is made. Only an exact-count, contiguous
ordered logical-index vector with stable unique opaque keys, distinct exact
provider-supplied `AutomationNodeId` values for distinct keys, finite
non-inverted bounds, and the current container/policy/mount/data/policy/
measurement/semantic/coordinate/budget fence becomes ordered
`VirtualLayoutSemanticProjection` evidence. Duplicate semantic IDs and
same-key, same-fence ID drift against an existing pin reject atomically;
not-found, unavailable, deferred, rejected, malformed, and stale results are
all-or-nothing and do not mutate the existing one-item pin. The provider ID is
carried unchanged through each projection and the ordered batch, while the
opaque key remains the only lifecycle/authority identity. Focus, capture,
runtime/materialization/layout, refresh, scroll, paint, hit-test, and
automation-snapshot state remains untouched. Path, coordinate-space, and
cross-range deduplication work remain unshipped. This adds private evidence
only and does not change the retained estimates: generic ~97%, Declarative
identity 71%, layout 97%, and broad coverage `901 / 11` (~81.91%).

The downstream semantic/materialization classification boundary is now shipped
as crate-private synchronous evidence. It accepts only a successfully validated
`VirtualLayoutSemanticProjectionBatch` plus the matching live
`RuntimeVirtualLayoutRecord` and materialization store, and never calls a
semantic provider again. Before consulting active slots it requires exact
equality of the batch request and the live/store materialization fence for
container, stable policy identity, mount generation, data/policy/measurement/
semantic revisions, coordinate-space identity, and admitted budget. Missing,
retired, lifecycle-indeterminate, or authority-less materialization evidence,
registration-only evidence, and any mismatch reject the complete result.
Exact-key matching uses only `VirtualLayoutItemKey::stable_equals`, preserves
range order and every provider semantic field, and rejects same-key index drift,
another key occupying an in-range index, unstable equality, ambiguity, or
malformed evidence. Each materialized classification carries an exact slot
identity and retained payload-root `NodeId`; unmaterialized entries retain a
separate `Unmaterialized` origin while the existing projection authority stays
unchanged. The payload is not cloned and the generated wrapper ID never replaces
the provider `AutomationNodeId`. The operation has no provider, pin,
materialization, refresh, layout, traversal, snapshot, focus, capture,
lifecycle, scheduler, renderer, or product side effect. Path insertion,
coordinate resolution/resolver invocation, final ordering, collision/ID
admission, cross-range deduplication, and semantic-tree work are not
responsibilities of the classifier itself; the private compositor below
consumes its result for the bounded logical/custom tree step.

The private automation-tree compositor is now shipped as staged, crate-private
evidence. It consumes already validated classification batches, admits
`Logical` coordinates unchanged, admits `Custom` only with an exact private
transform witness, and normalizes input
by exact registration fence, container, and logical index. Exact same-key/index
overlaps coalesce only when semantic, geometry, provider-ID, origin, and fence
evidence all agree; conflicting overlap, key/index drift, duplicate payload
roots, unstable equality, aggregate registration-budget overflow, and hard-cap
overflow reject atomically. It requires exact unique ordinary container anchors
and exact direct generated wrapper roots, replaces materialized wrappers in
place while preserving descendants, and inserts each unmaterialized provider
leaf once with private flattened authority `materialized = false`. Ordinary,
descendant, provider, container, and cross-range IDs share one namespace; only
the exact generated wrapper being replaced may be displaced. A final uniqueness
audit runs after staging, while source snapshots and runtime state remain
unchanged on failure. Public APIs and serialized schema remain unchanged; no
provider invocation, scheduling/demand ownership, custom transform, focus/action
or product wiring is added. Estimates remain unchanged: generic ~97%,
Declarative identity 71%, layout 97%, and broad coverage `901 / 11`
(~81.91%).

The crate-private semantic-demand owner/provider-attempt/retention kernel and
private atomic whole-surface publication/composition kernel are now shipped. The
owner assigns one crate-private semantic-demand owner per `SurfaceRuntime`, one
active contiguous logical range-demand slot per mounted virtual container, and
the existing independent one-item semantic pin. Only an explicit
semantic/accessibility range request or explicit required-item pin is demand;
registration is capability-only, and viewport/overscan/paint/hit-test,
provider-availability, item-count, diagnostics, and snapshot reads are not
demand. The target is logical-only, rejects `Custom` before provider invocation
without an identity-transform fallback, admits at most
`MAX_VIRTUAL_LAYOUT_REGISTRATIONS` 64 registrations, bounds each registration
and `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` at 1024 entries, caps aggregate active
range length at 1024, and permits at most one provider call per
container/attempt.

Demand generation and attempt sequence, an exact per-slot provider fence, and
exact private retention are shipped for identity, mount,
data/policy/measurement/semantic revisions, coordinate, budget, exact demand,
provider identity/generation, source, demand generation, attempt, and
cancellation. Private whole-surface publication fences and composition add
materialization/classification authority, ordinary projection generation, and
complete surface demand-set generation.
Only explicit `refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` call a provider. Registration,
opening, enumeration, snapshot reads, ordinary repaint, viewport/visibility/
overscan, paint-only work, provider-availability reads, item count, diagnostics,
IME/native events, and unchanged refresh do not create demand or call a
provider; materialization/ordinary-projection changes may reclassify retained
exact evidence without provider reentry. `Found`,
`NotFound`, terminal `Unavailable(NoProvider/Unsupported)`,
`Unavailable(DataUnavailable)`, `Deferred`, `Rejected`/malformed, and stale/superseded
outcomes have explicit owner staging, slot, and fallback behavior. The private
publication kernel represents every active member, classifies retained evidence
without provider reentry, rejects stale/cancelled or incomplete members, and
stages a complete logical composition only after all publication fences match.
Focused direct tests cover incomplete A+B, complete A+B, terminal membership,
pin-only routing, cancelled completion, retained reclassification, and the
compositor's conflict and union-cap vetoes.
`Unmaterialized`/`materialized = false` remains authoritative, semantics do not
authorize materialization, scrolling, actions, focus, paint, hit testing,
scheduling, rendering, or provider authority, and snapshot functions remain
pure reads. The generic logical production consumer below now implements the
public snapshot selection/visibility and session ownership boundary. The
bounded public custom-coordinate attachment and normalized native consumer are
implemented below; direct native resolver conversion/reconstruction, live host
acceptance, product consumers, scheduler/backoff/fairness, and multiple active
ranges per container remain separate gaps. The normative public declarative provider attachment contract is
shipped at its qualified boundary and recorded in
`docs/VIRTUAL_LAYOUT_DESIGN.md`. This implementation earns the Public API
evidence point: generic ~97%, Declarative identity 71%, layout 97%, and broad
coverage `903 / 11` (~82.09%).

## Next production consumer: semantic automation session (normative; generic logical implementation shipped)

This is the shipped generic logical consumer of the private
semantic-demand/provider-attempt/retention and whole-surface
publication/composition kernels. The first consumer is generic and
backend-neutral, not a native adapter or product integration. The caller/host
owns session intent and MUST explicitly open, refresh, retry, and close it.
`SurfaceRuntime` owns bounded session state, demand membership,
cancellation/supersession, selected publication, and publication lifetime.
Mounted virtual-layout runtime owns provider registration. Callers MUST NOT
infer demand from paint order, visibility, viewport/overscan, item count,
provider availability, diagnostics, or snapshot reads. Session/container
identity is opaque and runtime-issued; callers cannot fabricate provider
identity or authority.

The shipped operations are `open_semantic_automation_session`,
`semantic_automation_containers`, `refresh_semantic_automation_session`,
`retry_semantic_automation_session`, `selected_semantic_automation_snapshot`,
and `close_semantic_automation_session`, with the corresponding opaque demand,
handle, result, status, and fallback types under `radiant::runtime`. Ordinary
`automation_snapshot(&self)` and `automation_target_snapshot(&self)` remain
pure ordinary reads. Explicit refresh and retry are the only provider-calling
operations: refresh atomically replaces the complete demand set, while retry
reattempts the unchanged set. Opening and closing perform provider-free
lifecycle mutation. A separate pure selected semantic snapshot read returns
the last accepted session publication or the conservative ordinary baseline
plus a typed status. This contract invents no public
provider-registration API.

Opening establishes one bounded empty session and an exact session generation.
The first explicit refresh supplies any initial demand members, which start at
attempt one. Refresh atomically replaces the whole session demand set and
supersedes/cancels prior work. An unchanged retry increments only the attempt.
Closing cancels before retiring the generation and clears selected publication
and demand. The first implementation allows one
active semantic session per `SurfaceRuntime`, one contiguous logical range per
mounted container plus the existing independent one-item pin, at most 64
registrations, per-registration and 1024-entry caps, aggregate range length
1024, and at most one provider call per container/attempt. Automatic
retry/backoff and a scheduler are not part of this slice; `Deferred` returns to
the caller and only explicit retry reattempts.

Selection/publication carries session generation, demand generation, attempt,
request/range or pin, mount/container/policy identity, registration identity and
generation, data/policy/measurement/semantic revisions, provider identity/
generation, coordinate, budget, cancellation, materialization/classification
authority, ordinary projection generation, and complete-demand-set generation.
For `Custom`, the exact transform identity, application revision, runtime
resolver generation/token, source rectangle, ordinary anchor, destination clip,
and private transform witness are also required. A result is accepted only
when every required field matches exactly; stale, superseded, and cancelled
results are inert. Provider attempts are non-reentrant and cannot publish or
mutate runtime state directly.

The consumer stages the complete selected snapshot and status under the exact
fence and swaps only after every active demand member resolves or has an
eligible exact-fence fallback. It never publishes a partial subset. `Found` and
authoritative `NotFound` may participate in a complete publication.
`DataUnavailable` and `Deferred` retain only an eligible last-complete selection
for unchanged exact demand/fence; without that exact fallback they expose the
ordinary baseline and a typed non-success status. `NoProvider`/`Unsupported`
are terminal. `Rejected`/malformed, provider panic, and collision outcomes use
the conservative ordinary baseline even when an older selection exists. Stale,
cancelled, or superseded results are inert and do not mutate runtime state.
Changed demand, close, mount/identity/provider/revision/coordinate/budget
changes invalidate the old selection. Materialization/ordinary-projection
changes may reclassify retained exact provider evidence without provider
reentry when fences permit it. `Unmaterialized`/`materialized = false` never
authorizes materialization, scrolling, focus, action, paint, hit testing,
scheduling, or renderer work.

The generic consumer admits `Logical` unchanged and admits `Custom` only from
the qualified application-owned transform attachment. The synchronous `Rc`
resolver receives finite source geometry, the runtime-validated ordinary
anchor, the complete destination clip, host revisions, and its exact transform
revision; the runtime owns resolver lifetime, generation/token, panic/reentry
containment, clipping, exact witnesses, retention, and invalidation. It calls
the resolver only after destination validation and complete provider-output
validation during explicit refresh/retry, at most once per accepted entry.
Unsupported, singular, ambiguous, panic, invalid, overflowing, stale, or fully
clipped results fall back to the ordinary baseline without partial publication.
The existing private AppKit consumer accepts `Logical` unchanged and accepts
qualified `Custom` only through the normalized sidecar; it consumes only
resolved logical-window bounds and never invokes the custom resolver. This slice adds
the earned public evidence point: generic ~97%, Declarative identity 71%, layout
97%, and broad coverage `903 / 11` (~82.09%). Existing pure public snapshot
APIs and non-goals remain explicit.

## Public declarative provider attachment (normative; shipped; custom attachment bounded)

The alignment documents define one public declarative capability for attaching
semantic item/range providers and, separately, one qualified custom-coordinate
transform. The
qualified shipped vocabulary is
`radiant::application::VirtualLayoutParts<Message>`,
`virtual_layout_from_parts`, `radiant::runtime::VirtualLayoutRevisions`,
`VirtualLayoutSemanticProvider`, `VirtualLayoutSemanticRangeProvider`,
read-only item/range requests, `VirtualLayoutSemanticEntry`, generic
`VirtualLayoutSemanticProviderOutcome<T>` with `Found`, `NotFound`, `Unavailable`,
`Deferred`, and `Rejected`, plus
`radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransform`,
its request/outcome vocabulary, and
`VirtualLayoutParts::with_semantic_coordinate_transform`. These are qualified
exports and are not in the prelude. The boundary is synchronous,
single-threaded `Rc` with no `Send`/`Sync`/worker/scheduler promise. The
transform maps a complete `Custom(identity)` source rectangle directly to a
conservative logical-window AABB; it does not expose or assume matrices,
inverse transforms, point mapping, hit testing, or materialization.

`SurfaceRuntime` owns mounted registration, removal, provider replacement,
registration/mount/provider generations, lifetime cancellation, and exact
source tickets, bounded by 64 registrations, one range and one required-item
slot per container, 1024 entries per query and in aggregate, and at most one
provider call per container/attempt. There is no public imperative registration
API or application-owned mount generation. Only explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` may call providers, and only those
explicit semantic turns may invoke an attached custom transform after complete
provider validation; registration,
opening, enumeration, ordinary reads/repaint/viewport/visibility/overscan,
diagnostics, item count, provider availability, and IME/native events do not
create demand. `NoProvider` is runtime-synthesized; provider unavailable
reasons are `DataUnavailable`/`Unsupported`, and bounded deferred reasons are
`DataPending`/`SemanticPending`/`Retry`.

The normative behavior includes exact fence/source-ticket matching,
read-only callback isolation, reentry rejection, conservative panic mapping,
validated `Found`, authoritative `NotFound`, terminal missing/Unsupported,
exact-fence retention only for `DataUnavailable`/`Deferred`, conservative
baseline for rejection/panic/malformed/collision, inert stale/cancelled/
superseded results, atomic whole-surface publication, and preserved
`Unmaterialized` authority. Custom admission additionally requires one exact
ordinary anchor, complete current clip/ancestor chain, finite source/output,
and a private matching transform witness; nested custom declarations require
their own resolver. The private primary-window macOS/AppKit native
semantic accessibility query contract below translates explicit platform queries only through the
backend-neutral session model and is not a hidden provider owner. The full
acceptance matrix and native contract are in `docs/VIRTUAL_LAYOUT_DESIGN.md`;
direct native custom-resolver invocation/reconstruction, native actions for virtual/provider targets, new native AX focus exposure or transfer beyond existing ordinary runtime admission,
scrolling/materialization, scheduler/backoff/fairness, renderer/paint/
hit-testing/cache policy, product policy, multiple ranges, and prelude export
remain excluded.

This contract is normative and shipped. It is the bounded custom-coordinate
public-API evidence point; estimates are Public API 85% and broad coverage
`903 / 11` (~82.09%). Automated AppKit boundary evidence remains shipped; exact
fresh-bundle activated Computer Use/AppKit evidence verifies discoverability and
numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible. Estimates remain unchanged and no estimate
credit, including Platform credit, is awarded.

## Native semantic accessibility query consumer (normative; private primary-window macOS/AppKit consumer)

This documentation records the private primary-window macOS/AppKit production
consumer over the shipped generic logical semantic automation session. One
private native-window
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
`AXFocused` false/unclaimed, a value that is settable only for an eligible current ordinary materialized target, and exactly `AXIncrement` and
`AXDecrement`; the adapter never exposes or transfers native AX focus. Modern
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

The private native boundary now rests on the shipped provider-free declaration and
fence foundation. The qualified public
`radiant::application::virtual_layout::VirtualLayoutSemanticCardinality` value
carries an exact `usize` logical item count and separate `u64` cardinality
revision; an optional `VirtualLayoutParts` field and qualified
`VirtualLayoutParts::with_semantic_cardinality(...)` builder are shipped outside
the common prelude. The exact private registration/live-fence invalidation
foundation, normalized sidecar, native topology, bounded AppKit cardinality
query/child traversal, and the private primary-window platform consumer are
implemented. Automated AppKit boundary evidence remains shipped; exact
fresh-bundle activated Computer Use/AppKit evidence verifies discoverability and
numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible. The bounded normalized custom consumer is
implemented; no estimate credit, including Platform credit, is awarded. `None`
is unknown/unsupported, exact zero is supported, cardinality
is immutable declaration evidence rather than a callback or demand, is not capped
at 1024, and does not allocate proportional storage. Count reads, updates,
mount, and enumeration are provider-free. Unknown cardinality does not vend a
virtual child container; positive cardinality without a range provider is
unsupported for native child traversal and is not vended; exact zero is
representable without a provider.
AppKit count is exact, and range normalization uses checked subtraction and
zero/out-of-range/overflow, declared-budget, 1024-cap, and remaining-aggregate
checks without synthesizing keys from indices.
The macOS-only exact-range retry fixtures are cfg-gated from non-macOS test
builds; this CI portability correction changes no production behavior, public
API, target evidence, or estimate.

The exact `(count, cardinality_revision)` pair is fenced with registration
identity/generation, container/mount, existing revisions, coordinate, budget,
and provider generations using exact equality. Count/revision changes invalidate
affected semantic/native state provider-free; provider replacement preserves
count but invalidates provider publication; unmount, recovery, deactivation,
and close retire all state. The compositor produces one crate-private normalized
sidecar from the same staged `entries_by_container` union as
`VirtualLayoutAutomationComposition`. It retains container/mount/registration
authority, cardinality fence, logical index, stable key, provider
`AutomationNodeId`, final normalized node/path, materialization authority, and
publication fence. Raw range/pin members are not reconstructed by native code;
full-evidence overlap coalescing is the only same-key/index merge. Any
conflicting, ambiguous, duplicate, unstable, colliding, ordinary-ID, or
aggregate failure rejects the whole publication, and the sidecar is stored
atomically with `RuntimeSemanticAutomationSelection` composition/status/
ordinary/projection without parallel reconstruction or mixed selection.

The private native topology is one private root per primary content view/window,
one read-only virtual container per accepted anchor, and direct normalized logical
item children with duplicate placement suppressed. Runtime-issued private
container identities and monotonic item tokens are not derived from indices,
pointers, provider/serialized IDs, or bounds; continuity requires exact
lease/container/mount/cardinality-fence/key equality, and cardinality changes
retire tokens. Invalid tokens return nil/`NSNotFound` without provider calls.
Root/container/non-text roles map to `NSAccessibilityGroupRole`; only Text and
Readout map to `NSAccessibilityStaticTextRole`. Only role, exact
parent/children, finite frame, label, description/help, and static value are
exposed. State/action metadata is omitted, focus is false, actions are empty/
no-op, actionable roles are not created for buttons/toggles/sliders/tables/text
inputs, and defunct objects are empty/zero.

Callbacks remain non-blocking and provider/runtime-free. Valid explicit
item/range queries enqueue/coalesce one owned runtime turn. Pending count stays
exact; item/range reads return exact eligible same-fence retention or empty/nil,
never placeholders or mixed trees. An explicit repeat after `Deferred` may
retry; ordinary reads do not. Complete normalized native publication is atomic
and retained only under exact semantic/native coordinate/cardinality fences.
DataUnavailable/Deferred without exact fallback is empty/baseline; terminal
failures clear virtual publication; stale/cancelled results are inert. Only a
changed visible state posts exactly one
`NSAccessibilityLayoutChangedNotification` after main-thread queryability;
unchanged/pending/stale/cancelled/rejected work posts none. Retired custom
objects follow `UIElementDestroyed` notification lifecycle.

This documentation contract preserves the one-session bound, opaque private
handles, explicit refresh/retry-only demand, one range plus one required-item
slot, 64 registrations, 1024 per-query and aggregate caps, one provider call per
container/attempt, exact publication/fallback, `materialized = false`,
normalized logical bounds for Logical and qualified Custom authority, and pure snapshots.
It excludes new native AX focus exposure or transfer beyond existing ordinary runtime admission, native actions for virtual/provider targets, selection mutation, scroll/materialize,
scheduler/retry policy, render, product, direct native custom-resolver
invocation/reconstruction,
Wayland/Windows, auxiliary, multi-consumer, and public registry behavior.

This contract is limited to the private primary-window macOS/AppKit consumer.
Wayland, Windows, non-qualified/virtual native actions, new native AX focus exposure or transfer beyond existing ordinary runtime admission, scrolling, product policy, direct
native custom-resolver invocation/reconstruction, scheduler, and renderer
behavior remain excluded. The
bounded generic custom-coordinate attachment is covered above. Automated AppKit
boundary evidence remains shipped; exact fresh-bundle activated Computer Use/AppKit
evidence verifies discoverability and numeric action, bounded set-value, and
restart acceptance for this bounded primary-window consumer. VoiceOver-specific
acceptance remains unperformed; repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible. Estimates
remain unchanged and no estimate credit, including Platform credit, is awarded.

For the public provider-attachment evidence point, Slider/Knob, scheduler,
renderer, and product policy remain out of scope; the native contract above is
the private primary-window macOS/AppKit boundary.
The Input evidence moves from 96% to 97%, Numeric controls from 87% to 92%,
and Text moves from 73% to 74% for runtime focus/selection admission.
The evidence-backed total for this alignment sequence is now `903 / 11`
(~82.09%), with Public API 85%. Exact fresh-bundle activated Computer Use/AppKit
evidence verifies discoverability and numeric action, bounded set-value, and
restart acceptance for this bounded primary-window consumer. VoiceOver-specific
acceptance remains unperformed; repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible. Estimates
remain unchanged and no estimate credit, including Platform credit, is awarded.
The generic composition foundation, the single-line text consumer, the
NumericInput consumer, and the primary/auxiliary Winit consumer with its
bounded candidate-area publication remain distinct from deferred matching-key
suppression, other native-adapter, and product-policy boundaries.
The public `KeyboardModifier`/`NumericStepModifiers` selector and
`NumericInputBuilder::step_modifiers(...)` attachment are now the explicit
complete-mode keyboard consumer policy. The selector evaluates lossless
`KeyboardModifiers` samples without allocation or retained state; the widget
stores `None` when unconfigured or exactly `Some(policy)` when attached and
recomputes the selected step for every sample. No automatic platform policy is
introduced; compatibility mode and an unconfigured complete widget remain
inert.
The native keyboard boundary now also ships a lossless widget-modifier
projection alongside the unchanged host-shortcut `KeyPress` projection:
Linux/Windows Control remains host `command` for shortcut resolution but reaches
an unhandled focused widget as `control`, while Super/Meta reaches it as
`command`; combined and Shift/Alt states remain independent across press,
repeat, and release. Host handling remains first, and handled shortcuts do not
reach widgets. This prerequisite correction has zero impact on the estimates
and does not ship numeric stepping, capture, transactions, or a numeric
consumer.
The generic metadata-aware focused-key routing kernel is now shipped as the
backend-neutral routing authority consumed by complete-mode `KeyboardAdjustment`.
It adds
defaulted object-safe widget opt-in/captured-key queries, one private fixed-size
controller capture record, host-first uncaptured-initial routing, owner-first
continuations and cancellation, stale/orphan/competing ignore behavior, exact
metadata preservation, and conservative refresh reconciliation. Native Vello
normalizes evidence and delegates to the same controller authority; synthetic,
backend-neutral, and native/direct fixtures cover equivalent decisions. Existing
widgets retain the key-only `preempts_host_shortcut_key` compatibility path.
The kernel itself remains generic: the numeric widget supplies the step,
transaction, output, and typed-failure semantics.

The generic pointer-press admission/capture kernel is now shipped as the next
qualified controller foundation. `radiant::widgets::PointerPressAdmission`
provides default-compatible Legacy, managed exact-widget/exact-button capture,
and Blocked admission; the controller validates focus and continuation authority
at dispatch, cancellation, and refresh boundaries and keeps bounded
button-specific orphan-release suppression. Scrollbar/layout precedence,
legacy capture and mapper ownership, pointer metadata, and all existing widget
contracts remain unchanged. This foundation earned no PointerScrub credit by
itself; the later NumericInput consumer below now supplies the bounded
PointerScrub behavior.
The exact-head accepted NumericInput PointerScrub consumer now uses that kernel
for explicitly configured complete-mode widgets. Its 11 deterministic fixtures,
public API and runtime managed-capture coverage, source guardrails, full
all-target/all-feature validation, and pointer-motion performance evidence cover
policy selection, finite in-bounds normalization without clamping, anchor and
step-change reprojection, no-op accumulation, lifecycle rollback, typed initial
and update failures, exact metadata, fixed-capacity output, and wheel fall-through.
This slice updates the estimates by one point in Input/provenance/edit lifecycle,
one point in Text/focus/selection, and five points in Numeric controls:
`860 + 1 + 1 + 5 = 867`, `867 / 11 = 78.8181...%`, reported as approximately
`78.82%`; the generic estimate remains about 97% and the other target gaps
remain unchanged at that earlier cycle.
The accepted generic wheel-routing foundation adds qualified exact line/pixel
samples, default-compatible widget hooks, owner-first managed continuity, and a
bounded Idle/Active/Blocked lifecycle with conservative refresh, focus, removal,
replacement, authority, disabled, and read-only handling. Legacy phase-less hit
testing remains metadata-free while exact samples preserve metadata. Its focused
runtime/custom-widget/public API and guardrail coverage, native compatibility
regression coverage, full library validation, and current-head performance
evidence are shipped. The later accepted NumericInput wheel consumer adds one
point in Input/provenance/edit lifecycle, one point in Text/focus/selection, and
five points in Numeric controls. The resulting evidence-backed total is
`867 + 1 + 1 + 5 = 874`, `874 / 11 = 79.4545...%`, reported as approximately
`79.45%`; the generic estimate remains about 97%.
The numeric interaction output mapping is now shipped for the TextEdit and
complete-mode keyboard lifecycles. It fixes the exact `on_interaction` mapper
type and associated error order, one interaction batch/mapper/host dispatch per
input or teardown boundary, nested TextEdit terminal shapes, bounded keyboard
edit and failure shapes, compatibility-only `on_edit`, mapper exclusivity,
validator acceptance, retiring-mapper mode selection, and generic
host-first/owner-first focused-key routing. The earlier bounded keyboard
consumer slice moved the estimates from 854 to 860; the PointerScrub consumer
above records 867, and the accepted NumericInput wheel consumer records 874.
The accepted native wheel adapter slice now maps winit line and pixel deltas to
validated backend-neutral `WheelSample` values, preserves DPI-adjusted sign,
TouchPhase lifecycle, modifiers, timestamps, and sequence ranges, and routes
phaseful events through the exact wheel seam. Explicit lifecycle boundaries
flush pending phase-less compatibility input; phaseful samples bypass the
vector-only GPU/scroll queues so units and phase cannot be erased, while exact
hit testing remains metadata-neutral. Native Vello and auxiliary-window paths
share the adapter, and focused conversion, routing, full-library, integration,
guardrail, cross-target, and remote CI evidence are shipped. Phase-less callers
retain the existing compatibility/coalescing path, and malformed exact input
retains the sanitized fallback without constructing an invalid exact sample.
This platform slice moves the evidence-backed total from 874 to 875:
`874 + 1 = 875`, `875 / 11 = 79.5454...%`, reported as approximately
`79.55%`; the generic estimate remains about 97%.
