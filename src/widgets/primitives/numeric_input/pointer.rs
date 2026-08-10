use std::rc::Rc;

use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Rect},
    },
    widgets::{
        EditEvent, InteractionProvenance, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputEditBatch, NumericInputInteraction, NumericInputInteractionBatch,
        NumericScrubAttempt, NumericStep, PointerModifiers, WidgetOutput,
    },
};

pub(super) trait PointerScrubInteractionPolicy<T> {
    fn scrub(
        &self,
        value: &T,
        normalized_delta: f32,
        step: NumericStep,
        attempt: NumericScrubAttempt,
        provenance: InteractionProvenance,
        rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<T, Option<WidgetOutput>>;

    fn format(
        &self,
        value: &T,
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<String, Option<WidgetOutput>>;

    fn encode_edit(&self, edit: NumericInputEditBatch<T>) -> Option<WidgetOutput>;
}

struct NoPointerScrubInteractionPolicy;

impl<T: Clone + 'static> PointerScrubInteractionPolicy<T> for NoPointerScrubInteractionPolicy {
    fn scrub(
        &self,
        _value: &T,
        _normalized_delta: f32,
        _step: NumericStep,
        _attempt: NumericScrubAttempt,
        _provenance: InteractionProvenance,
        _rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<T, Option<WidgetOutput>> {
        Err(None)
    }

    fn format(
        &self,
        _value: &T,
        _attempt: NumericScrubAttempt,
        _normalized_delta: f32,
        _step: NumericStep,
        _provenance: InteractionProvenance,
        _rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<String, Option<WidgetOutput>> {
        Err(None)
    }

    fn encode_edit(&self, edit: NumericInputEditBatch<T>) -> Option<WidgetOutput> {
        Some(WidgetOutput::typed(edit))
    }
}

pub(super) fn no_pointer_scrub_interaction_policy<T: Clone + 'static>()
-> Rc<dyn PointerScrubInteractionPolicy<T>> {
    Rc::new(NoPointerScrubInteractionPolicy)
}

struct CompletePointerScrubInteractionPolicy<T, C, A> {
    codec: Rc<C>,
    adjustment: Rc<A>,
    marker: std::marker::PhantomData<fn() -> T>,
}

impl<T, C, A> CompletePointerScrubInteractionPolicy<T, C, A> {
    fn failure(
        rollback: Option<NumericInputEditBatch<T>>,
        failure: NumericInputInteraction<T, A::Error, C::Error>,
    ) -> Option<WidgetOutput>
    where
        T: Clone + 'static,
        C: NumericCodec<T> + 'static,
        A: NumericAdjustment<T> + 'static,
        A::Error: 'static,
        C::Error: 'static,
    {
        let interaction = match rollback {
            Some(cancel) => NumericInputInteractionBatch::from_interactions(&[
                NumericInputInteraction::edit(cancel),
                failure,
            ])?,
            None => NumericInputInteractionBatch::from_interactions(&[failure])?,
        };
        Some(WidgetOutput::typed(interaction))
    }
}

impl<T, C, A> PointerScrubInteractionPolicy<T> for CompletePointerScrubInteractionPolicy<T, C, A>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    fn scrub(
        &self,
        value: &T,
        normalized_delta: f32,
        step: NumericStep,
        attempt: NumericScrubAttempt,
        provenance: InteractionProvenance,
        rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<T, Option<WidgetOutput>> {
        self.adjustment
            .scrub(value, normalized_delta, step)
            .map_err(|error| {
                let cancelled = rollback.is_some();
                Self::failure(
                    rollback,
                    NumericInputInteraction::pointer_scrub_failed(
                        attempt,
                        normalized_delta,
                        step,
                        provenance,
                        error,
                        cancelled,
                    ),
                )
            })
    }

    fn format(
        &self,
        value: &T,
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        rollback: Option<NumericInputEditBatch<T>>,
    ) -> Result<String, Option<WidgetOutput>> {
        let mut draft = String::new();
        self.codec
            .format_editable(value, &mut draft)
            .map_err(|error| {
                let cancelled = rollback.is_some();
                Self::failure(
                    rollback,
                    NumericInputInteraction::pointer_format_failed(
                        attempt,
                        normalized_delta,
                        step,
                        provenance,
                        error,
                        cancelled,
                    ),
                )
            })?;
        Ok(draft)
    }

    fn encode_edit(&self, edit: NumericInputEditBatch<T>) -> Option<WidgetOutput> {
        Some(WidgetOutput::typed(NumericInputInteractionBatch::<
            T,
            A::Error,
            C::Error,
        >::from_edit(edit)))
    }
}

pub(super) fn complete_pointer_scrub_interaction_policy<T, C, A>(
    codec: Rc<C>,
    adjustment: Rc<A>,
) -> Rc<dyn PointerScrubInteractionPolicy<T>>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    Rc::new(CompletePointerScrubInteractionPolicy {
        codec,
        adjustment,
        marker: std::marker::PhantomData,
    })
}

/// Retained state for one admitted semantic numeric pointer scrub.
#[derive(Clone)]
pub(super) struct PointerScrubState<T> {
    pub(super) session: NumericEditSession<T>,
    pub(super) anchor: Point,
    pub(super) anchor_value: T,
    pub(super) step: NumericStep,
    pub(super) published_update: bool,
    pub(super) start_text: String,
    pub(super) start_caret: usize,
    pub(super) start_selection_anchor: usize,
}

impl<T: Clone> PointerScrubState<T> {
    pub(super) fn new(
        value: T,
        draft: String,
        position: Point,
        step: NumericStep,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        start_caret: usize,
        start_selection_anchor: usize,
    ) -> Self {
        Self {
            session: NumericEditSession::begin(
                value.clone(),
                draft.clone(),
                pointer_provenance(modifiers, timestamp),
            ),
            anchor: position,
            anchor_value: value,
            step,
            published_update: false,
            start_text: draft,
            start_caret,
            start_selection_anchor,
        }
    }

    /// Return a finite in-bounds horizontal displacement, including zero.
    ///
    /// Invalid geometry or a sample outside the widget is rejected without
    /// clamping and without moving the current anchor.
    pub(super) fn normalized_delta(&self, bounds: Rect, position: Point) -> Option<f32> {
        if !bounds.is_finite()
            || bounds.width() <= 0.0
            || !position.is_finite()
            || !self.anchor.is_finite()
            || !bounds.contains(self.anchor)
            || !bounds.contains(position)
        {
            return None;
        }
        let delta = (position.x - self.anchor.x) / bounds.width();
        delta.is_finite().then_some(delta)
    }

    pub(super) fn reanchor(&mut self, position: Point, value: T, step: NumericStep) {
        self.anchor = position;
        self.anchor_value = value;
        self.step = step;
    }

    pub(super) fn accept_update(
        &mut self,
        value: T,
        draft: String,
        position: Point,
        provenance: InteractionProvenance,
    ) -> Option<NumericInputEditBatch<T>> {
        let begin = self.session.begin_event().clone();
        let edit = if self.published_update {
            let update = begin.update(value.clone(), provenance)?;
            NumericInputEditBatch::from_events(&[update])?
        } else {
            let update = begin.clone().update(value.clone(), provenance)?;
            NumericInputEditBatch::from_events(&[begin, update])?
        };
        self.session.replace_draft(draft);
        self.anchor = position;
        self.anchor_value = value;
        self.published_update = true;
        Some(edit)
    }

    pub(super) fn rollback_batch(&self) -> Option<NumericInputEditBatch<T>> {
        let cancel = self
            .session
            .clone()
            .cancel(pointer_provenance(PointerModifiers::default(), None))
            .ok()?;
        NumericInputEditBatch::from_events(&[cancel])
    }

    pub(super) fn commit(
        self,
        value: T,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Result<EditEvent<T>, Self> {
        let Self {
            session,
            anchor,
            anchor_value,
            step,
            published_update,
            start_text,
            start_caret,
            start_selection_anchor,
        } = self;
        let provenance = pointer_provenance(modifiers, timestamp);
        match session.commit(value, provenance) {
            Ok(event) => Ok(event),
            Err(session) => Err(Self {
                session,
                anchor,
                anchor_value,
                step,
                published_update,
                start_text,
                start_caret,
                start_selection_anchor,
            }),
        }
    }

    pub(super) fn cancel(self) -> Result<EditEvent<T>, Self> {
        let Self {
            session,
            anchor,
            anchor_value,
            step,
            published_update,
            start_text,
            start_caret,
            start_selection_anchor,
        } = self;
        let provenance = pointer_provenance(PointerModifiers::default(), None);
        match session.cancel(provenance) {
            Ok(event) => Ok(event),
            Err(session) => Err(Self {
                session,
                anchor,
                anchor_value,
                step,
                published_update,
                start_text,
                start_caret,
                start_selection_anchor,
            }),
        }
    }
}

fn pointer_provenance(
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers,
        timestamp,
        sequence_range: None,
    }
}
