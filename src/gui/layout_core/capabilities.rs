//! UI-local capability registration for backend-neutral layout containers.
//!
//! This module establishes the registration and revision-evidence boundary for
//! optional container layout interaction. It intentionally does not define
//! pointer input or context types, hit regions, capture, runtime-local state
//! storage, or event handling. In particular, registering a capability here is
//! not a shipped `split_pane` runtime; those interaction pieces remain a later
//! layer.

use std::{any::Any, fmt, rc::Rc};

/// Contract revision understood by [`LayoutCapabilities`].
pub const LAYOUT_CAPABILITIES_CONTRACT_VERSION: u16 = 1;

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
/// This initial contract exposes revision evidence only. Pointer input,
/// layout-event contexts, hit-region routing, capture, runtime-local state,
/// and event handling are deliberately not part of this slice.
pub trait LayoutInteraction<Message> {
    /// Return typed revision evidence for this capability's layout behavior.
    ///
    /// Existing implementations inherit the conservative default. A custom
    /// capability may return [`LayoutInteractionRevision::exact`] when all
    /// relevant output changes are represented by an `Eq + 'static` value.
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::conservative()
    }
}

/// Owned descriptor for the optional UI-local capabilities of one layout
/// container.
///
/// The descriptor is intentionally not `Send` or `Sync`: its interaction is
/// retained in an [`Rc`] and belongs to the owning UI thread. It registers
/// capability objects but does not route input or provide runtime interaction
/// state. A registered descriptor therefore remains registration and revision
/// evidence, not a shipped `split_pane` runtime.
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
        LAYOUT_CAPABILITIES_CONTRACT_VERSION, LayoutCapabilities, LayoutInteraction,
        LayoutInteractionRevision,
    };
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
}
