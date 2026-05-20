//! Single child-process popup host lifecycle.

#[cfg(all(target_os = "windows", not(test)))]
use super::super::model::POPUP_POSITION;
#[cfg(all(target_os = "windows", not(test)))]
use super::platform;
#[cfg(not(test))]
use super::{prewarm, process};
use crate::model::PopupMode;
use std::process::Child;

#[derive(Debug, Default)]
pub(super) struct PopupHost {
    pub(super) child: Option<Child>,
    pub(super) mode: Option<PopupMode>,
    pub(super) ready: bool,
}

impl PopupHost {
    #[cfg(not(test))]
    pub(super) fn start_prewarm(
        &mut self,
        mode: PopupMode,
    ) -> std::result::Result<(), &'static str> {
        self.reap_finished_child();
        if self.mode == Some(mode) && self.child.is_some() {
            return Ok(());
        }

        self.shutdown();
        let child = process::spawn_popup_process(mode, true)?;
        self.mode = Some(mode);
        self.child = Some(child);
        self.ready = false;
        Ok(())
    }

    #[cfg(not(test))]
    pub(super) fn finish_prewarm(&mut self) {
        if self.ready {
            return;
        }
        if let Some(child) = self.child.as_mut() {
            prewarm::finish_prewarm_child(child);
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
    pub(super) fn open(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
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
        self.child = Some(process::spawn_popup_process(mode, false)?);
        self.mode = Some(mode);
        Ok(())
    }

    pub(super) fn shutdown(&mut self) {
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
    pub(super) fn wait_until_ready(&self, timeout: std::time::Duration) -> bool {
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
