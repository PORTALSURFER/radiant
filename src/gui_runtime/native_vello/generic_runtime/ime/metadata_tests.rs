use crate::widgets::interaction::CompositionStartContext;
use crate::{
    gui::{
        input::InputTimestamp,
        types::{Rect, Vector2},
    },
    layout::LayoutOutput,
    runtime::{
        Command, PaintPrimitive, RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        CompositionPhase, CompositionRange, CompositionSample, Widget, WidgetCommon, WidgetInput,
        WidgetOutput, WidgetSizing,
    },
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::event::Ime;

type Observations = Rc<RefCell<Vec<(CompositionPhase, Option<InputTimestamp>)>>>;

#[derive(Clone)]
struct Probe {
    common: WidgetCommon,
    active: bool,
    observations: Observations,
}

impl Widget for Probe {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn handle_input(&mut self, _: Rect, _: WidgetInput) -> Option<WidgetOutput> {
        None
    }
    fn accepts_composition_input(&self) -> bool {
        true
    }
    fn accepts_text_input(&self) -> bool {
        true
    }
    fn composition_start_context(&self) -> Option<CompositionStartContext> {
        let range = CompositionRange::new(0, 0, 0).ok()?;
        CompositionStartContext::new(range, range).ok()
    }
    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        self.active = matches!(
            sample.phase(),
            CompositionPhase::Start | CompositionPhase::Update
        );
        self.observations
            .borrow_mut()
            .push((sample.phase(), sample.timestamp()));
        None
    }
    fn handle_hidden_composition_update(
        &mut self,
        _: String,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetOutput> {
        self.observations
            .borrow_mut()
            .push((CompositionPhase::Update, timestamp));
        None
    }
    fn retains_managed_composition(&self) -> bool {
        self.active
    }
    fn append_paint(
        &self,
        _: &mut Vec<PaintPrimitive>,
        _: Rect,
        _: &LayoutOutput,
        _: &ThemeTokens,
    ) {
    }
}

struct Bridge(Observations);
impl RuntimeBridge<()> for Bridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            Probe {
                common: WidgetCommon::new(7, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                    .with_keyboard_focus(),
                active: false,
                observations: Rc::clone(&self.0),
            },
            WidgetMessageMapper::typed(|message: ()| message),
        )))
    }
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
}

#[test]
fn native_receipt_timestamp_reaches_visible_hidden_and_terminal_widget_hooks() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runner = super::GenericNativeVelloRunner::new(
        crate::gui_runtime::NativeRunOptions::default(),
        Bridge(Rc::clone(&observations)),
        Vector2::new(200.0, 40.0),
    );
    assert!(runner.core.runtime.focus_widget(7));
    let stamps = [
        InputTimestamp::capture(),
        InputTimestamp::capture(),
        InputTimestamp::capture(),
        InputTimestamp::capture(),
        InputTimestamp::capture(),
    ];
    for (event, stamp) in [
        (Ime::Preedit("あ".into(), Some((0, 3))), stamps[0]),
        (Ime::Preedit("界".into(), None), stamps[1]),
        (Ime::Commit("界".into()), stamps[2]),
        (Ime::Preedit("new".into(), None), stamps[3]),
        (Ime::Disabled, stamps[4]),
    ] {
        let _ = runner.route_native_ime_event_with_timestamp(event, Some(stamp));
    }
    use CompositionPhase::{Cancel, Commit, Start, Update};
    assert_eq!(
        *observations.borrow(),
        vec![
            (Start, Some(stamps[0])),
            (Update, Some(stamps[0])),
            (Update, Some(stamps[1])),
            (Commit, Some(stamps[2])),
            (Start, Some(stamps[3])),
            (Update, Some(stamps[3])),
            (Cancel, Some(stamps[4])),
        ]
    );
}
