# Overlay focus and dismissal

`Layer::modal(view)` declares a modal focus scope. `Layer::focus_policy` can
select `OverlayFocusPolicy::None`, `Restore`, or `Modal` independently of the
layer's pointer input policy. Other declarative layer kinds default to `None`.

A modal activates its first eligible target and confines sequential, spatial,
explicit, and semantic focus transfers to its active subtree. Nested overlays
belong to their parent's scope. Background semantic nodes advertise no actions
while a modal owns focus; semantic ordering follows rendering order.

`Restore` records prior focus without automatically activating or trapping the
overlay. Closing it restores focus only when focus belonged to that overlay;
an explicit choice outside it, including cleared focus, is preserved.

Prior focus is an incarnation-checked bookmark, not a reusable widget ID.
Closing nested modals unwinds removed scopes before restoring a surviving
bookmark. A retired or absent bookmark selects the first current eligible
focus target in the surviving modal or base. An empty modal owns the boundary
with no focused widget. It does not allow traversal into background content.

Opening a modal respects the incumbent widget's `prepare_focus_loss` veto.
A veto preserves the previous complete surface. An allowed decision is consumed
once for that exact publication and retained owner. Focus callbacks may update
the application synchronously; earlier publication bookkeeping cannot overwrite
that newer surface. Existing capture and composition retirement paths run
before removed owners can be reused.

`Layer::dismiss_on_escape(message)` dispatches an ordinary application message
for the current topmost active overlay. Focused editing and key capture take
precedence. An unconsumed initial Escape dismisses one overlay; repeats do not
cascade through the stack. A topmost overlay without an Escape callback does
not borrow its parent's callback. A closing runtime admits no dismissal.
`dismiss_on_outside_click` retains its existing pointer-press semantics and uses
the same nested rendering order. Pointer blocking remains an explicit
`block_input` or outside-dismissal policy.

Declarative children render above their parents even when their layer kind
would otherwise sort below the parent. Unrelated roots retain kind and
stable declaration order. Malformed or over-budget focus source evidence is
rejected conservatively. Focus projection retains at most 64 overlay records
and 65,536 source identity/membership entries; a later overlay cannot bypass
an earlier exhausted evidence budget.

Raw `SurfaceLayer` construction keeps `LayerKind` as rendering order only.
Call `focus_owner(owner, policy)` with an `OverlayFocusOwner` retained across
compatible projections to opt into focus behavior. The owner is UI-local and
does not hold widgets, runtime handles, or dismissal messages. Declarative
Escape callbacks remain in UI-local surface state, outside frozen source
metadata.

Dynamic widget anchors and their layout/omission contract remain outstanding
under OPT-1394; existing coordinate-based placement builders are unchanged by
this focus and dismissal slice.
