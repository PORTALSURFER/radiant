use super::LayoutContext;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutDiagnosticCode;
use crate::gui::layout_core::tree::NodeId;

impl<'a> LayoutContext<'a> {
    pub(crate) fn normalize_constraints(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
    ) -> Constraints {
        let mut min_w = constraints.min_w;
        let mut max_w = constraints.max_w;
        let mut min_h = constraints.min_h;
        let mut max_h = constraints.max_h;

        if !min_w.is_finite() {
            min_w = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min width was non-finite and was clamped",
            );
        }
        if !max_w.is_finite() {
            max_w = f32::INFINITY;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max width was non-finite and was clamped",
            );
        }
        if !min_h.is_finite() {
            min_h = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min height was non-finite and was clamped",
            );
        }
        if !max_h.is_finite() {
            max_h = f32::INFINITY;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max height was non-finite and was clamped",
            );
        }

        if min_w < 0.0 {
            min_w = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum width was clamped",
            );
        }
        if min_h < 0.0 {
            min_h = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum height was clamped",
            );
        }
        if max_w < min_w {
            max_w = min_w;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::ConstraintContradiction,
                "width constraints were contradictory and were normalized",
            );
        }
        if max_h < min_h {
            max_h = min_h;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::ConstraintContradiction,
                "height constraints were contradictory and were normalized",
            );
        }

        Constraints {
            min_w,
            max_w,
            min_h,
            max_h,
        }
    }

    pub(crate) fn clamp_width(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        self.clamp_axis(node_id, constraints, value, true)
    }

    pub(crate) fn clamp_height(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        self.clamp_axis(node_id, constraints, value, false)
    }

    pub(crate) fn clamp_main(
        &mut self,
        node_id: NodeId,
        horizontal: bool,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        if horizontal {
            self.clamp_width(node_id, constraints, value)
        } else {
            self.clamp_height(node_id, constraints, value)
        }
    }

    pub(crate) fn clamp_cross(
        &mut self,
        node_id: NodeId,
        horizontal: bool,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        if horizontal {
            self.clamp_height(node_id, constraints, value)
        } else {
            self.clamp_width(node_id, constraints, value)
        }
    }

    fn clamp_axis(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
        is_width: bool,
    ) -> f32 {
        let normalized = self.normalize_constraints(node_id, constraints);
        let sanitized = if !value.is_finite() {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "non-finite size was clamped",
            );
            0.0
        } else if value < 0.0 {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative size was clamped",
            );
            0.0
        } else {
            value
        };

        if is_width {
            normalized.clamp_w(sanitized)
        } else {
            normalized.clamp_h(sanitized)
        }
    }
}
