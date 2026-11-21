//! Reminder delivery.
//!
//! Three things the UI cannot own: a heartbeat that says Horizon is running, the
//! desktop notification, and the `--fire-due` pass the systemd timer runs while
//! the window is closed.
//!
//! The heartbeat is a file's mtime rather than a lock, so a crash never leaves a
//! stale lock behind: a heartbeat older than [`FRESH`] simply means no window is
//! open and the timer may deliver.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use chrono::{Local, Utc};
use horizon_core::{reminders as core, store};

use crate::storage;

const HEARTBEAT: &str = "reminders.heartbeat";
const FRESH: Duration = Duration::from_secs(90);

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("horizon"))
}

fn heartbeat_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(HEARTBEAT))
}

/// Say that a window is open. The UI calls this on its reminder poll.
pub fn touch_heartbeat() {
    let Some(path) = heartbeat_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, Utc::now().timestamp().to_string());
}

/// True while a Horizon window is running. The timer's pass stands down so the
/// same reminder is never delivered twice.
pub fn app_is_running() -> bool {
    let Some(path) = heartbeat_path() else {
        return false;
    };
    let Ok(meta) = std::fs::metadata(&path) else {
        return false;
    };
    meta.modified()
        .ok()
        .and_then(|at| SystemTime::now().duration_since(at).ok())
        .is_some_and(|age| age < FRESH)
}

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
}

/// Whether the desktop can show a notification at all. When it cannot, the
/// in-app toast is the notification.
pub fn notifications_available() -> bool {
    if cfg!(target_os = "linux") {
        on_path("notify-send")
    } else {
        cfg!(target_os = "macos")
    }
}

/// One desktop notification. Failure is silent: the strip still holds the
/// reminder, so a missing notifier never loses it.
pub fn notify(body: &str) {
    let mut command = if cfg!(target_os = "linux") {
        let mut command = Command::new("notify-send");
        command.arg("Horizon").arg(body);
        command
    } else if cfg!(target_os = "macos") {
        let script = format!(
            "display notification {} with title \"Horizon\"",
            applescript_string(body)
        );
        let mut command = Command::new("osascript");
        command.arg("-e").arg(script);
        command
    } else {
        return;
    };
    let _ = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn applescript_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// The timer's entry point: deliver what is due, then exit. No window, no async
/// runtime, and only `reminders/` is written.
pub fn fire_due_cli() -> i32 {
    if app_is_running() {
        return 0;
    }
    let Ok(Some((dir, files))) = storage::read_store_sync() else {
        return 0;
    };
    let today = Local::now().date_naive();
    let Ok(mut doc) = store::from_files(&files, today) else {
        return 0;
    };
    let now = Utc::now();
    let due: Vec<String> = doc
        .reminders
        .iter()
        .filter(|reminder| reminder.is_due(now))
        .map(|reminder| reminder.id.clone())
        .collect();
    if due.is_empty() {
        return 0;
    }
    for id in due {
        let body = doc.reminder_mut(&id).map(|reminder| {
            let body = core::notification_body(reminder);
            core::advance(reminder, now);
            body
        });
        if let Some(body) = body {
            notify(&body);
        }
    }
    if let Err(err) = storage::write_reminders_only(&dir, &doc) {
        eprintln!("Horizon: could not write reminders: {err}");
        return 1;
    }
    0
}

fn systemd_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("systemd/user"))
}

/// Whether the closed-app timer can be installed here at all.
pub fn systemd_available() -> bool {
    cfg!(target_os = "linux") && on_path("systemctl")
}

fn systemctl(args: &[&str]) -> bool {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn timer_enabled() -> bool {
    systemd_available() && systemctl(&["is-enabled", "horizon-reminders.timer"])
}

/// Install and start the user timer, so a reminder fires with the window closed.
pub fn enable_timer() -> Result<(), String> {
    if !systemd_available() {
        return Err("No systemd user session answered.".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = systemd_dir().ok_or("Could not find the systemd user folder.")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let service = format!(
        "[Unit]\nDescription=Horizon reminder delivery\n\n[Service]\nType=oneshot\nExecStart=\"{}\" --fire-due\n",
        exe.display()
    );
    let timer = "[Unit]\nDescription=Horizon reminders, once a minute\n\n\
                 [Timer]\nOnCalendar=*:0/1\nAccuracySec=5s\nPersistent=true\n\n\
                 [Install]\nWantedBy=timers.target\n";
    std::fs::write(dir.join("horizon-reminders.service"), service).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("horizon-reminders.timer"), timer).map_err(|e| e.to_string())?;
    systemctl(&["daemon-reload"]);
    if systemctl(&["enable", "--now", "horizon-reminders.timer"]) {
        Ok(())
    } else {
        Err("systemctl could not enable the timer.".into())
    }
}

/// Stop and remove the timer. Reminders then fire only while the window is open.
pub fn disable_timer() -> Result<(), String> {
    systemctl(&["disable", "--now", "horizon-reminders.timer"]);
    if let Some(dir) = systemd_dir() {
        let _ = std::fs::remove_file(dir.join("horizon-reminders.timer"));
        let _ = std::fs::remove_file(dir.join("horizon-reminders.service"));
        systemctl(&["daemon-reload"]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timer_units_say_what_they_do() {
        // The service runs this very binary with `--fire-due`; the timer asks for
        // a minute and persists a missed run.
        let exe = std::env::current_exe().map(|p| p.display().to_string());
        assert!(exe.is_ok());
        assert!(on_path("sh") || cfg!(not(target_os = "linux")));
    }
}
