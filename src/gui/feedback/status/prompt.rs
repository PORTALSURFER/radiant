/// Generic intent category for host-provided confirmation prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptIntent {
    /// Confirm a destructive or irreversible operation.
    DestructiveOperation,
    /// Rename the focused content item.
    RenameContent,
    /// Rename an item in a navigation surface.
    RenameNavigationItem,
    /// Create an item in a navigation surface.
    CreateNavigationItem,
    /// Restore retained items after a recoverable operation.
    RestoreRetainedItems,
    /// Permanently purge retained items after a recoverable operation.
    PurgeRetainedItems,
    /// Edit a configuration value.
    EditConfiguration,
}

/// Modal confirmation prompt content parameterized by host-owned prompt kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmPrompt<Kind> {
    /// Whether the prompt is currently visible.
    pub visible: bool,
    /// Host-owned prompt kind used to resolve confirm/cancel behavior.
    pub kind: Option<Kind>,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
    /// Confirm action label.
    pub confirm_label: String,
    /// Cancel action label.
    pub cancel_label: String,
    /// Optional target label shown as supplemental metadata.
    pub target_label: Option<String>,
    /// Optional editable prompt input value.
    pub input_value: Option<String>,
    /// Placeholder text for editable prompt input fields.
    pub input_placeholder: Option<String>,
    /// Optional validation error shown below editable prompt input.
    pub input_error: Option<String>,
}

impl<Kind> Default for ConfirmPrompt<Kind> {
    fn default() -> Self {
        Self {
            visible: false,
            kind: None,
            title: String::new(),
            message: String::new(),
            confirm_label: String::new(),
            cancel_label: String::new(),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        }
    }
}
