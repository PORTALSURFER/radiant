# Declarative animation

`ViewNode::animatable(Rc<dyn Animatable>)` attaches an immutable presentation capability. Custom layout containers receive the capability directly; other views use a stable ordinary column wrapper. Apply a continuity key to the returned view when it can move among siblings.

The capability declares finite scalar `AnimationTarget`s and optional `FeedbackAnimation` properties. Each property ID must be unique within the capability. Targets specify an initial value, terminal value, `Transition` and `AnimationImpact`. Linear and quadratic ease-out transitions are supported. Zero duration applies immediately. Invalid values and unrepresentable clock deadlines are rejected conservatively.

The accepted runtime owns samples. Branch construction, discarded projections and declaration reads do not start clocks. The runtime identifies an owner by its accepted ancestry, source compatibility and capability type. A changed target retargets from the current interpolated value. An unchanged target continues without restarting. Removing an owner or property cancels it; reinsertion starts a fresh initial transition. No worker completion or application message delivers animation samples.

`Animatable::append_paint` reads `AnimationValues` and adds the container's chrome to the normal paint plan. `AnimationImpact::Paint` rebuilds base paint while retaining application projection and geometry. This implementation does not claim retained paint-segment optimization. Geometry properties use the normal layout path. Custom layout capabilities receive the same sample snapshot in `measure` and `place`, with the original `LayoutPolicy` available as fallback. Declaration objects are never mutated per frame.

The window owns one bounded animator: at most 1024 declared properties, 256 active finite transitions, 64 shared feedback groups and 256 feedback consumers. Excess active transitions resolve statically; rejected feedback uses its declared static value. A shared group has one period and phase. Conflicting periods are rejected. Feedback outside the viewport or omitted by layout uses its static fallback and does not consume a shared clock. Source traversal is bounded to 128 levels and 65536 relevant nodes; duplicate identities or invalid declarations retire the previous accepted animation set. Trees without animation skip collection.

Active work requests one aggregate deadline at up to 60 Hz, subject to native window scheduling. Completion removes demand. Reduced motion resolves finite targets and static feedback with no animation wakeups. Hidden or occluded windows freeze values and resume without counting hidden time. Native visibility, surface occlusion and device-recovery concealment use this policy; custom hosts call `SurfaceRuntime::set_animation_hidden`. The deterministic host exposes the same operation and installs virtual time before initial animation admission.

`SurfaceRuntime::declarative_animation_status` reports active, retained, feedback-group, feedback-consumer and rejection counts, changed frames and hidden state. It is observational and does not decide admission.

Run `cargo run --example declarative_animation` for a headless custom-container example and JSON trace. Its tests verify visible paint changes, geometry updates, no per-frame application projection, retargeting, removal, reduced motion and hidden-window behavior.
