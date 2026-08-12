# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~82.00% (902 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 84% |
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

The broad estimate is the unweighted mean of the category rows:
`(84 + 71 + 97 + 97 + 74 + 92 + 96 + 78 + 71 + 66 + 76) / 11 = 82.0%`,
reported as approximately `82.00%`.
The generic architecture-sequence estimate remains about 97%; this slice adds
the first executable native Winit consumer without changing the estimates.
The broad estimate remains intentionally `902 / 11` (~82.00%) and Public API
remains 84% until shipped validation is complete; no design-only credit is
awarded for this contract or implementation. Other native adapters,
scheduler-, renderer-, and product-policy boundaries remain separate.
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
conservatively cancels. Matching-key suppression, candidate windows, other
native IME adapters, and product behavior remain unshipped. Complete-mode
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
coordinate resolution/custom transforms, final ordering, collision/ID
admission, cross-range deduplication, and semantic-tree work are not
responsibilities of the classifier itself; the private compositor below
consumes its result for the bounded logical-only tree step.

The private automation-tree compositor is now shipped as staged, crate-private
evidence. It consumes already validated classification batches, admits only
`Logical` coordinates, rejects `Custom` before insertion, and normalizes input
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
public snapshot selection/visibility and session ownership boundary. Custom-
coordinate transformation, production/native and product consumers,
scheduler/backoff/fairness, multiple active ranges per container, and public
provider-registration/API wiring remain deferred and unimplemented. The
normative/planned public declarative Logical-only provider contract is recorded
in `docs/VIRTUAL_LAYOUT_DESIGN.md`, but its provider path remains unshipped. The
consumer adds one public-API evidence point: generic ~97%, Declarative identity
71%, layout 97%, and broad coverage `902 / 11` (~82.00%).

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
generation,
data/policy/measurement/semantic revisions, provider identity/generation,
coordinate, budget, cancellation, materialization/classification authority,
ordinary projection generation, and complete-demand-set generation. A result is
accepted only when every required field matches exactly; stale, superseded, and
cancelled results are inert. Provider attempts are non-reentrant and cannot
publish or mutate runtime state directly.

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

The first consumer admits only `Logical`; `Custom` is rejected before provider
invocation with no identity fallback. A future transform contract must define
the owner, source/destination, supported class and revision,
finite/non-inverted conversion, clipping/nesting, and conservative
singular/stale/unsupported/ambiguous behavior before custom coordinates are
admitted. This shipped consumer adds one public-API evidence point: generic
~97%, Declarative identity 71%, layout 97%, and broad coverage `902 / 11`
(~82.00%). Existing pure public snapshot APIs and non-goals remain explicit.

## Planned public declarative provider attachment (normative; implementation unshipped)

The four primary alignment documents now define one public declarative
Logical-only capability for attaching semantic item/range providers. The
qualified proposed vocabulary is
`radiant::application::VirtualLayoutParts<Message>`,
`virtual_layout_from_parts`, `radiant::runtime::VirtualLayoutRevisions`,
`VirtualLayoutSemanticProvider`, `VirtualLayoutSemanticRangeProvider`,
read-only item/range requests, `VirtualLayoutSemanticEntry`, and generic
`VirtualLayoutSemanticProviderOutcome<T>` with `Found`, `NotFound`, `Unavailable`,
`Deferred`, and `Rejected`. These are planned names only, are not shipped
exports, and are not in the prelude. The first boundary is synchronous,
single-threaded `Rc` with no `Send`/`Sync`/worker/scheduler promise and no
custom-coordinate field.

`SurfaceRuntime` will own mounted registration, removal, provider replacement,
registration/mount/provider generations, lifetime cancellation, and exact
source tickets, bounded by 64 registrations, one range and one required-item
slot per container, 1024 entries per query and in aggregate, and at most one
provider call per container/attempt. There is no public imperative registration
API or application-owned mount generation. Only explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` may call providers; registration,
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
`Unmaterialized` authority. Future native accessibility may translate explicit
platform queries only through the backend-neutral session model; it is a
separate later contract and not a hidden provider owner. The full acceptance
matrix and non-goals are in `docs/VIRTUAL_LAYOUT_DESIGN.md`; those non-goals are
custom transforms, native accessibility/tree/actions, focus,
scrolling/materialization, scheduler/backoff/fairness, renderer/paint/
hit-testing/cache policy, product policy, multiple ranges, and prelude export.

This contract is normative/planned and implementation-unshipped. It earns zero
alignment estimate credit; all estimates above remain exactly unchanged,
including broad coverage `902 / 11` (~82.00%) and Public API 84%.

Slider/Knob, platform, scheduler, renderer, and product policy remain out of
scope for this slice.
The Input evidence moves from 96% to 97%, Numeric controls from 87% to 92%,
and Text moves from 73% to 74% for runtime focus/selection admission.
The evidence-backed total remains `902 / 11` (~82.00%) for this alignment
sequence; this native Winit slice does not award estimate credit before shipped
validation. The generic composition foundation, the single-line text consumer,
the NumericInput consumer, and the primary/auxiliary Winit consumer remain
distinct from the deferred matching-key, candidate-window, other native-adapter,
and product-policy boundaries.
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
