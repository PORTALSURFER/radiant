use super::super::SurfaceRuntime;
use super::super::interaction_state::RuntimeManagedCompositionState;
use crate::{
    gui::types::{Rect, Vector2},
    layout::LayoutOutput,
    runtime::{
        Command, PaintPrimitive, RepaintScope, RuntimeBridge, SurfaceNode, UiSurface,
        WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        CompositionPhase, CompositionRange, CompositionSample, Widget, WidgetCommon, WidgetId,
        WidgetInput, WidgetOutput, WidgetSizing,
    },
};

const OWNER: WidgetId = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositionHostMessage {
    Cancel { owner_id: WidgetId },
}

#[derive(Clone)]
struct CompositionProbeWidget {
    common: WidgetCommon,
    active: bool,
}

impl CompositionProbeWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(OWNER, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                .with_keyboard_focus(),
            active: false,
        }
    }
}

impl Widget for CompositionProbeWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn accepts_composition_input(&self) -> bool {
        true
    }

    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        self.active = matches!(
            sample.phase(),
            CompositionPhase::Start | CompositionPhase::Update
        );
        (sample.phase() == CompositionPhase::Cancel)
            .then(|| WidgetOutput::typed(CompositionHostMessage::Cancel { owner_id: OWNER }))
    }

    fn retains_managed_composition(&self) -> bool {
        self.active
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
struct CompositionProbeBridge {
    host_messages: Vec<CompositionHostMessage>,
    projections: usize,
}

impl RuntimeBridge<CompositionHostMessage> for CompositionProbeBridge {
    fn project_surface(&mut self) -> std::sync::Arc<UiSurface<CompositionHostMessage>> {
        self.projections += 1;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            CompositionProbeWidget::new(),
            WidgetMessageMapper::typed(|message: CompositionHostMessage| message),
        )))
    }

    fn update(&mut self, message: CompositionHostMessage) -> Command<CompositionHostMessage> {
        self.host_messages.push(message);
        Command::repaint(RepaintScope::Projection)
    }
}

fn valid_start() -> CompositionSample {
    let range = CompositionRange::new(0, 0, 0).expect("empty scalar range");
    CompositionSample::start(range, range).expect("valid composition start")
}

fn malformed_update() -> CompositionSample {
    CompositionSample::Update {
        preedit: String::from("あ"),
        selection: CompositionRange::new(0, 0, 2).expect("mismatched scalar evidence"),
        timestamp: None,
    }
}

#[test]
fn invalid_terminal_clears_runtime_owner_before_mapped_cancel_refresh() {
    let mut runtime =
        SurfaceRuntime::new(CompositionProbeBridge::default(), Vector2::new(160.0, 40.0));

    assert!(runtime.focus_widget(OWNER));
    assert_eq!(
        runtime.dispatch_composition_sample(valid_start()),
        Some(OWNER)
    );
    let projections_before_invalid = runtime.bridge().projections;

    assert_eq!(
        runtime.dispatch_composition_sample(malformed_update()),
        None
    );

    assert_eq!(
        runtime.interaction.composition_dispatch_observations,
        vec![
            (
                CompositionPhase::Start,
                RuntimeManagedCompositionState::Active { widget_id: OWNER },
            ),
            (
                CompositionPhase::Cancel,
                RuntimeManagedCompositionState::Idle
            ),
        ]
    );
    assert_eq!(
        runtime.bridge().host_messages,
        vec![CompositionHostMessage::Cancel { owner_id: OWNER }]
    );
    assert!(runtime.bridge().projections > projections_before_invalid);
    assert!(runtime.surface().find_widget(OWNER).is_some());
    assert_eq!(
        runtime.interaction.composition.managed_composition,
        RuntimeManagedCompositionState::Blocked
    );
}

#[test]
fn hidden_update_uses_fixed_owner_and_legacy_default_cancel_fallback() {
    let mut runtime =
        SurfaceRuntime::new(CompositionProbeBridge::default(), Vector2::new(160.0, 40.0));

    assert!(runtime.focus_widget(OWNER));
    assert_eq!(
        runtime.dispatch_composition_sample(valid_start()),
        Some(OWNER)
    );
    assert_eq!(
        runtime.dispatch_hidden_composition_update(String::from("hidden"), None),
        Some(OWNER)
    );
    assert_eq!(
        runtime.bridge().host_messages,
        vec![CompositionHostMessage::Cancel { owner_id: OWNER }]
    );
    assert_eq!(
        runtime.interaction.composition.managed_composition,
        RuntimeManagedCompositionState::Blocked
    );

    assert_eq!(
        runtime.dispatch_hidden_composition_update(String::from("stale"), None),
        None
    );
    assert_eq!(
        runtime.bridge().host_messages,
        vec![CompositionHostMessage::Cancel { owner_id: OWNER }]
    );
}

#[test]
fn composition_sequence_is_not_reused_after_terminal_and_never_wraps() {
    let mut runtime =
        SurfaceRuntime::new(CompositionProbeBridge::default(), Vector2::new(160.0, 40.0));
    assert!(runtime.focus_widget(OWNER));
    let first = runtime
        .dispatch_composition_start_with_sequence(valid_start())
        .unwrap();
    runtime.dispatch_composition_sample(CompositionSample::cancel());
    let second = runtime
        .dispatch_composition_start_with_sequence(valid_start())
        .unwrap();
    assert!(second > first);
    runtime.dispatch_composition_sample(CompositionSample::cancel());
    runtime.interaction.composition.sequence = u64::MAX;
    assert_eq!(
        runtime.dispatch_composition_start_with_sequence(valid_start()),
        None
    );
    assert_eq!(runtime.dispatch_composition_sample(valid_start()), None);
    assert_eq!(runtime.managed_composition_sequence(), None);
}
