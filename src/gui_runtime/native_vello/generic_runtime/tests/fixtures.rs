#[path = "fixtures/demo.rs"]
mod demo;
#[path = "fixtures/frame.rs"]
mod frame;
#[path = "fixtures/gpu_wheel.rs"]
mod gpu_wheel;
#[path = "fixtures/input.rs"]
mod input;
#[path = "fixtures/native_pointer.rs"]
mod native_pointer;

pub(super) use demo::*;
pub(super) use frame::*;
pub(super) use gpu_wheel::*;
pub(super) use input::*;
pub(super) use native_pointer::*;
