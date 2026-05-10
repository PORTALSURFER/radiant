//! Multi-window manifest built without opening platform windows.

use radiant::prelude::*;

fn main() {
    let manifest = build_window_manifest();
    let view_count = build_window_views().len();
    for spec in manifest.specs() {
        println!(
            "radiant_window_spec key={} title={} size={:?} min_size={:?} views={}",
            spec.key,
            spec.title(),
            spec.inner_size(),
            spec.min_inner_size(),
            view_count
        );
    }
}

fn build_window_manifest() -> WindowManifest {
    WindowManifest::from_specs([
        radiant::window("Radiant Main Workspace")
            .size(900, 620)
            .min_size(640, 420)
            .spec("main"),
        WindowSpec::new("inspector", "Radiant Inspector")
            .size(360, 520)
            .min_size(300, 360)
            .target_fps(60),
        WindowSpec::new("preview", "Radiant Preview")
            .size(480, 320)
            .drag_and_drop(false),
    ])
    .expect("example manifest has unique stable window keys")
}

fn build_window_views() -> Vec<View> {
    vec![
        column([
            text("Main workspace").height(28.0).fill_width(),
            button("Open inspector").message(()).size(140.0, 32.0),
        ])
        .padding(16.0)
        .spacing(10.0),
        column([
            text("Inspector").height(28.0).fill_width(),
            text("Each window can own a separate bridge or static view.").wrap(),
        ])
        .padding(16.0)
        .spacing(10.0),
        column([
            text("Preview").height(28.0).fill_width(),
            badge("Passive").message(()).size(92.0, 26.0),
        ])
        .padding(16.0)
        .spacing(10.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn multi_window_manifest_preserves_stable_window_specs() {
        let manifest = build_window_manifest();
        let views = build_window_views();

        assert_eq!(manifest.len(), 3);
        assert_eq!(views.len(), manifest.len());
        assert_eq!(
            manifest.keys().collect::<Vec<_>>(),
            ["main", "inspector", "preview"]
        );
        assert_eq!(
            manifest.get("main").unwrap().inner_size(),
            Some([900.0, 620.0])
        );
        assert!(!manifest.get("preview").unwrap().drag_and_drop_enabled());
        let first_view = views.into_iter().next().expect("main view exists");
        assert!(first_view.into_surface().find_widget(3).is_some());
    }
}
