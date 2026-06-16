use crate::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::{LayoutDiagnosticCode, NodeId},
    runtime::{
        PaintPrimitive, PaintText, RuntimeDiagnostics, SurfacePaintStats, SurfaceRuntime,
        empty_paint_plan_for_layout, push_stroke_rect,
    },
    theme::ThemeTokens,
    widgets::{FocusBehavior, WidgetProminence, WidgetState, WidgetStyle, WidgetTone},
};

use super::RuntimeBridge;

const DEVTOOLS_OVERLAY_WIDGET_ID: NodeId = u64::MAX - 2048;
const DEVTOOLS_SELECTED_BOUNDS_WIDGET_ID: NodeId = u64::MAX - 2047;

/// Runtime-local devtools overlay policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DevtoolsOverlayOptions {
    /// Whether runtime devtools overlay paint is enabled.
    pub enabled: bool,
}

impl DevtoolsOverlayOptions {
    /// Return disabled devtools overlay options.
    pub const fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Return enabled devtools overlay options.
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Return whether the devtools overlay is enabled.
    pub const fn is_enabled(self) -> bool {
        self.enabled
    }
}

/// Backend-neutral snapshot for Radiant devtools and debug inspector UIs.
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsSnapshot {
    /// Current logical runtime viewport.
    pub viewport: Rect,
    /// Full projected surface tree with runtime interaction and layout metadata.
    pub root: DevtoolsNodeSnapshot,
    /// Best current inspected node candidate.
    pub selected_node_id: Option<NodeId>,
    /// Aggregate paint primitive counts for the current runtime frame.
    pub paint: SurfacePaintStats,
    /// Generic runtime diagnostics available to debug panels.
    pub diagnostics: RuntimeDiagnostics,
}

/// One projected surface node in a [`DevtoolsSnapshot`].
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsNodeSnapshot {
    /// Stable layout/widget node id.
    pub node_id: NodeId,
    /// Generic projected node kind.
    pub kind: DevtoolsNodeKind,
    /// Current resolved layout bounds, when the node participates in layout.
    pub bounds: Option<Rect>,
    /// Widget-specific runtime state for widget leaves.
    pub widget: Option<DevtoolsWidgetSnapshot>,
    /// Layout diagnostics emitted for this node.
    pub layout_diagnostics: Vec<DevtoolsLayoutDiagnostic>,
    /// Child nodes in surface tree order.
    pub children: Vec<DevtoolsNodeSnapshot>,
}

/// Generic surface node kind shown by devtools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevtoolsNodeKind {
    /// Root scene node.
    Scene,
    /// Layout container node.
    Container,
    /// Widget leaf node.
    Widget,
    /// Non-interactive overlay descriptor.
    Overlay,
    /// Floating child tree.
    FloatingLayer,
}

/// Widget leaf state shown by devtools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DevtoolsWidgetSnapshot {
    /// Focus participation contract.
    pub focus: FocusBehavior,
    /// Whether the widget can receive runtime focus.
    pub focusable: bool,
    /// Whether the widget participates in keyboard focus traversal.
    pub keyboard_focusable: bool,
    /// Whether the widget can receive pointer hit testing.
    pub receives_pointer_hit_testing: bool,
    /// Whether the widget accepts wheel input before scroll fallback.
    pub accepts_wheel_input: bool,
    /// Whether the widget accepts stable pointer move routing.
    pub accepts_pointer_move: bool,
    /// Whether this widget is the current pointer-capture target.
    pub captured: bool,
    /// Shared widget interaction and visual state.
    pub state: WidgetState,
}

/// One layout diagnostic attached to a devtools node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevtoolsLayoutDiagnostic {
    /// Stable diagnostic category.
    pub code: LayoutDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return a backend-neutral devtools snapshot for the current runtime frame.
    pub fn devtools_snapshot(&self) -> DevtoolsSnapshot {
        self.devtools_snapshot_with_theme(&ThemeTokens::default())
    }

    /// Return a backend-neutral devtools snapshot using a caller-provided theme.
    pub fn devtools_snapshot_with_theme(&self, theme: &ThemeTokens) -> DevtoolsSnapshot {
        let mut plan = empty_paint_plan_for_layout(self.context().layout, theme);
        self.base_paint_plan_into(theme, &mut plan);
        let paint = plan.stats();
        DevtoolsSnapshot {
            viewport: self.context().viewport,
            root: self
                .surface()
                .root()
                .devtools_snapshot_node(self.pointer_capture(), self.context().layout),
            selected_node_id: self
                .pointer_capture()
                .or_else(|| self.focused_widget())
                .or_else(|| self.hovered_widget())
                .or_else(|| self.hovered_container())
                .or_else(|| self.hovered_scroll_affordance()),
            paint,
            diagnostics: self.runtime_diagnostics(),
        }
    }

    /// Configure runtime devtools overlay paint.
    pub fn set_devtools_overlay_options(&mut self, options: DevtoolsOverlayOptions) {
        self.devtools_overlay = options;
        self.repaint_requested |= options.enabled;
    }

    /// Return current runtime devtools overlay paint options.
    pub const fn devtools_overlay_options(&self) -> DevtoolsOverlayOptions {
        self.devtools_overlay
    }

    pub(in crate::runtime) fn append_devtools_overlay_paint(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if !self.devtools_overlay.enabled {
            return;
        }

        let snapshot = self.devtools_snapshot_with_theme(theme);
        if let Some(bounds) = snapshot.selected_node_bounds() {
            push_stroke_rect(
                primitives,
                DEVTOOLS_SELECTED_BOUNDS_WIDGET_ID,
                bounds,
                Rgba8::new(255, 190, 90, 235),
                2.0,
            );
        }

        crate::runtime::paint::push_overlay_panel(
            primitives,
            DEVTOOLS_OVERLAY_WIDGET_ID,
            devtools_overlay_panel_rect(snapshot.viewport),
            Some(PaintText::from(devtools_overlay_label(&snapshot))),
            theme,
            WidgetStyle {
                tone: WidgetTone::Accent,
                prominence: WidgetProminence::Strong,
            },
        );
    }
}

impl DevtoolsSnapshot {
    /// Return the currently selected devtools node, when present.
    pub fn selected_node(&self) -> Option<&DevtoolsNodeSnapshot> {
        self.selected_node_id
            .and_then(|node_id| self.root.find_node(node_id))
    }

    fn selected_node_bounds(&self) -> Option<Rect> {
        self.selected_node().and_then(|node| node.bounds)
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
}

fn devtools_overlay_panel_rect(viewport: Rect) -> Rect {
    let width = viewport.width().min(360.0).max(180.0);
    let height = 64.0;
    Rect::from_min_size(
        Point::new(
            (viewport.max.x - width - 12.0).max(12.0),
            viewport.min.y + 12.0,
        ),
        Vector2::new(width, height),
    )
}

fn devtools_overlay_label(snapshot: &DevtoolsSnapshot) -> String {
    let selected = snapshot
        .selected_node_id
        .map(|node_id| format!("selected=#{node_id}"))
        .unwrap_or_else(|| String::from("selected=none"));
    format!(
        "Radiant devtools | {selected} | paint={} | handlers={} | workers={}",
        snapshot.paint.total,
        snapshot.diagnostics.ui.update_handlers,
        snapshot.diagnostics.business.running
    )
}
