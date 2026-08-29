use std::{marker::PhantomData, rc::Rc};

/// Zero-sized marker for values owned by one UI runtime.
///
/// The `Rc` phantom data keeps the marker, and therefore its owners, out of
/// cross-thread transfers without allocating or changing runtime behavior.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct UiAffinity(PhantomData<Rc<()>>);

impl UiAffinity {
    pub(crate) const fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::UiAffinity;

    #[test]
    fn marker_is_zero_sized() {
        assert_eq!(std::mem::size_of::<UiAffinity>(), 0);
    }
}
