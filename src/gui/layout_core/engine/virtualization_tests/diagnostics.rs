use super::*;

#[test]
fn virtualization_policy_is_ignored_for_unsupported_content_kind() {
    let root = scroll_with_content(ContainerKind::Wrap, 128, VirtualizationAxis::Vertical, 0.0);
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationPolicyIgnored)
    );
    assert!(!output.virtual_windows.contains_key(&1));
}

#[test]
fn virtualization_debug_primitives_are_emitted() {
    let root = scroll_with_content(
        ContainerKind::Column,
        512,
        VirtualizationAxis::Vertical,
        8.0,
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 320.0));
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 100.0)),
        &state,
        LayoutDebugOptions::all_enabled(),
    );

    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::ViewportBounds)
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::VirtualWindowBounds)
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::CulledRegion)
    );
}

#[test]
fn invalid_virtualization_overscan_is_clamped() {
    let root = scroll_with_content(
        ContainerKind::Column,
        128,
        VirtualizationAxis::Vertical,
        -32.0,
    );
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 100.0)),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationWindowClamped)
    );
}
