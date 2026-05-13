use super::super::{ViewNode, slot::AxisSlotBehavior};

impl<Message> ViewNode<Message> {
    /// Fill remaining space on the parent main axis and stretch on the cross axis.
    pub fn fill(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining horizontal space in the parent layout.
    pub fn fill_width(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining vertical space in the parent layout.
    pub fn fill_height(mut self) -> Self {
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining main-axis space with the provided weight.
    pub fn grow(mut self, weight: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(weight);
        self.slot.height = AxisSlotBehavior::Fill(weight);
        self
    }

    /// Use intrinsic parent slot sizing on both axes.
    pub fn intrinsic(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Intrinsic;
        self.slot.height = AxisSlotBehavior::Intrinsic;
        self
    }

    /// Use a fixed parent slot width.
    pub fn width(mut self, width: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fixed(width);
        self
    }

    /// Use a percentage of the parent width when this node is in a row.
    pub fn width_percent(mut self, ratio: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Percent(ratio);
        self
    }

    /// Use a fixed parent slot height.
    pub fn height(mut self, height: f32) -> Self {
        self.slot.height = AxisSlotBehavior::Fixed(height);
        self
    }

    /// Use a percentage of the parent height when this node is in a column.
    pub fn height_percent(mut self, ratio: f32) -> Self {
        self.slot.height = AxisSlotBehavior::Percent(ratio);
        self
    }
}
