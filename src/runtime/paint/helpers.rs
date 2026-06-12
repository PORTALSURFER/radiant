mod batches;
mod paths;
mod rects;
mod sink;
#[cfg(test)]
mod tests;
mod text;

pub use batches::{push_fill_rect_batch, push_stroke_rect_batch};
pub use paths::{push_fill_polygon, push_stroke_polyline};
pub use rects::{push_fill_rect, push_stroke_rect, push_visible_fill_rect};
pub use sink::WidgetPaint;
pub use text::{PaintTextMetrics, push_text, push_text_run_with_metrics};
