//! UI-local capability registration and layout-target declarations for
//! backend-neutral layout containers.
//!
//! This module establishes the registration, revision-evidence, declaration,
//! and typed input boundary for optional container layout interaction. Runtime
//! capture remains owned by the surface controller; the capability only
//! receives an input and records bounded event decisions in its context.

use std::{
    any::{Any, TypeId},
    fmt,
    num::NonZeroU64,
    rc::Rc,
};

use super::tree::NodeId;
use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::{Point, Rect},
    },
    widgets::{PointerButton, PointerModifiers},
};

/// Contract revision understood by [`LayoutCapabilities`].
pub const LAYOUT_CAPABILITIES_CONTRACT_VERSION: u16 = 4;

/// The state-aware layout interaction contract revision.
pub const LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION: u16 = 4;

/// The projection/query-only capability contract retained for compatibility.
pub const LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION: u16 = 2;

pub(crate) const fn supports_layout_capabilities_contract(version: u16) -> bool {
    matches!(
        version,
        LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION
            | 3
            | LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION
    )
}

pub(crate) const fn supports_layout_input_contract(version: u16) -> bool {
    matches!(version, 3 | LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION)
}

pub(crate) const fn supports_layout_state_input_contract(version: u16) -> bool {
    version == LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION
}

/// Stable identity for one projected layout interaction target.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LayoutTargetIdentity {
    /// Stable container node that owns the interaction capability.
    pub container_id: NodeId,
    /// Stable region identity within that container.
    pub region_id: LayoutHitRegionId,
}

impl LayoutTargetIdentity {
    /// Construct a target identity from its container and region identities.
    pub const fn new(container_id: NodeId, region_id: LayoutHitRegionId) -> Self {
        Self {
            container_id,
            region_id,
        }
    }
}

/// Opaque identity for one runtime-owned layout interaction state slot.
///
/// The identity combines the mounted container identity, the concrete
/// `'static` state type, and the caller-owned schema version. The concrete
/// type identity is intentionally private: callers construct and compare
/// identities through typed methods without depending on `TypeId` or
/// `Any`.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ContainerStateId {
    container_id: NodeId,
    state_type: TypeId,
    schema_version: u16,
}

impl ContainerStateId {
    /// Construct a typed state identity for one mounted container.
    pub fn new<T>(container_id: NodeId, schema_version: u16) -> Self
    where
        T: 'static,
    {
        Self {
            container_id,
            state_type: TypeId::of::<T>(),
            schema_version,
        }
    }

    /// Construct a typed state identity using an explicit type-oriented name.
    pub fn for_type<T>(container_id: NodeId, schema_version: u16) -> Self
    where
        T: 'static,
    {
        Self::new::<T>(container_id, schema_version)
    }

    /// Return the mounted container identity.
    pub const fn container_id(self) -> NodeId {
        self.container_id
    }

    /// Return the caller-owned state schema version.
    pub const fn schema_version(self) -> u16 {
        self.schema_version
    }

    /// Return whether this identity names the supplied concrete state type.
    pub fn is<T>(self) -> bool
    where
        T: 'static,
    {
        self.state_type == TypeId::of::<T>()
    }

    pub(crate) fn same_container(self, other: Self) -> bool {
        self.container_id == other.container_id
    }
}

impl fmt::Debug for ContainerStateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerStateId")
            .field("container_id", &self.container_id)
            .field("schema_version", &self.schema_version)
            .finish_non_exhaustive()
    }
}

/// Runtime-owned identity for one exact mounted container-state slot.
///
/// The generation is issued only by the runtime-owned state store. Keeping it
/// separate from [`ContainerStateId`] lets the store reject identities from a
/// retired mount even when the declaration's type and schema are unchanged.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct MountedContainerStateId {
    state_id: ContainerStateId,
    generation: NonZeroU64,
}

impl MountedContainerStateId {
    pub(crate) const fn new(state_id: ContainerStateId, generation: NonZeroU64) -> Self {
        Self {
            state_id,
            generation,
        }
    }

    pub(crate) const fn generation(self) -> NonZeroU64 {
        self.generation
    }
}

/// Immutable borrowed view of one exact mounted container-state slot.
///
/// The mounted identity travels with the view so a read remains associated
/// with the exact generation that admitted it. The underlying state remains
/// runtime-local and may be a non-`Send` value.
#[derive(Clone, Copy)]
pub(crate) struct MountedContainerStateRead<'a> {
    mounted_id: MountedContainerStateId,
    state: &'a dyn Any,
}

impl<'a> MountedContainerStateRead<'a> {
    pub(crate) const fn new(mounted_id: MountedContainerStateId, state: &'a dyn Any) -> Self {
        Self { mounted_id, state }
    }

    /// Return the exact mounted identity that admitted this view.
    pub(crate) const fn mounted_id(self) -> MountedContainerStateId {
        self.mounted_id
    }

    /// Borrow the state as the requested concrete type.
    pub(crate) fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.state.downcast_ref::<T>()
    }

    /// Alias for [`Self::downcast_ref`] for collection-style typed access.
    pub(crate) fn get<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.downcast_ref::<T>()
    }
}

/// Explicit initializer and typed identity for one layout interaction state.
///
/// The initializer is UI-local and may return values that are not `Send`, such
/// as `Rc<Cell<_>>`. It is called by the runtime only when the matching slot is
/// first mounted or when its concrete type/schema changes.
#[derive(Clone)]
pub struct ContainerStateDeclaration {
    id: ContainerStateId,
    initializer: Rc<dyn Fn() -> Box<dyn Any>>,
}

impl ContainerStateDeclaration {
    /// Declare a `'static` state value for one mounted container.
    pub fn new<T, Initializer>(
        container_id: NodeId,
        schema_version: u16,
        initializer: Initializer,
    ) -> Self
    where
        T: 'static,
        Initializer: Fn() -> T + 'static,
    {
        Self {
            id: ContainerStateId::new::<T>(container_id, schema_version),
            initializer: Rc::new(move || Box::new(initializer()) as Box<dyn Any>),
        }
    }

    /// Return the typed identity used by the runtime-owned state store.
    pub const fn id(&self) -> ContainerStateId {
        self.id
    }

    /// Return the mounted container identity.
    pub const fn container_id(&self) -> NodeId {
        self.id.container_id()
    }

    /// Return the caller-owned state schema version.
    pub const fn schema_version(&self) -> u16 {
        self.id.schema_version()
    }

    /// Return whether this declaration uses the supplied concrete state type.
    pub fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        self.id.is::<T>()
    }

    pub(crate) fn initialize(&self) -> Box<dyn Any> {
        (self.initializer)()
    }
}

impl fmt::Debug for ContainerStateDeclaration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerStateDeclaration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Typed access context supplied to a state-aware layout interaction.
///
/// A context without a declaration is valid and gives every typed lookup
/// `None`. This keeps state-free v4 interactions allocation-free while allowing
/// the same object-safe callback to serve stateful and stateless capabilities.
pub struct LayoutContainerStateContext<'a> {
    container_id: NodeId,
    state_id: Option<ContainerStateId>,
    state: Option<&'a mut dyn Any>,
}

impl<'a> LayoutContainerStateContext<'a> {
    /// Return the container receiving this interaction.
    pub const fn container_id(&self) -> NodeId {
        self.container_id
    }

    /// Return the mounted state identity, if this interaction declared state.
    pub const fn state_id(&self) -> Option<ContainerStateId> {
        self.state_id
    }

    /// Return whether this interaction has a mounted state slot.
    pub const fn has_state(&self) -> bool {
        self.state.is_some()
    }

    /// Borrow the declared state as the requested concrete type.
    pub fn state_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        let state = self.state.as_deref_mut()?;
        state.downcast_mut::<T>()
    }

    /// Alias for [`Self::state_mut`] for callers that prefer collection-style
    /// typed access.
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.state_mut::<T>()
    }

    pub(crate) fn from_runtime(
        container_id: NodeId,
        state_id: Option<ContainerStateId>,
        state: Option<&'a mut dyn Any>,
    ) -> Self {
        Self {
            container_id,
            state_id,
            state,
        }
    }
}

/// Pointer input offered to a version-3 or version-4 layout interaction capability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutInput {
    /// Pointer hover or captured motion moved to `position`.
    PointerMove {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Modifier state captured with this pointer sample.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
        /// Optional opaque native sample sequence range.
        sequence_range: Option<InputSequenceRange>,
    },
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// A pointer button press started at `position`.
    PointerPress {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Button that started the press.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// A pointer double-click completed at `position`.
    PointerDoubleClick {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Button that completed the double-click.
        button: PointerButton,
        /// Modifier state at double-click time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// A pointer button release occurred at `position`.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Button that ended the press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Runtime-owned layout capture was cancelled at a safe boundary.
    PointerCaptureCancelled {
        /// Last known pointer position when cancellation was observed.
        position: Point,
        /// Latest modifier state available at cancellation.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
        /// Optional opaque native sample sequence range.
        sequence_range: Option<InputSequenceRange>,
    },
}

/// Bounded decisions returned by one layout interaction callback.
pub struct LayoutEventContext<Message> {
    target: LayoutTargetIdentity,
    container_bounds: Option<Rect>,
    target_bounds: Option<Rect>,
    divider_bounds: Option<Rect>,
    direction: crate::gui::layout_core::WritingDirection,
    handled: bool,
    capture_requested: bool,
    release_requested: bool,
    repaint_requested: bool,
    work_requested: bool,
    message: Option<Message>,
}

impl<Message> LayoutEventContext<Message> {
    /// Create an empty event context for one projected target.
    pub fn new(target: LayoutTargetIdentity) -> Self {
        Self {
            target,
            container_bounds: None,
            target_bounds: None,
            divider_bounds: None,
            direction: crate::gui::layout_core::WritingDirection::Ltr,
            handled: false,
            capture_requested: false,
            release_requested: false,
            repaint_requested: false,
            work_requested: false,
            message: None,
        }
    }

    pub(crate) fn with_geometry(
        target: LayoutTargetIdentity,
        container_bounds: Option<Rect>,
        target_bounds: Option<Rect>,
        divider_bounds: Option<Rect>,
    ) -> Self {
        let mut context = Self::new(target);
        context.container_bounds = container_bounds;
        context.target_bounds = target_bounds;
        context.divider_bounds = divider_bounds;
        context
    }

    pub(crate) fn with_direction(
        mut self,
        direction: crate::gui::layout_core::WritingDirection,
    ) -> Self {
        self.direction = direction;
        self
    }

    pub(crate) const fn writing_direction(&self) -> crate::gui::layout_core::WritingDirection {
        self.direction
    }

    /// Return the target identity receiving this event.
    pub const fn target(&self) -> LayoutTargetIdentity {
        self.target
    }

    /// Return the final logical bounds of the declaring container, when the
    /// runtime supplied geometry for this event.
    pub const fn container_bounds(&self) -> Option<Rect> {
        self.container_bounds
    }

    /// Return the clip-constrained logical bounds of the target, when the
    /// runtime supplied geometry for this event.
    pub const fn target_bounds(&self) -> Option<Rect> {
        self.target_bounds
    }

    pub(crate) const fn divider_bounds(&self) -> Option<Rect> {
        self.divider_bounds
    }

    /// Return whether the capability claimed this event.
    pub const fn handled(&self) -> bool {
        self.handled
    }

    /// Mark this event as handled. Only a handled fresh layout event prevents
    /// the runtime from falling back to widget routing.
    pub fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }

    /// Mark this event as handled and return the mutable context.
    pub fn handle(&mut self) -> &mut Self {
        self.handled = true;
        self
    }

    /// Request runtime-owned pointer capture for this target.
    pub fn capture_pointer(&mut self) {
        self.capture_requested = true;
        self.release_requested = false;
    }

    /// Request release of runtime-owned pointer capture for this target.
    pub fn release_pointer(&mut self) {
        self.release_requested = true;
        self.capture_requested = false;
    }

    /// Return whether capture was requested.
    pub const fn capture_requested(&self) -> bool {
        self.capture_requested
    }

    /// Return whether release was requested.
    pub const fn release_requested(&self) -> bool {
        self.release_requested
    }

    /// Request a repaint without requiring a surface projection.
    pub fn request_repaint(&mut self) {
        self.repaint_requested = true;
    }

    /// Request bounded runtime work and the repaint needed to service it.
    pub fn request_work(&mut self) {
        self.work_requested = true;
    }

    /// Return whether repaint was requested.
    pub const fn repaint_requested(&self) -> bool {
        self.repaint_requested
    }

    /// Return whether runtime work was requested.
    pub const fn work_requested(&self) -> bool {
        self.work_requested
    }

    /// Emit at most one host-defined message. A second message is ignored.
    pub fn emit_message(&mut self, message: Message) -> bool {
        if self.message.is_some() {
            return false;
        }
        self.message = Some(message);
        true
    }

    /// Take the one message admitted by this context.
    pub fn take_message(&mut self) -> Option<Message> {
        self.message.take()
    }
}

/// Stable identity for one hit region declared by a layout capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LayoutHitRegionId(pub u64);

impl LayoutHitRegionId {
    /// Construct a region identity from a stable caller-owned value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the stable caller-owned value.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// One sanitized local normalized hit region declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutHitRegion {
    id: LayoutHitRegionId,
    bounds: Rect,
}

impl LayoutHitRegion {
    /// Construct a validated local normalized hit region.
    ///
    /// Finite inverted coordinates are normalized before clipping to
    /// `[0, 1]`. Non-finite or non-positive results are rejected.
    pub fn new(
        id: LayoutHitRegionId,
        bounds: Rect,
    ) -> Result<Self, LayoutHitRegionDeclarationError> {
        Ok(Self {
            id,
            bounds: sanitize_normalized_bounds(bounds)?,
        })
    }

    /// Return the stable identity within the declaring container.
    pub const fn id(self) -> LayoutHitRegionId {
        self.id
    }

    /// Return the positive normalized local bounds.
    pub const fn bounds(self) -> Rect {
        self.bounds
    }
}

/// Why a layout hit-region declaration was not admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutHitRegionDeclarationError {
    /// The supplied rectangle contained a non-finite coordinate or extent.
    NonFiniteBounds,
    /// The normalized and clipped rectangle had a non-positive extent.
    NonPositiveBounds,
    /// The region identity was already declared by the same container.
    DuplicateIdentity,
}

fn sanitize_normalized_bounds(bounds: Rect) -> Result<Rect, LayoutHitRegionDeclarationError> {
    if !bounds.is_finite() {
        return Err(LayoutHitRegionDeclarationError::NonFiniteBounds);
    }
    let min = crate::gui::types::Point::new(
        bounds.min.x.min(bounds.max.x).clamp(0.0, 1.0),
        bounds.min.y.min(bounds.max.y).clamp(0.0, 1.0),
    );
    let max = crate::gui::types::Point::new(
        bounds.min.x.max(bounds.max.x).clamp(0.0, 1.0),
        bounds.min.y.max(bounds.max.y).clamp(0.0, 1.0),
    );
    let normalized = Rect::from_min_max(min, max);
    normalized
        .has_finite_positive_area()
        .then_some(normalized)
        .ok_or(LayoutHitRegionDeclarationError::NonPositiveBounds)
}

/// Read-only projected logical target for a declared layout hit region.
///
/// This type identifies a container/region pair and carries the final logical
/// bounds after normalized projection and ancestor clipping. It does not
/// grant event-dispatch, keyboard-focus, or pointer-capture authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutHitTarget {
    /// Stable container node that declared the target.
    pub container_id: NodeId,
    /// Stable region identity within the declaring container.
    pub region_id: LayoutHitRegionId,
    /// Projected and clip-constrained logical bounds.
    pub bounds: Rect,
}

impl LayoutHitTarget {
    /// Return the stable identity of this projected target.
    pub const fn identity(self) -> LayoutTargetIdentity {
        LayoutTargetIdentity::new(self.container_id, self.region_id)
    }
}

/// Read-only projection diagnostics for one traversal installation.
///
/// Invalid rectangles are reported immediately by [`LayoutHitRegion::new`].
/// Duplicate identities are contextual: the first declaration wins and later
/// declarations are counted here without displacing any valid declaration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutHitRegionDiagnostics {
    duplicate_declarations: u64,
}

impl LayoutHitRegionDiagnostics {
    /// Return the number of duplicate declarations ignored during projection.
    pub const fn duplicate_declarations(self) -> u64 {
        self.duplicate_declarations
    }

    pub(crate) fn record_duplicate(&mut self) {
        self.duplicate_declarations = self.duplicate_declarations.saturating_add(1);
    }
}

/// Typed revision evidence for one exported [`LayoutInteraction`] capability.
///
/// Exact values are compared by their `Eq` implementations and concrete types.
/// The value is UI-local, so it may retain arbitrary `Rc`-owned state; hashes
/// and caller-provided fingerprints are intentionally not part of this
/// contract. The conservative default is used when a capability cannot prove
/// that its layout-interaction output is unchanged.
#[derive(Clone)]
pub struct LayoutInteractionRevision {
    representation: RevisionRepresentation,
}

#[derive(Clone, Default)]
enum RevisionRepresentation {
    #[default]
    Conservative,
    Exact(Rc<dyn RevisionValue>),
}

impl Default for LayoutInteractionRevision {
    fn default() -> Self {
        Self::conservative()
    }
}

impl fmt::Debug for LayoutInteractionRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LayoutInteractionRevision")
            .field(
                "representation",
                &match self.representation {
                    RevisionRepresentation::Conservative => "conservative",
                    RevisionRepresentation::Exact(_) => "exact",
                },
            )
            .finish()
    }
}

impl PartialEq for LayoutInteractionRevision {
    fn eq(&self, other: &Self) -> bool {
        match (&self.representation, &other.representation) {
            (RevisionRepresentation::Conservative, RevisionRepresentation::Conservative) => true,
            (RevisionRepresentation::Exact(previous), RevisionRepresentation::Exact(current)) => {
                previous.equals(&**current)
            }
            _ => false,
        }
    }
}

impl Eq for LayoutInteractionRevision {}

trait RevisionValue: Any {
    fn equals(&self, other: &dyn RevisionValue) -> bool;
}

impl<T> RevisionValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn RevisionValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn RevisionValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl LayoutInteractionRevision {
    /// Return the safe fallback for capabilities that cannot prove exact
    /// changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: RevisionRepresentation::Conservative,
        }
    }

    /// Build exact typed evidence for a capability's layout-interaction
    /// output.
    #[must_use]
    pub fn exact<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            representation: RevisionRepresentation::Exact(Rc::new(value)),
        }
    }

    pub(crate) fn is_exact(&self) -> bool {
        matches!(self.representation, RevisionRepresentation::Exact(_))
    }
}

/// UI-local, object-safe capability for a container-specific layout
/// interaction.
///
/// The runtime may ask a capability for normalized hit-region declarations after
/// the container receives its final logical bounds, and version-3/version-4
/// runtimes may offer typed pointer input to the same capability. Pointer
/// capture remains a runtime-owned decision represented by
/// [`LayoutEventContext`]. Version 4 additionally supplies the optional
/// runtime-owned [`LayoutContainerStateContext`].
pub trait LayoutInteraction<Message> {
    /// Return typed revision evidence for this capability's layout behavior.
    ///
    /// Existing implementations inherit the conservative default. A custom
    /// capability may return [`LayoutInteractionRevision::exact`] when all
    /// relevant output changes are represented by an `Eq + 'static` value.
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::conservative()
    }

    /// Visit local normalized hit regions for the final container bounds.
    ///
    /// `local_bounds` is the finite final logical container bounds when the
    /// runtime calls this method. Capabilities should construct regions with
    /// [`LayoutHitRegion::new`] before visiting them. That constructor performs
    /// normalization, clipping, and finite/positive validation; duplicate
    /// identities are diagnosed during runtime projection. The default visits
    /// no regions. The visitor is object-safe and does not require a `Message`
    /// value.
    fn visit_hit_regions(&self, local_bounds: Rect, visitor: &mut dyn FnMut(LayoutHitRegion)) {
        let _ = local_bounds;
        let _ = visitor;
    }

    /// Declare optional runtime-owned state for this mounted container.
    ///
    /// The declaration is consulted only by contract version 4. The default is
    /// stateless, so existing implementations allocate no runtime state. The
    /// `container_id` argument keeps the public declaration type-safe without
    /// requiring a capability object to retain a second copy of its mounted
    /// identity.
    fn state(&self, container_id: NodeId) -> Option<ContainerStateDeclaration> {
        self.state_declaration(container_id)
    }

    /// Compatibility-named state declaration hook.
    ///
    /// [`Self::state`] is the primary hook. This additive alias keeps the
    /// declaration wording explicit for implementations that prefer it.
    fn state_declaration(&self, _container_id: NodeId) -> Option<ContainerStateDeclaration> {
        None
    }

    /// Handle one version-3 layout pointer input.
    ///
    /// The default is intentionally unhandled so version-2 projection/query
    /// capabilities and existing custom implementations remain source
    /// compatible. A callback must call [`LayoutEventContext::set_handled`] or
    /// [`LayoutEventContext::handle`] to prevent widget fallback.
    fn handle_layout_input(&self, _input: LayoutInput, _context: &mut LayoutEventContext<Message>) {
    }

    /// Handle one version-4 layout pointer input with optional typed state.
    ///
    /// Existing implementations inherit a delegation to
    /// [`Self::handle_layout_input`], preserving object safety and source
    /// compatibility. State mutation alone has no runtime side effects; the
    /// event context remains the only way to request repaint/work or emit a
    /// message.
    fn handle_layout_input_with_state(
        &self,
        input: LayoutInput,
        context: &mut LayoutEventContext<Message>,
        _state: &mut LayoutContainerStateContext<'_>,
    ) {
        self.handle_layout_input(input, context);
    }
}

/// Owned descriptor for the optional UI-local capabilities of one layout
/// container.
///
/// The descriptor is intentionally not `Send` or `Sync`: its interaction is
/// retained in an [`Rc`] and belongs to the owning UI thread. It registers
/// capability objects and their read-only projected target declarations but
/// does not route input itself or store runtime interaction state. The runtime
/// owns capture and dispatch while the descriptor supplies the capability
/// contract.
pub struct LayoutCapabilities<Message> {
    /// Descriptor contract revision understood by the runtime.
    pub contract_version: u16,
    /// Optional UI-local layout interaction capability.
    pub interaction: Option<Rc<dyn LayoutInteraction<Message>>>,
}

impl<Message> Clone for LayoutCapabilities<Message> {
    fn clone(&self) -> Self {
        Self {
            contract_version: self.contract_version,
            interaction: self.interaction.clone(),
        }
    }
}

impl<Message> LayoutCapabilities<Message> {
    /// Build a descriptor with no optional layout capabilities.
    pub const fn none() -> Self {
        Self {
            contract_version: LAYOUT_CAPABILITIES_CONTRACT_VERSION,
            interaction: None,
        }
    }

    /// Build an empty descriptor ready for explicit capability registration.
    pub const fn new() -> Self {
        Self::none()
    }

    /// Register a UI-local layout interaction capability.
    pub fn interaction(mut self, interaction: Rc<dyn LayoutInteraction<Message>>) -> Self {
        self.interaction = Some(interaction);
        self
    }

    /// Register a concrete UI-local layout interaction capability by wrapping
    /// it in an [`Rc`].
    pub fn interaction_local<I>(self, interaction: I) -> Self
    where
        I: LayoutInteraction<Message> + 'static,
    {
        self.interaction(Rc::new(interaction))
    }

    /// Return whether this descriptor exports a layout interaction capability.
    pub const fn has_interaction(&self) -> bool {
        self.interaction.is_some()
    }

    /// Return revision evidence for the optional layout interaction.
    pub fn interaction_revision(&self) -> Option<LayoutInteractionRevision> {
        self.interaction
            .as_ref()
            .map(|interaction| interaction.revision())
    }
}

impl<Message> Default for LayoutCapabilities<Message> {
    fn default() -> Self {
        Self::none()
    }
}

impl<Message> fmt::Debug for LayoutCapabilities<Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LayoutCapabilities")
            .field("contract_version", &self.contract_version)
            .field("interaction", &self.interaction.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LAYOUT_CAPABILITIES_CONTRACT_VERSION, LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION,
        LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION, LayoutCapabilities,
        LayoutContainerStateContext, LayoutEventContext, LayoutHitRegion,
        LayoutHitRegionDeclarationError, LayoutHitRegionId, LayoutInput, LayoutInteraction,
        LayoutInteractionRevision, LayoutTargetIdentity,
    };
    use crate::gui::types::{Point, Rect, Vector2};
    use std::{cell::Cell, rc::Rc};

    struct LocalInteraction {
        revision: LayoutInteractionRevision,
        local_marker: Rc<Cell<u8>>,
    }

    impl LayoutInteraction<()> for LocalInteraction {
        fn revision(&self) -> LayoutInteractionRevision {
            self.revision.clone()
        }
    }

    #[test]
    fn interaction_revision_is_typed_and_conservative_by_default() {
        assert_eq!(
            LayoutInteractionRevision::default(),
            LayoutInteractionRevision::conservative()
        );
        assert!(!LayoutInteractionRevision::conservative().is_exact());
        assert!(LayoutInteractionRevision::exact("layout").is_exact());
        assert_eq!(
            LayoutInteractionRevision::exact("layout"),
            LayoutInteractionRevision::exact("layout")
        );
        assert_ne!(
            LayoutInteractionRevision::exact("layout"),
            LayoutInteractionRevision::exact("changed")
        );
        assert_ne!(
            LayoutInteractionRevision::exact(1_u32),
            LayoutInteractionRevision::exact(1_u64)
        );

        let defaulted = LocalInteraction {
            revision: LayoutInteractionRevision::conservative(),
            local_marker: Rc::new(Cell::new(0)),
        };
        let defaulted_revision = LayoutInteraction::<()>::revision(&defaulted);
        assert_eq!(
            defaulted_revision,
            LayoutInteractionRevision::conservative()
        );
        assert_eq!(defaulted.local_marker.get(), 0);
    }

    #[test]
    fn descriptor_registers_and_clones_ui_local_rc_without_thread_bounds() {
        let absent = LayoutCapabilities::<()>::default();
        assert_eq!(
            absent.contract_version,
            LAYOUT_CAPABILITIES_CONTRACT_VERSION
        );
        assert!(!absent.has_interaction());
        assert_eq!(absent.interaction_revision(), None);

        let marker = Rc::new(Cell::new(0));
        let interaction: Rc<dyn LayoutInteraction<()>> = Rc::new(LocalInteraction {
            revision: LayoutInteractionRevision::exact("layout"),
            local_marker: Rc::clone(&marker),
        });
        let present = LayoutCapabilities::new().interaction(Rc::clone(&interaction));
        assert!(present.has_interaction());
        assert!(Rc::ptr_eq(
            present
                .interaction
                .as_ref()
                .expect("registered interaction"),
            &interaction
        ));
        assert_eq!(
            present.interaction_revision(),
            Some(LayoutInteractionRevision::exact("layout"))
        );
        let cloned = present.clone();
        assert!(Rc::ptr_eq(
            cloned.interaction.as_ref().expect("cloned interaction"),
            &interaction
        ));
    }

    #[test]
    fn hit_region_constructor_normalizes_clips_and_rejects_invalid_bounds() {
        let region = LayoutHitRegion::new(
            LayoutHitRegionId::new(7),
            Rect::from_min_max(Point::new(1.25, 0.75), Point::new(-0.25, 0.25)),
        )
        .expect("finite inverted region should normalize and clip");
        assert_eq!(region.id(), LayoutHitRegionId::new(7));
        assert_eq!(
            region.bounds(),
            Rect::from_min_max(Point::new(0.0, 0.25), Point::new(1.0, 0.75))
        );

        assert_eq!(
            LayoutHitRegion::new(
                LayoutHitRegionId::new(8),
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(f32::NAN, 1.0)),
            ),
            Err(LayoutHitRegionDeclarationError::NonFiniteBounds)
        );
        assert_eq!(
            LayoutHitRegion::new(
                LayoutHitRegionId::new(9),
                Rect::from_min_max(Point::new(0.2, 0.2), Point::new(0.2, 0.8)),
            ),
            Err(LayoutHitRegionDeclarationError::NonPositiveBounds)
        );
        assert_eq!(
            LayoutHitRegion::new(
                LayoutHitRegionId::new(10),
                Rect::from_min_max(Point::new(2.0, 0.0), Point::new(3.0, 1.0)),
            ),
            Err(LayoutHitRegionDeclarationError::NonPositiveBounds)
        );
    }

    #[test]
    fn visit_hit_regions_is_object_safe_and_default_is_empty() {
        let interaction: Rc<dyn LayoutInteraction<()>> = Rc::new(LocalInteraction {
            revision: LayoutInteractionRevision::conservative(),
            local_marker: Rc::new(Cell::new(0)),
        });
        let mut visited = Vec::new();
        interaction.visit_hit_regions(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 10.0)),
            &mut |region| visited.push(region),
        );
        assert!(visited.is_empty());
    }

    #[test]
    fn versioned_layout_input_is_object_safe_and_context_is_bounded() {
        assert_eq!(LAYOUT_CAPABILITIES_CONTRACT_VERSION, 4);
        assert_eq!(LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION, 2);
        assert_eq!(LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION, 4);

        struct InputInteraction {
            calls: Rc<Cell<u8>>,
        }

        impl LayoutInteraction<u8> for InputInteraction {
            fn handle_layout_input(
                &self,
                input: LayoutInput,
                context: &mut LayoutEventContext<u8>,
            ) {
                assert!(matches!(input, LayoutInput::PointerPress { .. }));
                self.calls.set(self.calls.get().saturating_add(1));
                context.handle();
                context.capture_pointer();
                context.request_repaint();
                context.request_work();
                assert!(context.emit_message(7));
                assert!(!context.emit_message(8));
            }
        }

        let calls = Rc::new(Cell::new(0));
        let interaction: Rc<dyn LayoutInteraction<u8>> = Rc::new(InputInteraction {
            calls: Rc::clone(&calls),
        });
        let target = LayoutTargetIdentity::new(41, LayoutHitRegionId::new(9));
        let mut context = LayoutEventContext::new(target);

        interaction.handle_layout_input(
            LayoutInput::PointerPress {
                position: Point::new(3.0, 4.0),
                button: crate::widgets::PointerButton::Primary,
                modifiers: crate::widgets::PointerModifiers::default(),
                timestamp: None,
            },
            &mut context,
        );

        assert_eq!(calls.get(), 1);
        assert_eq!(context.target(), target);
        assert!(context.handled());
        assert!(context.capture_requested());
        assert!(!context.release_requested());
        assert!(context.repaint_requested());
        assert!(context.work_requested());
        assert_eq!(context.take_message(), Some(7));
        assert_eq!(context.take_message(), None);

        let mut delegated_context = LayoutEventContext::new(target);
        let mut state = LayoutContainerStateContext::from_runtime(target.container_id, None, None);
        interaction.handle_layout_input_with_state(
            LayoutInput::PointerPress {
                position: Point::new(3.0, 4.0),
                button: crate::widgets::PointerButton::Primary,
                modifiers: crate::widgets::PointerModifiers::default(),
                timestamp: None,
            },
            &mut delegated_context,
            &mut state,
        );
        assert_eq!(calls.get(), 2);
        assert!(delegated_context.handled());
        assert_eq!(delegated_context.take_message(), Some(7));
    }
}
