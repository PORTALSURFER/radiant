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
        let active_icon = theme.accent_warning;
        Self {
            select: ToolbarIcon::new(SELECT_ICON, active_icon, theme.text_muted),
            brush: ToolbarIcon::new(BRUSH_ICON, active_icon, theme.text_muted),
            erase: ToolbarIcon::new(ERASE_ICON, active_icon, theme.text_muted),
            snap: ToolbarIcon::new(SNAP_ICON, active_icon, theme.text_muted),
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
    pub(super) active_glyph: Arc<SvgIcon>,
    pub(super) inactive_glyph: Arc<SvgIcon>,
}

impl ToolbarIcon {
    fn new(svg: &str, active: Rgba8, inactive: Rgba8) -> Self {
        Self {
            active_glyph: Arc::new(
                SvgIcon::from_svg(&with_current_color(svg, active))
                    .expect("active toolbar icon SVG should parse"),
            ),
            inactive_glyph: Arc::new(
                SvgIcon::from_svg(&with_current_color(svg, inactive))
                    .expect("inactive toolbar icon SVG should parse"),
            ),
        }
    }

    pub(super) fn glyph(&self, active: bool) -> &SvgIcon {
        if active {
            &self.active_glyph
        } else {
            &self.inactive_glyph
        }
    }
}

fn with_current_color(svg: &str, color: Rgba8) -> String {
    let color = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
    svg.replacen(
        "<svg ",
        &format!(r#"<svg color="{color}" fill="currentColor" "#),
        1,
    )
}

const SELECT_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <polygon points="5,3 18,13 12,14 15,21 12,22 9,15 5,19" />
</svg>
"#;

const BRUSH_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 15 3 L 21 9 L 12 18 L 6 12 Z" />
  <path d="M 5 13 L 11 19 L 8 22 L 2 22 L 2 16 Z" />
</svg>
"#;

const ERASE_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 8 4 L 21 17 L 15 23 L 2 10 Z" />
  <rect x="7" y="16" width="11" height="4" />
</svg>
"#;

const SNAP_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="3" width="6" height="12" />
  <rect x="14" y="3" width="6" height="12" />
  <rect x="4" y="17" width="16" height="4" />
</svg>
"#;
