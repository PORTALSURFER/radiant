use std::path::PathBuf;

/// Platform-neutral request for host-visible OS or shell services.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformRequest {
    /// Ask the platform integration to choose a folder.
    PickFolder(FileDialogRequest),
    /// Ask the platform integration to choose an existing file.
    PickFile(FileDialogRequest),
    /// Ask the platform integration to choose a save path.
    SaveFile(FileDialogRequest),
    /// Ask the platform integration to open a local path with the OS shell.
    OpenPath(PathBuf),
    /// Ask the platform integration to open a URL with the OS shell.
    OpenUrl(String),
    /// Ask the platform integration to show a confirmation dialog.
    Confirm(ConfirmDialogRequest),
}

/// Platform-neutral result for host-visible OS or shell services.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformResponse {
    /// The request completed without returning additional data.
    Completed,
    /// The user chose a path.
    Path(PathBuf),
    /// The user canceled a path picker.
    Canceled,
    /// The user answered a confirmation dialog.
    Confirmation(ConfirmationResponse),
}

/// Request metadata for a file or folder dialog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialogRequest {
    /// Dialog title.
    pub title: Option<String>,
    /// Initial directory, when known.
    pub directory: Option<PathBuf>,
    /// Initial filename for save dialogs.
    pub filename: Option<String>,
    /// File type filters for file dialogs.
    pub filters: Vec<FileDialogFilter>,
}

impl FileDialogRequest {
    /// Build an empty file dialog request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the initial directory.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Set the initial filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Add one file type filter.
    pub fn filter(mut self, name: impl Into<String>, extensions: impl Into<Vec<String>>) -> Self {
        self.filters.push(FileDialogFilter {
            name: name.into(),
            extensions: extensions.into(),
        });
        self
    }
}

/// File type filter for file dialogs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialogFilter {
    /// User-visible filter name.
    pub name: String,
    /// Extensions without leading dots.
    pub extensions: Vec<String>,
}

/// Request metadata for a confirmation dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmDialogRequest {
    /// Dialog title.
    pub title: String,
    /// Primary dialog text.
    pub message: String,
    /// Confirmation severity.
    pub level: ConfirmationLevel,
    /// Button set requested by the host.
    pub buttons: ConfirmationButtons,
}

impl ConfirmDialogRequest {
    /// Build a confirmation dialog request.
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: ConfirmationLevel::Info,
            buttons: ConfirmationButtons::OkCancel,
        }
    }

    /// Set the confirmation severity.
    pub fn level(mut self, level: ConfirmationLevel) -> Self {
        self.level = level;
        self
    }

    /// Set the requested button set.
    pub fn buttons(mut self, buttons: ConfirmationButtons) -> Self {
        self.buttons = buttons;
        self
    }
}

/// Confirmation severity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmationLevel {
    /// Informational prompt.
    #[default]
    Info,
    /// Warning prompt.
    Warning,
    /// Error prompt.
    Error,
}

/// Confirmation button set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmationButtons {
    /// Single acknowledgement button.
    Ok,
    /// Acknowledge or cancel.
    #[default]
    OkCancel,
    /// Explicit yes or no.
    YesNo,
}

/// Confirmation response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationResponse {
    /// User accepted the prompt.
    Accepted,
    /// User rejected the prompt.
    Rejected,
    /// User canceled or dismissed the prompt.
    Canceled,
}

/// Callback mapped into a host message when a platform service completes.
pub type PlatformCompletion<Message> =
    Box<dyn FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static>;
