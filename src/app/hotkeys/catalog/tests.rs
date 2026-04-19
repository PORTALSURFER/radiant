use crate::app::{hotkeys::iter_hotkey_bindings, FocusContextModel, UiAction};

use super::HOTKEY_BINDINGS;

#[test]
fn hotkey_ids_are_unique() {
    for (index, binding) in HOTKEY_BINDINGS.iter().enumerate() {
        let duplicate = HOTKEY_BINDINGS
            .iter()
            .skip(index + 1)
            .find(|candidate| candidate.id == binding.id);
        assert!(duplicate.is_none(), "duplicate hotkey id: {}", binding.id);
    }
}

#[test]
fn hotkey_gestures_are_unique_within_scope() {
    for (index, binding) in HOTKEY_BINDINGS.iter().enumerate() {
        let duplicate = HOTKEY_BINDINGS.iter().skip(index + 1).find(|candidate| {
            candidate.scope == binding.scope && candidate.gesture == binding.gesture
        });
        assert!(
            duplicate.is_none(),
            "duplicate scoped hotkey gesture for {:?}: {:?}",
            binding.scope,
            binding.gesture
        );
    }
}

#[test]
fn source_list_actions_are_scoped_to_sources_focus() {
    let source_actions: Vec<_> = iter_hotkey_bindings()
        .filter(|binding| binding.scope == super::super::SOURCES_SCOPE)
        .collect();
    assert!(!source_actions.is_empty());
    assert!(source_actions
        .iter()
        .any(|binding| matches!(binding.action, UiAction::ReloadFocusedSourceRow)));
    assert!(source_actions
        .iter()
        .any(|binding| matches!(binding.action, UiAction::MoveSourceFocus { delta: -1 })));
    assert!(iter_hotkey_bindings().all(|binding| {
        binding.scope != super::super::SOURCES_SCOPE
            || binding.scope.matches(FocusContextModel::SourcesList)
    }));
}
