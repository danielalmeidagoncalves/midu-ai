//! Leaving the app, and showing the store in the file manager.
//!
//! Both are desktop concerns: a native window has no back button, and a folder
//! of Markdown is only useful if you can get to it.

mod native {
    use std::process::{Command, Stdio};

    /// Open the Markdown store in the platform's file manager. Returns the
    /// message to show in the footer, so failure is visible rather than silent.
    pub fn open_store_folder() -> Result<(), String> {
        let Some(dir) = crate::storage::store_path() else {
            return Err("The Markdown store is not open.".to_string());
        };
        let opened = on_macos(&dir) || on_linux(&dir);
        if opened {
            Ok(())
        } else {
            Err(format!("No file manager answered. The folder is {dir}"))
        }
    }

    #[cfg(target_os = "macos")]
    fn on_macos(dir: &str) -> bool {
        spawned(Command::new("open").arg(dir))
    }

    #[cfg(not(target_os = "macos"))]
    fn on_macos(_dir: &str) -> bool {
        false
    }

    #[cfg(target_os = "linux")]
    fn on_linux(dir: &str) -> bool {
        spawned(Command::new("xdg-open").arg(dir))
    }

    #[cfg(not(target_os = "linux"))]
    fn on_linux(_dir: &str) -> bool {
        false
    }

    #[allow(dead_code, reason = "only the platform-specific openers call it")]
    fn spawned(command: &mut Command) -> bool {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }

    /// Ask the platform for a folder, starting from `start`. `None` means the
    /// dialog was cancelled or there is no desktop session to draw it in.
    pub fn ask_folder(start: &str) -> Option<String> {
        rfd::FileDialog::new()
            .set_title("Choose the folder Horizon should live in")
            .set_directory(start)
            .pick_folder()
            .map(|path| path.display().to_string())
    }

    /// Close the window and, with it, the process.
    pub fn quit() {
        std::process::exit(0);
    }
}

pub use native::{ask_folder, open_store_folder, quit};
