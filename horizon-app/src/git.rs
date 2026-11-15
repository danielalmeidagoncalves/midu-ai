//! A read-only status reader over libgit2.
//!
//! Horizon does not run git: the owner commits, pushes and pulls by hand in
//! the terminal. This module answers one question — what state is the store
//! repository in right now: how many files differ from the last commit, and
//! how far the branch runs ahead of the remote. The remote half is read from
//! the remote-tracking ref, which the owner's own `git fetch` or `git pull`
//! refreshes, so the app never touches the network.
//!
//! The store folder *is* the repository. `state.md` is gitignored, so device
//! state never leaves the machine.

use std::path::Path;

use git2::{Repository, StatusOptions};

pub const BRANCH: &str = "main";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub changed: usize,
    pub ahead: usize,
    pub behind: usize,
    pub remote: Option<String>,
}

pub struct Git {
    repo: Repository,
}

impl Git {
    /// The repository, if the folder is one. The app never creates it: a
    /// store without a git repository reports "Not a repository" until the
    /// owner runs `git init`.
    pub fn open(dir: &Path) -> Option<Self> {
        Repository::open(dir).ok().map(|repo| Self { repo })
    }

    pub fn status(&self) -> Result<Status, String> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let changed = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(|e| e.message().to_string())?
            .len();

        let mut ahead = 0;
        let mut behind = 0;
        if let (Some(local), Some(remote)) = (self.head_oid(), self.remote_oid()) {
            if let Ok((a, b)) = self.repo.graph_ahead_behind(local, remote) {
                ahead = a;
                behind = b;
            }
        }

        Ok(Status {
            changed,
            ahead,
            behind,
            remote: self.remote_url(),
        })
    }

    pub fn remote_url(&self) -> Option<String> {
        self.repo
            .find_remote("origin")
            .ok()
            .and_then(|r| r.url().ok().map(str::to_string))
    }

    /// The tree of the current commit. It changes when the owner's own git
    /// operations rewrite the working tree — a pull, a checkout, a reset —
    /// so the watcher uses it to know when the folder must be re-read.
    pub fn head_tree(&self) -> Option<git2::Oid> {
        self.repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .map(|c| c.tree_id())
    }

    fn head_oid(&self) -> Option<git2::Oid> {
        self.repo.head().ok().and_then(|h| h.target())
    }

    fn remote_oid(&self) -> Option<git2::Oid> {
        self.repo
            .find_reference(&format!("refs/remotes/origin/{BRANCH}"))
            .ok()
            .and_then(|r| r.target())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("horizon-git-{tag}-{}-{stamp}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, path: &str, text: &str) {
        let target = dir.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }

    fn entry(text: &str) -> String {
        format!("---\nkind: \"entry\"\nversion: 2\nid: \"e-x\"\nday: \"2026-11-04\"\ntitle: \"{text}\"\n---\n")
    }

    /// A folder that is not a repository is reported as such: the watcher
    /// shows "Not a repository" until the owner runs `git init`.
    #[test]
    fn a_folder_without_a_repository_is_not_a_repository() {
        let root = temp_dir("norepo");
        assert!(Git::open(&root).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn status_counts_work_and_a_clean_tree_commits_nothing() {
        let root = temp_dir("status");
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap();
        let git = Git::open(&root).unwrap();
        assert_eq!(git.status().unwrap().changed, 0);
        write(&root, "entries/one.md", &entry("Call mum"));
        assert_eq!(git.status().unwrap().changed, 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
