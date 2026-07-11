use crate::{
    application::ViewNode,
    widgets::{InteractiveRowVisualStateParts, WidgetId, WidgetStyle},
};

use super::InteractiveRowBuilder;
use projection::DenseInteractiveRowUnderlayChrome;

mod model;
mod policy;
mod projection;

pub use projection::interactive_row_underlay;

/// Builder for arbitrary row content backed by a generic interactive row.
pub struct InteractiveRowUnderlayBuilder<Message> {
    content: ViewNode<Message>,
    row: InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    input_key: Option<String>,
    row_key: Option<String>,
    style: Option<WidgetStyle>,
    visual_state: InteractiveRowVisualStateParts,
    chrome: DenseInteractiveRowUnderlayChrome,
    dense_chrome: bool,
}
