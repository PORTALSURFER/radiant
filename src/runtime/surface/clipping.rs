use super::*;
use crate::layout::ContainerKind;
use std::collections::BTreeMap;

impl<Message> SurfaceNode<Message> {
    fn collect_widget_clip_ancestors(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        clips: &mut BTreeMap<WidgetId, Vec<NodeId>>,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    scroll_stack.push(container.id);
                }
                for child in &container.children {
                    child
                        .child
                        .collect_widget_clip_ancestors(scroll_stack, clips);
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(widget) => {
                if !scroll_stack.is_empty() {
                    clips.insert(widget.id(), scroll_stack.clone());
                }
            }
            Self::Overlay(_) => {}
        }
    }

    fn collect_container_clip_ancestors(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        clips: &mut BTreeMap<NodeId, Vec<NodeId>>,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    scroll_stack.push(container.id);
                }
                if container.style.is_some() && container.hoverable && !scroll_stack.is_empty() {
                    clips.insert(container.id, scroll_stack.clone());
                }
                for child in &container.children {
                    child
                        .child
                        .collect_container_clip_ancestors(scroll_stack, clips);
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn widget_clip_ancestors(&self) -> BTreeMap<WidgetId, Vec<NodeId>> {
        let mut clips = BTreeMap::new();
        self.root
            .collect_widget_clip_ancestors(&mut Vec::new(), &mut clips);
        clips
    }

    pub(in crate::runtime) fn container_clip_ancestors(&self) -> BTreeMap<NodeId, Vec<NodeId>> {
        let mut clips = BTreeMap::new();
        self.root
            .collect_container_clip_ancestors(&mut Vec::new(), &mut clips);
        clips
    }
}
