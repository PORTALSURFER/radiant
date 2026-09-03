use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

/// Maximum UTF-8 byte length accepted for a platform-owned text value.
pub const MAX_PLATFORM_TEXT_BYTES: usize = 16 * 1024;
/// Maximum UTF-8 byte length accepted for one platform path representation.
pub const MAX_PLATFORM_PATH_BYTES: usize = 4 * 1024;
/// Maximum number of paths carried by one platform request or response.
pub const MAX_PLATFORM_PATH_COUNT: usize = 64;
/// Maximum title length for one neutral notification request.
pub const MAX_NOTIFICATION_TITLE_BYTES: usize = 256;
/// Maximum body length for one neutral notification request.
pub const MAX_NOTIFICATION_BODY_BYTES: usize = 4 * 1024;
/// Maximum text length for an in-process clipboard value.
pub const MAX_CLIPBOARD_TEXT_BYTES: usize = 16 * 1024;

/// Platform capability named in a typed service failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlatformService {
    /// File and folder dialogs.
    FileDialog,
    /// Shell operations such as opening paths or URLs.
    Shell,
    /// External system clipboard operations.
    Clipboard,
    /// Clipboard coordination owned by one Radiant application instance.
    InProcessClipboard,
    /// Confirmation dialogs.
    Confirmation,
    /// Neutral transient notifications.
    Notification,
}

impl fmt::Display for PlatformService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FileDialog => "file dialog",
            Self::Shell => "shell",
            Self::Clipboard => "clipboard",
            Self::InProcessClipboard => "in-process clipboard",
            Self::Confirmation => "confirmation",
            Self::Notification => "notification",
        })
    }
}

/// Structured failure returned by a platform service boundary.
///
/// The enum deliberately contains no native handles or unbounded request
/// payloads. Backend text is retained only for the bounded transport case;
/// callers that need stable branching should match the typed variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformFailure {
    /// The selected service is not implemented by this adapter.
    Unsupported(PlatformService),
    /// The service exists but is currently unavailable.
    Unavailable(PlatformService),
    /// The platform denied access to the service.
    PermissionDenied(PlatformService),
    /// The request failed local validation before host admission.
    InvalidRequest,
    /// The adapter returned a response incompatible with the request.
    InvalidResponse,
    /// The runtime or service was already closed.
    Closed,
    /// A bounded ingress or host lane rejected the request for capacity.
    Capacity,
    /// A transport or backend failure with a bounded diagnostic message.
    Transport(String),
    /// No value was available in the in-process clipboard slot.
    ClipboardEmpty,
    /// The slot contained a value of a different requested type.
    ClipboardTypeMismatch {
        /// Requested clipboard representation.
        requested: ClipboardFormat,
        /// Representation currently retained by the slot.
        available: ClipboardFormat,
    },
    /// A clipboard value exceeded one of the bounded value limits.
    ClipboardValueTooLarge,
}

/// Compatibility alias for callers that prefer an `Error`-named platform type.
pub type PlatformError = PlatformFailure;

impl PlatformFailure {
    /// Construct an unsupported-capability failure.
    pub const fn unsupported(service: PlatformService) -> Self {
        Self::Unsupported(service)
    }

    /// Construct an unavailable-service failure.
    pub const fn unavailable(service: PlatformService) -> Self {
        Self::Unavailable(service)
    }

    /// Construct a permission-denied failure.
    pub const fn permission_denied(service: PlatformService) -> Self {
        Self::PermissionDenied(service)
    }

    /// Construct a local request-validation failure.
    pub const fn invalid_request() -> Self {
        Self::InvalidRequest
    }

    /// Construct an adapter-response validation failure.
    pub const fn invalid_response() -> Self {
        Self::InvalidResponse
    }

    /// Construct a closed-runtime failure.
    pub const fn closed() -> Self {
        Self::Closed
    }

    /// Construct an ingress-capacity failure.
    pub const fn capacity() -> Self {
        Self::Capacity
    }

    /// Construct an empty in-process clipboard failure.
    pub const fn clipboard_empty() -> Self {
        Self::ClipboardEmpty
    }

    /// Construct a clipboard-bounds failure.
    pub const fn clipboard_value_too_large() -> Self {
        Self::ClipboardValueTooLarge
    }

    /// Construct a bounded transport failure from backend text.
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport(bounded_string(message.into(), MAX_PLATFORM_TEXT_BYTES))
    }

    /// Check the bounded display form for compatibility with legacy error assertions.
    pub fn contains(&self, pattern: &str) -> bool {
        self.to_string().contains(pattern)
    }

    pub(crate) fn bounded(self) -> Self {
        match self {
            Self::Transport(message) => {
                Self::Transport(bounded_string(message, MAX_PLATFORM_TEXT_BYTES))
            }
            other => other,
        }
    }
}

impl fmt::Display for PlatformFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(service) => write!(formatter, "{service} is unsupported"),
            Self::Unavailable(service) => write!(formatter, "{service} is unavailable"),
            Self::PermissionDenied(service) => write!(formatter, "permission denied for {service}"),
            Self::InvalidRequest => formatter.write_str("invalid platform request"),
            Self::InvalidResponse => formatter.write_str("unexpected platform response"),
            Self::Closed => formatter.write_str("platform runtime is closed"),
            Self::Capacity => formatter.write_str("platform result ingress is at capacity"),
            Self::Transport(message) => write!(formatter, "platform transport failure: {message}"),
            Self::ClipboardEmpty => formatter.write_str("in-process clipboard is empty"),
            Self::ClipboardTypeMismatch {
                requested,
                available,
            } => write!(
                formatter,
                "in-process clipboard type mismatch: requested {requested:?}, available {available:?}"
            ),
            Self::ClipboardValueTooLarge => {
                formatter.write_str("in-process clipboard value is too large")
            }
        }
    }
}

impl Error for PlatformFailure {}

impl From<String> for PlatformFailure {
    fn from(message: String) -> Self {
        Self::transport(message)
    }
}

impl From<&str> for PlatformFailure {
    fn from(message: &str) -> Self {
        Self::transport(message)
    }
}

impl From<PlatformFailure> for String {
    fn from(error: PlatformFailure) -> Self {
        error.to_string()
    }
}

/// Type of a bounded in-process clipboard value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardFormat {
    /// UTF-8 text.
    Text,
    /// An owned finite list of paths.
    FilePaths,
}

/// Opaque generation identity for the app-instance clipboard slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClipboardIdentity(u64);

impl ClipboardIdentity {
    pub(crate) const fn new(generation: u64) -> Self {
        Self(generation)
    }
}

/// Error returned when constructing a bounded in-process clipboard value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardValueError {
    /// Text exceeded [`MAX_CLIPBOARD_TEXT_BYTES`].
    TextTooLarge,
    /// The path list exceeded [`MAX_PLATFORM_PATH_COUNT`].
    TooManyPaths,
    /// A path exceeded [`MAX_PLATFORM_PATH_BYTES`].
    PathTooLarge,
    /// An in-process file-path value cannot be empty.
    EmptyPaths,
}

impl fmt::Display for ClipboardValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TextTooLarge => "clipboard text is too large",
            Self::TooManyPaths => "clipboard contains too many paths",
            Self::PathTooLarge => "clipboard path is too large",
            Self::EmptyPaths => "clipboard file paths cannot be empty",
        })
    }
}

impl Error for ClipboardValueError {}

/// Owned, bounded, typed content retained by one application-instance
/// clipboard coordinator. It is never sent to an adapter host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipboardValue {
    /// UTF-8 text content.
    Text(String),
    /// Owned file paths.
    FilePaths(Vec<PathBuf>),
}

impl ClipboardValue {
    /// Construct a bounded text value.
    pub fn text(text: impl Into<String>) -> Result<Self, ClipboardValueError> {
        let text = text.into();
        if text.len() > MAX_CLIPBOARD_TEXT_BYTES {
            return Err(ClipboardValueError::TextTooLarge);
        }
        Ok(Self::Text(text))
    }

    /// Construct a bounded file-path value.
    pub fn file_paths(paths: impl Into<Vec<PathBuf>>) -> Result<Self, ClipboardValueError> {
        let paths = paths.into();
        validate_paths(&paths, true)?;
        Ok(Self::FilePaths(paths))
    }

    /// Return the stored value's representation type.
    pub const fn format(&self) -> ClipboardFormat {
        match self {
            Self::Text(_) => ClipboardFormat::Text,
            Self::FilePaths(_) => ClipboardFormat::FilePaths,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        match self {
            Self::Text(text) if text.len() <= MAX_CLIPBOARD_TEXT_BYTES => Ok(()),
            Self::FilePaths(paths) => {
                validate_paths(paths, true).map_err(|_| PlatformFailure::ClipboardValueTooLarge)
            }
            Self::Text(_) => Err(PlatformFailure::ClipboardValueTooLarge),
        }
    }
}

/// Compatibility vocabulary for typed in-process clipboard content.
pub type ClipboardContent = ClipboardValue;

/// Compatibility vocabulary for a clipboard representation selector.
pub type ClipboardContentFormat = ClipboardFormat;

/// Compatibility vocabulary for an opaque clipboard generation.
pub type InProcessClipboardIdentity = ClipboardIdentity;

/// Severity requested for a neutral adapter notification.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NotificationLevel {
    /// Informational notice.
    #[default]
    Info,
    /// Successful operation notice.
    Success,
    /// Warning notice.
    Warning,
    /// Error notice.
    Error,
}

/// Owned, bounded, adapter-facing notification request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationRequest {
    /// Short notification title.
    pub title: String,
    /// Notification body.
    pub body: String,
    /// Requested semantic severity.
    pub level: NotificationLevel,
}

impl NotificationRequest {
    /// Construct an informational notification.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            level: NotificationLevel::Info,
        }
    }

    /// Set the notification severity.
    pub const fn level(mut self, level: NotificationLevel) -> Self {
        self.level = level;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        if self.title.len() > MAX_NOTIFICATION_TITLE_BYTES
            || self.body.len() > MAX_NOTIFICATION_BODY_BYTES
        {
            return Err(PlatformFailure::InvalidRequest);
        }
        Ok(())
    }
}

/// Compatibility vocabulary for an adapter notification request.
pub type PlatformNotificationRequest = NotificationRequest;

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
    /// Ask the platform integration to copy file paths to the system clipboard.
    CopyFilePaths(Vec<PathBuf>),
    /// Ask the platform integration to read text from the system clipboard.
    ReadText,
    /// Ask the platform integration to read file paths from the system clipboard.
    ReadFilePaths,
    /// Ask the platform integration to show a confirmation dialog.
    Confirm(ConfirmDialogRequest),
    /// Ask the adapter to post a neutral transient notification.
    Notify(NotificationRequest),
    /// Read a typed value from this application's in-process clipboard slot.
    ///
    /// The controller handles this variant locally; it is never passed to a
    /// platform adapter.
    ReadClipboard(ClipboardFormat),
    /// Replace this application's in-process clipboard slot with a typed value.
    ///
    /// The controller handles this variant locally; it is never passed to a
    /// platform adapter.
    WriteClipboard(ClipboardValue),
}

impl PlatformRequest {
    /// Return the capability represented by this request.
    pub const fn service(&self) -> PlatformService {
        match self {
            Self::PickFolder(_) | Self::PickFile(_) | Self::SaveFile(_) => {
                PlatformService::FileDialog
            }
            Self::OpenPath(_) | Self::RevealPath(_) | Self::OpenUrl(_) => PlatformService::Shell,
            Self::CopyText(_) | Self::CopyFilePaths(_) | Self::ReadText | Self::ReadFilePaths => {
                PlatformService::Clipboard
            }
            Self::Confirm(_) => PlatformService::Confirmation,
            Self::Notify(_) => PlatformService::Notification,
            Self::ReadClipboard(_) | Self::WriteClipboard(_) => PlatformService::InProcessClipboard,
        }
    }

    /// Construct a notification request through the common platform route.
    pub fn notify(request: NotificationRequest) -> Self {
        Self::Notify(request)
    }

    /// Construct an in-process typed clipboard read request.
    pub const fn read_clipboard(format: ClipboardFormat) -> Self {
        Self::ReadClipboard(format)
    }

    /// Construct an in-process typed clipboard write request.
    pub fn write_clipboard(value: ClipboardValue) -> Self {
        Self::WriteClipboard(value)
    }

    /// Compatibility name for the in-process clipboard read constructor.
    pub const fn read_in_process_clipboard(format: ClipboardFormat) -> Self {
        Self::ReadClipboard(format)
    }

    /// Compatibility name for the in-process clipboard write constructor.
    pub fn write_in_process_clipboard(value: ClipboardValue) -> Self {
        Self::WriteClipboard(value)
    }

    /// Validate bounded request data before host admission.
    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        match self {
            Self::PickFolder(request) | Self::PickFile(request) | Self::SaveFile(request) => {
                request.validate()
            }
            Self::OpenPath(path) | Self::RevealPath(path) => validate_path(path),
            Self::OpenUrl(url) | Self::CopyText(url) => validate_text(url),
            Self::CopyFilePaths(paths) => {
                validate_paths(paths, true).map_err(|_| PlatformFailure::InvalidRequest)
            }
            Self::ReadText | Self::ReadFilePaths | Self::ReadClipboard(_) => Ok(()),
            Self::Confirm(request) => request.validate(),
            Self::Notify(request) => request.validate(),
            Self::WriteClipboard(value) => value.validate(),
        }
    }

    /// Return whether the controller must satisfy this request locally.
    pub(crate) const fn is_in_process_clipboard(&self) -> bool {
        matches!(self, Self::ReadClipboard(_) | Self::WriteClipboard(_))
    }

    /// Validate an adapter result before a UI mapper is allowed to run.
    pub(crate) fn validate_result(&self, result: &PlatformResult) -> Result<(), PlatformFailure> {
        let Ok(response) = result else {
            return Ok(());
        };
        self.validate()?;
        let shape_matches = match self {
            Self::PickFolder(_) | Self::PickFile(_) | Self::SaveFile(_) => {
                matches!(
                    response,
                    PlatformResponse::Path(_) | PlatformResponse::Canceled
                )
            }
            Self::OpenPath(_)
            | Self::RevealPath(_)
            | Self::OpenUrl(_)
            | Self::CopyText(_)
            | Self::CopyFilePaths(_)
            | Self::Notify(_)
            | Self::WriteClipboard(_) => matches!(response, PlatformResponse::Completed),
            Self::ReadText => matches!(response, PlatformResponse::Text(_)),
            Self::ReadFilePaths => matches!(response, PlatformResponse::FilePaths(_)),
            Self::Confirm(_) => matches!(response, PlatformResponse::Confirmation(_)),
            Self::ReadClipboard(format) => matches!(
                response,
                PlatformResponse::Clipboard(value) if value.format() == *format
            ),
        };
        if !shape_matches {
            return Err(PlatformFailure::InvalidResponse);
        }
        response.validate()
    }
}

/// Platform-neutral result for host-visible OS or shell services.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformResponse {
    /// The request completed without returning additional data.
    Completed,
    /// The user chose a path.
    Path(PathBuf),
    /// The platform returned text.
    Text(String),
    /// The platform returned file paths.
    FilePaths(Vec<PathBuf>),
    /// The user canceled a path picker.
    Canceled,
    /// The user answered a confirmation dialog.
    Confirmation(ConfirmationResponse),
    /// A typed value read from the app-instance clipboard coordinator.
    Clipboard(ClipboardValue),
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

    /// Consume and return text from a clipboard-style platform request.
    pub fn into_text(self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Consume and return file paths from a clipboard-style platform request.
    pub fn into_file_paths(self) -> Option<Vec<PathBuf>> {
        match self {
            Self::FilePaths(paths) => Some(paths),
            _ => None,
        }
    }

    /// Consume and return a typed in-process clipboard value.
    pub fn into_clipboard(self) -> Option<ClipboardValue> {
        match self {
            Self::Clipboard(value) => Some(value),
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

    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        match self {
            Self::Completed | Self::Canceled | Self::Confirmation(_) => Ok(()),
            Self::Path(path) => validate_path(path).map_err(|_| PlatformFailure::InvalidResponse),
            Self::Text(text) => validate_text(text).map_err(|_| PlatformFailure::InvalidResponse),
            Self::FilePaths(paths) => {
                validate_paths(paths, true).map_err(|_| PlatformFailure::InvalidResponse)
            }
            Self::Clipboard(value) => value
                .validate()
                .map_err(|_| PlatformFailure::InvalidResponse),
        }
    }
}

/// Result returned to platform-service completion callbacks.
pub type PlatformResult = Result<PlatformResponse, PlatformFailure>;

/// Opaque identity for one UI-owned platform completion mapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlatformCompletionIdentity {
    pub(crate) id: u64,
    pub(crate) epoch: u64,
}

/// Send-safe platform result delivery awaiting UI-owned mapping.
pub(crate) enum PlatformResultDelivery {
    Completed {
        identity: PlatformCompletionIdentity,
        result: PlatformResult,
    },
    Discarded {
        identity: PlatformCompletionIdentity,
    },
}

/// Ergonomic decoders for platform-service callback results.
pub trait PlatformResultExt {
    /// Consume a completion-style response, propagating platform errors.
    fn into_completed(self) -> Result<(), PlatformFailure>;

    /// Consume a picker-style response, accepting a chosen path or cancellation.
    fn into_path_or_canceled(self) -> Result<Option<PathBuf>, PlatformFailure>;

    /// Consume and return the confirmation response from a confirmation dialog.
    fn into_confirmation(self) -> Result<ConfirmationResponse, PlatformFailure>;

    /// Consume and return text from a clipboard-style platform request.
    fn into_text(self) -> Result<String, PlatformFailure>;

    /// Consume and return file paths from a clipboard-style platform request.
    fn into_file_paths(self) -> Result<Vec<PathBuf>, PlatformFailure>;

    /// Consume and return a typed in-process clipboard value.
    fn into_clipboard(self) -> Result<ClipboardValue, PlatformFailure>;
}

impl PlatformResultExt for PlatformResult {
    fn into_completed(self) -> Result<(), PlatformFailure> {
        self?.into_completed().map_err(unexpected_platform_response)
    }

    fn into_path_or_canceled(self) -> Result<Option<PathBuf>, PlatformFailure> {
        self?
            .into_path_or_canceled()
            .map_err(unexpected_platform_response)
    }

    fn into_confirmation(self) -> Result<ConfirmationResponse, PlatformFailure> {
        match self? {
            PlatformResponse::Confirmation(response) => Ok(response),
            other => Err(unexpected_platform_response(other)),
        }
    }

    fn into_text(self) -> Result<String, PlatformFailure> {
        match self? {
            PlatformResponse::Text(text) => Ok(text),
            other => Err(unexpected_platform_response(other)),
        }
    }

    fn into_file_paths(self) -> Result<Vec<PathBuf>, PlatformFailure> {
        match self? {
            PlatformResponse::FilePaths(paths) => Ok(paths),
            other => Err(unexpected_platform_response(other)),
        }
    }

    fn into_clipboard(self) -> Result<ClipboardValue, PlatformFailure> {
        match self? {
            PlatformResponse::Clipboard(value) => Ok(value),
            _ => Err(PlatformFailure::InvalidResponse),
        }
    }
}

fn unexpected_platform_response(_response: PlatformResponse) -> PlatformFailure {
    PlatformFailure::InvalidResponse
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

    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        if self
            .title
            .as_ref()
            .is_some_and(|title| title.len() > MAX_PLATFORM_TEXT_BYTES)
            || self
                .filename
                .as_ref()
                .is_some_and(|filename| filename.len() > MAX_PLATFORM_TEXT_BYTES)
            || self
                .directory
                .as_ref()
                .is_some_and(|directory| path_len(directory) > MAX_PLATFORM_PATH_BYTES)
            || self.filters.len() > MAX_PLATFORM_PATH_COUNT
            || self.filters.iter().any(|filter| {
                filter.name.len() > MAX_PLATFORM_TEXT_BYTES
                    || filter.extensions.len() > MAX_PLATFORM_PATH_COUNT
                    || filter
                        .extensions
                        .iter()
                        .any(|extension| extension.len() > MAX_PLATFORM_TEXT_BYTES)
            })
        {
            return Err(PlatformFailure::InvalidRequest);
        }
        Ok(())
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

    pub(crate) fn validate(&self) -> Result<(), PlatformFailure> {
        if self.title.len() > MAX_NOTIFICATION_TITLE_BYTES
            || self.message.len() > MAX_NOTIFICATION_BODY_BYTES
        {
            return Err(PlatformFailure::InvalidRequest);
        }
        Ok(())
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
pub type PlatformCompletion<Message> = Box<dyn FnOnce(PlatformResult) -> Message + 'static>;

/// Result-only completion sink for custom platform hosts.
///
/// The sink carries no application message or mapper. The runtime invokes its
/// callback only to enqueue a [`PlatformResult`] for a later UI turn.
pub struct RuntimePlatformResultSink {
    identity: PlatformCompletionIdentity,
    callback: Option<Box<dyn FnOnce(PlatformResultDelivery) + Send + 'static>>,
}

impl RuntimePlatformResultSink {
    pub(crate) fn new(
        identity: PlatformCompletionIdentity,
        callback: impl FnOnce(PlatformResultDelivery) + Send + 'static,
    ) -> Self {
        Self {
            identity,
            callback: Some(Box::new(callback)),
        }
    }

    /// Deliver one result to the runtime's deferred ingress.
    pub fn send(mut self, result: PlatformResult) {
        if let Some(callback) = self.callback.take() {
            callback(PlatformResultDelivery::Completed {
                identity: self.identity,
                result: result.map_err(PlatformFailure::bounded),
            });
        }
    }

    pub(crate) fn into_delivery(mut self, result: PlatformResult) -> PlatformResultDelivery {
        self.callback.take();
        PlatformResultDelivery::Completed {
            identity: self.identity,
            result: result.map_err(PlatformFailure::bounded),
        }
    }
}

impl Drop for RuntimePlatformResultSink {
    fn drop(&mut self) {
        if let Some(callback) = self.callback.take() {
            callback(PlatformResultDelivery::Discarded {
                identity: self.identity,
            });
        }
    }
}

/// Boxed fallback returned when a result-only host declines a request.
pub type PlatformResultServiceFallback = Box<(PlatformRequest, RuntimePlatformResultSink)>;

/// Boxed fallback returned when a bridge declines a platform service request.
pub type PlatformServiceFallback<Message> = Box<(PlatformRequest, PlatformCompletion<Message>)>;

fn bounded_string(mut value: String, limit: usize) -> String {
    if value.len() <= limit {
        return value;
    }
    let mut boundary = limit;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

fn path_len(path: &Path) -> usize {
    path.to_string_lossy().len()
}

fn validate_path(path: &Path) -> Result<(), PlatformFailure> {
    if path_len(path) > MAX_PLATFORM_PATH_BYTES {
        Err(PlatformFailure::InvalidRequest)
    } else {
        Ok(())
    }
}

fn validate_text(text: &str) -> Result<(), PlatformFailure> {
    if text.len() > MAX_PLATFORM_TEXT_BYTES {
        Err(PlatformFailure::InvalidRequest)
    } else {
        Ok(())
    }
}

fn validate_paths(paths: &[PathBuf], require_non_empty: bool) -> Result<(), ClipboardValueError> {
    if require_non_empty && paths.is_empty() {
        return Err(ClipboardValueError::EmptyPaths);
    }
    if paths.len() > MAX_PLATFORM_PATH_COUNT {
        return Err(ClipboardValueError::TooManyPaths);
    }
    if paths
        .iter()
        .any(|path| path_len(path) > MAX_PLATFORM_PATH_BYTES)
    {
        return Err(ClipboardValueError::PathTooLarge);
    }
    Ok(())
}

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
            PlatformResultExt::into_path_or_canceled(Err(PlatformFailure::transport(
                "dialog unavailable",
            ))),
            Err(PlatformFailure::transport("dialog unavailable"))
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

    #[test]
    fn platform_result_ext_decodes_clipboard_results() {
        let paths = vec![PathBuf::from("/samples/kick.wav")];
        assert_eq!(
            PlatformResultExt::into_text(Ok(PlatformResponse::Text(String::from("copied")))),
            Ok(String::from("copied"))
        );
        assert_eq!(
            PlatformResultExt::into_file_paths(Ok(PlatformResponse::FilePaths(paths.clone()))),
            Ok(paths)
        );
    }

    #[test]
    fn request_validation_distinguishes_malformed_success_from_typed_failure() {
        let invalid_request = PlatformRequest::CopyText("x".repeat(MAX_PLATFORM_TEXT_BYTES + 1));
        assert_eq!(
            invalid_request.validate(),
            Err(PlatformFailure::InvalidRequest)
        );
        assert_eq!(
            invalid_request.validate_result(&Err(PlatformFailure::InvalidRequest)),
            Ok(())
        );
        assert_eq!(
            invalid_request.validate_result(&Ok(PlatformResponse::Completed)),
            Err(PlatformFailure::InvalidRequest)
        );

        let malformed_response = PlatformRequest::ReadText;
        assert_eq!(
            malformed_response.validate_result(&Ok(PlatformResponse::Completed)),
            Err(PlatformFailure::InvalidResponse)
        );
        assert_eq!(
            malformed_response.validate_result(&Ok(PlatformResponse::Text(
                "x".repeat(MAX_PLATFORM_TEXT_BYTES + 1),
            ))),
            Err(PlatformFailure::InvalidResponse)
        );
    }

    #[test]
    fn platform_failures_are_typed_bounded_and_displayable() {
        let failures = [
            PlatformFailure::Unsupported(PlatformService::Notification),
            PlatformFailure::Unavailable(PlatformService::FileDialog),
            PlatformFailure::PermissionDenied(PlatformService::Clipboard),
            PlatformFailure::InvalidRequest,
            PlatformFailure::InvalidResponse,
            PlatformFailure::Closed,
            PlatformFailure::Capacity,
            PlatformFailure::transport("backend failed"),
            PlatformFailure::ClipboardEmpty,
            PlatformFailure::ClipboardTypeMismatch {
                requested: ClipboardFormat::Text,
                available: ClipboardFormat::FilePaths,
            },
            PlatformFailure::ClipboardValueTooLarge,
        ];

        for failure in failures {
            assert!(!failure.to_string().is_empty());
            let error: &dyn Error = &failure;
            assert!(!error.to_string().is_empty());
        }

        let oversized = PlatformFailure::Transport("x".repeat(MAX_PLATFORM_TEXT_BYTES + 1));
        assert!(oversized.to_string().len() <= MAX_PLATFORM_TEXT_BYTES + 32);
    }

    #[test]
    fn notification_and_clipboard_values_enforce_owned_bounds() {
        let notification =
            NotificationRequest::new("x".repeat(MAX_NOTIFICATION_TITLE_BYTES + 1), "body");
        assert_eq!(
            PlatformRequest::Notify(notification).validate(),
            Err(PlatformFailure::InvalidRequest)
        );
        assert_eq!(
            ClipboardValue::text("x".repeat(MAX_CLIPBOARD_TEXT_BYTES + 1)),
            Err(ClipboardValueError::TextTooLarge)
        );
        assert_eq!(
            ClipboardValue::file_paths(Vec::new()),
            Err(ClipboardValueError::EmptyPaths)
        );
        let too_many_paths = (0..=MAX_PLATFORM_PATH_COUNT)
            .map(|index| PathBuf::from(format!("/tmp/{index}")))
            .collect::<Vec<_>>();
        assert_eq!(
            ClipboardValue::file_paths(too_many_paths),
            Err(ClipboardValueError::TooManyPaths)
        );
    }

    #[test]
    fn result_sink_bounds_backend_transport_text_at_the_adapter_boundary() {
        use std::sync::{Arc, Mutex};

        let observed = Arc::new(Mutex::new(None));
        let observed_by_sink = Arc::clone(&observed);
        let identity = PlatformCompletionIdentity { id: 9, epoch: 1 };
        RuntimePlatformResultSink::new(identity, move |delivery| {
            if let PlatformResultDelivery::Completed { result, .. } = delivery {
                *observed_by_sink.lock().expect("result lock") = Some(result);
            }
        })
        .send(Err(PlatformFailure::Transport(
            "x".repeat(MAX_PLATFORM_TEXT_BYTES + 1),
        )));

        let result = observed
            .lock()
            .expect("result lock")
            .take()
            .expect("bounded transport result");
        let Err(PlatformFailure::Transport(message)) = result else {
            panic!("expected transport failure");
        };
        assert_eq!(message.len(), MAX_PLATFORM_TEXT_BYTES);
    }

    #[test]
    fn result_sink_send_into_delivery_and_drop_are_mutually_exclusive() {
        use std::sync::{Arc, Mutex};

        let identity = PlatformCompletionIdentity { id: 1, epoch: 1 };
        let events = Arc::new(Mutex::new(Vec::new()));
        let send_events = Arc::clone(&events);
        let sink = RuntimePlatformResultSink::new(identity, move |delivery| {
            send_events
                .lock()
                .expect("events lock")
                .push(matches!(delivery, PlatformResultDelivery::Completed { .. }));
        });
        sink.send(Ok(PlatformResponse::Completed));
        assert_eq!(events.lock().expect("events lock").as_slice(), &[true]);

        let into_events = Arc::clone(&events);
        let sink = RuntimePlatformResultSink::new(identity, move |delivery| {
            into_events
                .lock()
                .expect("events lock")
                .push(matches!(delivery, PlatformResultDelivery::Completed { .. }));
        });
        let delivery = sink.into_delivery(Ok(PlatformResponse::Completed));
        assert!(matches!(delivery, PlatformResultDelivery::Completed { .. }));
        assert_eq!(events.lock().expect("events lock").len(), 1);

        let drop_events = Arc::clone(&events);
        let sink = RuntimePlatformResultSink::new(identity, move |delivery| {
            drop_events
                .lock()
                .expect("events lock")
                .push(matches!(delivery, PlatformResultDelivery::Discarded { .. }));
        });
        std::thread::spawn(move || drop(sink))
            .join()
            .expect("sink drop thread");
        assert_eq!(
            events.lock().expect("events lock").as_slice(),
            &[true, true]
        );
    }
}
