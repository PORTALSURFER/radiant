//! Bounded records of observable layout work needed when reusing geometry.

use super::*;
use crate::gui::layout_core::{constraints::Constraints, engine::LayoutDiagnostic};

pub(super) enum Event {
    Measure {
        child: usize,
        constraints: Constraints,
    },
    Diagnostic(LayoutDiagnostic),
}

pub(in crate::gui::layout_core::engine) struct Trace {
    children: Vec<NodeId>,
    pub(super) events: Vec<Event>,
    measuring: bool,
    pub(super) aborted: bool,
}

impl Trace {
    pub(super) fn new(root: &LayoutNode) -> Option<Self> {
        let LayoutNode::Container(container) = root else {
            return None;
        };
        if container.children.len() + 1 < MIN_FRAGMENT_NODES
            || container.children.len() + 1 > MAX_FRAGMENT_NODES
            || NodeInput::from_node(root, None).is_none()
            || !container
                .children
                .iter()
                .all(|child| matches!(child.child, LayoutNode::Widget(_)))
        {
            return None;
        }
        Some(Self {
            children: container
                .children
                .iter()
                .map(|child| child.child.id())
                .collect(),
            events: Vec::new(),
            measuring: false,
            aborted: false,
        })
    }

    fn record(&mut self, event: Event) {
        if self.aborted {
            return;
        }
        if self.events.len() == MAX_FRAGMENT_EVENTS {
            self.aborted = true;
            self.events.clear();
            return;
        }
        self.events.push(event);
    }

    pub(in crate::gui::layout_core::engine) fn begin_measure(
        &mut self,
        node: &LayoutNode,
        constraints: Constraints,
    ) {
        if self.measuring {
            self.aborted = true;
            return;
        }
        let Some(child) = self.children.iter().position(|id| *id == node.id()) else {
            self.aborted = true;
            return;
        };
        self.record(Event::Measure { child, constraints });
        self.measuring = true;
    }

    pub(in crate::gui::layout_core::engine) fn end_measure(&mut self) {
        self.measuring = false;
    }

    pub(in crate::gui::layout_core::engine) fn diagnostic(
        &mut self,
        diagnostic: &LayoutDiagnostic,
    ) {
        if !self.measuring {
            self.record(Event::Diagnostic(diagnostic.clone()));
        }
    }
}

impl Fragment {
    pub(in crate::gui::layout_core::engine) fn replay(
        &self,
        root: &LayoutNode,
        context: &mut super::super::context::LayoutContext<'_>,
    ) {
        let LayoutNode::Container(container) = root else {
            unreachable!("only captured flat containers replay")
        };
        for event in &self.events {
            match event {
                Event::Measure { child, constraints } => {
                    super::super::measure::measure_node(
                        &container.children[*child].child,
                        *constraints,
                        context,
                    );
                }
                Event::Diagnostic(diagnostic) => {
                    context.output.diagnostics.push(diagnostic.clone())
                }
            }
        }
        for (node, rect) in &self.nodes {
            context.output.rects.insert(node.id, *rect);
        }
        context.output.stats.materialized_nodes += self.nodes.len();
    }
}
