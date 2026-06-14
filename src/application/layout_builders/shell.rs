//! Generic application shell layout builder.

use crate::{
    application::{Overlays, ViewNode, column, row},
    widgets::WidgetStyle,
};

/// Builder for a common application shell: top bar, workspace row, and status bar.
pub struct WorkspaceShellBuilder<Message> {
    workspace: ViewNode<Message>,
    top_bar: Option<ViewNode<Message>>,
    leading_sidebar: Option<ViewNode<Message>>,
    trailing_sidebar: Option<ViewNode<Message>>,
    status_bar: Option<ViewNode<Message>>,
    outer_spacing: f32,
    workspace_spacing: f32,
    padding: f32,
    style: Option<WidgetStyle>,
    overlays: Option<Overlays<Message>>,
}

impl<Message> WorkspaceShellBuilder<Message> {
    /// Add the top bar region above the workspace row.
    pub fn top_bar(mut self, view: ViewNode<Message>) -> Self {
        self.top_bar = Some(view);
        self
    }

    /// Add the leading sidebar region before the main workspace.
    pub fn leading_sidebar(mut self, view: ViewNode<Message>) -> Self {
        self.leading_sidebar = Some(view);
        self
    }

    /// Add the trailing sidebar region after the main workspace.
    pub fn trailing_sidebar(mut self, view: ViewNode<Message>) -> Self {
        self.trailing_sidebar = Some(view);
        self
    }

    /// Add the status bar region below the workspace row.
    pub fn status_bar(mut self, view: ViewNode<Message>) -> Self {
        self.status_bar = Some(view);
        self
    }

    /// Set spacing between top, workspace, and status regions.
    pub fn outer_spacing(mut self, spacing: f32) -> Self {
        self.outer_spacing = spacing;
        self
    }

    /// Set spacing between leading sidebar, workspace, and trailing sidebar.
    pub fn workspace_spacing(mut self, spacing: f32) -> Self {
        self.workspace_spacing = spacing;
        self
    }

    /// Set outer shell padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set outer shell style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Attach view-local overlays to the shell owner region.
    pub fn overlays(mut self, overlays: Overlays<Message>) -> Self {
        self.overlays = Some(overlays);
        self
    }

    /// Build the shell view.
    pub fn build(self) -> ViewNode<Message> {
        let mut workspace_children = Vec::new();
        workspace_children.extend(self.leading_sidebar);
        workspace_children.push(self.workspace);
        workspace_children.extend(self.trailing_sidebar);

        let workspace_row = row(workspace_children)
            .fill()
            .spacing(self.workspace_spacing);

        let mut shell_children = Vec::new();
        shell_children.extend(self.top_bar);
        shell_children.push(workspace_row);
        shell_children.extend(self.status_bar);

        let mut shell = column(shell_children)
            .padding(self.padding)
            .spacing(self.outer_spacing)
            .fill();
        if let Some(style) = self.style {
            shell = shell.style(style);
        }
        if let Some(overlays) = self.overlays {
            shell = shell.overlays(overlays);
        }
        shell
    }
}

/// Start a generic application shell around the main workspace region.
pub fn workspace_shell<Message>(workspace: ViewNode<Message>) -> WorkspaceShellBuilder<Message> {
    WorkspaceShellBuilder {
        workspace,
        top_bar: None,
        leading_sidebar: None,
        trailing_sidebar: None,
        status_bar: None,
        outer_spacing: 4.0,
        workspace_spacing: 4.0,
        padding: 0.0,
        style: None,
        overlays: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, Layer, overlays, scene, text, workspace_shell},
        layout::{ContainerKind, LayoutNode, Vector2},
    };

    #[test]
    fn workspace_shell_builds_top_workspace_and_status_regions() {
        let layout = workspace_shell(text::<()>("Main"))
            .top_bar(text("Top"))
            .status_bar(text("Status"))
            .build()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(shell) = layout else {
            panic!("workspace shell should lower to an outer column");
        };
        assert_eq!(shell.policy.kind, ContainerKind::Column);
        assert_eq!(shell.children.len(), 3);

        let LayoutNode::Container(workspace_row) = &shell.children[1].child else {
            panic!("workspace region should lower to a row");
        };
        assert_eq!(workspace_row.policy.kind, ContainerKind::Row);
        assert_eq!(workspace_row.children.len(), 1);
    }

    #[test]
    fn workspace_shell_includes_optional_sidebars_around_workspace() {
        let layout = workspace_shell(text::<()>("Main"))
            .leading_sidebar(text("Leading"))
            .trailing_sidebar(text("Trailing"))
            .build()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(shell) = layout else {
            panic!("workspace shell should lower to an outer column");
        };
        assert_eq!(shell.children.len(), 1);

        let LayoutNode::Container(workspace_row) = &shell.children[0].child else {
            panic!("workspace region should lower to a row");
        };
        assert_eq!(workspace_row.policy.kind, ContainerKind::Row);
        assert_eq!(workspace_row.children.len(), 3);
    }

    #[test]
    fn workspace_shell_fills_available_space() {
        let surface = workspace_shell(text::<()>("Main")).build().into_surface();
        let root = surface.layout_node();
        let root_id = root.id();
        let layout = surface.layout_at_size(Vector2::new(640.0, 480.0));

        assert_eq!(
            layout
                .rects
                .get(&root_id)
                .map(|rect| Vector2::new(rect.width(), rect.height())),
            Some(Vector2::new(640.0, 480.0))
        );
    }

    #[test]
    fn workspace_shell_preserves_owner_overlays() {
        let labels = scene::<()>(
            workspace_shell(text("Main"))
                .top_bar(text("Top"))
                .overlays(overlays().layer(Layer::floating(text("Floating"))))
                .build(),
        )
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
        .paint_plan
        .text_label_strings();

        assert_eq!(labels, ["Top", "Main", "Floating"]);
    }
}
