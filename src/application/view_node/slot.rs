use crate::layout::{SizeModeCross, SizeModeMain, SlotParams};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct SlotBehavior {
    pub(super) width: AxisSlotBehavior,
    pub(super) height: AxisSlotBehavior,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum AxisSlotBehavior {
    #[default]
    Default,
    Intrinsic,
    Fill(f32),
    Percent(f32),
    Fixed(f32),
}

impl SlotBehavior {
    pub(super) fn to_slot_params(self, horizontal: bool) -> SlotParams {
        let main_axis = if horizontal { self.width } else { self.height };
        let cross_axis = if horizontal { self.height } else { self.width };
        SlotParams {
            size_main: main_axis.to_main(),
            size_cross: cross_axis.to_cross(),
            ..SlotParams::fill()
        }
    }
}

impl AxisSlotBehavior {
    fn to_main(self) -> SizeModeMain {
        match self {
            Self::Default | Self::Intrinsic => SizeModeMain::Intrinsic,
            Self::Fill(weight) => SizeModeMain::Fill(weight.max(0.0)),
            Self::Percent(ratio) => SizeModeMain::Percent(ratio.max(0.0)),
            Self::Fixed(value) => SizeModeMain::Fixed(value.max(0.0)),
        }
    }

    fn to_cross(self) -> SizeModeCross {
        match self {
            Self::Default | Self::Intrinsic => SizeModeCross::Intrinsic,
            Self::Fill(_) | Self::Percent(_) => SizeModeCross::Fill,
            Self::Fixed(value) => SizeModeCross::Fixed(value.max(0.0)),
        }
    }
}
