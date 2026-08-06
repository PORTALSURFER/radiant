# Radiant Target-Alignment Status

This document tracks Radiant's implementation alignment with the normative
target. It is a durable progress snapshot, not a measure of line count,
documentation completeness, or workflow activity. GitHub remains the delivery
record for individual slices and reviews.

## Snapshot

- Snapshot date: **2026-08-06**
- Canonical main: **`d05f6e8e`**
- Overall estimate: **~84%**
- Working range: **79–89%**
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
claiming parser, domain, widget, or runtime integration. The estimate moves
modestly because numeric edit-session state is now executable and validated,
while no product-specific numeric policy or virtualization behavior is claimed.

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
- finite linear/log `ValueMapping`; and
- deterministic, allocation-free `ValueFormat`.

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
numeric parser/domain policy remain contract-dependent.

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
3. **Richer text editing.** Complete multiline editing, IME/composition, and
   native accessibility semantics.
4. **Production frame wiring.** Complete reconciliation, damage propagation,
   render-boundary selection, and retained-surface cache admission in the
   production runtime path.
5. **Profiling and performance proof.** Add first-class `ProfilingMode`,
   `FrameProfile`, a debug inspector, and broader performance validation.
6. **Platform expansion.** Broaden Linux/Windows runtime validation and
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
- [Query capability public tests](../tests/virtual_layout_public_api.rs)
- [Edit lifecycle and provenance](../src/widgets/interaction/edit.rs)
- [Numeric edit session](../src/widgets/interaction/numeric_edit.rs)
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
| 2026-08-06 | `0bf39879` | ~77% (71–83%, medium confidence) | The normative runtime consumer boundary was contract-ready on the PR #1606 base; direct registration remained blocked on the private retained-item admission adapter and a separate `SurfaceRuntime` registration/two-pass bridge; no runtime behavior, public API, or capability version changed. |
| 2026-08-06 | `21e7b62a` | ~77% (71–83%, medium confidence) | PR #1606 merged the docs-only normative runtime consumer boundary with exact `SurfaceRuntime` ownership, synchronous shell/item sequencing, batch identity admission, fallible scoped lowering, immutable retained payloads, and explicit close/failure rules. The estimate remains unchanged because direct runtime registration and retained-item admission are still unshipped; the next dependency-correct item is the private retained-item adapter. |
| 2026-08-06 | `d63a7f32` | ~77% (71–83%, medium confidence) | PR #1607 merged the private tuple-scoped retained-item admission prerequisite with deterministic wrapper identity, declarative/custom-widget output guards, typed rejection, panic containment, and focused/full validation. The estimate remained unchanged because whole-batch admission, materialization integration, and direct runtime registration were still unshipped; the next dependency-correct item was the complete private retained-item batch adapter. |
| 2026-08-06 | `bd296da3` | ~79% (73–85%, medium confidence) | PR #1608 merged the private complete retained-item batch adapter with exact accepted-key/slot matching, whole-shell-plus-active-batch identity admission, deterministic tuple-scoped wrappers, immutable `SurfaceNode` payloads, typed recoverable rejection, 57 focused tests, full local validation, green CI, and exact-head Terra APPROVE. The estimate moves modestly because materialization-store integration and the separate `SurfaceRuntime` registration/two-pass bridge remain unshipped; that bridge is the next dependency-correct item. |
| 2026-08-06 | `ee861887` | ~83% (78–88%, medium confidence) | PR #1609 merged the private synchronous `SurfaceRuntime` virtual-layout registration/two-pass bridge with bounded registration admission, shell-first layout/query sequencing, complete-batch publication through the existing materialization store, retained-subtree reuse that skips unchanged post-cache projection/layout, conservative missing/changed/duplicate/deferred/unavailable/terminal handling, and same-ID virtual-to-ordinary regression evidence. Full local validation and green required CI passed with exact-head Terra APPROVE. Generic runtime consumer integration is now shipped; the next generic dependency-correct item is the parser-agnostic `NumericEditSession`, while product-specific virtualization and `split_pane` semantics remain product-dependent. |
| 2026-08-06 | `d05f6e8e` | ~84% (79–89%, medium confidence) | PR #1610 merged the qualified, parser-agnostic `NumericEditSession<T>` with verbatim draft preservation, one shared `EditEvent::Begin`, source-safe same-source commit/cancel, foreign-source session preservation, generic-domain/public-API coverage, full local validation, green required CI, and exact-head Terra APPROVE. Numeric-control alignment moves from 35% to a conservative 42%; the next numeric integration requires concrete parser/domain/widget policy, while product-specific virtualization and `split_pane` semantics remain product-dependent. |
