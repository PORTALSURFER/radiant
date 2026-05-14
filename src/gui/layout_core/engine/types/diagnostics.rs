use super::super::super::tree::NodeId;
use std::borrow::Cow;

/// Stable diagnostic category emitted during measure/layout normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDiagnosticCode {
    /// A negative size or coordinate was clamped to a non-negative value.
    NegativeSizeClamped,
    /// A constraint range (`min > max`) was normalized to a valid range.
    ConstraintContradiction,
    /// A requested overflow policy was unsupported and defaulted to a fallback.
    OverflowPolicyDefaulted,
    /// Content overflow was detected for the node.
    OverflowOccurred,
    /// A provided scroll offset was outside legal bounds and clamped.
    InvalidScrollOffsetClamped,
    /// A virtualization policy was ignored because it could not be applied.
    VirtualizationPolicyIgnored,
    /// A computed virtualization window was clamped to legal bounds.
    VirtualizationWindowClamped,
    /// Virtualization fell back because alignment-resolved windows were invalid.
    VirtualizationAlignmentFallback,
    /// Virtualization fell back because span resolution could not be trusted.
    VirtualizationSpanResolutionFallback,
}

/// Layout diagnostic emitted when invalid states are normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDiagnostic {
    /// Node that triggered the diagnostic.
    pub node_id: NodeId,
    /// Stable diagnostic category.
    pub code: LayoutDiagnosticCode,
    /// Human-readable diagnostic message. Static engine diagnostics are
    /// borrowed to keep normal layout passes allocation-lean.
    pub message: Cow<'static, str>,
}
