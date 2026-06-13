use crate::widgets::WidgetSizing;

pub(in crate::application) trait OptionalBaseline {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self;
}

impl OptionalBaseline for WidgetSizing {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self {
        if let Some(baseline) = baseline {
            self.with_baseline(baseline)
        } else {
            self
        }
    }
}
