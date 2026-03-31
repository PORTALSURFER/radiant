use super::super::*;

#[test]
fn high_refresh_present_mode_candidates_prefer_non_vsync_fallback_before_vsync() {
    assert_eq!(
        present_mode_candidates(120),
        &[
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::AutoVsync,
        ]
    );
    assert_eq!(present_mode_candidates(240), present_mode_candidates(120));
}

#[test]
fn standard_present_mode_candidates_use_vsync_only() {
    assert_eq!(present_mode_candidates(60), &[wgpu::PresentMode::AutoVsync]);
    assert_eq!(present_mode_candidates(119), present_mode_candidates(60));
}

#[test]
fn select_present_mode_prefers_mailbox_for_high_refresh_when_supported() {
    let supported_present_modes = [
        wgpu::PresentMode::Mailbox,
        wgpu::PresentMode::Immediate,
        wgpu::PresentMode::Fifo,
    ];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::Mailbox
    );
}

#[test]
fn select_present_mode_falls_back_to_immediate_when_mailbox_is_unavailable() {
    let supported_present_modes = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::Immediate
    );
}

#[test]
fn select_present_mode_uses_auto_vsync_when_only_fifo_is_available() {
    let supported_present_modes = [wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::AutoVsync
    );
}

#[test]
fn select_present_mode_keeps_standard_refresh_on_auto_vsync() {
    let supported_present_modes = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(60, &supported_present_modes),
        wgpu::PresentMode::AutoVsync
    );
}
