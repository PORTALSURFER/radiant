use super::{
    super::*,
    fixtures::{PlatformCommandBridge, QueuedCommandBridge},
};
use crate::runtime::{FileDialogRequest, PlatformRequest, PlatformResponse};
use std::path::PathBuf;

#[test]
fn platform_request_dispatches_through_bridge_completion() {
    let bridge = PlatformCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = PlatformRequest::PickFolder(FileDialogRequest::new().title("Choose library"));

    let outcome =
        runtime.execute_command(Command::platform_request(
            request.clone(),
            |result| match result.expect("platform request should complete") {
                PlatformResponse::Canceled => 7,
                _ => 0,
            },
        ));

    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(runtime.bridge().requests, vec![request]);
    assert_eq!(runtime.bridge().dispatched, vec![7]);
}

#[test]
fn platform_request_supports_shell_open_variants() {
    let bridge = PlatformCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let path = PathBuf::from(r"C:\samples");

    runtime.execute_command(Command::platform_request(
        PlatformRequest::OpenPath(path.clone()),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 1,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::OpenUrl(String::from("https://example.invalid")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 2,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::RevealPath(path.join("kick.wav")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 3,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::CopyText(String::from("C:/samples/kick.wav")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 4,
            _ => 0,
        },
    ));

    assert_eq!(
        runtime.bridge().requests,
        vec![
            PlatformRequest::OpenPath(path.clone()),
            PlatformRequest::OpenUrl(String::from("https://example.invalid")),
            PlatformRequest::RevealPath(path.join("kick.wav")),
            PlatformRequest::CopyText(String::from("C:/samples/kick.wav")),
        ]
    );
    assert_eq!(runtime.bridge().dispatched, vec![1, 2, 3, 4]);
}

#[test]
fn unsupported_platform_request_reports_error_message() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = PlatformRequest::PickFolder(FileDialogRequest::new());

    let outcome = runtime.execute_command(Command::platform_request(request, |result| {
        usize::from(result.is_err())
    }));

    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}
