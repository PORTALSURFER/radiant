use crate::{
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, NativeFileDrop, NativeFileOpen, ScrollUpdate},
};

/// Optional host policy for scroll, native-window, native-file, and shortcut input.
pub trait RuntimeInputHost<Message> {
    /// Observe runtime-owned scroll movement.
    fn scroll_updated(&mut self, _update: ScrollUpdate) -> Option<Command<Message>> {
        None
    }

    /// Handle a native file drag/drop event.
    fn native_file_drop(&mut self, _drop: NativeFileDrop) -> Command<Message> {
        Command::none()
    }

    /// Handle a native operating-system request to open files.
    fn native_file_open(&mut self, _open: NativeFileOpen) -> Command<Message> {
        Command::none()
    }

    /// Observe the native main window regaining operating-system focus.
    fn native_focus_regained(&mut self) -> Command<Message> {
        Command::none()
    }

    /// Resolve one keyboard press against host-owned shortcuts.
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        ShortcutResolution::unhandled()
    }
}

pub(crate) struct RuntimeInputCapability<Bridge, Message> {
    pub scroll_updated: fn(&mut Bridge, ScrollUpdate) -> Option<Command<Message>>,
    pub native_file_drop: fn(&mut Bridge, NativeFileDrop) -> Command<Message>,
    pub native_file_open: fn(&mut Bridge, NativeFileOpen) -> Command<Message>,
    pub native_focus_regained: fn(&mut Bridge) -> Command<Message>,
    pub resolve_key_press:
        fn(&mut Bridge, Option<KeyPress>, KeyPress, FocusSurface) -> ShortcutResolution<Message>,
}

impl<Bridge, Message> RuntimeInputCapability<Bridge, Message>
where
    Bridge: RuntimeInputHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            scroll_updated: Bridge::scroll_updated,
            native_file_drop: Bridge::native_file_drop,
            native_file_open: Bridge::native_file_open,
            native_focus_regained: Bridge::native_focus_regained,
            resolve_key_press: Bridge::resolve_key_press,
        }
    }
}
