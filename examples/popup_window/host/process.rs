//! Popup example child-process command construction.

use crate::model::{POPUP_ARG, POPUP_MODE_ARG, POPUP_PREWARM_ARG, PopupMode};

#[cfg(not(test))]
use std::process::Child;
#[cfg(all(target_os = "windows", not(test)))]
use std::process::Stdio;

#[cfg(not(test))]
pub(super) fn spawn_popup_process(
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
}
