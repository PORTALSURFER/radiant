# Radiant Target-Alignment Status

This document tracks Radiant's implementation alignment with the normative
target. It is a durable progress snapshot, not a measure of line count,
documentation completeness, or workflow activity. GitHub remains the delivery
record for individual slices and reviews.

## Snapshot

- Snapshot date: **2026-08-06**
- Canonical main: **`0bf39879`**
- Overall estimate: **~77%**
- Working range: **71–83%**
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
Runtime materialization/consumer integration gaps remain.

This snapshot distinguishes architecture readiness from shipped runtime
behavior: the missing virtualization consumer boundary is now contract-ready
for implementation, but direct registration, retained-item admission, and the
two-pass `SurfaceRuntime` bridge are not shipped.

## Alignment by category

| Category | Alignment | Evidence / status |
| --- | ---: | --- |
| Public API and module boundaries | 80% | Explicit public/module boundaries and prelude hygiene are shipped; the full target surface is not. |
| Declarative model, identity, reconciliation | 70% | Stable identity, revision, and continuity foundations are shipped; complete production reconciliation remains. |
| Input, provenance, and edit lifecycle | 80% | Shared provenance and `EditEvent` lifecycle are adopted by `Slider`, `Knob`, and `PanelResizeState`; broader consumers remain. |
| Layout, composition, virtualization | 88% | Backend-neutral `SplitPaneLayout` geometry, UI-local capability/revision evidence, revision-2 declared hit-region projection/query, generic version-3 layout pointer admission/capture, runtime-owned version-4 typed container state, the qualified query-only keyed virtualization capability, a private query-only keyed visible-window coordinator, and a private materialization/recycling correctness kernel are shipped in PRs #1597–#1605. The runtime consumer boundary is contract-ready, but retained-item admission, direct registration/two-pass integration, product-specific `split_pane` behavior, and the remaining executable product virtualization proof remain unshipped. |
| Text, focus, and selection | 60% | Focus and selection foundations exist; richer multiline/IME/composition editing and native accessibility remain. |
| Numeric controls | 35% | Finite linear/log `ValueMapping` and deterministic allocation-free `ValueFormat` are shipped; edit-session and control integration are not. |
| Runtime, effects, and scheduling | 65% | Runtime controller and host-facing lifecycle foundations exist; the complete effects/scheduling target is not wired. |
| Rendering, invalidation, retained GPU surfaces | 60% | Revision/damage direction is present; production render-boundary and cache-admission wiring remains. |
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
- finite linear/log `ValueMapping`; and
- deterministic, allocation-free `ValueFormat`.

These foundations make later slices safer and more composable. They do not
mean that every target consumer, runtime path, platform, or integration is
complete.

## Architecture readiness versus shipped runtime behavior

The normative consumer contract is ready for the next implementation sequence:
`SurfaceRuntime` will eventually own one materialization record per mounted
virtual-container generation, with shell-discoverable registration evidence and
a synchronous two-stage mount/refresh boundary. Direct registration is blocked
on two prerequisites: the next executable **private retained-item adapter** must
provide fallible scoped `ViewNode` lowering, whole-batch identity admission, and
immutable `SurfaceNode` payload construction; a separate later PR must provide
the `SurfaceRuntime` registration/two-pass bridge.

No executable virtual consumer behavior is shipped by this contract update. The
current coordinator and materialization kernel remain private correctness
foundations; they are not registered with `SurfaceRuntime`, and no public
registration/API or capability contract version has changed.

## Remaining gaps, ordered by leverage

1. **Generic virtualization consumer and product-specific consumers.**
   PR #1602 landed the normative contract, PR #1603 shipped the bounded
   query-only capability, PR #1604 shipped private accepted-window
   reconciliation with logical-index deltas, conservative anchor/fallback
   evidence, and exact owner/revision fences, and PR #1605 shipped the private
   bounded materialization/recycling correctness kernel. Direct runtime
   registration is blocked on the private retained-item admission adapter and
   the separate `SurfaceRuntime` registration/two-pass bridge defined in the
   normative contract. The next executable PR is explicitly the **private
   retained-item adapter**: it adapts accepted keyed slots into fallible scoped
   `ViewNode` lowering, whole-batch identity admission, and immutable
   `SurfaceNode` payloads without scheduling or product state. Add `split_pane`
   interaction/state/ratio semantics only when the product contract is defined;
   PR #1601 supplies the generic runtime state lifecycle needed by these slices.
2. **Numeric edit session.** Add a parser-agnostic `NumericEditSession` with a
   runtime-local draft and typed commit/cancel semantics.
3. **Numeric and input integration.** Complete numeric attachment,
   `numeric_input`, widget/runtime/focus integration, and the pointer,
   keyboard, and accessibility domain contract around those paths.
4. **Richer text editing.** Complete multiline editing, IME/composition, and
   native accessibility semantics.
5. **Production frame wiring.** Complete reconciliation, damage propagation,
   render-boundary selection, and retained-surface cache admission in the
   production runtime path.
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
- [Query capability public tests](../tests/virtual_layout_public_api.rs)
- [Edit lifecycle and provenance](../src/widgets/interaction/edit.rs)
- [Value mapping](../src/widgets/interaction/value.rs)
- [Value formatting](../src/widgets/interaction/format.rs)
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
| 2026-08-06 | `0bf39879` | ~77% (71–83%, medium confidence) | The normative runtime consumer boundary is contract-ready but not shipped: direct registration is blocked on the private retained-item admission adapter and a separate `SurfaceRuntime` registration/two-pass bridge; no runtime behavior, public API, or capability version changed. |
