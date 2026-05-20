pub(super) const MIN_COLUMNS: usize = 1;
pub(super) const MAX_COLUMNS: usize = 5;
pub(super) const MIN_ROWS: usize = 1;
pub(super) const MAX_ROWS: usize = 5;
pub(super) const MIN_DEPTH: usize = 0;
pub(super) const MAX_DEPTH: usize = 4;

#[derive(Clone, Debug)]
pub(super) struct LayoutDemoState {
    pub(super) show_sidebar: bool,
    pub(super) show_nested: bool,
    pub(super) columns: usize,
    pub(super) rows: usize,
    pub(super) depth: usize,
}

impl Default for LayoutDemoState {
    fn default() -> Self {
        Self {
            show_sidebar: true,
            show_nested: true,
            columns: 3,
            rows: 2,
            depth: 2,
        }
    }
}
