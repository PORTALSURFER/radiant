use super::InteractiveRowUnderlayBuilder;
use crate::{
    gui::list::{DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette},
    widgets::{
        InteractiveRowVisualStateParts, WidgetId, WidgetStyle, stable_widget_id,
        stable_widget_id_u64,
    },
};

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Paint Radiant's standard dense-row chrome behind the visible content.
    ///
    /// Use this for list, tree, sidebar, picker, and inspector rows whose
    /// content is app-owned but whose hover, pressed, selected, and drop-target
    /// feedback should follow Radiant's generic dense-row policy.
    pub fn dense_chrome(mut self) -> Self {
        self.dense_chrome = true;
        self
    }

    /// Mark the row as selected by host-owned state and paint dense-row chrome.
    pub fn selected(mut self, selected: bool) -> Self {
        self.visual_state.selected = selected;
        self.dense_chrome = true;
        self
    }

    /// Mark the row as a committed operation target and paint dense-row chrome.
    pub fn active_target(mut self, active_target: bool) -> Self {
        self.visual_state.active_target = active_target;
        self.dense_chrome = true;
        self
    }

    /// Mark the row as a valid operation candidate and paint dense-row chrome.
    pub fn candidate(mut self, candidate: bool) -> Self {
        self.visual_state.candidate = candidate;
        self.dense_chrome = true;
        self
    }

    /// Apply host-owned visual state and paint dense-row chrome.
    pub fn visual_state(mut self, parts: InteractiveRowVisualStateParts) -> Self {
        self.visual_state = parts;
        self.dense_chrome = true;
        self
    }

    /// Assign a stable widget id to the backing interactive row.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self.input_key = None;
        self
    }

    /// Assign a stable key to the backing interactive row.
    ///
    /// Use this when a dynamic row should refresh retained widget state based
    /// on caller-owned projection state but does not need a public numeric id.
    pub fn input_key(mut self, key: impl Into<String>) -> Self {
        self.input_key = Some(key.into());
        self.input_id = None;
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned text key.
    ///
    /// Use this for dynamic rows whose focus, hover, drag, or drop identity
    /// should survive projection changes without app-local input-id helpers.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self.input_key = None;
        self
    }

    /// Assign one stable row key to the composed row and its backing input widget.
    ///
    /// Use this for dynamic rows whose outer view subtree and retained input
    /// widget should follow the same caller-owned row identity. Prefer
    /// [`Self::input_id`] or [`Self::input_key`] plus a view-node key only when
    /// those identities deliberately differ.
    pub fn stable_row_identity(mut self, input_scope: u64, row_key: impl Into<String>) -> Self {
        let row_key = row_key.into();
        self.input_id = Some(stable_widget_id(input_scope, row_key.as_str()));
        self.input_key = None;
        self.row_key = Some(row_key);
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned numeric key.
    ///
    /// Use this for dynamic rows keyed by durable numeric IDs or enum indexes
    /// without allocating temporary strings.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self.input_key = None;
        self
    }

    /// Apply an explicit style to the backing interactive row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Override dense-row fill colors for this underlay.
    pub fn dense_chrome_palette(mut self, palette: DenseRowPalette) -> Self {
        self.chrome.palette = Some(palette);
        self.dense_chrome = true;
        self
    }

    /// Add a leading-edge marker to the dense-row underlay chrome.
    pub fn leading_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.leading_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a leading-edge marker when `condition` is true.
    pub fn leading_marker_if(mut self, condition: bool, marker: DenseRowMarkerStyle) -> Self {
        if condition {
            self.chrome.leading_marker = Some(marker);
            self.dense_chrome = true;
        }
        self
    }

    /// Add a second leading-edge marker painted after the primary marker.
    pub fn leading_overlay_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.leading_overlay_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a second leading-edge marker when `condition` is true.
    pub fn leading_overlay_marker_if(
        mut self,
        condition: bool,
        marker: DenseRowMarkerStyle,
    ) -> Self {
        if condition {
            self.chrome.leading_overlay_marker = Some(marker);
            self.dense_chrome = true;
        }
        self
    }

    /// Add a second leading-edge marker while the primary pointer is held down.
    pub fn pressed_leading_overlay_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.pressed_leading_overlay_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a trailing-edge marker to the dense-row underlay chrome.
    pub fn trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.trailing_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a trailing-edge marker when `condition` is true.
    pub fn trailing_marker_if(mut self, condition: bool, marker: DenseRowMarkerStyle) -> Self {
        if condition {
            self.chrome.trailing_marker = Some(marker);
            self.dense_chrome = true;
        }
        self
    }

    /// Add a trailing-edge marker while the row is hovered, unless a stronger
    /// host-owned trailing marker is already present.
    pub fn hover_trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.hover_trailing_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add an inset outline to the dense-row underlay chrome.
    pub fn outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.chrome.outline = Some(outline);
        self.dense_chrome = true;
        self
    }

    /// Add an inset outline when `condition` is true.
    pub fn outline_if(mut self, condition: bool, outline: DenseRowOutlineStyle) -> Self {
        if condition {
            self.chrome.outline = Some(outline);
            self.dense_chrome = true;
        }
        self
    }

    /// Add an inset outline while the primary pointer is held down.
    ///
    /// Release clears the retained pressed state, so pointer focus remains a
    /// transient visual independent from host-owned keyboard focus.
    pub fn pressed_outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.chrome.pressed_outline = Some(outline);
        self.dense_chrome = true;
        self
    }
}
