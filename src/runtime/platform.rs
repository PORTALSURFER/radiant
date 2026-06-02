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
    /// Ask the platform integration to reveal or select a local path in the OS file manager.
    RevealPath(PathBuf),
    /// Ask the platform integration to open a URL with the OS shell.
    OpenUrl(String),
    /// Ask the platform integration to copy text to the system clipboard.
    CopyText(String),
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

impl PlatformResponse {
    /// Return `true` when the platform request completed without additional data.
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Return `true` when the user canceled a picker-style platform request.
    pub const fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }

    /// Borrow the path returned by a picker-style platform request.
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Path(path) => Some(path.as_path()),
            _ => None,
        }
    }

    /// Consume and return the path from a picker-style platform request.
    pub fn into_path(self) -> Option<PathBuf> {
        match self {
            Self::Path(path) => Some(path),
            _ => None,
        }
    }

    /// Consume a picker-style response, accepting a chosen path or cancellation.
    ///
    /// Returns the original response as `Err` when the response came from a
    /// different platform request kind.
    pub fn into_path_or_canceled(self) -> Result<Option<PathBuf>, Self> {
        match self {
            Self::Path(path) => Ok(Some(path)),
            Self::Canceled => Ok(None),
            other => Err(other),
        }
    }

    /// Consume a completion-style response.
    ///
    /// Returns the original response as `Err` when the response came from a
    /// request kind that returns data.
    pub fn into_completed(self) -> Result<(), Self> {
        match self {
            Self::Completed => Ok(()),
            other => Err(other),
        }
    }

    /// Borrow the confirmation response returned by a confirmation dialog.
    pub const fn confirmation(&self) -> Option<ConfirmationResponse> {
        match self {
            Self::Confirmation(response) => Some(*response),
            _ => None,
        }
    }

    /// Consume and return the confirmation response from a confirmation dialog.
    pub fn into_confirmation(self) -> Option<ConfirmationResponse> {
        match self {
            Self::Confirmation(response) => Some(response),
            _ => None,
        }
    }
}

/// Result returned to platform-service completion callbacks.
pub type PlatformResult = Result<PlatformResponse, String>;

/// Ergonomic decoders for platform-service callback results.
pub trait PlatformResultExt {
    /// Consume a completion-style response, propagating platform errors.
    fn into_completed(self) -> Result<(), String>;

    /// Consume a picker-style response, accepting a chosen path or cancellation.
    fn into_path_or_canceled(self) -> Result<Option<PathBuf>, String>;

    /// Consume and return the confirmation response from a confirmation dialog.
    fn into_confirmation(self) -> Result<ConfirmationResponse, String>;
}

impl PlatformResultExt for PlatformResult {
    fn into_completed(self) -> Result<(), String> {
        self?.into_completed().map_err(unexpected_platform_response)
    }

    fn into_path_or_canceled(self) -> Result<Option<PathBuf>, String> {
        self?
            .into_path_or_canceled()
            .map_err(unexpected_platform_response)
    }

    fn into_confirmation(self) -> Result<ConfirmationResponse, String> {
        match self? {
            PlatformResponse::Confirmation(response) => Ok(response),
            other => Err(unexpected_platform_response(other)),
        }
    }
}

fn unexpected_platform_response(response: PlatformResponse) -> String {
    format!("unexpected platform response: {response:?}")
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

/// Named fields for constructing a confirmation dialog request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmDialogParts {
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
    /// Build a confirmation dialog request from named parts.
    pub fn from_parts(parts: ConfirmDialogParts) -> Self {
        Self {
            title: parts.title,
            message: parts.message,
            level: parts.level,
            buttons: parts.buttons,
        }
    }

    /// Build a confirmation dialog request.
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::from_parts(ConfirmDialogParts {
            title: title.into(),
            message: message.into(),
            level: ConfirmationLevel::Info,
            buttons: ConfirmationButtons::OkCancel,
        })
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
pub type PlatformCompletion<Message> = Box<dyn FnOnce(PlatformResult) -> Message + Send + 'static>;

/// Boxed fallback returned when a bridge declines a platform service request.
pub type PlatformServiceFallback<Message> = Box<(PlatformRequest, PlatformCompletion<Message>)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_result_ext_decodes_picker_results() {
        let path = PathBuf::from("/samples");

        assert_eq!(
            PlatformResultExt::into_path_or_canceled(Ok(PlatformResponse::Path(path.clone()))),
            Ok(Some(path))
        );
        assert_eq!(
            PlatformResultExt::into_path_or_canceled(Ok(PlatformResponse::Canceled)),
            Ok(None)
        );
        assert_eq!(
            PlatformResultExt::into_path_or_canceled(Err(String::from("dialog unavailable"))),
            Err(String::from("dialog unavailable"))
        );
    }

    #[test]
    fn platform_result_ext_rejects_wrong_response_shapes() {
        let error = PlatformResultExt::into_completed(Ok(PlatformResponse::Path(PathBuf::from(
            "/samples",
        ))))
        .expect_err("completion decoder should reject path responses");

        assert!(error.contains("unexpected platform response"));

        let error = PlatformResultExt::into_path_or_canceled(Ok(PlatformResponse::Completed))
            .expect_err("picker decoder should reject completion responses");

        assert!(error.contains("unexpected platform response"));
    }

    #[test]
    fn platform_result_ext_decodes_completion_and_confirmation_results() {
        assert_eq!(
            PlatformResultExt::into_completed(Ok(PlatformResponse::Completed)),
            Ok(())
        );
        assert_eq!(
            PlatformResultExt::into_confirmation(Ok(PlatformResponse::Confirmation(
                ConfirmationResponse::Accepted,
            ))),
            Ok(ConfirmationResponse::Accepted)
        );
    }
}
