# Radiant Target-Alignment Status

This document tracks Radiant's implementation alignment with the normative
target. It is a durable progress snapshot, not a measure of line count,
documentation completeness, or workflow activity. GitHub remains the delivery
record for individual slices and reviews.

## Snapshot

- Snapshot date: **2026-08-06**
- Canonical main: **`ed2bfcb7`**
- Overall estimate: **~93%**
- Working range: **88–98%**
- Confidence: **medium**

The estimate reflects the combination of shipped contract, implementation,
tests, documentation, and integration evidence. Target-only prose, diagrams,
or examples do not count as shipped implementation; they establish the
contract against which progress is measured. PR #1602 reduced contract
ambiguity and sequencing risk without changing the numeric estimate; PR #1603
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
full-scene encoding. The estimate moves modestly because this closes the
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
admitted. The estimate moves modestly because timer/platform owner integration,
overlay/keyed-node cancellation, and scheduling policy remain unshipped.

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
| Text, focus, and selection | 60% | Focus and selection foundations exist; richer multiline/IME/composition editing and native accessibility remain. |
| Numeric controls | 42% | Finite linear/log `ValueMapping`, deterministic allocation-free `ValueFormat`, and the parser-agnostic `NumericEditSession<T>` draft/commit/cancel foundation are shipped; parser/domain policy and control integration are not. |
| Runtime, effects, and scheduling | 75% | PRs #1617–#1619 ship generic lifecycle authority/diagnostics, the native Vello recovery bridge, and the first stable owner/origin/cancellation consumer for worker effects: accepted recovery is coupled to controller state, recovery preserves effects, auxiliary generations survive dispatch/completion/chaining, and destructive retirement fences only matching worker registrations. Timer/platform owner integration, overlay/keyed-node cancellation, and the complete scheduling target remain. |
| Rendering, invalidation, retained GPU surfaces | 78% | Revision/damage direction, private committed native paint-segment benefit evidence, bounded observational admission, plan-index-preserving sparse artifact residency, executable mixed assembly, admission-gated sparse publication, and explicit admission-aware render-boundary selection with conservative full-scene fallback are shipped; renderer-owned retained-resource lifetime/budgeting, platform profiling, and product-specific cache policy remain. |
| Platform, windowing, and host boundaries | 60% | macOS-first host-facing boundaries are established; broader Linux/Windows runtime validation remains. |
| Diagnostics, profiling, and performance validation | 50% | Bounded diagnostics and validation foundations exist; first-class profiling/debug inspection and broader proof remain. |
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
splitting the global ingress. Timer/platform owner integration and overlay or
keyed-node cancellation remain; the sequence still does not add configurable
budgets, fairness, or synthetic GPU-host acceptance.

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
2. **Numeric and input integration.** Complete numeric attachment,
   `numeric_input`, widget/runtime/focus integration, and the pointer,
   keyboard, and accessibility domain contract around those paths. This is
   now the next dependency-correct numeric item, but parser, locale, range,
   formatting, and product interaction policy must come from a concrete
   consumer rather than being invented in generic Radiant.
3. **Runtime, effects, and scheduling integration.** PRs #1617–#1619 ship
   generic lifecycle authority/diagnostics, the bounded native Vello
   recovery/effect-preservation bridge, and stable auxiliary-window
   owner/origin/cancellation for worker effects: accepted recovery and
   successful primary/auxiliary completion are coupled to controller state,
   recovery preserves effects, and destructive auxiliary retirement fences only
   its matching worker registrations. The next dependency-correct runtime
   candidate is applying the same owner fence to timer effects and platform
   completions; overlay/keyed-node cancellation follows, then configurable
   scheduling budgets and fair multi-window policy. Do not claim scheduler
   fairness until those ownership and renderer boundaries are concrete.
4. **Richer text editing.** Complete multiline editing, IME/composition, and
   native accessibility semantics.
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
6. **Profiling and performance proof.** Add first-class `ProfilingMode`,
   `FrameProfile`, a debug inspector, and broader performance validation.
7. **Platform expansion.** Broaden Linux/Windows runtime validation and
   platform implementation behind the existing boundaries.

dB, tempo, and other custom numeric formats remain later work after the
parser-agnostic edit-session and generic numeric integration contracts are
established.

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
- [Native recovery/controller bridge](../src/gui_runtime/native_vello/generic_runtime/runner.rs)
- [Controller recovery lifecycle boundary](../src/runtime/controller/state/lifecycle.rs)
- [Runtime owner and auxiliary generation fence](../src/runtime/controller/owner.rs)
- [Worker effect owner/origin routing](../src/runtime/controller/effects.rs)
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
2. update the overall estimate and working range;
3. update the affected category score and evidence/status;
4. move an item only when code, tests, documentation, and integration justify
   the change;
5. record the next dependency-correct gap; and
6. keep GitHub as the delivery record rather than duplicating its workflow
   ledger here.

### Initial entry

| Date | Canonical main | Overall | Note |
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
