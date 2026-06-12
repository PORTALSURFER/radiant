//! Compatibility-only direct state-callback application types.
//!
//! New application code should use `View<Message>` plus explicit update
//! handlers. This module remains only for older composite helpers while they
//! are migrated to message-first APIs.

#[allow(unused_imports)]
pub use super::details_list::{selectable_sortable_details_list, sortable_details_list};
#[allow(unused_imports)]
pub use super::menu::{ContextMenuOverlayParts, MenuItem, MenuItemParts, MenuParts};
#[allow(unused_imports)]
pub use super::menu::{
    context_menu_overlay, context_menu_overlay_from_parts, menu, menu_from_parts,
};
#[allow(unused_imports)]
pub use super::property_panel::selectable_property_panel;
pub use super::state::StateAction;
#[allow(unused_imports)]
pub use super::tree_list::{tree_list, tree_list_with_drag};

/// Application view node type for direct state-callback apps.
pub type StateView<State> = super::View<StateAction<State>>;
