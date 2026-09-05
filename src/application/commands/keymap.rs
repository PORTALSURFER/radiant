use super::{CommandDescriptor, CommandId, CommandRegistry, CommandShortcut};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt};

const MAX_BYTES: usize = 65_536;
const MAX_ENTRIES: usize = 1024;

/// Why a stored entry remains inactive without being discarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapProblem {
    /// The entry does not match the supported command/bindings schema.
    MalformedEntry,
    /// The persistent command identifier is invalid.
    InvalidCommand,
    /// At least one binding is malformed or unsupported.
    InvalidBinding,
    /// Multiple entries address the same command; all such overrides are inactive.
    DuplicateCommand,
    /// No current registration supplies this command; the entry is retained for later use.
    UnavailableCommand,
}

/// One data-only diagnostic associated with a preserved stored entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapDiagnostic {
    /// Zero-based entry in the persisted array.
    pub entry: usize,
    /// Persistent command bytes when the entry contains a string identity.
    pub command: Option<String>,
    /// Reason the entry cannot currently provide an override.
    pub problem: KeymapProblem,
}

/// The document cannot be admitted as a bounded supported keymap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeymapError {
    /// Invalid JSON syntax or serialization failure.
    Syntax(String),
    /// The root lacks the supported object and entries-array shape.
    MalformedDocument,
    /// A version other than 1 was supplied.
    UnsupportedVersion,
    /// Input exceeds 64 KiB, 1,024 entries, or 32 bindings per override.
    Capacity,
    /// Active scope identities in a validation request are ambiguous.
    InvalidScopes,
    /// A requested programmatic override contains an invalid binding.
    InvalidBinding,
}
impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid keymap: {self:?}")
    }
}
impl std::error::Error for KeymapError {}

/// Immutable versioned keymap data, preserving inactive and unknown entries on round-trip.
#[derive(Clone, Debug)]
pub struct Keymap {
    document: Value,
    overrides: BTreeMap<CommandId, Vec<CommandShortcut>>,
    diagnostics: Vec<KeymapDiagnostic>,
}
impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

impl Keymap {
    /// Construct a keymap using every registered default binding.
    pub fn new() -> Self {
        Self {
            document: json!({"version":1,"entries":[]}),
            overrides: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }
    /// Parse a bounded JSON document. Bad individual entries remain preserved and inactive.
    pub fn from_json(input: &str) -> Result<Self, KeymapError> {
        if input.len() > MAX_BYTES {
            return Err(KeymapError::Capacity);
        }
        let document = serde_json::from_str(input)
            .map_err(|error: serde_json::Error| KeymapError::Syntax(error.to_string()))?;
        Self::from_document(document)
    }
    fn from_document(document: Value) -> Result<Self, KeymapError> {
        let object = document.as_object().ok_or(KeymapError::MalformedDocument)?;
        if object.get("version").and_then(Value::as_u64) != Some(1) {
            return Err(KeymapError::UnsupportedVersion);
        }
        let entries = object
            .get("entries")
            .and_then(Value::as_array)
            .ok_or(KeymapError::MalformedDocument)?;
        if entries.len() > MAX_ENTRIES {
            return Err(KeymapError::Capacity);
        }
        let mut overrides = BTreeMap::new();
        let mut diagnostics = Vec::new();
        let mut seen: BTreeMap<CommandId, Vec<usize>> = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate() {
            let command = entry
                .get("command")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let mut diagnostic = |problem| {
                diagnostics.push(KeymapDiagnostic {
                    entry: index,
                    command: command.clone(),
                    problem,
                })
            };
            let Some(raw_id) = command.as_deref() else {
                diagnostic(KeymapProblem::MalformedEntry);
                continue;
            };
            let Ok(id) = CommandId::new(raw_id) else {
                diagnostic(KeymapProblem::InvalidCommand);
                continue;
            };
            seen.entry(id.clone()).or_default().push(index);
            if !entry.as_object().is_some_and(|fields| {
                fields
                    .keys()
                    .all(|key| matches!(key.as_str(), "command" | "bindings"))
            }) {
                diagnostic(KeymapProblem::MalformedEntry);
                continue;
            }
            let Some(raw_bindings) = entry.get("bindings").and_then(Value::as_array) else {
                diagnostic(KeymapProblem::MalformedEntry);
                continue;
            };
            if raw_bindings.len() > 32 {
                diagnostic(KeymapProblem::InvalidBinding);
                continue;
            }
            let Ok(bindings) =
                serde_json::from_value::<Vec<CommandShortcut>>(Value::Array(raw_bindings.clone()))
            else {
                diagnostic(KeymapProblem::InvalidBinding);
                continue;
            };
            if bindings.iter().any(|binding| !binding.valid()) {
                diagnostic(KeymapProblem::InvalidBinding);
                continue;
            }
            overrides.insert(id, bindings);
        }
        for (id, entries) in seen {
            if entries.len() > 1 {
                overrides.remove(&id);
                diagnostics.extend(entries.into_iter().map(|entry| KeymapDiagnostic {
                    entry,
                    command: Some(id.as_str().into()),
                    problem: KeymapProblem::DuplicateCommand,
                }));
            }
        }
        diagnostics.sort_by_key(|diagnostic| diagnostic.entry);
        Ok(Self {
            document,
            overrides,
            diagnostics,
        })
    }
    /// Serialize all data, including inactive entries and unknown root metadata, without file I/O.
    pub fn to_json(&self) -> String {
        self.document.to_string()
    }
    /// Borrow preserved entries for a keymap editor or diagnostics view.
    pub fn entries(&self) -> &[Value] {
        self.document["entries"]
            .as_array()
            .map_or(&[], Vec::as_slice)
    }
    /// Replace a command's overrides. An empty list explicitly unbinds; removing the override restores defaults.
    pub fn override_bindings(
        self,
        id: &CommandId,
        bindings: Vec<CommandShortcut>,
    ) -> Result<Self, KeymapError> {
        if bindings.len() > 32 {
            return Err(KeymapError::Capacity);
        }
        if bindings.iter().any(|binding| !binding.valid()) {
            return Err(KeymapError::InvalidBinding);
        }
        self.replace_entry(id, Some(json!({"command":id.as_str(),"bindings":bindings})))
    }
    /// Remove this command's stored override and use its registered defaults again.
    pub fn remove_override(self, id: &CommandId) -> Result<Self, KeymapError> {
        self.replace_entry(id, None)
    }
    fn replace_entry(
        mut self,
        id: &CommandId,
        replacement: Option<Value>,
    ) -> Result<Self, KeymapError> {
        let entries = self
            .document
            .get_mut("entries")
            .and_then(Value::as_array_mut)
            .ok_or(KeymapError::MalformedDocument)?;
        entries.retain(|entry| entry.get("command").and_then(Value::as_str) != Some(id.as_str()));
        if let Some(entry) = replacement {
            entries.push(entry);
        }
        if self.document.to_string().len() > MAX_BYTES {
            return Err(KeymapError::Capacity);
        }
        Self::from_document(self.document)
    }
    /// Return parse diagnostics plus commands currently absent from the static registry.
    pub fn diagnostics(&self, registry: &CommandRegistry) -> Vec<KeymapDiagnostic> {
        let mut diagnostics = self.diagnostics.clone();
        for (entry, value) in self.entries().iter().enumerate() {
            if let Some(raw) = value.get("command").and_then(Value::as_str)
                && let Ok(id) = CommandId::new(raw)
                && registry.get(&id).is_none()
            {
                diagnostics.push(KeymapDiagnostic {
                    entry,
                    command: Some(raw.into()),
                    problem: KeymapProblem::UnavailableCommand,
                });
            }
        }
        diagnostics.sort_by_key(|diagnostic| diagnostic.entry);
        diagnostics
    }
    pub(super) fn effective<'a>(
        &'a self,
        descriptor: &'a CommandDescriptor,
    ) -> &'a [CommandShortcut] {
        self.overrides
            .get(&descriptor.id)
            .map_or(descriptor.defaults.as_slice(), Vec::as_slice)
    }
}
