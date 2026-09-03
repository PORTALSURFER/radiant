use super::LayoutContext;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutDiagnosticCode;
use crate::gui::layout_core::tree::NodeId;
use crate::gui::layout_core::validated_geometry::normalize_constraint_axis;

impl<'a> LayoutContext<'a> {
    pub(crate) fn normalize_constraints(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
    ) -> Constraints {
        let (min_w, max_w) = normalize_constraint_axis(constraints.min_w, constraints.max_w);
        let (min_h, max_h) = normalize_constraint_axis(constraints.min_h, constraints.max_h);

        if !constraints.min_w.is_finite() {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min width was non-finite and was clamped",
            );
        }
        if !constraints.max_w.is_finite() && constraints.max_w != f32::INFINITY {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max width was non-finite and was clamped",
            );
        }
        if !constraints.min_h.is_finite() {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min height was non-finite and was clamped",
            );
        }
        if !constraints.max_h.is_finite() && constraints.max_h != f32::INFINITY {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max height was non-finite and was clamped",
            );
        }

        if constraints.min_w.is_finite() && constraints.min_w < 0.0 {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum width was clamped",
            );
        }
        if constraints.min_h.is_finite() && constraints.min_h < 0.0 {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum height was clamped",
            );
        }
        if constraints.max_w.is_finite() && constraints.max_w < min_w {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::ConstraintContradiction,
                "width constraints were contradictory and were normalized",
            );
        }
        if constraints.max_h.is_finite() && constraints.max_h < min_h {
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
