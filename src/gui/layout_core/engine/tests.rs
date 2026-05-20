//! Unit tests for strict slot layout engine behavior.

use crate::gui::layout_core::{
    constraints::Constraints,
    model::{SizeModeCross, SizeModeMain, SlotParams},
};

mod debug;
mod diagnostics;
mod layout;
mod scroll;

#[path = "tests/scratch.rs"]
mod scratch;

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Intrinsic,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}
