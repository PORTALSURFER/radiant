//! Child-process popup host management for the popup example.

#[cfg(all(target_os = "windows", not(test)))]
use super::model::POPUP_POSITION;
use super::model::PopupMode;
use std::process::Child;

#[path = "platform.rs"]
mod platform;
#[path = "host/prewarm.rs"]
mod prewarm;
#[path = "host/process.rs"]
mod process;

pub(super) use platform::hide_current_popup_window;
#[cfg(not(test))]
use prewarm::finish_prewarm_child;
#[cfg(not(test))]
use process::spawn_popup_process;

#[derive(Debug, Default)]
pub(super) struct PopupHosts {
    drag_preview: PopupHost,
    tooltip: PopupHost,
    command_palette: PopupHost,
}

#[derive(Debug, Default)]
struct PopupHost {
    child: Option<Child>,
    mode: Option<PopupMode>,
    ready: bool,
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

impl PopupHost {
    #[cfg(not(test))]
    fn start_prewarm(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
        self.reap_finished_child();
        if self.mode == Some(mode) && self.child.is_some() {
            return Ok(());
        }

        self.shutdown();
        let child = spawn_popup_process(mode, true)?;
        self.mode = Some(mode);
        self.child = Some(child);
        self.ready = false;
        Ok(())
    }

    #[cfg(not(test))]
    fn finish_prewarm(&mut self) {
        if self.ready {
            return;
        }
        if let Some(child) = self.child.as_mut() {
            finish_prewarm_child(child);
            self.ready = true;
        }
    }

    #[cfg(not(test))]
    fn prepare(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
        self.start_prewarm(mode)?;
        self.finish_prewarm();
        Ok(())
    }

    #[cfg(not(test))]
    fn open(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
        self.prepare(mode)?;

        #[cfg(target_os = "windows")]
        {
            let child_id = self
                .child
                .as_ref()
                .map(Child::id)
                .ok_or("popup host is not running")?;
            if platform::show_popup_window(child_id, POPUP_POSITION, false) {
                focus_popup_after_reveal(child_id);
                return Ok(());
            }
            if platform::wait_for_popup_window(child_id, std::time::Duration::from_millis(250))
                && platform::show_popup_window(child_id, POPUP_POSITION, false)
            {
                focus_popup_after_reveal(child_id);
                return Ok(());
            }
        }

        self.shutdown();
        self.child = Some(spawn_popup_process(mode, false)?);
        self.mode = Some(mode);
        Ok(())
    }

    fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.mode = None;
        self.ready = false;
    }

    #[cfg(not(test))]
    fn reap_finished_child(&mut self) {
        let finished = self
            .child
            .as_mut()
            .and_then(|child| child.try_wait().ok())
            .flatten()
            .is_some();
        if finished {
            self.child = None;
            self.mode = None;
            self.ready = false;
        }
    }

    #[cfg(not(test))]
    fn wait_until_ready(&self, timeout: std::time::Duration) -> bool {
        let Some(process_id) = self.child.as_ref().map(Child::id) else {
            return false;
        };

        #[cfg(target_os = "windows")]
        {
            platform::wait_for_visible_popup_window(process_id, timeout)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = timeout;
            true
        }
    }
}

#[cfg(all(target_os = "windows", not(test)))]
fn focus_popup_after_reveal(process_id: u32) {
    std::thread::spawn(move || {
        let _ = platform::focus_popup_window(process_id);
    });
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
