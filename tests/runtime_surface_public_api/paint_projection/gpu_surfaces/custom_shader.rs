use super::*;

#[test]
fn custom_shader_gpu_surface_uses_normal_paint_plan_path() {
    let descriptor = GpuShaderSurfaceDescriptor::new("spectral-meter")
        .wgsl_source(
            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
        )
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .uniform_bytes([1, 2, 3, 4])
        .storage_bytes([5, 6])
        .vertex_count(6);
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 41,
            sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            key: 9001,
            revision: 7,
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(descriptor),
            },
        }),
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );

    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    let Some(PaintPrimitive::GpuSurface(gpu)) = plan.primitives.first() else {
        panic!("expected gpu surface primitive");
    };
    assert_eq!(gpu.widget_id, 41);
    assert_eq!(gpu.key, 9001);
    assert_eq!(gpu.revision, 7);
    let GpuSurfaceContent::CustomShader { descriptor } = &gpu.content else {
        panic!("expected custom shader gpu content");
    };
    assert_eq!(descriptor.shader_key, "spectral-meter");
    assert!(descriptor.wgsl_source.as_deref().is_some_and(|source| {
        source.contains("@vertex")
            && source.contains("vertex_main")
            && source.contains("@fragment")
            && source.contains("fragment_main")
    }));
    assert_eq!(descriptor.entry_point, "vertex_main");
    assert_eq!(
        descriptor.fragment_entry_point.as_deref(),
        Some("fragment_main")
    );
    assert_eq!(descriptor.uniform_bytes.as_ref(), &[1, 2, 3, 4]);
    assert_eq!(descriptor.storage_bytes.as_ref(), &[5, 6]);
    assert_eq!(descriptor.vertex_count, 6);
}
