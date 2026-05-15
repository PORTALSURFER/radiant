use crate::gui_runtime::native_vello::UiRect;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SceneClipState {
    pushed_depth: usize,
    suppressed_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SceneClipBegin {
    PushLayer,
    Suppress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SceneClipEnd {
    PopLayer,
    Suppressed,
    Unmatched,
}

impl SceneClipBegin {
    pub(super) fn pushes_layer(self) -> bool {
        matches!(self, Self::PushLayer)
    }
}

impl SceneClipEnd {
    pub(super) fn pops_layer(self) -> bool {
        matches!(self, Self::PopLayer)
    }
}

impl SceneClipState {
    pub(super) fn begin(&mut self, rect: UiRect) -> SceneClipBegin {
        if self.is_suppressed() || !rect.has_finite_positive_area() {
            self.suppressed_depth = self.suppressed_depth.saturating_add(1);
            return SceneClipBegin::Suppress;
        }
        self.pushed_depth = self.pushed_depth.saturating_add(1);
        SceneClipBegin::PushLayer
    }

    pub(super) fn end(&mut self) -> SceneClipEnd {
        if self.suppressed_depth > 0 {
            self.suppressed_depth -= 1;
            return SceneClipEnd::Suppressed;
        }
        if self.pushed_depth > 0 {
            self.pushed_depth -= 1;
            return SceneClipEnd::PopLayer;
        }
        SceneClipEnd::Unmatched
    }

    pub(super) fn is_suppressed(&self) -> bool {
        self.suppressed_depth > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Point;

    #[test]
    fn scene_clip_state_suppresses_invalid_clip_until_matching_end() {
        let valid = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let invalid = UiRect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(10.0, 10.0));
        let mut state = SceneClipState::default();

        assert_eq!(state.begin(valid), SceneClipBegin::PushLayer);
        assert!(!state.is_suppressed());
        assert_eq!(state.begin(invalid), SceneClipBegin::Suppress);
        assert!(state.is_suppressed());
        assert_eq!(state.begin(valid), SceneClipBegin::Suppress);
        assert!(state.is_suppressed());
        assert_eq!(state.end(), SceneClipEnd::Suppressed);
        assert!(state.is_suppressed());
        assert_eq!(state.end(), SceneClipEnd::Suppressed);
        assert!(!state.is_suppressed());
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
        assert_eq!(state.end(), SceneClipEnd::Unmatched);
    }
}
