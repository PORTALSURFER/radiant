//! UI-local declarative attachments, separate from frozen source evidence.
use super::{
    CommandBinding, CommandScope, CommandScopeError, CommandScopeKind, CommandSuppression,
};
use crate::layout::NodeId;
use std::{
    any::Any,
    fmt,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ATTACHMENT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct CommandScopeAttachment {
    pub(crate) automatic: bool,
    pub(crate) kind: CommandScopeKind,
    pub(crate) incarnation: u64,
    pub(crate) diagnostic: Option<CommandScopeError>,
    value: Option<Rc<dyn Any>>,
}
impl fmt::Debug for CommandScopeAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandScopeAttachment")
            .field("kind", &self.kind)
            .field("incarnation", &self.incarnation)
            .field("diagnostic", &self.diagnostic)
            .finish_non_exhaustive()
    }
}
impl CommandScopeAttachment {
    pub(crate) fn automatic<Context: 'static>(
        bindings: impl IntoIterator<Item = CommandBinding<Context>>,
    ) -> Self {
        Self::build(
            CommandScope::new("automatic", CommandScopeKind::Editor { depth: 0 }, bindings),
            true,
        )
    }
    pub(crate) fn explicit<Context: 'static>(scope: CommandScope<Context>) -> Self {
        Self::build(Ok(scope), false)
    }
    fn build<Context: 'static>(
        scope: Result<CommandScope<Context>, CommandScopeError>,
        automatic: bool,
    ) -> Self {
        let (kind, value, mut diagnostic) = match scope {
            Ok(scope) => (scope.kind, Some(Rc::new(scope) as Rc<dyn Any>), None),
            Err(error) => (CommandScopeKind::Editor { depth: 0 }, None, Some(error)),
        };
        let incarnation = NEXT_ATTACHMENT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .ok()
            .unwrap_or(0);
        if incarnation == 0 {
            diagnostic = Some(CommandScopeError::IdentityExhausted);
        }
        Self {
            automatic,
            kind,
            incarnation,
            diagnostic,
            value,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ResolvedCommandScope {
    pub(crate) node_id: NodeId,
    pub(crate) kind: CommandScopeKind,
    pub(crate) attachment: CommandScopeAttachment,
}

/// Borrowed active command attachments from the committed runtime view tree.
///
/// Hosts can project typed snapshots without interpreting application context.
/// Values remain UI-local and must not be stored as unqualified native actions.
#[derive(Clone, Copy)]
pub struct CommandScopeProjection<'a> {
    scopes: &'a [ResolvedCommandScope],
    error: Option<CommandSuppression>,
}
impl<'a> CommandScopeProjection<'a> {
    /// An empty projection for hosts without declarative command attachments.
    pub fn empty() -> Self {
        Self {
            scopes: &[],
            error: None,
        }
    }
    pub(crate) fn new(
        scopes: &'a [ResolvedCommandScope],
        error: Option<CommandSuppression>,
    ) -> Self {
        Self { scopes, error }
    }
    pub(crate) fn combined(
        self,
        inherited: &[ResolvedCommandScope],
        inherited_error: Option<CommandSuppression>,
    ) -> (Vec<ResolvedCommandScope>, Option<CommandSuppression>) {
        if let Some(error) = inherited_error.or(self.error) {
            return (Vec::new(), Some(error));
        }
        if inherited.len() + self.scopes.len() > 64 {
            return (Vec::new(), Some(CommandSuppression::Capacity));
        }
        (inherited.iter().chain(self.scopes).cloned().collect(), None)
    }
    /// Resolve current attachment context types, structural scope depth and automatic identities.
    ///
    /// A mismatched context type, ambiguous ownership or exceeded capacity is terminal;
    /// no subset is silently used as a fallback.
    pub fn scopes<Context: 'static>(
        &self,
    ) -> Result<Vec<CommandScope<Context>>, CommandSuppression> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.scopes.len() > 64 {
            return Err(CommandSuppression::Capacity);
        }
        let mut scopes = Vec::with_capacity(self.scopes.len());
        for record in self.scopes {
            if record.attachment.diagnostic.is_some() {
                return Err(CommandSuppression::InvalidScopes);
            }
            let Some(scope) = record
                .attachment
                .value
                .as_ref()
                .and_then(|value| value.downcast_ref::<CommandScope<Context>>())
            else {
                return Err(CommandSuppression::ContextMismatch);
            };
            let mut scope = scope.clone();
            if record.attachment.automatic {
                scope.id = format!("view:{}", record.node_id);
            }
            scope.kind = record.kind;
            scopes.push(scope);
        }
        Ok(scopes)
    }
    /// Iterate construction diagnostics in the current active projection.
    pub fn diagnostics(&self) -> impl Iterator<Item = (NodeId, &CommandScopeError)> {
        self.scopes.iter().filter_map(|record| {
            record
                .attachment
                .diagnostic
                .as_ref()
                .map(|error| (record.node_id, error))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{CommandId, IntoView, text};

    #[test]
    fn duplicate_bindings_keep_a_diagnostic_and_cannot_project() {
        let id = CommandId::new("save").unwrap();
        let records = [ResolvedCommandScope {
            node_id: 42,
            kind: CommandScopeKind::Editor { depth: 1 },
            attachment: CommandScopeAttachment::automatic([
                CommandBinding::new(id.clone(), 1),
                CommandBinding::new(id, 2),
            ]),
        }];
        let projection = CommandScopeProjection::new(&records, None);
        assert_eq!(projection.diagnostics().count(), 1);
        assert!(matches!(
            projection.scopes::<i32>(),
            Err(CommandSuppression::InvalidScopes)
        ));
    }

    #[test]
    fn scope_attachments_are_excluded_from_component_caches_and_freeze_only_identity() {
        struct Context;
        let plain = text::<()>("plain").id(42).into_node();
        assert_eq!(plain.component_cache_node_count(10), Some(1));
        let scoped = text::<()>("plain")
            .id(42)
            .commands([CommandBinding::new(
                CommandId::new("save").unwrap(),
                Context,
            )])
            .into_node();
        assert_eq!(scoped.component_cache_node_count(10), None);
        let metadata = scoped.source_metadata_handle().unwrap();
        assert_eq!(
            metadata.freeze().command_incarnation,
            scoped.command_scope().map(|scope| scope.incarnation)
        );
        assert_ne!(
            metadata.command_incarnation,
            plain.source_metadata_handle().unwrap().command_incarnation
        );
    }
}
