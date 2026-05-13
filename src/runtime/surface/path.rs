use crate::layout::NodeId;

const INLINE_WIDGET_PATH_LEN: usize = 4;
const INLINE_CLIP_ANCESTOR_LEN: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
struct InlinePath<T, const N: usize> {
    inline: [T; N],
    len: u8,
    overflow: Option<Box<[T]>>,
}

impl<T, const N: usize> InlinePath<T, N>
where
    T: Copy + Default,
{
    fn from_slice(path: &[T]) -> Self {
        if path.len() <= N {
            let mut inline = [T::default(); N];
            inline[..path.len()].copy_from_slice(path);
            return Self {
                inline,
                len: path.len() as u8,
                overflow: None,
            };
        }
        Self {
            inline: [T::default(); N],
            len: 0,
            overflow: Some(path.into()),
        }
    }

    fn as_slice(&self) -> &[T] {
        self.overflow
            .as_deref()
            .unwrap_or(&self.inline[..self.len as usize])
    }

    #[cfg(test)]
    fn is_inline(&self) -> bool {
        self.overflow.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct WidgetPath(InlinePath<usize, INLINE_WIDGET_PATH_LEN>);

impl WidgetPath {
    pub(in crate::runtime) fn from_slice(path: &[usize]) -> Self {
        Self(InlinePath::from_slice(path))
    }

    pub(in crate::runtime) fn as_slice(&self) -> &[usize] {
        self.0.as_slice()
    }

    #[cfg(test)]
    pub(in crate::runtime) fn is_inline(&self) -> bool {
        self.0.is_inline()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct ClipAncestors(InlinePath<NodeId, INLINE_CLIP_ANCESTOR_LEN>);

impl ClipAncestors {
    pub(in crate::runtime) fn from_slice(ancestors: &[NodeId]) -> Self {
        Self(InlinePath::from_slice(ancestors))
    }

    pub(in crate::runtime) fn as_slice(&self) -> &[NodeId] {
        self.0.as_slice()
    }

    #[cfg(test)]
    pub(in crate::runtime) fn is_inline(&self) -> bool {
        self.0.is_inline()
    }
}
