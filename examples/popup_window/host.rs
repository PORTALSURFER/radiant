//! Child-process popup host management for the popup example.

use super::model::PopupMode;

#[path = "host/child.rs"]
mod child;
#[path = "platform.rs"]
mod platform;
#[path = "host/prewarm.rs"]
mod prewarm;
#[path = "host/process.rs"]
mod process;

use child::PopupHost;
pub(super) use platform::hide_current_popup_window;

#[derive(Debug, Default)]
pub(super) struct PopupHosts {
    drag_preview: PopupHost,
    tooltip: PopupHost,
    command_palette: PopupHost,
}

impl PopupHosts {
    fn host_mut(&mut self, mode: PopupMode) -> &mut PopupHost {
        match mode {
            PopupMode::DragPreview => &mut self.drag_preview,
            PopupMode::Tooltip => &mut self.tooltip,
            PopupMode::CommandPalette => &mut self.command_palette,
        }
    }

    pub(super) fn shutdown(&mut self) {
        for mode in PopupMode::ALL {
            self.host_mut(mode).shutdown();
        }
    }

    #[cfg(not(test))]
    fn wait_until_ready(&mut self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        PopupMode::ALL.into_iter().all(|mode| {
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            self.host_mut(mode).wait_until_ready(deadline - now)
        })
    }
}

#[cfg(not(test))]
pub(super) fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).start_prewarm(mode)?;
    }
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).finish_prewarm();
    }
    if !hosts.wait_until_ready(std::time::Duration::from_secs(5)) {
        return Err("popup hosts did not initialize");
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).mode = Some(mode);
        hosts.host_mut(mode).ready = true;
    }
    Ok(())
}

#[cfg(not(test))]
pub(super) fn open_popup_host(
    hosts: &mut PopupHosts,
    mode: PopupMode,
) -> std::result::Result<(), &'static str> {
    hosts.host_mut(mode).open(mode)
}

#[cfg(test)]
pub(super) fn open_popup_host(
    hosts: &mut PopupHosts,
    mode: PopupMode,
) -> std::result::Result<(), &'static str> {
    hosts.host_mut(mode).mode = Some(mode);
    hosts.host_mut(mode).ready = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_hosts_prepare_all_modes_without_replacing_on_selection() {
        let mut hosts = PopupHosts::default();

        prepare_popup_hosts(&mut hosts).expect("test prewarm should succeed");

        assert_eq!(hosts.drag_preview.mode, Some(PopupMode::DragPreview));
        assert_eq!(hosts.tooltip.mode, Some(PopupMode::Tooltip));
        assert_eq!(hosts.command_palette.mode, Some(PopupMode::CommandPalette));
        assert!(hosts.drag_preview.ready);
        assert!(hosts.tooltip.ready);
        assert!(hosts.command_palette.ready);
    }
}
