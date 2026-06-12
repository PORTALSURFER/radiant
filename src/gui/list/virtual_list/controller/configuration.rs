use super::VirtualListController;
use crate::gui::list::VirtualListProjection;

impl VirtualListController {
    /// Set the total logical item count and clamp dependent state.
    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        if self
            .focused_index
            .is_some_and(|index| index >= self.total_items)
        {
            self.focused_index = None;
        }
        self.clamp_viewport_start();
    }

    /// Set the visible logical item count and clamp dependent state.
    pub fn set_viewport_len(&mut self, viewport_len: usize) {
        self.viewport_len = viewport_len;
        self.clamp_viewport_start();
    }

    /// Set the materialization overscan.
    pub fn set_overscan(&mut self, overscan: usize) {
        self.overscan = overscan;
    }

    /// Set the focus-follow guard band.
    pub fn set_guard_band(&mut self, guard_band: usize) {
        self.guard_band = guard_band;
    }

    /// Configure the stable geometry inputs for a projection pass.
    ///
    /// The current viewport start is clamped after the item and viewport counts
    /// are updated, so callers can safely reuse one controller while filters,
    /// sorts, window sizes, or overscan policy change.
    pub fn configure(
        &mut self,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
    ) {
        self.total_items = total_items;
        self.viewport_len = viewport_len;
        self.overscan = overscan;
        self.guard_band = guard_band;
        if self
            .focused_index
            .is_some_and(|index| index >= self.total_items)
        {
            self.focused_index = None;
        }
        self.clamp_viewport_start();
    }

    /// Configure the stable geometry inputs from a named projection value.
    pub fn configure_projection(&mut self, projection: VirtualListProjection) {
        self.configure(
            projection.total_items(),
            projection.viewport_len(),
            projection.overscan(),
            projection.guard_band(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualListController;

    #[test]
    fn configure_clamps_viewport_and_invalid_focus() {
        let mut controller = VirtualListController::with_items(20, 6);
        controller.set_viewport_start(14);
        controller.focus(18);

        controller.configure(5, 3, 1, 1);

        assert_eq!(controller.total_items(), 5);
        assert_eq!(controller.viewport_len(), 3);
        assert_eq!(controller.overscan(), 1);
        assert_eq!(controller.guard_band(), 1);
        assert_eq!(controller.viewport_start(), 2);
        assert_eq!(controller.focused_index(), None);
    }
}
