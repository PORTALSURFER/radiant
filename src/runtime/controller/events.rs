mod dispatch;
mod model;
mod outcome;
mod pointer;
#[cfg(test)]
mod tests;

pub use model::Event;
pub use outcome::{PointerClickOutcome, PointerMoveOutcome};
