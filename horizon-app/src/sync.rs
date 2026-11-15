//! A read-only watcher that reports the state of the store's git repository.
//!
//! The owner runs git by hand: commits, pushes and pulls in the terminal.
//! Horizon never touches git itself. One thread polls the repository every
//! few seconds and publishes a snapshot the brief can render. When the owner
//! pulls and the working tree changes, the watcher bumps a revision so the
//! UI reloads the document before the next save can overwrite the new files.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crate::git::Git;
use crate::storage;

/// The watcher polls the repository locally; the network is never touched.
const POLL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncState {
    pub configured: bool,
    pub remote: Option<String>,
    pub changed: usize,
    pub ahead: usize,
    pub behind: usize,
    /// True when the store folder is not a git repository at all.
    pub no_repo: bool,
    /// Bumped when the working tree changed under the app (a manual pull).
    /// The UI reloads the document when this changes, so memory never
    /// drifts from the folder.
    pub revision: u64,
    /// The folder on this device that holds the Markdown files.
    pub store: String,
}

pub struct Service {
    tx: Mutex<Sender<()>>,
    state: Arc<Mutex<SyncState>>,
}

static SERVICE: OnceLock<Service> = OnceLock::new();

/// Start the one watcher thread. Safe to call more than once; later calls are no-ops.
pub fn start() {
    if SERVICE.get().is_some() {
        return;
    }
    let Some(dir) = storage::store_dir() else {
        return;
    };
    let (tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(SyncState::default()));
    let worker_state = Arc::clone(&state);
    let spawned = std::thread::Builder::new()
        .name("horizon-sync".into())
        .spawn(move || run(dir, rx, worker_state));
    if spawned.is_ok() {
        let _ = SERVICE.set(Service {
            tx: Mutex::new(tx),
            state,
        });
    }
}

pub fn handle() -> Option<&'static Service> {
    SERVICE.get()
}

impl Service {
    /// The folder changed on disk. The watcher looks at once instead of
    /// waiting for the next poll.
    pub fn touch(&self) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(());
        }
    }

    pub fn snapshot(&self) -> SyncState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

fn update(state: &Arc<Mutex<SyncState>>, f: impl FnOnce(&mut SyncState)) {
    if let Ok(mut state) = state.lock() {
        f(&mut state);
    }
}

fn run(dir: std::path::PathBuf, rx: Receiver<()>, state: Arc<Mutex<SyncState>>) {
    let git = Git::open(&dir);
    let dir = dir.display().to_string();
    update(&state, |s| {
        s.store = dir.clone();
        s.remote = git.as_ref().and_then(|g| g.remote_url());
        s.configured = s.remote.is_some();
        s.no_repo = git.is_none();
    });
    let mut seen_tree = git.as_ref().and_then(|g| g.head_tree());
    poll(&git, &state, &mut seen_tree);

    // A touch wakes the poll early; otherwise the poll runs on its own
    // two-second cadence. Errors are ignored: the watcher only reports.
    while let Ok(()) | Err(RecvTimeoutError::Timeout) = rx.recv_timeout(POLL) {
        poll(&git, &state, &mut seen_tree);
    }
}

fn poll(git: &Option<Git>, state: &Arc<Mutex<SyncState>>, seen_tree: &mut Option<git2::Oid>) {
    let Some(git) = git else {
        return;
    };
    let Ok(status) = git.status() else {
        return;
    };
    // A manual pull, checkout or reset rewrites the tree. Bump the revision
    // so the UI reloads the document before its next save can overwrite it.
    let tree = git.head_tree();
    if tree != *seen_tree {
        *seen_tree = tree;
        update(state, |s| s.revision = s.revision.wrapping_add(1));
    }
    update(state, |s| {
        s.changed = status.changed;
        s.ahead = status.ahead;
        s.behind = status.behind;
        s.remote = status.remote.clone();
        s.configured = status.remote.is_some();
    });
}

/// What the brief says about storage and git. One place decides the words,
/// so the strip and the brief cannot disagree.
#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub label: String,
    /// One sentence for the `title` attribute: what the label means right now.
    pub detail: String,
    /// `calm` (nothing to do), `wait` (a step is pending), `alert` (something is wrong).
    pub tone: &'static str,
}

/// `changed` counts files that differ from the last commit, so it covers both
/// a second-old write and a minute-old one. `behind` wins over `ahead`: reading
/// the other device is the step that needs attention.
pub fn status(state: &SyncState) -> Status {
    let wait = |label: String, detail: &str| Status {
        label,
        detail: detail.to_string(),
        tone: "wait",
    };
    if state.no_repo {
        return Status {
            label: "Not a repository".into(),
            detail: "The store folder is not a git repository yet — run `git init` in it to keep versions."
                .into(),
            tone: "alert",
        };
    }
    if state.changed > 0 {
        return wait(
            format!("{} uncommitted", state.changed),
            "Saved to disk. Commit them in the store folder when the writing is at a rest.",
        );
    }
    if state.behind > 0 {
        return wait(
            format!("{} to take", state.behind),
            "The remote has versions this device has not read yet — run `git pull`.",
        );
    }
    if state.ahead > 0 {
        return wait(
            format!("{} to send", state.ahead),
            "This device has versions the remote does not have yet — run `git push`.",
        );
    }
    if !state.configured {
        return Status {
            label: "No remote".into(),
            detail: "Every entry is a Markdown file in the store folder. Add a remote with `git remote add origin …` to keep a copy elsewhere."
                .into(),
            tone: "calm",
        };
    }
    Status {
        label: "In sync".into(),
        detail: "Every entry is committed and the remote matches this device.".into(),
        tone: "calm",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> SyncState {
        SyncState {
            configured: true,
            ..SyncState::default()
        }
    }

    #[test]
    fn no_repository_says_so() {
        let s = status(&SyncState {
            no_repo: true,
            ..SyncState::default()
        });
        assert_eq!(s.label, "Not a repository");
        assert_eq!(s.tone, "alert");
        assert!(s.detail.contains("git init"));
    }

    #[test]
    fn clean_and_configured_says_in_sync() {
        let s = status(&state());
        assert_eq!(s.label, "In sync");
        assert!(s.detail.contains("remote matches"));
    }

    #[test]
    fn uncommitted_files_are_named() {
        let s = status(&SyncState {
            changed: 3,
            ..state()
        });
        assert_eq!(s.label, "3 uncommitted");
        assert_eq!(s.tone, "wait");
    }

    #[test]
    fn unpushed_commits_are_named() {
        let s = status(&SyncState {
            ahead: 2,
            ..state()
        });
        assert_eq!(s.label, "2 to send");
        assert!(s.detail.contains("git push"));
    }

    #[test]
    fn taking_the_remote_wins_over_sending() {
        let s = status(&SyncState {
            ahead: 2,
            behind: 1,
            ..state()
        });
        assert_eq!(s.label, "1 to take");
        assert!(s.detail.contains("git pull"));
    }

    #[test]
    fn no_remote_says_how_to_add_one() {
        let s = status(&SyncState::default());
        assert_eq!(s.label, "No remote");
        assert!(s.detail.contains("git remote add"));
    }
}
