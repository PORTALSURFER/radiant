use super::*;
use crate::model::ToolId;

#[derive(Clone, Debug)]
pub(super) struct ToolbarIcons {
    pub(super) select: ToolbarIcon,
    pub(super) brush: ToolbarIcon,
    erase: ToolbarIcon,
    snap: ToolbarIcon,
}

impl ToolbarIcons {
    pub(super) fn new(theme: &ThemeTokens) -> Self {
        let active = theme.accent_warning;
        let inactive = theme.text_muted;
        Self {
            select: ToolbarIcon::new(&SELECT_ICON, active, inactive),
            brush: ToolbarIcon::new(&BRUSH_ICON, active, inactive),
            erase: ToolbarIcon::new(&ERASE_ICON, active, inactive),
            snap: ToolbarIcon::new(&SNAP_ICON, active, inactive),
        }
    }

    pub(super) fn icon(&self, tool: ToolId) -> ToolbarIcon {
        match tool {
            ToolId::Select => self.select.clone(),
            ToolId::Brush => self.brush.clone(),
            ToolId::Erase => self.erase.clone(),
            ToolId::Snap => self.snap.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ToolbarIcon {
    cache: &'static SvgIconTintCache,
    palette: SvgIconTintPalette,
}

impl ToolbarIcon {
    fn new(cache: &'static SvgIconTintCache, active: Rgba8, inactive: Rgba8) -> Self {
        Self {
            cache,
            palette: SvgIconTintPalette::new(inactive, active, inactive),
        }
    }

    pub(super) fn glyph(&self, active: bool) -> SvgIcon {
        self.cache.icon_for_state(self.palette, true, active)
    }
}

static SELECT_ICON: SvgIconTintCache = SvgIconTintCache::new(
    r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <polygon points="5,3 18,13 12,14 15,21 12,22 9,15 5,19" />
</svg>
"#,
);

static BRUSH_ICON: SvgIconTintCache = SvgIconTintCache::new(
    r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 15 3 L 21 9 L 12 18 L 6 12 Z" />
  <path d="M 5 13 L 11 19 L 8 22 L 2 22 L 2 16 Z" />
</svg>
"#,
);

static ERASE_ICON: SvgIconTintCache = SvgIconTintCache::new(
    r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 8 4 L 21 17 L 15 23 L 2 10 Z" />
  <rect x="7" y="16" width="11" height="4" />
</svg>
"#,
);

static SNAP_ICON: SvgIconTintCache = SvgIconTintCache::new(
    r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="3" width="6" height="12" />
  <rect x="14" y="3" width="6" height="12" />
  <rect x="4" y="17" width="16" height="4" />
</svg>
"#,
);
