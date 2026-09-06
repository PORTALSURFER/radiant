//! Deterministic backend-neutral focus observation, transfer and spatial traversal.
use radiant::prelude::*;
use radiant::{
    layout::Vector2,
    runtime::{FocusDirection, FocusScope, FocusTransferOutcome, FocusTraversal, SurfaceRuntime},
};

fn exercise() {
    let mut runtime = SurfaceRuntime::new(
        app(())
            .view(|_: &()| {
                column([
                    button("Open").message(()).id(1),
                    button("Save").message(()).id(2),
                ])
                .id(100)
                .focus_scope(FocusScope::spatial_grid())
            })
            .update(|_, _| {})
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let target = runtime
        .focus_target(1)
        .expect("current materialized button");
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.transfer_focus(&target),
        FocusTransferOutcome::Admitted(1)
    );
    let bookmark = runtime.capture_focus().expect("focused widget");
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::Admitted(2)
    );
    assert_eq!(
        runtime.restore_focus(&bookmark),
        FocusTransferOutcome::Admitted(1)
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::Admitted(2)
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::NoDestination
    );
}
fn main() {
    exercise();
    println!("Observed a focus target, transferred focus, and traversed current geometry.");
}
#[cfg(test)]
mod tests {
    #[test]
    fn deterministic_focus_navigation_uses_public_runtime_authority() {
        super::exercise();
    }
}
