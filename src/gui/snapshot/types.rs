use serde::Serialize;

/// Serializable color captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

/// Serializable point captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotPoint {
    /// X coordinate in logical window space.
    pub x: f32,
    /// Y coordinate in logical window space.
    pub y: f32,
}

/// Serializable rectangle captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRect {
    /// Minimum X coordinate in logical window space.
    pub x: f32,
    /// Minimum Y coordinate in logical window space.
    pub y: f32,
    /// Width in logical points.
    pub width: f32,
    /// Height in logical points.
    pub height: f32,
}

/// Serializable primitive captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SnapshotPrimitive {
    /// Filled rectangle primitive.
    Rect {
        /// Primitive bounds.
        rect: SnapshotRect,
        /// Fill color.
        color: SnapshotColor,
    },
    /// Filled circle primitive.
    Circle {
        /// Circle center.
        center: SnapshotPoint,
        /// Circle radius.
        radius: f32,
        /// Fill color.
        color: SnapshotColor,
    },
    /// Filled linear-gradient primitive.
    LinearGradient {
        /// Primitive bounds.
        rect: SnapshotRect,
        /// Gradient start point.
        start: SnapshotPoint,
        /// Gradient end point.
        end: SnapshotPoint,
        /// Gradient start color.
        start_color: SnapshotColor,
        /// Gradient end color.
        end_color: SnapshotColor,
    },
    /// RGBA image primitive.
    Image {
        /// Image placement bounds.
        rect: SnapshotRect,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Image RGBA pixels.
        pixels: Vec<u8>,
    },
}

/// Text alignment captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotTextAlign {
    /// Left-aligned text.
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
}

/// Serializable text run captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotTextRun {
    /// Text content.
    pub text: String,
    /// Text anchor position.
    pub position: SnapshotPoint,
    /// Font size in logical points.
    pub font_size: f32,
    /// Text color.
    pub color: SnapshotColor,
    /// Optional max width for text layout.
    pub max_width: Option<f32>,
    /// Text alignment.
    pub align: SnapshotTextAlign,
}

/// Deterministic snapshot of one rendered GUI frame.
#[derive(Debug, Clone, Serialize)]
pub struct VisualSnapshot {
    /// Fixture name.
    pub name: String,
    /// Viewport width in logical pixels.
    pub viewport_width: u32,
    /// Viewport height in logical pixels.
    pub viewport_height: u32,
    /// Frame clear color.
    pub clear_color: SnapshotColor,
    /// Number of captured primitives.
    pub primitive_count: usize,
    /// Number of captured text runs.
    pub text_run_count: usize,
    /// Captured paint primitives.
    pub primitives: Vec<SnapshotPrimitive>,
    /// Captured text runs.
    pub text_runs: Vec<SnapshotTextRun>,
}
