use crate::gui_runtime::native_vello::UiRect;

#[derive(Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct SceneClipState {
    frames: Vec<SceneClipFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SceneClipFrame {
    Pushed { effective_rect: UiRect },
    Redundant { effective_rect: UiRect },
    Suppressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum SceneClipBegin {
    PushLayer,
    Redundant,
    Suppress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum SceneClipEnd {
    PopLayer,
    Redundant,
    Suppressed,
    Unmatched,
}

impl SceneClipBegin {
    pub(in crate::gui_runtime::native_vello) fn pushes_layer(self) -> bool {
        matches!(self, Self::PushLayer)
    }
}

impl SceneClipEnd {
    pub(in crate::gui_runtime::native_vello) fn pops_layer(self) -> bool {
        matches!(self, Self::PopLayer)
    }
}

impl SceneClipState {
    pub(in crate::gui_runtime::native_vello) fn begin(&mut self, rect: UiRect) -> SceneClipBegin {
        if self.is_suppressed() || !rect.has_finite_positive_area() {
            self.frames.push(SceneClipFrame::Suppressed);
            return SceneClipBegin::Suppress;
        }
        if let Some(active_rect) = self.active_effective_rect() {
            if rect_contains_rect(rect, active_rect) {
                self.frames.push(SceneClipFrame::Redundant {
                    effective_rect: active_rect,
                });
                return SceneClipBegin::Redundant;
            }
            self.frames.push(SceneClipFrame::Pushed {
                effective_rect: active_rect.clamp_to(rect),
            });
        } else {
            self.frames.push(SceneClipFrame::Pushed {
                effective_rect: rect,
            });
        }
        SceneClipBegin::PushLayer
    }

    pub(in crate::gui_runtime::native_vello) fn end(&mut self) -> SceneClipEnd {
        match self.frames.pop() {
            Some(SceneClipFrame::Suppressed) => SceneClipEnd::Suppressed,
            Some(SceneClipFrame::Redundant { .. }) => SceneClipEnd::Redundant,
            Some(SceneClipFrame::Pushed { .. }) => SceneClipEnd::PopLayer,
            None => SceneClipEnd::Unmatched,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn is_suppressed(&self) -> bool {
        matches!(self.frames.last(), Some(SceneClipFrame::Suppressed))
    }

    fn active_effective_rect(&self) -> Option<UiRect> {
        match self.frames.last()? {
            SceneClipFrame::Pushed { effective_rect }
            | SceneClipFrame::Redundant { effective_rect } => Some(*effective_rect),
            SceneClipFrame::Suppressed => None,
        }
    }
}

fn rect_contains_rect(container: UiRect, inner: UiRect) -> bool {
    container.min.x <= inner.min.x
        && container.min.y <= inner.min.y
        && container.max.x >= inner.max.x
        && container.max.y >= inner.max.y
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

    #[test]
    fn containing_nested_clip_is_redundant() {
        let mut state = SceneClipState::default();
        let ancestor = UiRect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let containing = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(30.0, 30.0));

        assert_eq!(state.begin(ancestor), SceneClipBegin::PushLayer);
        assert_eq!(state.begin(containing), SceneClipBegin::Redundant);
        assert_eq!(state.end(), SceneClipEnd::Redundant);
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
    }

    #[test]
    fn narrowing_nested_clip_still_pushes_and_updates_effective_clip() {
        let mut state = SceneClipState::default();
        let ancestor = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(30.0, 30.0));
        let narrowing = UiRect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let containing_narrowing =
            UiRect::from_min_max(Point::new(5.0, 5.0), Point::new(25.0, 25.0));

        assert_eq!(state.begin(ancestor), SceneClipBegin::PushLayer);
        assert_eq!(state.begin(narrowing), SceneClipBegin::PushLayer);
        assert_eq!(state.begin(containing_narrowing), SceneClipBegin::Redundant);
        assert_eq!(state.end(), SceneClipEnd::Redundant);
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
    }

    #[test]
    fn nested_clip_under_redundant_frame_uses_same_active_ancestor() {
        let mut state = SceneClipState::default();
        let ancestor = UiRect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let containing = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(30.0, 30.0));
        let narrowing = UiRect::from_min_max(Point::new(12.0, 12.0), Point::new(18.0, 18.0));

        assert_eq!(state.begin(ancestor), SceneClipBegin::PushLayer);
        assert_eq!(state.begin(containing), SceneClipBegin::Redundant);
        assert_eq!(state.begin(narrowing), SceneClipBegin::PushLayer);
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
        assert_eq!(state.end(), SceneClipEnd::Redundant);
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
    }

    #[test]
    fn invalid_clip_suppresses_nested_frames_until_balanced() {
        let valid = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let invalid = UiRect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(10.0, 10.0));
        let mut state = SceneClipState::default();

        assert_eq!(state.begin(valid), SceneClipBegin::PushLayer);
        assert_eq!(state.begin(invalid), SceneClipBegin::Suppress);
        assert_eq!(state.begin(valid), SceneClipBegin::Suppress);
        assert!(state.is_suppressed());
        assert_eq!(state.end(), SceneClipEnd::Suppressed);
        assert_eq!(state.end(), SceneClipEnd::Suppressed);
        assert!(!state.is_suppressed());
        assert_eq!(state.end(), SceneClipEnd::PopLayer);
    }

    #[test]
    fn unmatched_clip_end_is_explicit() {
        assert_eq!(SceneClipState::default().end(), SceneClipEnd::Unmatched);
    }
}
