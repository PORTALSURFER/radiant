mod chrome;
mod geometry;
mod layer;
mod parts;
#[cfg(test)]
mod tests;

pub use chrome::{
    closeable_panel_section_from_parts, panel_section, panel_section_from_header_parts,
    panel_section_from_parts, panel_section_resize_header,
};
pub use geometry::PanelSectionGeometry;
pub use layer::{
    DialogLayerParts, PanelSectionLayerParts, closeable_dialog_layer,
    closeable_dialog_layer_from_parts, closeable_panel_section_layer_from_parts, dialog_layer,
    dialog_layer_from_parts, panel_section_layer_from_parts,
};
pub use parts::{PanelSectionHeaderParts, PanelSectionParts};
