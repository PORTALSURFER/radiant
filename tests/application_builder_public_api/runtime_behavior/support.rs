use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoadingMessage {
    Start,
    Loaded(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FocusMessage {
    FocusName,
}

pub(crate) fn wait_for_runtime_message<Bridge, Message>(
    runtime: &mut SurfaceRuntime<Bridge, Message>,
) -> radiant::runtime::CommandOutcome
where
    Bridge: RuntimeBridge<Message>,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let finished = runtime.drain_runtime_messages();
        if finished.messages_dispatched > 0 || Instant::now() >= deadline {
            break finished;
        }
        thread::sleep(Duration::from_millis(1));
    }
}
