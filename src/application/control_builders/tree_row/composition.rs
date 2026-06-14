use std::sync::Arc;

use crate::{
    application::{MappedWidget, ViewNode, disclosure_button, row, spacer},
    gui::list::tree_guide_indent,
    runtime::WidgetMessageMapper,
    widgets::InteractiveRowActions,
};

use super::{
    builder::TreeRowBuilder,
    hit_target::{TreeRowHitTarget, TreeRowHitTargetParts},
};

impl TreeRowBuilder {
    /// Attach interactive row actions and build the row.
    pub fn interactive_actions<Message: Clone + Send + Sync + 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        self.build(None, actions)
    }

    pub(super) fn build<Message: Clone + Send + Sync + 'static>(
        self,
        toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        let row_height = self.row_height;
        let depth = self.depth;
        let guide_style = self.guide_style;
        let row_key = self.row_key.clone();
        let hit_key = self
            .hit_key
            .clone()
            .or_else(|| row_key.as_ref().map(|key| format!("{key}-hit")));
        let expander = self.expander(toggle);
        let mut hit_target = self.hit_target(actions).fill_width().height(row_height);
        if let Some(hit_key) = hit_key {
            hit_target = hit_target.key(hit_key);
        }

        let mut row = row([tree_guide_indent(depth, guide_style), expander, hit_target])
            .spacing(1.0)
            .fill_width()
            .height(row_height);
        if let Some(row_key) = row_key {
            row = row.key(row_key);
        }
        row
    }

    fn expander<Message: Clone + Send + Sync + 'static>(
        &self,
        toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    ) -> ViewNode<Message> {
        let key = self.row_key.as_ref().map(|key| format!("{key}-expander"));
        let mut expander = if self.has_children {
            if let Some(toggle) = toggle {
                disclosure_button(self.expanded)
                    .mapped(move |_| toggle())
                    .subtle()
                    .size(self.expander_width, self.row_height)
            } else {
                disclosure_button(self.expanded)
                    .enabled(false)
                    .passive()
                    .subtle()
                    .size(self.expander_width, self.row_height)
            }
        } else {
            spacer().size(self.expander_width, self.row_height)
        };
        if let Some(key) = key {
            expander = expander.key(key);
        }
        expander
    }

    fn hit_target<Message: Clone + Send + Sync + 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        let input_id = self.input_id;
        let widget = TreeRowHitTarget::new(TreeRowHitTargetParts {
            label: self.label,
            selected: self.selected,
            drag_drop: self.drag_drop,
            palette: self.palette,
            drop_target_outline: self.drop_target_outline,
            selected_hover_marker: self.selected_hover_marker,
            normal_label_color: self.normal_label_color,
            highlighted_label_color: self.highlighted_label_color,
            actions,
        });
        let mut view = crate::application::view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::typed(|message: Message| message),
        ));
        if let Some(input_id) = input_id {
            view = view.id(input_id);
        }
        view
    }
}
