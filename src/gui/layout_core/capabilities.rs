//! UI-local capability registration and layout-target declarations for
//! backend-neutral layout containers.
//!
//! This module establishes the registration, revision-evidence, and
//! declaration boundary for optional container layout interaction. It does not
//! define pointer input or context types, capture, runtime-local state storage,
//! or event handling. Declared regions are projected for read-only inspection;
//! routing those targets remains a later layer.

use std::{any::Any, fmt, rc::Rc};

use super::tree::NodeId;
use crate::gui::types::Rect;

/// Contract revision understood by [`LayoutCapabilities`].
pub const LAYOUT_CAPABILITIES_CONTRACT_VERSION: u16 = 2;

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
/// The runtime may ask a capability for normalized hit-region
/// declarations after the container receives its final logical bounds. Those
/// declarations are projected into a read-only runtime target index. Pointer
/// routing, capture, runtime-local state, and event handling are deliberately
/// not part of this slice.
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
}

/// Owned descriptor for the optional UI-local capabilities of one layout
/// container.
///
/// The descriptor is intentionally not `Send` or `Sync`: its interaction is
/// retained in an [`Rc`] and belongs to the owning UI thread. It registers
/// capability objects and their read-only projected target declarations but
/// does not route input or provide runtime interaction state. A registered
/// descriptor therefore remains a declaration/projection contract, not a
/// shipped `split_pane` runtime.
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
        LAYOUT_CAPABILITIES_CONTRACT_VERSION, LayoutCapabilities, LayoutHitRegion,
        LayoutHitRegionDeclarationError, LayoutHitRegionId, LayoutInteraction,
        LayoutInteractionRevision,
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
}
