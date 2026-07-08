//! Application builders for generic interactive row input surfaces.

mod builder;

#[cfg(test)]
mod tests;

pub use crate::widgets::InteractiveRowActions;
pub use builder::{
    DenseRowPolicy, InteractiveRowBuilder, InteractiveRowUnderlayBuilder, interactive_row,
    interactive_row_underlay, row_actions,
};
