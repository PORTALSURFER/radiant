//! Shared child storage for immutable surface snapshots.

use super::{SurfaceChild, SurfaceNode};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub(in crate::runtime::surface) struct SurfaceChildren<Message> {
    children: Rc<Vec<SurfaceChild<Message>>>,
    clone_safe: bool,
}

impl<Message> Clone for SurfaceChildren<Message> {
    fn clone(&self) -> Self {
        if self.clone_safe {
            Self {
                children: Rc::clone(&self.children),
                clone_safe: true,
            }
        } else {
            Self::from(self.children.as_ref().clone())
        }
    }
}

impl<Message> From<Vec<SurfaceChild<Message>>> for SurfaceChildren<Message> {
    fn from(children: Vec<SurfaceChild<Message>>) -> Self {
        let clone_safe = children.iter().all(|child| match &child.child {
            SurfaceNode::Container(container) => container.children.clone_safe,
            SurfaceNode::Widget(widget) => {
                widget.widget().as_any().is::<crate::widgets::TextWidget>()
            }
            _ => false,
        });
        Self {
            children: Rc::new(children),
            clone_safe,
        }
    }
}

impl<Message> FromIterator<SurfaceChild<Message>> for SurfaceChildren<Message> {
    fn from_iter<T: IntoIterator<Item = SurfaceChild<Message>>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<Message> Deref for SurfaceChildren<Message> {
    type Target = Vec<SurfaceChild<Message>>;

    fn deref(&self) -> &Self::Target {
        &self.children
    }
}

impl<Message> DerefMut for SurfaceChildren<Message> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // A mutable borrow can install arbitrary custom widgets. Keep later
        // clones conservative until a fresh owned vector is classified.
        self.clone_safe = false;
        Rc::make_mut(&mut self.children)
    }
}

impl<Message> IntoIterator for SurfaceChildren<Message> {
    type Item = SurfaceChild<Message>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        Rc::unwrap_or_clone(self.children).into_iter()
    }
}

impl<'a, Message> IntoIterator for &'a SurfaceChildren<Message> {
    type Item = &'a SurfaceChild<Message>;
    type IntoIter = std::slice::Iter<'a, SurfaceChild<Message>>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.iter()
    }
}

impl<'a, Message> IntoIterator for &'a mut SurfaceChildren<Message> {
    type Item = &'a mut SurfaceChild<Message>;
    type IntoIter = std::slice::IterMut<'a, SurfaceChild<Message>>;

    fn into_iter(self) -> Self::IntoIter {
        self.clone_safe = false;
        Rc::make_mut(&mut self.children).iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::ContainerPolicy,
        runtime::{SurfaceContainer, SurfaceNode},
    };

    enum NonCloneMessage {}

    struct CloneProbe {
        common: crate::widgets::WidgetCommon,
        clones: Rc<std::cell::Cell<usize>>,
        interior: std::cell::Cell<usize>,
    }

    impl Clone for CloneProbe {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                common: self.common.clone(),
                clones: Rc::clone(&self.clones),
                interior: std::cell::Cell::new(self.interior.get()),
            }
        }
    }

    impl crate::widgets::Widget for CloneProbe {
        fn common(&self) -> &crate::widgets::WidgetCommon {
            &self.common
        }
        fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
            &mut self.common
        }
        fn handle_input(
            &mut self,
            _: crate::gui::types::Rect,
            _: crate::widgets::WidgetInput,
        ) -> Option<crate::widgets::WidgetOutput> {
            None
        }
        fn append_paint(
            &self,
            _: &mut Vec<crate::runtime::PaintPrimitive>,
            _: crate::gui::types::Rect,
            _: &crate::layout::LayoutOutput,
            _: &crate::theme::ThemeTokens,
        ) {
        }
    }

    #[test]
    fn custom_widget_clone_hooks_and_interior_state_remain_independent() {
        let clones = Rc::new(std::cell::Cell::new(0));
        let original = SurfaceContainer::<NonCloneMessage>::new(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(SurfaceNode::static_widget(CloneProbe {
                common: crate::widgets::WidgetCommon::fixed(2, 20.0, 20.0),
                clones: Rc::clone(&clones),
                interior: std::cell::Cell::new(0),
            }))],
        );
        let snapshot = original.clone();
        assert_eq!(clones.get(), 1);
        let probe = |container: &SurfaceContainer<NonCloneMessage>| {
            let SurfaceNode::Widget(widget) = &container.children[0].child else {
                panic!("widget fixture");
            };
            widget
                .widget()
                .as_any()
                .downcast_ref::<CloneProbe>()
                .unwrap()
                .interior
                .get()
        };
        let SurfaceNode::Widget(widget) = &original.children[0].child else {
            panic!("widget fixture");
        };
        widget
            .widget()
            .as_any()
            .downcast_ref::<CloneProbe>()
            .unwrap()
            .interior
            .set(7);
        assert_eq!(probe(&original), 7);
        assert_eq!(probe(&snapshot), 0);
    }

    #[test]
    fn inserting_a_custom_widget_retires_prior_sharing_eligibility() {
        let clones = Rc::new(std::cell::Cell::new(0));
        let mut changed =
            SurfaceContainer::<NonCloneMessage>::new(1, ContainerPolicy::default(), Vec::new());
        let original = changed.clone();
        changed
            .children
            .push(SurfaceChild::fill(SurfaceNode::static_widget(CloneProbe {
                common: crate::widgets::WidgetCommon::fixed(2, 20.0, 20.0),
                clones: Rc::clone(&clones),
                interior: std::cell::Cell::new(0),
            })));
        let snapshot = changed.clone();
        assert!(original.children.is_empty());
        assert_eq!(clones.get(), 1);
        assert_eq!(snapshot.children[0].child.id(), 2);
    }

    fn child(id: u64) -> SurfaceChild<NonCloneMessage> {
        SurfaceChild::fill(SurfaceNode::Container(SurfaceContainer::new(
            id,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    #[test]
    fn editing_a_cloned_container_preserves_the_previous_snapshot() {
        let original = SurfaceContainer::new(1, ContainerPolicy::default(), vec![child(2)]);
        let mut changed = original.clone();
        changed.children.push(child(3));
        let SurfaceNode::Container(nested) = &mut changed.children[0].child else {
            panic!("container fixture");
        };
        nested.id = 20;
        nested.children.push(child(21));
        assert_eq!(original.children.len(), 1);
        assert_eq!(original.children[0].child.id(), 2);
        let SurfaceNode::Container(nested) = &original.children[0].child else {
            panic!("container fixture");
        };
        assert!(nested.children.is_empty());
        assert_eq!(changed.children.len(), 2);
        assert_eq!(changed.children[0].child.id(), 20);
    }

    #[test]
    fn consuming_a_clone_keeps_the_retained_snapshot_available() {
        let original: SurfaceChildren<_> = vec![child(2), child(3)].into();
        let retained = original.clone();
        let ids: Vec<_> = original.into_iter().map(|child| child.child.id()).collect();
        assert_eq!(ids, [2, 3]);
        assert_eq!(
            retained
                .iter()
                .map(|child| child.child.id())
                .collect::<Vec<_>>(),
            ids
        );
    }
}
