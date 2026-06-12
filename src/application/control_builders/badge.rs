//! Application builders for generic badge and pill controls.

mod interactive;
mod message;
mod passive;

#[cfg(test)]
mod tests;

pub use interactive::{InteractiveBadgeBuilder, interactive_badge};
pub use message::{badge_mapped, badge_message};
pub use passive::{BadgeBuilder, badge};
