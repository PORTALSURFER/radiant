use crate::{gui::types::Rect, layout::NodeId};

use super::{DevtoolsInspectorProjection, DevtoolsNodeSnapshot, DevtoolsSnapshot, DevtoolsTreeRow};

impl DevtoolsSnapshot {
    /// Return flattened tree and detail data for inspector views.
    pub fn inspector_projection(&self) -> DevtoolsInspectorProjection {
        let mut tree_rows = Vec::new();
        self.root
            .push_tree_rows(self.selected_node_id, 0, &mut tree_rows);
        DevtoolsInspectorProjection {
            tree_rows,
            selected_details: selected_detail_lines(self),
            runtime_details: runtime_detail_lines(self),
        }
    }
}

impl DevtoolsNodeSnapshot {
    /// Return this node or one of its descendants by stable node id.
    pub fn find_node(&self, node_id: NodeId) -> Option<&Self> {
        if self.node_id == node_id {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.find_node(node_id))
    }

    fn push_tree_rows(
        &self,
        selected_node_id: Option<NodeId>,
        depth: usize,
        rows: &mut Vec<DevtoolsTreeRow>,
    ) {
        rows.push(self.tree_row(selected_node_id, depth));
        for child in &self.children {
            child.push_tree_rows(selected_node_id, depth + 1, rows);
        }
    }

    fn tree_row(&self, selected_node_id: Option<NodeId>, depth: usize) -> DevtoolsTreeRow {
        let widget = self.widget.as_ref();
        let state = widget.map(|widget| widget.state).unwrap_or_default();
        DevtoolsTreeRow {
            node_id: self.node_id,
            depth,
            kind: self.kind,
            label: devtools_tree_label(self, depth),
            selected: selected_node_id == Some(self.node_id),
            bounds: self.bounds,
            focusable: widget.is_some_and(|widget| widget.focusable),
            hovered: state.hovered,
            pressed: state.pressed,
            focused: state.focused,
            captured: widget.is_some_and(|widget| widget.captured),
            disabled: state.disabled,
            read_only: state.read_only,
        }
    }
}

pub(super) fn format_bounds(bounds: Option<Rect>) -> String {
    bounds
        .map(|rect| {
            format!(
                "{:.0},{:.0} {:.0}x{:.0}",
                rect.min.x,
                rect.min.y,
                rect.width(),
                rect.height()
            )
        })
        .unwrap_or_else(|| String::from("none"))
}

fn selected_detail_lines(snapshot: &DevtoolsSnapshot) -> Vec<String> {
    let node = snapshot.selected_node().unwrap_or(&snapshot.root);
    let mut lines = vec![
        format!("id: #{}", node.node_id),
        format!("type: {:?}", node.kind),
        format!("bounds: {}", format_bounds(node.bounds)),
        format!("children: {}", node.children.len()),
    ];
    if let Some(widget) = node.widget.as_ref() {
        lines.push(format!("role: {:?}", widget.semantics.role));
        if let Some(label) = &widget.semantics.label {
            lines.push(format!("label: {label}"));
        }
        if let Some(value) = &widget.semantics.value_text {
            lines.push(format!("value: {value}"));
        }
        if let Some(checked) = widget.semantics.checked {
            lines.push(format!("checked: {checked}"));
        }
        lines.push(format!(
            "semantics: selected={} disabled={} read_only={} focusable={} focused={} live={:?}",
            widget.semantics.selected,
            widget.semantics.disabled,
            widget.semantics.read_only,
            widget.semantics.focusable,
            widget.semantics.focused,
            widget.semantics.live_region
        ));
        lines.push(format!("focus: {:?}", widget.focus));
        lines.push(format!(
            "flags: focusable={} key={} hit={} wheel={} move={}",
            widget.focusable,
            widget.keyboard_focusable,
            widget.receives_pointer_hit_testing,
            widget.accepts_wheel_input,
            widget.accepts_pointer_move
        ));
        lines.push(format!(
            "state: hover={} pressed={} focused={} captured={} disabled={} read_only={}",
            widget.state.hovered,
            widget.state.pressed,
            widget.state.focused,
            widget.captured,
            widget.state.disabled,
            widget.state.read_only
        ));
    }
    if node.layout_diagnostics.is_empty() {
        lines.push(String::from("layout diagnostics: none"));
    } else {
        lines.push(format!(
            "layout diagnostics: {}",
            node.layout_diagnostics.len()
        ));
        lines.extend(
            node.layout_diagnostics
                .iter()
                .take(2)
                .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message)),
        );
    }
    lines
}

fn runtime_detail_lines(snapshot: &DevtoolsSnapshot) -> Vec<String> {
    vec![
        format!(
            "paint: total={} text={} gpu={} clips={}",
            snapshot.paint.total,
            snapshot.paint.text,
            snapshot.paint.gpu_surfaces,
            snapshot.paint.clips
        ),
        format!(
            "ui handlers: total={} slow={}",
            snapshot.diagnostics.ui.update_handlers, snapshot.diagnostics.ui.slow_update_handlers
        ),
        format!(
            "business: queued={} running={} completed={} cancelled={}",
            snapshot.diagnostics.business.queued,
            snapshot.diagnostics.business.running,
            snapshot.diagnostics.business.completed,
            snapshot.diagnostics.business.cancelled
        ),
        format!(
            "business max queue={:.1}ms run={:.1}ms",
            snapshot.diagnostics.business.max_queue_delay.as_secs_f64() * 1000.0,
            snapshot.diagnostics.business.max_run_duration.as_secs_f64() * 1000.0
        ),
    ]
}

fn devtools_tree_label(node: &DevtoolsNodeSnapshot, depth: usize) -> String {
    let mut label = format!("{}{:?} #{}", "  ".repeat(depth), node.kind, node.node_id);
    if let Some(bounds) = node.bounds {
        label.push_str(&format!(" {}", format_bounds(Some(bounds))));
    }
    if let Some(widget) = node.widget.as_ref() {
        label.push_str(&format!(" role={:?}", widget.semantics.role));
        if let Some(semantic_label) = &widget.semantics.label {
            label.push_str(&format!(" label=\"{semantic_label}\""));
        }
        if widget.state.hovered {
            label.push_str(" hover");
        }
        if widget.state.pressed {
            label.push_str(" pressed");
        }
        if widget.state.focused {
            label.push_str(" focused");
        }
        if widget.captured {
            label.push_str(" captured");
        }
        if widget.state.disabled {
            label.push_str(" disabled");
        }
        if widget.state.read_only {
            label.push_str(" read-only");
        }
    }
    label
}
