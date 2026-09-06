//! Public child command service boundaries and ownership bounds.
use radiant::application::{
    CommandDescriptor, CommandDispatchStatus, CommandDispatcher, CommandId, CommandInput,
    CommandInvocation, CommandKey, CommandRegistry, CommandRequest, CommandScopeProjection,
    CommandService, Keymap, TextKey,
};
use radiant::gui::shortcuts::ShortcutPlatform;

#[test]
fn public_child_service_clones_without_context_or_message_clone_bounds() {
    struct Context;
    struct Message;
    let registry = CommandRegistry::new([CommandDescriptor::new(
        CommandId::new("action").unwrap(),
        TextKey::new("action", "Action"),
    )])
    .unwrap();
    let service: CommandService<Message> = CommandService::new(
        registry,
        CommandDispatcher::new(|_: CommandInvocation<Context>| {
            panic!("empty child scopes must not invoke mapper")
        }),
        Keymap::new(),
    );
    let child = service.clone();
    let input = CommandInput::logical(CommandKey::Character("k".into()), ShortcutPlatform::Mac);
    let dispatch = child.resolve(
        CommandRequest::Input(&input),
        CommandScopeProjection::empty(),
    );
    assert_eq!(dispatch.status, CommandDispatchStatus::Unhandled);
    assert!(dispatch.message.is_none());
    let rows = child
        .presentations(
            CommandScopeProjection::empty(),
            &[CommandId::new("action").unwrap()],
            &Default::default(),
            ShortcutPlatform::Mac,
        )
        .unwrap();
    assert_eq!(rows[0].label, "Action");
    assert!(!rows[0].enabled);
}
