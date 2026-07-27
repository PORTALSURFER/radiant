use super::{
    super::*,
    fixtures::{PlatformCommandBridge, QueuedCommandBridge},
};
use crate::layout::ContainerPolicy;
use crate::runtime::{
    FileDialogRequest, PlatformRequest, PlatformResponse, RuntimeBridge, RuntimeHostCapabilities,
    RuntimePlatformResultHost, RuntimePlatformResultSink, SurfaceNode,
};
use std::sync::Arc;

#[derive(Default)]
struct SynchronousResultBridge {
    dispatched: Vec<usize>,
}

impl RuntimeBridge<usize> for SynchronousResultBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_platform_results()
    }
}

impl RuntimePlatformResultHost for SynchronousResultBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        sink.send(Ok(PlatformResponse::Completed));
        Ok(())
    }
}
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
    runtime.execute_command(Command::platform_request(
        PlatformRequest::CopyFilePaths(vec![path.join("kick.wav")]),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 5,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::ReadText,
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 6,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::ReadFilePaths,
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 7,
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
            PlatformRequest::CopyFilePaths(vec![path.join("kick.wav")]),
            PlatformRequest::ReadText,
            PlatformRequest::ReadFilePaths,
        ]
    );
    assert_eq!(runtime.bridge().dispatched, vec![1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn unsupported_platform_request_reports_error_message() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = PlatformRequest::PickFolder(FileDialogRequest::new());

    let outcome = runtime.execute_command(Command::platform_request(request, |result| {
        usize::from(result.is_err())
    }));

    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().dispatched.is_empty());

    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

#[test]
fn synchronous_result_host_completion_is_deferred_to_next_drain() {
    let mut runtime = SurfaceRuntime::new(
        SynchronousResultBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    let outcome = runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::OpenUrl(String::from("https://example.invalid")),
        |result| usize::from(result.is_ok()),
    ));
    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().dispatched.is_empty());

    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}
