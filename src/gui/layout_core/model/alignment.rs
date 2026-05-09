//! Alignment, sizing, and overflow policy enums.

/// Main-axis sizing mode for a slot.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeModeMain {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill remaining space by weight.
    Fill(f32),
    /// Percentage of parent content space.
    Percent(f32),
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Cross-axis sizing mode for a slot.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeModeCross {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill available cross-axis space.
    Fill,
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Main-axis alignment within a container.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAlign {
    /// Pack children toward the start edge.
    Start,
    /// Center the packed child run.
    Center,
    /// Pack children toward the end edge.
    End,
    /// Distribute free space only between children.
    SpaceBetween,
    /// Distribute free space before, between, and after children with half-sized edges.
    SpaceAround,
    /// Distribute free space evenly before, between, and after children.
    SpaceEvenly,
}

/// Cross-axis alignment for children within a container.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossAlign {
    /// Align children to the start edge.
    Start,
    /// Center children in the cross axis.
    Center,
    /// Align children to the end edge.
    End,
    /// Stretch children to fill the available cross-axis span.
    Stretch,
}

/// Explicit overflow policy for containers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Clip child content to the container bounds.
    Clip,
    /// Keep a viewport and expose overflow through scroll offsets.
    Scroll,
    /// Wrap items onto additional lines or tracks.
    Wrap,
    /// Compress children before overflow escapes the container.
    Shrink,
}
