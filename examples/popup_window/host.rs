//! Child-process popup host management for the popup example.

use super::model::{POPUP_ARG, POPUP_MODE_ARG, POPUP_PREWARM_ARG, PopupMode};
#[cfg(all(target_os = "windows", not(test)))]
use super::model::{POPUP_POSITION, POPUP_PREWARM_POSITION};
use std::process::Child;
#[cfg(all(target_os = "windows", not(test)))]
use std::process::Stdio;

#[path = "platform.rs"]
mod platform;

pub(super) use platform::hide_current_popup_window;

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
            if platform::show_popup_window(child_id, POPUP_POSITION, true) {
                return Ok(());
            }
            if platform::wait_for_popup_window(child_id, std::time::Duration::from_millis(250))
                && platform::show_popup_window(child_id, POPUP_POSITION, true)
            {
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
            platform::wait_for_popup_window(process_id, timeout)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = timeout;
            true
        }
    }
}

#[cfg(all(target_os = "windows", not(test)))]
fn finish_prewarm_child(child: &mut Child) {
    platform::wait_for_first_present_profile(child, std::time::Duration::from_secs(3));
    let process_id = child.id();
    let _ = platform::wait_for_hidden_popup_window(process_id, std::time::Duration::from_secs(2));
    prime_hidden_show_path(process_id);
}

#[cfg(all(not(target_os = "windows"), not(test)))]
fn finish_prewarm_child(_child: &mut Child) {}

#[cfg(all(target_os = "windows", not(test)))]
fn prime_hidden_show_path(process_id: u32) {
    for step in hidden_show_prime_steps() {
        if !prime_hidden_show_step(process_id, step) {
            return;
        }
    }
}

#[cfg(all(target_os = "windows", not(test)))]
fn prime_hidden_show_step(process_id: u32, step: PopupPrimeStep) -> bool {
    match step {
        PopupPrimeStep::Show { focus } => {
            platform::show_popup_window(process_id, POPUP_PREWARM_POSITION, focus)
        }
        PopupPrimeStep::WaitVisible => platform::wait_for_visible_popup_window(
            process_id,
            std::time::Duration::from_millis(250),
        ),
        PopupPrimeStep::Hide => platform::hide_popup_window(process_id),
        PopupPrimeStep::WaitHidden => platform::wait_for_hidden_popup_window(
            process_id,
            std::time::Duration::from_millis(250),
        ),
        PopupPrimeStep::MoveToRevealPosition => {
            platform::move_popup_window(process_id, POPUP_POSITION)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupPrimeStep {
    Show { focus: bool },
    WaitVisible,
    Hide,
    WaitHidden,
    MoveToRevealPosition,
}

fn hidden_show_prime_steps() -> [PopupPrimeStep; 9] {
    [
        PopupPrimeStep::Show { focus: false },
        PopupPrimeStep::WaitVisible,
        PopupPrimeStep::Hide,
        PopupPrimeStep::WaitHidden,
        PopupPrimeStep::Show { focus: true },
        PopupPrimeStep::WaitVisible,
        PopupPrimeStep::Hide,
        PopupPrimeStep::WaitHidden,
        PopupPrimeStep::MoveToRevealPosition,
    ]
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

#[cfg(not(test))]
fn spawn_popup_process(
    mode: PopupMode,
    prewarmed: bool,
) -> std::result::Result<Child, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "could not resolve current executable")?;
    let mut command = std::process::Command::new(executable);
    command.args(popup_process_args(mode, prewarmed));
    if prewarmed {
        command.env("RADIANT_NATIVE_STARTUP_PROFILE", "1");
        #[cfg(target_os = "windows")]
        command.stderr(Stdio::piped());
    }
    command.spawn().map_err(|_| "could not start popup process")
}

fn popup_process_args(mode: PopupMode, prewarmed: bool) -> Vec<&'static str> {
    let mut args = vec![POPUP_ARG, POPUP_MODE_ARG, mode.arg()];
    if prewarmed {
        args.push(POPUP_PREWARM_ARG);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_process_args_mark_prewarmed_hosts() {
        assert_eq!(
            popup_process_args(PopupMode::Tooltip, true),
            vec![
                POPUP_ARG,
                POPUP_MODE_ARG,
                PopupMode::Tooltip.arg(),
                POPUP_PREWARM_ARG
            ]
        );
        assert_eq!(
            popup_process_args(PopupMode::Tooltip, false),
            vec![POPUP_ARG, POPUP_MODE_ARG, PopupMode::Tooltip.arg()]
        );
    }

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

    #[test]
    fn hidden_show_prime_steps_include_focused_open_path() {
        assert_eq!(
            hidden_show_prime_steps(),
            [
                PopupPrimeStep::Show { focus: false },
                PopupPrimeStep::WaitVisible,
                PopupPrimeStep::Hide,
                PopupPrimeStep::WaitHidden,
                PopupPrimeStep::Show { focus: true },
                PopupPrimeStep::WaitVisible,
                PopupPrimeStep::Hide,
                PopupPrimeStep::WaitHidden,
                PopupPrimeStep::MoveToRevealPosition,
            ]
        );
    }
}
