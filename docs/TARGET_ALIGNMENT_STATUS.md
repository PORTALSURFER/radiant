# Radiant Target-Alignment Status

This document tracks Radiant's implementation alignment with the normative
target. It is a durable progress snapshot, not a measure of line count,
documentation completeness, or workflow activity. GitHub remains the delivery
record for individual slices and reviews.

## Snapshot

- Snapshot date: **2026-08-07**
- Canonical main: **`e537a07d`**
- Generic architecture-sequence completion: **~97% (92–99%, medium confidence)**
- Broad end-to-end target coverage: **~74.45% (reported as ~74%)**

These are different measurements. Generic architecture-sequence completion
reflects the evidence-backed dependency sequence of shipped generic contracts,
implementation, tests, documentation, and integration evidence. Target-only
prose, diagrams, or examples do not count as shipped implementation; they
establish the contract against which that sequence is measured. Broad end-to-end
target coverage is the unweighted mean of the 11 category scores below:

`(80 + 70 + 82 + 96 + 63 + 46 + 93 + 78 + 70 + 66 + 75) / 11 = 74.45%`,
reported approximately as **~74%**. The ~97% sequence measure is not product
completeness and does not claim full end-to-end coverage or native acceptance.
Because the category rows are point estimates, no separate range is claimed for
the derived broad-coverage measure.

PR #1602 reduced contract
ambiguity and sequencing risk without changing the architecture-sequence
estimate; PR #1603
adds the first executable keyed query capability and PR #1604 adds a private,
query-only visible-window coordinator with accepted-state, logical-index delta,
anchor/fallback, exact-fence, and owner-token evidence. PR #1605 adds a
crate-private materialization/recycling correctness kernel with exact
accepted-commit admission, deterministic keyed lifecycle ordering, success-only
atomic publication, and terminal fail-stop behavior for lifecycle failure.
At that stage, runtime materialization and consumer integration gaps remained.
PR #1606 now ships the normative runtime consumer boundary as a docs-only
contract. PR #1607 ships the first executable private retained-item admission
prerequisite: tuple-scoped fallible lowering, deterministic wrapper identity,
and custom-widget output guarding. PR #1608 ships the complete private
shell-plus-active-batch adapter with exact accepted-key/slot admission and
immutable payload construction. PR #1609 now ships the private synchronous
`SurfaceRuntime` registration/two-pass bridge, including bounded registration
admission, shell-first layout/query sequencing, complete-batch publication
through the existing materialization store, retained-subtree reuse, and
conservative changed/missing/duplicate/terminal handling. PR #1610 now ships
the generic, parser-agnostic `NumericEditSession<T>` draft/transaction
foundation with source-safe terminal boundaries and qualified exports, without
claiming parser, domain, widget, or runtime integration. PR #1611 now ships a
private, per-window native paint-segment benefit ledger with exact committed
full-encode/assembly evidence, checked segment-local count deltas, bounded
history, generation fencing, and recovery clearing, without changing render
selection or cache admission. PR #1612 adds the private fixed-capacity,
generation-fenced admission state that consumes that evidence, requires two
beneficial non-zero-work reuses within an entry-local eight-epoch promotion
window, preserves short low-benefit hysteresis, and remains observational with
no renderer reader. PR #1613 now ships the private plan-index-preserving sparse
artifact residency contract: cardinality is separate from resident count, valid
holes remain uncompactable fixed-capacity slots, exact indexed lookup carries
scene-validity and target-generation fences, and malformed materializations
clear atomically. PR #1614 now makes that sparse contract executable in mixed
assembly: valid absent residents become per-frame fresh encodes, exact residents
reuse, invalid present evidence still vetoes atomically, and committed benefit
evidence uses the derived execution plan. PR #1615 now connects the existing
Warming/Admitted evidence to sparse artifact publication: fully validated dense
batches are filtered by the exact admission tuple, original plan indices and
cardinality remain intact, and full and mixed paths share one frame-state owner.
PR #1616 now makes render-boundary selection an explicit private native Vello
consumer: Warming probes and Admitted residents are selected only when exact
residency is present, while zero-selection and unsafe cases use authoritative
full-scene encoding. The generic architecture-sequence estimate moves
modestly because this closes the
generic render-selection gap without adding renderer-owned GPU lifetime or
budgeting, platform profiling, or product-specific numeric, virtualization, or
cache policy. PR #1617 now ships the generic controller-owned lifecycle
transition authority and bounded `RuntimeLifecycleDiagnostics`: construction,
closing, stopped, and typed recovery transitions are validated and recorded
with saturating counts and fixed-capacity history. PR #1618 now connects the
existing native Vello recovery episode to that authority: accepted recovery
records `Running -> Recovering`, successful primary and auxiliary completion
records `Recovering -> Running`, controller vetoes remain paused and route
through the existing closing path, and recovery itself does not cancel runtime
effects. PR #1619 now scopes controller-managed worker effects emitted from
auxiliary messages to stable auxiliary-window generations: origin survives
parent dispatch, completion mapping, and chained worker commands; destructive
retirement fences only the matching registrations with idempotent pending
accounting, while sibling/application work, cached hide, and recovery remain
admitted. PR #1620 now applies the same private owner/origin fence to
controller-managed timers: origin survives registration, opaque controller-wake
mapping, UI dispatch, and chained commands, while exact-generation retirement
drops matching mapper closures and repairs only matching latest slots. PR #1621
now extends the same private owner/origin fence to platform completions across
result-host acceptance, unsupported and
rejected fallback, both UI delivery paths, chained commands, and exact
auxiliary retirement; host-held sinks remain bounded and late deliveries are
inert before mapping. PR #1622 documented the declarative owner-selection and
cancellation contract, and PR #1623 now ships its first private executable
prerequisite: pre-flattening source identity seeds, independent keyed/overlay
topology, complete source traversal including non-interactive floating
descendants, and persistent authoritative source records across runtime and
virtual-layout projection. PR #1624 now ships the private executable
owner-candidate projection and exact selection resolver: keyed and overlay
identity remains independent, application default/outlive evidence is explicit,
scoped failures reject without fallback, and accepted projection installation
stays on final startup, refresh, relayout, and virtual-materialization
boundaries. PR #1625 now adds checked live generations, exact token fencing,
accepted-projection reconciliation, removal/reinsertion handling, exhaustion
containment, and close retirement without authorizing effects. PR #1626 now
ships the private explicit owner-request consumer: default and application-
outlive requests remain application-owned, accepted live overlay/keyed tokens
become generation-fenced private origins, rejected requests dispatch no work,
and worker, timer, platform, and chained paths recheck liveness before mapping
or reduction. PR #1627 now drains exact retired declarative generations at the
accepted projection boundary and eagerly removes matching worker, timer, and
platform registrations while preserving late-delivery and
sibling/application/later-generation fences. The generic architecture-sequence
estimate moves modestly to
~97% (92–99%); Runtime/effects/scheduling moves from 91% to 93%.
Public selection/cancellation policy, product integration, and scheduling
budgets/fairness remain later or product-dependent.

PR #1629 now ships display-only `ValueFormat` attachment through the official
application Slider and Knob builders: configured automation value text uses the
existing bounded policy, unconfigured controls retain their three-decimal text,
and normalized interaction values, edit batches, low-level constructors, and
public widget shapes remain unchanged. Numeric-controls alignment moves
conservatively from 42% to 46%; the generic architecture-sequence estimate
remains ~97% because parser, domain/range, mapping, numeric-input, focus, and
accessibility integration remain unshipped.

PR #1630 now ships the first bounded public profiling path: `ProfilingMode::Off`
and `Frame`, copyable `ProfilingOptions`, backend-neutral `FrameProfile`
projection, an independent `RuntimeFrameProfileHost` capability, and the
stateful application callback. Profiles are published only after successful
presentation, preserve primary/auxiliary correlation and bounded ordering, and
report GPU timing as explicitly unavailable. PR #1631 adds the checked live
macOS `.app` acceptance matrix for primary and auxiliary successful-present
profiles, Off silence, native close, and native zoom/resize. Diagnostics/
profiling alignment moves conservatively from 58% to 62%, and platform/windowing
alignment moves from 60% to 68%; broad target coverage is `810 / 11 = 73.64%`
(~74%). The generic architecture-sequence estimate remains ~97% because
Detailed profiling, runtime switching, inspector correlation, GPU timestamp
queries, renderer-owned lifetime/budgeting, and broader performance proof remain.

PR #1632 now exposes the existing observational `DevtoolsOverlayOptions`
through the normal stateful and window builders and adds a checked macOS live
devtools harness with ordinary controls, bounded text editing, and inspector
tree/selection/bounds/paint diagnostics. Diagnostics/profiling moves
conservatively from 62% to 66%; broad target coverage is
`814 / 11 = 74.00%` (~74%). The generic architecture-sequence estimate
remains ~97% because Detailed profiling, runtime switching, inspector/frame
correlation, GPU timestamp queries, renderer-owned lifetime/budgeting, and
broader performance proof remain.

PR #1633 now ships the macOS outgoing external-drag completion boundary:
valid launches remain pending until AppKit reports a terminal result, Copy
completes exactly once as accepted, Escape cancellation completes exactly once
as unaccepted, and stale/duplicate/replaced/closed/shutdown results remain
fenced. The public macOS acceptance harness passed live Finder Copy and
cancellation trials at exact head, including one received payload and no
additional payload after cancellation. Platform/windowing alignment moves
conservatively from 68% to 70%; broad target coverage is
`816 / 11 = 74.18%` (~74%). The generic architecture-sequence estimate
remains ~97% because incoming drops, Move/Link negotiation, Linux/Windows
runtime support, renderer-owned budgeting, and product-specific policy remain
outside this slice.

PR #1634 now ships the qualified single-line `TextInputRevision` authority
prerequisite: newer caller revisions apply the projected value, caret, and
selection; equal or older revisions preserve retained editing state; equal-
value newer revisions still apply projected selection; revision-mode changes
are explicit reset boundaries; and unrevisioned inputs retain legacy
value-equality synchronization. Text/focus/selection alignment moves
conservatively from 60% to 63%; broad target coverage is
`819 / 11 = 74.45%` (~74%). The generic architecture-sequence estimate
remains ~97% because this slice does not claim IME/preedit delivery,
composition, multiline editing, bidi behavior, clipboard/undo, or native
accessibility.

Rendering terminal boundary after PR #1616: no further generic rendering
implementation slice is selected from the current Radiant contracts. The
remaining target requires a supported renderer/adapter contract for resource
accounting, ownership, generation, asynchronous GPU timing, and per-resource
fence-pinned retirement, plus a named product workload for active-resource
importance, staleness, and degradation policy. Those decisions must arrive
before rendering-budget implementation resumes; this boundary does not block
the separately sequenced runtime/effects/ownership work.

This snapshot distinguishes architecture readiness from shipped runtime
behavior: the generic private virtualization consumer path is now shipped and
validated through the runtime bridge, but public/product-owned consumers,
`split_pane` semantics, and executable product virtualization proof remain
unshipped.

## Alignment by category

| Category | Alignment | Evidence / status |
| --- | ---: | --- |
| Public API and module boundaries | 80% | Explicit public/module boundaries and prelude hygiene are shipped; the full target surface is not. |
| Declarative model, identity, reconciliation | 70% | Stable identity, revision, and continuity foundations are shipped; complete production reconciliation remains. |
| Input, provenance, and edit lifecycle | 82% | Shared provenance and `EditEvent` lifecycle are adopted by `Slider`, `Knob`, and `PanelResizeState`; the generic parser-agnostic numeric edit-session boundary is now shipped, while broader consumers remain. |
| Layout, composition, virtualization | 96% | Backend-neutral `SplitPaneLayout` geometry, UI-local capability/revision evidence, revision-2 declared hit-region projection/query, generic version-3 layout pointer admission/capture, runtime-owned version-4 typed container state, the qualified query-only keyed virtualization capability, a private query-only keyed visible-window coordinator, a private materialization/recycling correctness kernel, the normative runtime consumer boundary, the tuple-scoped and complete private retained-item adapters, and the private synchronous `SurfaceRuntime` registration/two-pass bridge are shipped in PRs #1597–#1609. Product-specific `split_pane` behavior, public/product-owned virtualization consumers, and executable product virtualization proof remain unshipped. |
| Text, focus, and selection | 63% | Focus and selection foundations plus qualified single-line revision authority are shipped; richer multiline/IME/composition editing and native accessibility remain. |
| Numeric controls | 46% | Finite linear/log `ValueMapping`, deterministic allocation-free `ValueFormat`, parser-agnostic `NumericEditSession<T>`, and display-only `ValueFormat` attachment through the official Slider/Knob application builders are shipped; parser/domain/range policy, mapping, numeric input, and broader control integration are not. |
| Runtime, effects, and scheduling | 93% | PRs #1617–#1627 ship generic lifecycle authority/diagnostics, the native Vello recovery bridge, stable auxiliary-window owner/origin retirement consumers for worker, timer, and platform effects, the private declarative source-topology and owner-generation foundations, the explicit owner-request consumer, and eager exact-generation retirement at the existing worker, timer, and platform registries with conservative late-delivery vetoes. Public/product-facing selection/cancellation policy and scheduling budgets/fairness remain. |
| Rendering, invalidation, retained GPU surfaces | 78% | Revision/damage direction, private committed native paint-segment benefit evidence, bounded observational admission, plan-index-preserving sparse artifact residency, executable mixed assembly, admission-gated sparse publication, and explicit admission-aware render-boundary selection with conservative full-scene fallback are shipped; renderer-owned retained-resource lifetime/budgeting, platform profiling, and product-specific cache policy remain. |
| Platform, windowing, and host boundaries | 70% | macOS host-facing boundaries now have live `.app` acceptance for primary/auxiliary successful presents, independent profiling modes, native auxiliary close, native primary zoom/resize, the normal builder path for the observational devtools overlay, and outgoing Finder drag Copy/cancellation completion; Linux/Windows runtime implementation and validation remain explicitly deferred portability work and do not block this macOS-scoped goal. |
| Diagnostics, profiling, and performance validation | 66% | Bounded diagnostics, public Off/Frame `FrameProfile` delivery, checked live macOS successful-present acceptance, and normal builder exposure plus live inspector evidence for the observational devtools overlay are shipped; Detailed profiling, runtime switching, inspector/frame correlation, GPU timestamps, renderer-owned lifetime/budgeting, and broader performance proof remain. |
| Examples, documentation, and CI guardrails | 75% | Normative docs, API references, tests, and CI guardrails are substantial; target-only examples do not substitute for integration evidence. |

## Shipped foundations

The current foundation includes:

- explicit public/module boundaries and prelude hygiene;
- stable identity, revision, and continuity;
- interaction provenance and the shared `EditEvent` lifecycle adopted by
  `Slider`, `Knob`, and `PanelResizeState`;
- host-facing resize state;
- backend-neutral `SplitPaneLayout` geometry with finite bounds, minima evidence,
  and deterministic undersized fallback (PR #1597);
- UI-local `LayoutCapabilities` registration, typed revision evidence, and
  diagnostic-only container classification that does not authorize refresh or
  reuse (PR #1598);
- revision-2 UI-local declared hit regions with typed identities, deterministic
  clipped projection, stale-safe replacement, and observational
  `SurfaceRuntime` target queries (PR #1599);
- version-3 typed layout pointer admission, bounded event context, exact
  target/revision binding, runtime-owned capture, widget/scrollbar arbitration,
  outside-bounds delivery, conservative cancellation, and no-op move regression
  evidence (PR #1600);
- version-4 runtime-owned typed container state with exact container/type/schema
  identity, bounded lifecycle/capacity diagnostics, capture-state compatibility,
  pre-admission stale-slot pruning, and foreign-declaration rejection (PR #1601);
- the normative keyed virtual-layout/materialization ownership, identity, fence,
  anchor, lifecycle, and bounded-work contract (PR #1602; design evidence only,
  with no executable runtime/API change);
- the qualified `radiant::layout::VirtualLayoutPolicy` and
  `VirtualLayoutQueryExecutor` query-only capability with bounded output,
  typed outcomes/diagnostics, exact fence acceptance, and stable identity
  rejection (PR #1603; runtime registration and materialization remain future);
- the crate-private `VirtualLayoutWindowCoordinator` with exact scope,
  revision, owner-token, and one-shot execution fences; bounded accepted-window
  reconciliation; emission-order-independent logical-index deltas; conservative
  same-key anchor/fallback evidence; and foreign-token/reentrancy regression
  coverage (PR #1604; runtime materialization, recycling, and consumer
  integration remain future);
- the crate-private `VirtualLayoutMaterializationStore` with exact accepted-
  commit admission, pure host projection, keyed slot identity/generation,
  deterministic unmount/reset/reconcile/mount ordering, reset-only recycling,
  success-only atomic publication, terminal fail-stop lifecycle retirement,
  bounded diagnostics, and coordinator-local malformed-commit coverage (PR
  #1605; runtime consumer registration, replacement/recovery policy, and
  product integration remain future);
- the crate-private tuple-scoped, fallible retained-item admission prerequisite
  with deterministic wrapper identity, declarative and custom-widget output
  guards, typed identity/scene rejection, and panic containment (PR #1607);
- the crate-private complete retained-item batch adapter with exact accepted
  key/slot matching, whole-shell-plus-active-batch identity admission,
  bounded slot-scope validation, immutable `SurfaceNode` payloads, and
  recoverable pre-lifecycle rejection (PR #1608); and
- the private synchronous `SurfaceRuntime` virtual-layout registration and
  two-pass bridge with bounded registration admission, shell-first
  layout/query sequencing, complete-batch publication through the existing
  materialization store, retained-subtree reuse, and conservative
  changed/missing/duplicate/terminal handling (PR #1609); and
- the qualified, parser-agnostic `NumericEditSession<T>` with verbatim draft
  replacement, one shared `EditEvent::Begin`, source-safe caller-certified
  commit/cancel boundaries, foreign-source preservation, and no common-prelude
  export (PR #1610); and
- the private, per-window `NativePaintSegmentBenefitLedger` with exact
  committed full-encode and retained/mixed-assembly outcomes, checked
  segment-local Vello-count evidence, bounded observation history, conservative
  malformed/mixed-generation clearing, and target/recovery invalidation (PR
  #1611); and
- the private, fixed-capacity `NativePaintSegmentCacheAdmission` state machine
  with exact latest-frame projection, generation/epoch fencing, two-reuse
  non-zero-work promotion inside an entry-local eight-epoch window, bounded
  warming hysteresis, conservative malformed/unavailable/veto clearing, and
  target/recovery invalidation (PR #1612); and
- the private plan-index-preserving native paint artifact residency contract
  with separate plan cardinality, non-compacting fixed-capacity slots, exact
  indexed reuse/assembly fences, atomic malformed-state clearing, and sparse
  hole/zero-resident regression coverage (PR #1613); and
- the private mixed native paint assembly consumer that distinguishes exact
  residents, valid bounded absences, and invalid evidence; fresh-encodes only
  supported sparse holes, carries the derived execution plan through commit,
  and records actual fresh/reuse benefit evidence without changing admission
  policy (PR #1614); and
- the private admission-gated native paint artifact publication consumer that
  validates complete dense batches before filtering by the exact
  Warming/Admitted identity, span, revision, and target-generation tuple,
  preserves original sparse slots and nonzero plan cardinality, clears invalid
  state atomically, and routes full and mixed publication through one
  frame-state owner (PR #1615); and
- the private admission-aware native Vello render-boundary selector that
  intersects exact Warming/Admitted evidence with sparse residency and scene
  fences, keeps Warming probes reachable, attempts mixed assembly only when a
  resident is selected, and falls back to authoritative full-scene encoding
  for zero-selection or unsafe cases (PR #1616); and
- finite linear/log `ValueMapping`; and
- deterministic, allocation-free `ValueFormat`; and
- normal stateful/window builder `devtools_overlay(DevtoolsOverlayOptions)`
  setters plus a macOS-only live devtools acceptance harness with ordinary
  controls, bounded text input, and inspector projection tests (PR #1632); and
- the private macOS outgoing external-drag completion boundary with bounded
  pending/session state, exact WindowId-plus-identity routing, one-shot Copy
  and cancellation delivery, stale/duplicate/replacement/shutdown fencing,
  and live Finder acceptance coverage (PR #1633); and
- the qualified `radiant::widgets::TextInputRevision` authority prerequisite
  through the single-line application builder, with newer/equal/older,
  equal-value selection, identity, mode-transition, legacy, and clear-button
  propagation evidence (PR #1634); and
- the private generic `RuntimeLifecycleController` with validated
  `Starting`/`Running`/`Recovering`/`Closing`/`Stopped` transitions and
  qualified, bounded `RuntimeLifecycleDiagnostics` with saturating counts and
  fixed-capacity history (PR #1617).
- the crate-private native Vello recovery bridge that couples accepted native
  recovery admission and primary/auxiliary completion to the generic lifecycle
  authority, preserves effect ownership during recovery, propagates controller
  vetoes into the existing bounded native closing path, and records focused
  round-trip and veto evidence (PR #1618).
- the crate-private auxiliary worker-effect owner/origin bridge that preserves
  stable window generations through parent dispatch, worker completion mapping,
  and chained commands, and retires only matching worker registrations with
  idempotent pending accounting (PR #1619).
- the crate-private auxiliary timer-effect owner/origin bridge that preserves
  stable window generations through timer registration, opaque controller-wake
  mapping, UI dispatch, and chained commands, and retires only matching timer
  registrations with mapper cleanup and latest-slot repair (PR #1620).
- the crate-private auxiliary platform-completion owner/origin bridge that
  preserves stable window generations through result-host acceptance, fallback,
  both UI delivery paths, and chained commands, and retires only matching
  platform mappers without changing shared ingress accounting (PR #1621).
- the crate-private declarative source-topology bridge that preserves
  pre-flattening source identity seeds and independent keyed/overlay ancestry,
  records every canonical source node including non-interactive floating
  descendants, and keeps authoritative reusable source records through
  startup, refresh, virtual-layout, and geometry projection (PR #1623).
- the crate-private declarative owner projection and exact selection resolver
  that normalizes independent keyed/overlay candidates by structural scope plus
  compatibility, preserves source-local eligibility separately from accepted
  projection, rejects stale/incompatible scoped selections without fallback,
  and installs only final authoritative evidence (PR #1624).
- the crate-private declarative owner-generation ledger that preserves exact
  tokens for compatible accepted reprojection, retires removal/ambiguity and
  incompatible replacement, allocates checked fresh generations on
  reinsertion, fences runtime instances, and retires all live clones at close
  (PR #1625).
- the private explicit declarative owner-request consumer that keeps default and
  application-outlive work application-owned, converts accepted live
  overlay/keyed generations into private effect origins, vetoes rejected and
  retired owners before update/command reduction, and carries the origin through
  worker, timer, platform, and chained paths with focused late-delivery tests
  (PR #1626); and
- the private accepted-projection retirement handoff that records exact
  declarative generations retired by reconciliation and eagerly removes only
  matching worker registrations/pending admissions, timer registrations/latest
  slots, and platform-completion mappers, with stream closure, mapper-drop,
  late-delivery, sibling, application, later-generation, and fail-closed
  regression evidence (PR #1627).

These foundations make later slices safer and more composable. They do not
mean that every target consumer, runtime path, platform, or integration is
complete.

## Architecture readiness versus shipped runtime behavior

The normative consumer contract shipped in PR #1606 is now backed by the
private runtime bridge shipped in PR #1609. `SurfaceRuntime` owns one bounded
materialization record per mounted virtual-container generation, discovers
registration evidence from the projected shell, sequences shell layout before
the exact query and complete batch admission, and installs the committed
shell-plus-active subtree through the existing materialization store. The
bridge also proves unchanged refresh reuse and conservative missing, changed,
duplicate, deferred, unavailable, and terminal paths. It remains intentionally
private and has no product-owned lifecycle or registration consumer yet.

PR #1607 remains the item-level admission prerequisite and PR #1608 the
complete private batch adapter on which the bridge depends. PR #1610 also
establishes the generic numeric editing-session boundary, but no public
registration/API or capability contract version has changed; product-specific
virtualization consumers, `split_pane` interaction/state/ratio semantics, and
numeric parser/domain policy remain contract-dependent. PR #1611 adds the
native committed paint-segment benefit evidence needed before measured cache
admission, PR #1612 adds the bounded private observational admission state
derived from that evidence, and PR #1613 adds the indexed sparse
artifact-residency contract consumed by exact reuse and retained assembly. PR
#1614 now consumes valid sparse holes in the mixed assembly path, preserves
exact indexed reuse, vetoes invalid present evidence, and commits the derived
execution plan for factual benefit evidence. PR #1615 now filters fully
validated materializations through the current admission state, publishes only
exact eligible residents into their original sparse slots, and uses one owner
for full and mixed publication. PR #1616 now consumes that state at the native
Vello render boundary: exact Warming/Admitted residents can select mixed
assembly, while no-selected-resident and unsafe paths use the authoritative
encoder. These slices still do not authorize presentation, own GPU resource
lifetime or budgeting, or provide renderer/platform profiling or product cache
policy. PR #1617 makes the generic controller lifecycle explicit through one
validated transition authority and bounded diagnostics, and PR #1618 now
connects that authority to the existing native Vello recovery episode. PR #1619
adds the first worker-effect owner/origin consumer: an auxiliary window's
generation survives parent dispatch, worker completion mapping, and chained
commands, while destructive retirement fences matching registrations without
splitting the global ingress. PR #1620 extends that owner fence to timer
registration, controller-wake mapping, UI dispatch, and chained timers, with
exact-generation retirement and latest-slot repair. Platform-completion owner
integration is now shipped in PR #1621: result-host acceptance, unsupported and
rejected fallback, direct and queue delivery, and chained platform commands
preserve exact origin, while retirement detaches only matching mappers without
changing shared ingress accounting. PR #1623 now ships the private source-
topology prerequisite: extracted overlay roots retain their original
declarative scope and keyed ancestry while final runtime IDs remain unchanged,
source traversal covers every visibly laid-out node, and persistent runtime
scratch/probe ownership stays authoritative across startup, refresh, cache, and
geometry paths. PR #1624 now projects that accepted topology into independent
keyed/overlay candidate catalogs and resolves exact private selections with
typed rejection and no application/ancestor fallback; provisional probes cannot
replace accepted evidence. PR #1625 now adds checked live generations,
runtime-instance-safe tokens, deterministic accepted-projection reconciliation,
fresh reinsertion generations, exhaustion containment, and close retirement.
PR #1626 now consumes explicit requests as private live origins, keeps default
and application-outlive work application-owned, vetoes rejected/retired owners
before update and command reduction, and carries exact origin through worker,
timer, platform, and chained paths. PR #1627 now consumes reconciliation's
exact retired-token handoff at the accepted projection boundary and eagerly
retires matching registrations in the existing registries before later mapping
or reduction. The remaining runtime ownership work requires product-facing
selection/cancellation policy; the sequence still does not add configurable
budgets, fairness, or synthetic GPU-host acceptance.

PR #1633 now closes the macOS outgoing-drag host boundary through the existing
UI-owned completion mapper: admission is pending, AppKit's terminal callback
maps Copy or cancellation once, exact window/identity routing isolates primary
and auxiliary owners, and stale or post-shutdown deliveries are inert. Live
Finder evidence covers both accepted Copy and Escape cancellation. Incoming
drop semantics, Move/Link negotiation, and non-macOS runtime support remain
explicitly outside the current product scope.

## Remaining gaps, ordered by leverage

1. **Product-specific virtualization consumers and product proof.**
   PR #1602 landed the normative contract, PR #1603 shipped the bounded
   query-only capability, PR #1604 shipped private accepted-window
   reconciliation with logical-index deltas, conservative anchor/fallback
   evidence, and exact owner/revision fences, and PR #1605 shipped the private
   bounded materialization/recycling correctness kernel. PR #1607 provided
   the tuple-scoped item-level admission prerequisite, including custom-widget
   output guarding, and PR #1608 completed the private retained-item batch
   adapter with whole-shell/active-batch admission and immutable payloads, and
   PR #1609 shipped the private runtime registration/two-pass bridge that
   connects accepted adapter output to one runtime-owned materialization record
   without scheduling or product state. Remaining work here is product-owned
   virtualization/materialization proof and `split_pane` interaction,
   state, and ratio semantics; do not invent those contracts in generic Radiant.
   PR #1601 supplies the generic runtime state lifecycle if a product contract
   later makes those slices reasonable.
2. **Numeric and input integration.** Display-only `ValueFormat` attachment
   is now shipped through the official Slider/Knob application builders, with
   normalized interaction values and edit events intentionally unchanged.
   Remaining work is `ValueMapping`/range/domain integration, `numeric_input`,
   widget/runtime/focus integration, and the pointer, keyboard, and
   accessibility domain contract around those paths. Parser, locale, range,
   formatting, and product interaction policy must come from a concrete
   consumer rather than being invented in generic Radiant.
3. **Runtime, effects, and scheduling integration.** PRs #1617–#1625 ship
   generic lifecycle authority/diagnostics, the bounded native Vello
   recovery/effect-preservation bridge, and stable auxiliary-window
   owner/origin retirement for worker, timer, and platform effects: accepted
   recovery and successful primary/auxiliary completion are coupled to
   controller state, recovery preserves effects, and destructive auxiliary
   retirement fences only matching registrations with mapper cleanup and
   latest-slot repair across the shipped deferred lanes. PR #1623 also ships
   the private source-topology/retention prerequisite: pre-flattening
   declarative identity survives lowering, independent keyed and overlay
   candidates survive complete source traversal, and authoritative reusable
   records survive startup, refresh, virtual-layout cache, and geometry paths.
   PR #1624 adds private accepted owner-candidate projection and exact
   selection resolution with independent keyed/overlay candidates, explicit
   application default/outlive outcomes, typed no-fallback rejection, and
   provisional-probe isolation. PR #1625 adds checked live-generation
   reconciliation, exact runtime-instance tokens, fresh reinsertion fencing,
   exhaustion containment, and close retirement without effect authorization.
   PR #1626 now consumes the explicit request contract as a private live origin,
   keeps default/outlive work application-owned, vetoes rejected/retired owners
   before reduction, and carries worker/timer/platform/chained origin evidence
   through existing paths. PR #1627 now eagerly retires matching declarative
   registrations at reconciliation, so late deliveries are rejected before
   mapping and retained registrations are cleaned up at the owning registries.
   The remaining public selection/cancellation policy requires a concrete
   product consumer; configurable scheduling budgets and fair multi-window
   policy follow only after that product-facing ownership boundary is concrete.
   No further generic runtime slice is selected from the current contracts.
4. **Richer text editing.** PR #1634 ships the qualified single-line
   `TextInputRevision` authority prerequisite: newer projected revisions apply
   value/caret/selection, equal or older revisions cannot overwrite retained
   editing state, and switching revisioned/unrevisioned modes is an explicit
   reset boundary. Complete multiline editing, IME/composition delivery, and
   native accessibility semantics remain; those require a concrete macOS
   text-adapter and product/document contract rather than generic revision
   plumbing alone.
5. **Production frame wiring.** Complete reconciliation, damage propagation,
   selective production publication from the bounded admission signal,
   render-boundary selection, and measured retained-surface cache admission in
   the production runtime path. PR #1611 supplies bounded committed native
   paint-segment benefit evidence, PR #1612 supplies the bounded private
   observational admission signal, and PR #1613 supplies the exact
   plan-index-preserving sparse residency contract, PR #1614 makes valid
   sparse absence executable as fresh encoding during mixed assembly while
   recording factual benefit evidence, PR #1615 supplies the
   admission-to-residency publication consumer for exact Warming/Admitted
   residents, and PR #1616 supplies the admission-aware render-boundary
   selector with authoritative full-scene fallback. **Terminal boundary for
   the current generic sequence:** no implementation PR is selected for the
   remaining renderer-owned retained-resource lifetime/budgeting and measured
   renderer/platform profiling because current Radiant/Vello contracts do not
   expose the required accounting, ownership, asynchronous timing, or
   per-resource retirement evidence. Resume only after that renderer contract
   and a named product workload define importance, staleness, pressure, and
   degradation policy; product-specific cache policy remains outside generic
   Radiant.
6. **Profiling and performance proof.** PR #1630 ships the first bounded public
   `ProfilingMode::{Off, Frame}` and `FrameProfile` path with successful-present
   publication and explicit GPU-timing unavailability; PR #1631 supplies the
   checked live macOS primary/auxiliary acceptance matrix; and PR #1632 exposes
   the existing observational devtools overlay through normal application and
   window builders, with a live macOS inspector harness covering tree,
   selection, bounds, paint diagnostics, ordinary controls, and bounded text
   input. Remaining work is `Detailed(ProfileSelection)`, runtime mode
   switching, inspector/frame correlation, backend GPU timestamp queries,
   renderer-owned lifetime/budgeting, and broader performance validation.
   These require renderer/platform contracts or a broader named workload.
7. **Platform portability (deferred).** The current macOS native support and
   public profiling, devtools, and outgoing-drag acceptance boundaries are now
   live-validated. Linux/Windows runtime implementation and validation remain
   explicitly deferred portability work behind the existing boundaries and do
   not block the current macOS-scoped support goal. Incoming drops, Move/Link
   negotiation, and product-specific native accessibility/control semantics
   remain tracked with their text/input and product contracts.

dB, tempo, and other custom numeric formats remain later work after the
parser-agnostic edit-session, display attachment, and generic numeric
integration contracts are established.

## Evidence map

The target and implementation evidence for this snapshot is mapped here:

- [Project target](../docs/TARGET.md)
- [Normative design direction](../docs/DESIGN_DIRECTION.md)
- [Architecture and ownership map](../docs/ARCHITECTURE.md)
- [Current public API](../docs/API.md)
- [Normative virtual-layout design](../docs/VIRTUAL_LAYOUT_DESIGN.md)
- [Query-only virtual-layout capability](../src/gui/layout_core/virtual_layout.rs)
- [Query-only visible-window coordinator](../src/gui/layout_core/virtual_layout/coordinator.rs)
- [Private materialization/recycling kernel](../src/gui/layout_core/virtual_layout/materialization.rs)
- [Private retained-item admission prerequisite](../src/application/view_node/virtual_layout.rs)
- [Private retained-item batch adapter](../src/gui/layout_core/virtual_layout/adapter.rs)
- [Private runtime virtual-layout bridge](../src/runtime/controller/virtual_layout.rs)
- [Declarative source identity and overlay extraction](../src/application/view_node/identity.rs)
- [Declarative extracted-layer topology](../src/application/view_node.rs)
- [Declarative source-aware lowering](../src/application/view_node/lowering.rs)
- [Private runtime source traversal](../src/runtime/surface/source.rs)
- [Persistent runtime source projection scratch](../src/runtime/controller/scratch.rs)
- [Native paint-segment benefit evidence](../src/gui_runtime/native_vello/generic_runtime/retained_paint_segments/benefit.rs)
- [Native paint-segment admission state](../src/gui_runtime/native_vello/generic_runtime/retained_paint_segments/admission.rs)
- [Native paint artifact residency](../src/gui_runtime/native_vello/generic_runtime/scene/artifact_materialization.rs)
- [Native paint artifact publication owner](../src/gui_runtime/native_vello/generic_runtime/frame_state.rs)
- [Native paint render-boundary selection](../src/gui_runtime/native_vello/generic_runtime/retained_paint_segments/selection.rs)
- [Query capability public tests](../tests/virtual_layout_public_api.rs)
- [Edit lifecycle and provenance](../src/widgets/interaction/edit.rs)
- [Numeric edit session](../src/widgets/interaction/numeric_edit.rs)
- [Value mapping](../src/widgets/interaction/value.rs)
- [Value formatting](../src/widgets/interaction/format.rs)
- [Generic lifecycle diagnostics](../src/runtime/diagnostics/lifecycle.rs)
- [Public frame profiling model](../src/runtime/diagnostics/profile.rs)
- [Frame profiling host capability](../src/runtime/bridge/capabilities/diagnostics.rs)
- [macOS frame profiling acceptance harness](../examples/macos_frame_profile_acceptance.rs)
- [Stateful devtools overlay builder](../src/application/launch/stateful/builder.rs)
- [Window devtools overlay builder](../src/application/launch/window.rs)
- [macOS devtools overlay acceptance harness](../examples/macos_devtools_acceptance.rs)
- [macOS external-drag acceptance harness](../examples/macos_external_drag_acceptance.rs)
- [External-drag lifecycle contract](../src/gui_runtime/native_vello/generic_runtime/external_drag.rs)
- [External-drag platform routing](../src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs)
- [macOS external-drag source bridge](../src/gui_runtime/native_vello/generic_runtime/external_drag/macos/source.rs)
- [Native recovery/controller bridge](../src/gui_runtime/native_vello/generic_runtime/runner.rs)
- [Controller recovery lifecycle boundary](../src/runtime/controller/state/lifecycle.rs)
- [Runtime owner and auxiliary generation fence](../src/runtime/controller/owner.rs)
- [Worker effect owner/origin routing](../src/runtime/controller/effects.rs)
- [Timer effect owner/origin routing](../src/runtime/controller/timers.rs)
- [Platform completion owner/origin routing](../src/runtime/controller/platform.rs)
- [Platform completion host registration](../src/runtime/controller/host.rs)
- [Controller command dispatch and queue drain](../src/runtime/controller/commands/dispatch.rs)
- [Auxiliary message-origin handoff](../src/gui_runtime/native_vello/generic_runtime/auxiliary.rs)
- [Widget revision contract](../src/widgets/contract/revision.rs)
- [Refresh, identity, and damage controller](../src/runtime/controller/refresh.rs)

`ARCHITECTURE.md` warns that target-state boundaries are not proof of
implementation. The evidence map therefore distinguishes normative contracts
from concrete shipped source, tests, and integration evidence; a target API or
example is not promoted to shipped status without that supporting proof.

## Update protocol

After each merged alignment slice:

1. update the snapshot date and canonical main SHA;
2. update the generic architecture-sequence completion and its working range;
3. recompute broad end-to-end target coverage from the 11 category scores and
   record the arithmetic; do not treat it as product or native-acceptance
   completeness;
4. update the affected category score and evidence/status;
5. move an item only when code, tests, documentation, and integration justify
   the change;
6. record the next dependency-correct gap; and
7. keep GitHub as the delivery record rather than duplicating its workflow
   ledger here.

### Initial entry

Historical values in this table are generic architecture-sequence completion
measurements. The broad end-to-end target-coverage mean is reported only for
the current snapshot because prior snapshots did not record that derived metric.

| Date | Canonical main | Architecture-sequence completion | Note |
| --- | --- | ---: | --- |
| 2026-08-05 | `b6991a3a` | ~63% (56–70%, medium confidence) | Initial durable target-alignment snapshot; the next recommended gap is the unshipped parser-agnostic `NumericEditSession`. |
| 2026-08-05 | `3207bd7e` | ~63% (56–70%, medium confidence) | PR #1597 shipped backend-neutral split-pane geometry; layout alignment moves from 60% to a conservative 61%, while runtime interaction, `split_pane`, and virtualization proof remain. |
| 2026-08-05 | `d09d7613` | ~64% (57–71%, medium confidence) | PR #1598 shipped UI-local layout capability registration and diagnostic-only revision evidence; layout alignment moves to 63%, with hit-region projection next. |
| 2026-08-05 | `972a7936` | ~65% (58–72%, medium confidence) | PR #1599 shipped declared hit-region projection/query evidence; layout alignment moves to 67%, with pointer routing/capture/state/events next. |
| 2026-08-05 | `6f404ab6` | ~70% (63–77%, medium confidence) | PR #1600 shipped generic version-3 layout pointer admission and runtime-owned capture with exact-revision rebinding, conservative cancellation, arbitration, metadata, and regression evidence; remaining layout work is product-specific `split_pane` behavior and full virtualization proof. |
| 2026-08-05 | `16337d22` | ~73% (67–79%, medium confidence) | PR #1601 shipped runtime-owned version-4 typed container state with exact identity, bounded lifecycle/capacity diagnostics, capture compatibility, stale-slot pruning, and foreign-declaration rejection; next generic gap is the VirtualLayoutPolicy/materialization contract while product-specific `split_pane` behavior remains deferred. |
| 2026-08-05 | `b3da6a04` | ~73% (67–79%, medium confidence) | PR #1602 landed the normative keyed virtual-layout/materialization contract and makes the next dependency-correct item the query-only `VirtualLayoutPolicy` capability; the estimate stays unchanged because no executable virtualization behavior shipped. |
| 2026-08-05 | `4b6e1118` | ~74% (68–80%, medium confidence) | PR #1603 shipped the qualified query-only `VirtualLayoutPolicy`/`VirtualLayoutQueryExecutor` capability with bounded output, typed outcomes, exact fences, stable identity rejection, and public/guardrail tests; layout alignment moves to 82%, while keyed visible-window reconciliation and later materialization remain. |
| 2026-08-05 | `0789ea49` | ~76% (70–82%, medium confidence) | PR #1604 shipped the private query-only `VirtualLayoutWindowCoordinator` with bounded accepted-window reconciliation, logical-index/emission-order-independent deltas, conservative same-key anchor/fallback behavior, exact revision/owner-token fences, and full local/CI validation; the next generic gap is runtime materialization/recycling consumer integration, while authoritative removed-anchor evidence and product-specific `split_pane` behavior remain deferred. |
| 2026-08-05 | `4d2718e7` | ~77% (71–83%, medium confidence) | PR #1605 shipped the private query-only materialization/recycling correctness kernel with exact accepted-commit admission, keyed slot/generation continuity, deterministic lifecycle ordering, success-only atomic publication, terminal fail-stop lifecycle retirement, bounded diagnostics, and full local/CI validation; the next generic gap is runtime consumer registration/adaptation into retained `ViewNode`/`SurfaceNode` construction, while replacement/recovery policy and product-specific `split_pane` behavior remain deferred. |
| 2026-08-06 | `0bf39879` | ~77% (71–83%, medium confidence) | The normative runtime consumer boundary was contract-ready on the PR #1606 base; direct registration remained blocked on the private retained-item admission adapter and a separate `SurfaceRuntime` registration/two-pass bridge; no runtime behavior, public API, or capability version changed. |
| 2026-08-06 | `21e7b62a` | ~77% (71–83%, medium confidence) | PR #1606 merged the docs-only normative runtime consumer boundary with exact `SurfaceRuntime` ownership, synchronous shell/item sequencing, batch identity admission, fallible scoped lowering, immutable retained payloads, and explicit close/failure rules. The estimate remains unchanged because direct runtime registration and retained-item admission are still unshipped; the next dependency-correct item is the private retained-item adapter. |
| 2026-08-06 | `d63a7f32` | ~77% (71–83%, medium confidence) | PR #1607 merged the private tuple-scoped retained-item admission prerequisite with deterministic wrapper identity, declarative/custom-widget output guards, typed rejection, panic containment, and focused/full validation. The estimate remained unchanged because whole-batch admission, materialization integration, and direct runtime registration were still unshipped; the next dependency-correct item was the complete private retained-item batch adapter. |
| 2026-08-06 | `bd296da3` | ~79% (73–85%, medium confidence) | PR #1608 merged the private complete retained-item batch adapter with exact accepted-key/slot matching, whole-shell-plus-active-batch identity admission, deterministic tuple-scoped wrappers, immutable `SurfaceNode` payloads, typed recoverable rejection, 57 focused tests, full local validation, green CI, and exact-head Terra APPROVE. The estimate moves modestly because materialization-store integration and the separate `SurfaceRuntime` registration/two-pass bridge remain unshipped; that bridge is the next dependency-correct item. |
| 2026-08-06 | `ee861887` | ~83% (78–88%, medium confidence) | PR #1609 merged the private synchronous `SurfaceRuntime` virtual-layout registration/two-pass bridge with bounded registration admission, shell-first layout/query sequencing, complete-batch publication through the existing materialization store, retained-subtree reuse that skips unchanged post-cache projection/layout, conservative missing/changed/duplicate/deferred/unavailable/terminal handling, and same-ID virtual-to-ordinary regression evidence. Full local validation and green required CI passed with exact-head Terra APPROVE. Generic runtime consumer integration is now shipped; the next generic dependency-correct item is the parser-agnostic `NumericEditSession`, while product-specific virtualization and `split_pane` semantics remain product-dependent. |
| 2026-08-06 | `d05f6e8e` | ~84% (79–89%, medium confidence) | PR #1610 merged the qualified, parser-agnostic `NumericEditSession<T>` with verbatim draft preservation, one shared `EditEvent::Begin`, source-safe same-source commit/cancel, foreign-source session preservation, generic-domain/public-API coverage, full local validation, green required CI, and exact-head Terra APPROVE. Numeric-control alignment moves from 35% to a conservative 42%; the next numeric integration requires concrete parser/domain/widget policy, while product-specific virtualization and `split_pane` semantics remain product-dependent. |
| 2026-08-06 | `99e44373` | ~85% (80–90%, medium confidence) | PR #1611 merged the private, per-window native paint-segment benefit ledger with exact committed full-encode and retained/mixed-assembly outcome evidence, checked segment-local count deltas, bounded deterministic history, conservative malformed/mixed-generation handling, and target/recovery clearing. Focused and full local validation passed, required `quality` and `windows-compile` CI passed, and exact-head Terra APPROVE followed one test-only generation-fence correction. Rendering alignment moves conservatively from 60% to 63%; render-boundary selection and measured cache admission remain the next generic dependency-correct items, while product-specific virtualization/numeric policy remains product-dependent. |
| 2026-08-06 | `5eaabe50` | ~86% (81–91%, medium confidence) | PR #1612 merged the private fixed-capacity native paint-segment admission state and exact latest-frame projection. It requires two beneficial non-zero-work reuses within an entry-local eight-epoch warming window, clears malformed/unavailable/veto/generation/recovery evidence conservatively, preserves admitted-state short-burst hysteresis, and adds 12 focused policy tests. Focused and full local validation passed, required `quality` and `windows-compile` CI passed, and fresh exact-head Terra APPROVE followed one bounded-window correction. Rendering alignment moves conservatively from 63% to 66%; the next generic item is the plan-index-preserving sparse artifact-residency contract and enforcement, while production render-boundary selection, measured wiring, and product-specific policy remain. |
| 2026-08-06 | `364cfb63` | ~87% (82–92%, medium confidence) | PR #1613 merged the private plan-index-preserving sparse native paint artifact-residency contract with separate plan cardinality, non-compacting fixed-capacity slots, exact indexed reuse/assembly fences, atomic malformed-state clearing, and sparse-hole/zero-resident regression coverage. Focused and full local validation passed, Linux and Intel-macOS no-default-feature checks passed after installing the target standard libraries, required `quality` and `windows-compile` CI passed, and independent Terra APPROVE found no findings. Rendering alignment moves conservatively from 66% to 69%; the next generic item is the admission-to-residency consumer for selective sparse publication and mixed-assembly fresh encoding, while render-boundary selection, measured wiring, and product-specific policy remain. |
| 2026-08-06 | `d5cf572d` | ~88% (83–93%, medium confidence) | PR #1614 merged the private mixed native paint assembly consumer: valid sparse absences become supported per-frame fresh encodes, exact residents reuse, present corruption and unsupported holes veto atomically, and the derived execution plan supplies factual benefit evidence while admission remains observational. Focused and full local validation passed, Linux and Intel-macOS no-default-feature checks passed, corrected `perf_harness` baseline/compare matched 2/2 scenarios with 0 slower, required `quality` and `windows-compile` CI passed, and independent Terra APPROVE found no findings. Rendering alignment moves conservatively from 69% to 72%; the next generic item is selective admission-to-residency publication, while render-boundary selection, measured wiring, and product-specific policy remain. |
| 2026-08-06 | `63a83359` | ~89% (84–94%, medium confidence) | PR #1615 merged the private admission-to-residency consumer: exact Warming/Admitted identity, span, revision, and generation evidence now filters fully validated dense batches into their original sparse slots, preserves nonzero plan cardinality and atomic clearing, and routes full/mixed publication through one frame-state owner. Focused and full local validation passed, Linux and Intel-macOS no-default-feature checks passed, `perf_harness` baseline/compare matched 2/2 scenarios with 0 slower, required `quality` and `windows-compile` CI passed, and independent Terra APPROVE found no findings. Rendering alignment moves conservatively from 72% to 75%; the next generic item is render-boundary selection and measured retained-surface wiring, while GPU lifetime ownership and product-specific policy remain. |
| 2026-08-06 | `0e0b26ed` | ~90% (85–95%, medium confidence) | PR #1616 merged the private admission-aware native Vello render-boundary selector: exact Warming/Admitted evidence is intersected with sparse residency and scene/generation fences, mixed assembly requires at least one exact resident, valid holes/unselected entries remain fresh in original order, and zero-selection or unsafe cases use authoritative full-scene encoding. Focused and full local validation passed with 3,570 all-target/all-feature tests, Linux and Intel-macOS no-default-feature checks passed, the two Vello strategy probes reported 1,024/0 versus 256/4 work per iteration, the focused JSONL baseline round trip matched 1/1 with 0 slower, required `quality` and `windows-compile` CI passed, and independent Terra APPROVE found no findings. Rendering alignment moves conservatively from 75% to 78%; the next candidate is renderer-owned retained-resource lifetime/budgeting and measured renderer/platform profiling, while product-specific cache policy remains outside generic Radiant. |
| 2026-08-06 | `bcb1311f` | ~91% (86–96%, medium confidence) | PR #1617 merged the generic controller-owned lifecycle transition authority and bounded `RuntimeLifecycleDiagnostics`: legal construction/close/stop transitions are recorded, typed recovery vocabulary is available for the next native slice, invalid transitions are vetoed, and saturating sequence/counts plus fixed-capacity oldest-to-newest history are exposed under `radiant::runtime`. Focused and full local validation passed, including 2,555 library tests, 288 generic guardrails, examples, docs, all-target/all-feature checks, strict Clippy, and installed Linux/Intel-macOS no-default-feature checks; fresh exact-head Terra APPROVE found no findings after one fixed-capacity correction. Runtime/effects/scheduling alignment moves conservatively from 65% to 68%; the next candidate is native recovery/effect preservation, followed by stable owner/origin and cancellation contracts before scheduler budgets or fairness. |
| 2026-08-06 | `5d9bf7d6` | ~92% (87–97%, medium confidence) | PR #1618 merged the crate-private native Vello recovery bridge: accepted native recovery records `Running -> Recovering`, successful primary and auxiliary completion records `Recovering -> Running`, controller-closing vetoes leave native recovery paused and flow into the existing bounded shutdown path, and recovery does not cancel runtime effects on entry. Fresh exact-head validation passed with 734 native generic-runtime tests, 2,555 library tests plus integration targets, 288 guardrails, all-target/all-feature check, strict Clippy, formatting, and diff checks; Terra APPROVE followed one required split-brain correction. Runtime/effects/scheduling alignment moves conservatively from 68% to 72%; the next candidate is stable owner/origin/cancellation integration before scheduler budgets or fairness. |
| 2026-08-06 | `ed2bfcb7` | ~93% (88–98%, medium confidence) | PR #1619 merged the crate-private auxiliary worker-effect owner/origin bridge: stable window generations survive parent dispatch, worker completion mapping, and chained commands; destructive retirement fences only matching registrations, releases pending capacity idempotently, and preserves sibling/application work, cached hide, and recovery. Exact-head validation passed with 3,354 library/integration tests, 229 examples, 11 doctests, 288 guardrails, documentation, all-target/all-feature check, strict Clippy, Linux and Intel-macOS no-default-feature checks, formatting, and diff checks; Terra exact-head APPROVE found no findings. Runtime/effects/scheduling alignment moves conservatively from 72% to 75%; the next candidate is timer/platform owner integration before overlay/keyed-node cancellation, budgets, or fairness. |
| 2026-08-06 | `aa8bd77b` | ~94% (89–99%, medium confidence) | PR #1620 merged the crate-private auxiliary timer-effect owner/origin bridge: stable generations survive timer registration, opaque controller-wake mapping, UI dispatch, and chained commands; exact-generation retirement drops matching mapper closures, repairs only matching latest slots, and leaves sibling/application, same-key new-generation, cached-hide, recovery, and late-wake paths isolated. Exact-head validation passed with 2,568 library tests plus 1 ignored, 288 guardrails, all-target/all-feature check, strict Clippy, examples, documentation, Linux and Intel-macOS no-default-feature checks, formatting, and diff checks; Terra exact-head APPROVE found no findings. Runtime/effects/scheduling alignment moves conservatively from 75% to 78%; the next candidate is platform-completion owner/origin integration before overlay/keyed-node cancellation, budgets, or fairness. |
| 2026-08-06 | `fae45a23` | ~95% (90–99%, medium confidence) | PR #1621 merged the crate-private auxiliary platform-completion owner/origin bridge: stable generations survive result-host acceptance, unsupported and rejected fallback, direct and queue delivery, and chained platform commands; exact-generation retirement detaches only matching mappers, leaving host-held sinks bounded and late deliveries inert before mapping. Exact-head validation passed with 2,575 library/integration tests plus 1 ignored, platform command 19 and registry 7 focused tests, 288 guardrails, public API suites, examples, doctests, documentation, all-target/all-feature check, strict Clippy, Linux and Intel-macOS no-default-feature checks, formatting, and diff checks; Terra exact-head APPROVE found no findings. Runtime/effects/scheduling alignment moves conservatively from 78% to 82%; the next candidate is overlay/keyed-node effect cancellation before budgets or fairness. |
| 2026-08-06 | `1235f573` | ~95% (90–99%, medium confidence) | PR #1622 merged the documentation-only declarative effect-owner selection and cancellation contract: Application, exact auxiliary-window, overlay, and keyed-node ownership are distinguished; source provenance is candidate-only with explicit application-owned/outlive escape; identity/generation, same-update removal, late-result, recovery/cached-hide, shared-resource, and unkeyed-node boundaries are defined. No executable behavior or estimate changed. Exact-head validation passed with four-file scope, diff check, formatting, 288 guardrails, and documentation build; Terra exact-head APPROVE found no findings. The next code dependency is the private declarative owner-topology/source-provenance bridge before executable overlay/keyed-node cancellation, then budgets or fairness. |
| 2026-08-06 | `c963cbfd` | ~95% (90–99%, medium confidence) | PR #1623 merged the private declarative source-topology prerequisite: extracted overlay roots preserve pre-flattening structural scope and keyed ancestry while final lowered IDs remain authoritative; complete source traversal includes non-interactive floating descendants; and persistent runtime scratch/probe ownership remains authoritative through startup, refresh, unchanged virtual-layout cache, virtual-to-ordinary transition, and geometry relayout. The overall estimate remains ~95%; Runtime/effects/scheduling moves conservatively from 82% to 84% because executable owner projection/selection and cancellation remain unshipped. Exact-head validation passed with 2,588 library tests plus 1 ignored, 229 examples, 11 doctests plus 1 ignored, 288 guardrails, public API suites 204/23/101/6, all-target/all-feature check, strict Clippy, docs, formatting, diff check, and perf baseline 2/2 matched with 0 slower; GitHub quality and windows-compile passed; Terra exact-head APPROVE found no findings. The next candidate is private declarative owner projection/selection before exact-generation cancellation, then scheduling budgets or fairness. |
| 2026-08-07 | `dbc4b9ac` | ~95% (90–99%, medium confidence) | PR #1624 merged the private declarative owner-candidate projection and exact selection resolver: keyed and overlay candidates remain independent and use structural scope plus compatibility evidence; application default/outlive outcomes are explicit; scoped removal, reorder, incompatibility, sibling, duplicate, stale-capacity, and provisional-probe cases reject safely without fallback. The overall estimate remains ~95%; Runtime/effects/scheduling moves conservatively from 84% to 86% because exact-generation reconciliation/cancellation and effect-origin integration remain unshipped. Exact-head validation passed with 232 focused controller tests, public API/guardrail suites 204/23/101/6/288, all-target/all-feature check, strict Clippy, docs, formatting, portable Linux/Intel-macOS library checks, and performance comparison; GitHub quality (15m58s) and windows-compile passed; independent Terra exact-head APPROVE found no findings. The next candidate is exact-generation reconciliation and cancellation, then explicit-origin retirement before scheduling budgets or fairness. |
| 2026-08-07 | `98d23654` | ~95% (90–99%, medium confidence) | PR #1625 merged the private declarative owner-generation ledger: checked monotonic generations and runtime-instance-safe tokens preserve compatible accepted reprojection and keyed reorder, retire removal/ambiguity/incompatible replacement, allocate fresh reinsertion generations, contain exhaustion without disturbing compatible siblings, bind exact live selections, and retire all clones at close while recovery preserves them. The overall estimate remains ~95%; Runtime/effects/scheduling moves conservatively from 86% to 88% because no explicit owner-request consumer, effect-origin dispatch, or registry cancellation is claimed. Exact-head validation passed with 2,607 tests, 0 failures, 1 ignored, public guardrails, all-target/all-feature check, strict Clippy, docs, portable Linux/Intel-macOS checks, and benchmark comparison with no missing scenarios; corrected head passed GitHub quality (7m44s) and windows-compile (2m09s). Fresh exact-head Terra APPROVE followed one bounded correction for exhaustion and lifecycle evidence. The next candidate is a concrete explicit owner-request consumer; if no product consumer supplies that policy, remaining cancellation alignment is product-dependent before scheduling budgets or fairness. |
| 2026-08-07 | `180e75ce` | ~96% (91–99%, medium confidence) | PR #1626 merged the private explicit declarative owner-request consumer: default and application-outlive requests remain application-owned, exact accepted live overlay/keyed generations become private effect origins, rejected requests dispatch no work, and worker/timer/platform/chained paths reject retired origins before mapping or reduction. Full local validation passed with 2,615 library/integration tests, examples, doctests, all-target/all-feature check, strict Clippy, formatting, and diff checks; required GitHub `quality` (13m22s) and `windows-compile` (1m28s) passed, and exact-head Terra APPROVE found no findings. Runtime/effects/scheduling moves conservatively from 88% to 91%; eager declarative registry retirement remains the next generic item, while product-facing selection/cancellation policy and scheduler budgets/fairness remain later or product-dependent. |
| 2026-08-07 | `dd0c92e8` | ~97% (92–99%, medium confidence) | PR #1627 merged the private accepted-projection retirement handoff: exact retired declarative generations now eagerly remove matching worker registrations/pending admissions, timer registrations/latest slots, and platform-completion mappers while preserving late-delivery and sibling/application/later-generation isolation. Full local validation passed with 2,618 library tests plus integration targets, examples, 3 doctests plus 1 ignored and 8 compile-fail doctests, all-target/all-feature check, strict Clippy, formatting, and diff checks; required GitHub `quality` (11m51s) and `windows-compile` (1m53s) passed, and exact-head Terra APPROVE found no findings. Runtime/effects/scheduling moves conservatively from 91% to 93%; remaining public selection/cancellation policy and scheduler budgets/fairness require product/consumer contracts, while the other open target gaps require named product workloads, renderer contracts, or platform scope. |
| 2026-08-07 | `6e727ca2` | ~97% (92–99%, medium confidence) | PR #1628 merged the macOS-only support-scope and estimate correction: generic architecture-sequence completion remains ~97%, while the transparent 11-category broad end-to-end mean is ~72%. The target now identifies macOS as the current product and acceptance scope, with Linux/Windows retained as future portability targets; no native macOS acceptance or category-score change is claimed. Docs-only diff check passed, required GitHub `quality` (12m30s) and `windows-compile` (1m28s) passed, and exact-head Terra APPROVE found no findings. |
| 2026-08-07 | `ffe98d3f` | ~97% (92–99%, medium confidence) | PR #1629 merged display-only `ValueFormat` attachment through the official Slider and Knob application builders. Configured automation text uses the bounded policy; default text, normalized interaction values, edit batches, low-level constructors, and public widget shapes remain unchanged. Numeric-controls alignment moves conservatively from 42% to 46%; broad target coverage is `790 / 11 = 71.82%` (~72%), with no native macOS acceptance claimed. Focused local tests, formatting, diff check, and strict Clippy passed; GitHub `quality` (15m25s) and `windows-compile` (1m11s) passed; fresh exact-head Terra APPROVE found no findings after one test-coverage correction. |
| 2026-08-07 | `2f1668a3` | ~97% (92–99%, medium confidence) | PR #1630 merged the first bounded public Off/Frame profiling path: `ProfilingOptions` flows through native launch builders, `FrameProfile` projects successful-present diagnostics through an independent host capability and stateful callback, primary/auxiliary ordering and exhausted-sequence behavior are covered, and GPU timing is explicitly unavailable. Diagnostics/profiling moves from 50% to 58%; broad target coverage is `798 / 11 = 72.55%` (~73%). Local Clippy, 2,626 library tests plus integration targets, 288 guardrails, formatting, and diff checks passed; GitHub `quality` (8m56s) and `windows-compile` (1m21s) passed; fresh exact-head Terra APPROVE found no findings. No live native macOS presentation run or Linux/Windows support is claimed. |
| 2026-08-07 | `39c78255` | ~97% (92–99%, medium confidence) | PR #1631 merged the macOS-only live Off/Frame acceptance harness with fixed recorder evidence, independent primary/auxiliary successful-present profiles, Off silence, native auxiliary close, and native primary zoom/resize. A bounded Frame/Frame admission gate and regression test prevent callback arrival order from reversing window ownership. Diagnostics/profiling moves from 58% to 62%, platform/windowing from 60% to 68%, and broad target coverage is `810 / 11 = 73.64%` (~74%). Local focused validation passed with 6 example tests and 288 guardrails; the required GitHub `quality` and `windows-compile` checks passed; exact-head Terra APPROVE found no findings. Linux/Windows remain future portability targets; Detailed profiling, GPU timing, renderer-owned budgeting, and broader performance proof remain. |
| 2026-08-07 | `e9e5136d` | ~97% (92–99%, medium confidence) | PR #1632 merged the normal stateful/window builder exposure for the existing observational `DevtoolsOverlayOptions` plus a macOS-only live devtools acceptance harness. The harness exercises ordinary buttons, a toggle, bounded Unicode text input, and inspector tree/selection/bounds/paint diagnostics; non-macOS compilation is explicitly guarded and remains a portability fallback. The initial Windows run found only unconditional harness constants under `-D warnings`; the cfg-only correction produced final head `4ad7addf`, which passed local all-target/all-feature checks, strict Clippy, focused example/guardrail tests, and required GitHub `quality` (12m52s) and `windows-compile` (2m07s). Independent Terra exact-head APPROVE found no findings, and the PR merged as `e9e5136d`. Diagnostics/profiling moves from 62% to 66%; broad target coverage is `814 / 11 = 74.00%` (~74%). The remaining inspector limitations are documented: the Vello canvas has no native AX nodes for its in-surface controls, so pointer/text/focus interaction is covered by harness state/projection tests rather than claimed native accessibility. Detailed profiling, runtime switching, inspector/frame correlation, GPU timestamps, renderer-owned budgeting, and broader performance proof remain. |
| 2026-08-07 | `66859796` | ~97% (92–99%, medium confidence) | PR #1633 merged the macOS outgoing external-drag completion boundary: valid launches remain pending until AppKit reports a terminal result; Copy completes once as accepted; Escape cancellation completes once as unaccepted; and exact WindowId-plus-identity routing plus stale/duplicate/replacement/closed/shutdown fences remain intact. At exact head `03363f18`, local focused/all-target/strict-Clippy/docs gates and green `quality`/`windows-compile` CI passed, independent Terra specialist review ended APPROVE, and live macOS 26.5.2 arm64 Finder trials recorded one 46-byte payload for Copy and no additional payload for Escape cancellation. Platform/windowing moves from 68% to 70%; broad target coverage is `816 / 11 = 74.18%` (~74%). Linux/Windows runtime support, incoming drops, Move/Link negotiation, and broader product/renderer policy remain outside this macOS slice. |
| 2026-08-07 | `e537a07d` | ~97% (92–99%, medium confidence) | PR #1634 merged the qualified single-line `TextInputRevision` authority prerequisite: newer revisions apply projected value/caret/selection, equal or older revisions preserve retained editing state, equal-value newer revisions apply projected selection, revision-mode changes reset explicitly, and unrevisioned inputs retain legacy synchronization. Text/focus/selection moves conservatively from 60% to 63%; broad target coverage is `819 / 11 = 74.45%` (~74%). Local focused, public API, full test, example, doctest, formatting, diff, all-target/all-feature, strict-Clippy, and documentation gates passed; GitHub `quality` (14m30s) and `windows-compile` (1m33s) passed; independent Terra exact-head APPROVE found no findings. No live native acceptance was required because this slice adds no platform behavior. IME/composition, multiline editing, native accessibility, and product/document authority policy remain. |
