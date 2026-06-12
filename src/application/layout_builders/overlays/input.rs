use crate::application::{ViewNode, button, stack};

/// Build a full-size transparent layer that emits a dismiss message when activated.
///
/// Use this behind popovers, menus, dropdowns, and transient panels that should
/// close when the user clicks outside the foreground content.
pub fn dismiss_layer<Message>(message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button("")
        .message(message)
        .key("dismiss-layer")
        .input_only()
        .fill()
}

/// Stack base content with a transparent dismiss layer and foreground overlay.
///
/// Use this for transient menus, dropdowns, popovers, and inspectors where the
/// overlay should stay above an outside-click dismissal surface. The base
/// content remains visible underneath, while pointer activation outside the
/// foreground overlay emits `dismiss_message`.
pub fn dismissible_overlay<Message>(
    base: ViewNode<Message>,
    overlay: ViewNode<Message>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    stack([base, dismiss_layer(dismiss_message), overlay]).fill()
}

/// Layer transparent input over visible content in one stacked view.
///
/// This is useful for composite controls where the application owns the visual
/// row content but wants a generic button, interactive row, drag handle, or
/// other input surface to cover the same bounds without painting its own
/// chrome.
pub fn input_overlay<Message: 'static>(
    content: ViewNode<Message>,
    input: ViewNode<Message>,
) -> ViewNode<Message> {
    stack([content, input.input_only()])
}

/// Layer visible content over an input or feedback surface in one stacked view.
///
/// This is useful for composite rows where the input surface should still paint
/// hover, selection, drag, or drop-target feedback behind custom row content.
pub fn input_underlay<Message: 'static>(
    content: ViewNode<Message>,
    input: ViewNode<Message>,
) -> ViewNode<Message> {
    stack([input, content])
}
