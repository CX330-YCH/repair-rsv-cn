use std::{ffi::OsStr, process::Command};

pub(crate) fn background_command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Keep helper stdout/stderr pipes while suppressing a console window.
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}
