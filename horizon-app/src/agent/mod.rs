//! The agent: a conversation with an agentic harness beside the quarter.
//!
//! Horizon's store is a folder of Markdown, so an agent does not need an API
//! between it and the notes: it needs a working directory, a manual, and a
//! way to talk. This module owns the second and third.
//!
//! - The **service** is one long-lived child process per app run, owned by two
//!   threads (a JSONL reader and a stderr reader). The UI only reads a cloned
//!   snapshot, exactly like the sync watcher.
//! - The **manual** is a plain Markdown file in the app's config folder, passed
//!   to the harness with `--append-system-prompt`. It never enters the vault,
//!   so the store stays the owner's writing and the manual can be regenerated.
//! - The **transcript** is appended to a JSONL file beside the manual, so the
//!   conversation survives a restart even though the harness owns the session.
//! - After a run settles, the app reconciles the folder with memory
//!   (`storage::reconcile`), so the agent's writing appears without a commit.
//!
//! Only Pi is implemented. `AgentConfig::harness` is a plain id and the Pi
//! adapter is one module (`pi`), so a second harness is a new module plus one
//! match arm — not a second agent architecture.

pub mod pi;

use std::path::PathBuf;
use std::process::{Child, ChildStdin};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use horizon_core::{dates::Quarter, Doc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::storage;

/// How many transcript items survive a restart. A conversation is a reading
/// surface, not an archive; Pi keeps the session that matters.
const TRANSCRIPT_LIMIT: usize = 400;

/// The id written to `config.toml` and the only harness this build speaks.
pub const PI: &str = "pi";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    /// `pi` or `off`. A plain id, so a second harness does not change this shape.
    pub harness: String,
    /// The executable. `pi` on PATH by default.
    pub command: String,
    pub provider: String,
    /// Empty means "whatever Pi is configured to use".
    pub model: String,
    /// Empty means Pi's own default thinking level.
    pub thinking: String,
    /// Send a short description of what is on screen with every message.
    pub context: bool,
    /// Let the agent change the store. Off exposes a read-only tool set.
    pub write: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            harness: PI.into(),
            command: "pi".into(),
            provider: String::new(),
            model: String::new(),
            thinking: String::new(),
            context: true,
            write: true,
        }
    }
}

impl AgentConfig {
    /// Read the agent keys from the config file's flat `key = value` map. An
    /// absent file is Pi with its own defaults — the feature is on, the way the
    /// app's other local features are.
    pub fn from_settings(settings: &std::collections::BTreeMap<String, String>) -> Self {
        let default = Self::default();
        let text = |key: &str| {
            settings
                .get(key)
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let flag = |key: &str, fallback: bool| match settings.get(key).map(String::as_str) {
            Some("false") | Some("off") | Some("no") => false,
            Some(_) => true,
            None => fallback,
        };
        let harness = text("agent");
        Self {
            harness: if harness.is_empty() {
                default.harness
            } else {
                harness
            },
            command: {
                let command = text("agent.command");
                if command.is_empty() {
                    default.command
                } else {
                    command
                }
            },
            provider: text("agent.provider"),
            model: text("agent.model"),
            thinking: text("agent.thinking"),
            context: flag("agent.context", true),
            write: flag("agent.write", true),
        }
    }

    pub fn enabled(&self) -> bool {
        self.harness != "off" && !self.command.trim().is_empty()
    }

    /// The keys the brief's Agent block writes back.
    pub fn settings(&self) -> Vec<(&'static str, String)> {
        vec![
            ("agent", self.harness.clone()),
            ("agent.command", self.command.clone()),
            ("agent.provider", self.provider.clone()),
            ("agent.model", self.model.clone()),
            ("agent.thinking", self.thinking.clone()),
            ("agent.context", self.context.to_string()),
            ("agent.write", self.write.to_string()),
        ]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Configured, not started. The first message starts the harness.
    #[default]
    Idle,
    Starting,
    Ready,
    Working,
    /// The process exited (or was never there) and nothing is running.
    Stopped,
    Failed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Idle => "ready",
            Status::Starting => "starting",
            Status::Ready => "ready",
            Status::Working => "working",
            Status::Stopped => "stopped",
            Status::Failed => "error",
        }
    }
}

/// One line of the conversation. Tool calls are items of their own, not prose:
/// the transcript should say what happened to the files, not only what the model
/// said about it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatItem {
    User {
        text: String,
        #[serde(default)]
        context: bool,
    },
    Assistant {
        text: String,
    },
    Thinking {
        text: String,
    },
    Tool {
        #[serde(default)]
        id: String,
        name: String,
        detail: String,
        #[serde(default)]
        out: String,
        #[serde(default)]
        done: bool,
        #[serde(default)]
        failed: bool,
    },
    Notice {
        text: String,
        #[serde(default)]
        error: bool,
    },
}

/// What the UI draws. Cloned out of the service on a timer, like `SyncState`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentView {
    pub status: Status,
    /// A sentence for the status line and the title attribute.
    pub detail: String,
    pub model: String,
    pub items: Vec<ChatItem>,
    pub streaming: String,
    pub thinking: String,
    /// True between a prompt and `agent_settled`.
    pub busy: bool,
    /// A run ended but the app has not merged the folder back into memory yet.
    /// The saver stays paused for this window too: a save now would treat the
    /// agent's new files as stale and delete them.
    pub settling: bool,
    /// Bumped on every settled run; the app reconciles the folder when it moves.
    pub finished: u64,
    /// Whether the brief has the agent switched on.
    pub enabled: bool,
}

impl AgentView {
    fn push(&mut self, item: ChatItem) {
        if self.items.len() >= TRANSCRIPT_LIMIT {
            self.items.remove(0);
        }
        self.items.push(item);
    }

    fn flush_stream(&mut self) {
        let text = std::mem::take(&mut self.streaming);
        if !text.trim().is_empty() {
            self.push(ChatItem::Assistant { text });
        }
    }

    fn flush_thinking(&mut self) {
        let text = std::mem::take(&mut self.thinking);
        if !text.trim().is_empty() {
            self.push(ChatItem::Thinking { text });
        }
    }

    /// The model label is provider-qualified only when the provider answered.
    pub fn model_label(&self) -> String {
        if self.model.is_empty() {
            "model not reported".into()
        } else {
            self.model.clone()
        }
    }
}

struct Io {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

pub struct Service {
    state: Mutex<AgentView>,
    io: Mutex<Io>,
    config: Mutex<AgentConfig>,
    cwd: Mutex<PathBuf>,
    /// The harness's last diagnostic line; it explains a crash better than the
    /// exit status does.
    stderr: Mutex<String>,
}

static SERVICE: OnceLock<Service> = OnceLock::new();

/// Load the transcript once, before the first render, so the agent column shows
/// the conversation immediately instead of after the harness starts.
pub fn init() {
    if let Some(service) = SERVICE.get() {
        // The webview reloads the page after the vault is chosen and the Rust
        // statics survive that reload, so the folder and the settings are re-read
        // here. A folder change closes any harness started against the old one.
        let cwd = storage::store_dir().unwrap_or_default();
        let moved = service
            .cwd
            .lock()
            .map(|current| *current != cwd)
            .unwrap_or(false);
        if moved {
            service.stop();
            if let Ok(mut current) = service.cwd.lock() {
                *current = cwd;
            }
        }
        let config = storage::agent_config();
        let enabled = config.enabled();
        if let Ok(mut current) = service.config.lock() {
            *current = config;
        }
        if let Ok(mut state) = service.state.lock() {
            state.enabled = enabled;
        }
        return;
    }
    let _ = SERVICE.get_or_init(|| {
        let items = load_transcript();
        Service {
            state: Mutex::new(AgentView {
                items,
                status: Status::Idle,
                detail: "The first message starts Pi.".into(),
                enabled: storage::agent_config().enabled(),
                ..AgentView::default()
            }),
            io: Mutex::new(Io {
                child: None,
                stdin: None,
            }),
            config: Mutex::new(storage::agent_config()),
            cwd: Mutex::new(storage::store_dir().unwrap_or_default()),
            stderr: Mutex::new(String::new()),
        }
    });
}

fn service() -> Option<&'static Service> {
    SERVICE.get()
}

pub fn snapshot() -> Option<AgentView> {
    service().map(|s| s.state.lock().map(|v| v.clone()).unwrap_or_default())
}

/// Whether the folder is owned by the agent right now — a run in flight, or a
/// run that ended and has not been reconciled. The saver asks this live, not the
/// polled snapshot: the race this closes is milliseconds wide.
pub fn busy() -> bool {
    service()
        .and_then(|s| s.state.lock().ok().map(|v| v.busy || v.settling))
        .unwrap_or(false)
}

/// The app has merged the folder. The saver may write again.
pub fn reconciled() {
    if let Some(service) = service() {
        if let Ok(mut state) = service.state.lock() {
            state.settling = false;
        }
    }
}

/// Send a message.
pub fn restart(config: AgentConfig) {
    let Some(service) = service() else {
        return;
    };
    service.stop();
    if let Ok(mut current) = service.config.lock() {
        *current = config;
    }
    if let Ok(mut cwd) = service.cwd.lock() {
        *cwd = storage::store_dir().unwrap_or_default();
    }
    if let Ok(mut state) = service.state.lock() {
        state.status = Status::Idle;
        state.detail = "The first message starts Pi.".into();
        state.enabled = storage::agent_config().enabled();
    }
}

/// Send a message. `context` is the current view block, already built by the
/// caller; the harness's own configuration decides whether it is attached.
pub fn send(text: &str, context: Option<String>) -> Result<(), String> {
    let service = service().ok_or("The agent is not configured yet.")?;
    service.send(text, context)
}

pub fn abort() {
    if let Some(service) = service() {
        let _ = service.write(json!({ "type": "abort" }));
    }
}

/// Clear the conversation and ask the harness for a fresh session. The vault is
/// untouched: a new conversation is about the talking, not the notes.
pub fn new_conversation() -> Result<(), String> {
    let service = service().ok_or("The agent is not configured yet.")?;
    let running = service
        .io
        .lock()
        .map(|io| io.child.is_some())
        .unwrap_or(false);
    // A new conversation needs a live harness to forget the old session. If it
    // is not running, start it first; the model is not called until a prompt.
    let started = if running {
        Ok(())
    } else {
        service.ensure_started()
    };
    if running || started.is_ok() {
        service.write(json!({ "type": "new_session" }))?;
    }
    if let Ok(mut state) = service.state.lock() {
        state.items.clear();
        state.streaming.clear();
        state.thinking.clear();
        state.busy = false;
        state.status = if running || started.is_ok() {
            Status::Ready
        } else {
            Status::Idle
        };
        state.detail = "A new conversation. The notes are still here.".into();
        save_transcript(&state.items);
    }
    started
}

/// Close the child's stdin (an orderly shutdown), then give it a moment before
/// killing it. Called when Horizon quits so no harness outlives the window.
pub fn shutdown() {
    let Some(service) = service() else {
        return;
    };
    service.stop();
}

pub fn check(command: &str) -> Result<String, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("No command to check.".into());
    }
    let output = std::process::Command::new(command)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("Could not run `{command} --version`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{command} --version` exited with {}.",
            output.status
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Ok(format!("{command} answered."))
    } else {
        Ok(text)
    }
}

impl Service {
    fn ensure_started(&'static self) -> Result<(), String> {
        {
            let io = self
                .io
                .lock()
                .map_err(|_| "The agent's process lock is poisoned.")?;
            if io.child.is_some() {
                return Ok(());
            }
        }
        let config = self
            .config
            .lock()
            .map(|c| c.clone())
            .map_err(|_| "The agent's configuration is unreadable.")?;
        if !config.enabled() {
            return Err("The agent is switched off in the brief.".into());
        }
        let cwd = self
            .cwd
            .lock()
            .map(|c| c.clone())
            .map_err(|_| "The agent's folder is unreadable.")?;
        if !cwd.is_dir() {
            return Err("The store folder is not open, so the agent has nowhere to read.".into());
        }
        set_status(self, Status::Starting, "Starting the harness…");

        let knowledge = write_manual(&config, &cwd);
        let mut child = pi::spawn(&config, &cwd, knowledge.as_deref())?;
        let stdout = child
            .stdout
            .take()
            .ok_or("The harness gave no output stream.")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("The harness gave no error stream.")?;
        let stdin = child.stdin.take();

        {
            let mut io = self
                .io
                .lock()
                .map_err(|_| "The agent's process lock is poisoned.")?;
            io.child = Some(child);
            io.stdin = stdin;
        }

        let service: &'static Service = self;
        let _ = std::thread::Builder::new()
            .name("horizon-agent".into())
            .spawn(move || pi::read_stdout(service, stdout));
        let _ = std::thread::Builder::new()
            .name("horizon-agent-err".into())
            .spawn(move || pi::read_stderr(service, stderr));

        // What model did Pi choose? The answer is a normal RPC response.
        let _ = self.write(json!({ "id": "horizon-state", "type": "get_state" }));
        Ok(())
    }

    fn stop(&self) {
        let mut io = match self.io.lock() {
            Ok(io) => io,
            Err(_) => return,
        };
        // Dropping stdin is the protocol's orderly shutdown.
        io.stdin = None;
        if let Some(mut child) = io.child.take() {
            let deadline = Instant::now() + Duration::from_millis(1200);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(40));
                    }
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
        if let Ok(mut state) = self.state.lock() {
            state.busy = false;
            state.streaming.clear();
            state.thinking.clear();
            if matches!(
                state.status,
                Status::Starting | Status::Ready | Status::Working
            ) {
                state.status = Status::Stopped;
                state.detail =
                    "The harness is not running. Send a message to start it again.".into();
            }
        }
    }

    pub(crate) fn write(&self, value: serde_json::Value) -> Result<(), String> {
        use std::io::Write;
        let mut io = self
            .io
            .lock()
            .map_err(|_| "The agent's process lock is poisoned.".to_string())?;
        let stdin = io
            .stdin
            .as_mut()
            .ok_or("The harness is not running.".to_string())?;
        let line = serde_json::to_string(&value).map_err(|e| e.to_string())?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .map_err(|e| {
                set_status(
                    self,
                    Status::Failed,
                    &format!("The harness stopped listening: {e}"),
                );
                e.to_string()
            })
    }

    fn send(&'static self, text: &str, context: Option<String>) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        self.ensure_started()?;
        let config = self.config.lock().map(|c| c.clone()).unwrap_or_default();
        let include_context = config.context && context.is_some();
        let message = match (include_context, context) {
            (true, Some(context)) => {
                format!("<horizon-context>\n{context}\n</horizon-context>\n\n{text}")
            }
            _ => text.to_string(),
        };
        let busy = self.state.lock().map(|s| s.busy).unwrap_or(false);
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "The agent's transcript is unreadable.".to_string())?;
            state.push(ChatItem::User {
                text: text.to_string(),
                context: include_context,
            });
            state.streaming.clear();
            state.thinking.clear();
            state.busy = true;
            state.status = Status::Working;
            state.detail = "Pi is reading and writing.".into();
            save_transcript(&state.items);
        }
        let mut command = json!({
            "id": format!("prompt-{}", now_millis()),
            "type": "prompt",
            "message": message
        });
        if busy {
            // A message while the agent works is delivered after the current
            // turn's tools — a steer, not a second run in parallel.
            command["streamingBehavior"] = json!("steer");
        }
        self.write(command)
    }
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub(crate) fn set_status(service: &Service, status: Status, detail: &str) {
    if let Ok(mut state) = service.state.lock() {
        state.status = status;
        state.detail = detail.to_string();
    }
}

pub(crate) fn notice(service: &Service, text: impl Into<String>, error: bool) {
    if let Ok(mut state) = service.state.lock() {
        state.push(ChatItem::Notice {
            text: text.into(),
            error,
        });
        save_transcript(&state.items);
    }
}

pub(crate) fn fail(service: &Service, message: impl Into<String>) {
    let message = message.into();
    if let Ok(mut state) = service.state.lock() {
        state.status = Status::Failed;
        state.detail = message.clone();
        // A failure during a run is still a run that may have written files.
        if state.busy {
            state.busy = false;
            state.settling = true;
            state.finished = state.finished.wrapping_add(1);
        }
        state.push(ChatItem::Notice {
            text: message,
            error: true,
        });
        save_transcript(&state.items);
    }
}

/// The reader thread's single outcome: either the view changed, or a run settled
/// (which the app turns into a folder reconcile).
pub(crate) fn settle(service: &Service) {
    if let Ok(mut state) = service.state.lock() {
        state.flush_stream();
        state.flush_thinking();
        state.busy = false;
        state.settling = true;
        if !matches!(state.status, Status::Failed | Status::Stopped) {
            state.status = Status::Ready;
            state.detail = "Ready.".into();
        }
        state.finished = state.finished.wrapping_add(1);
        save_transcript(&state.items);
    }
}

pub(crate) fn saw_exit(service: &Service, code: Option<i32>) {
    if let Ok(mut state) = service.state.lock() {
        if matches!(state.status, Status::Failed) {
            return;
        }
        let was_running = state.busy;
        state.busy = false;
        state.status = Status::Stopped;
        state.detail = match code {
            Some(0) => "The harness closed its session.".into(),
            Some(code) => format!("The harness exited with status {code}."),
            None => "The harness exited.".into(),
        };
        if was_running {
            state.settling = true;
            state.finished = state.finished.wrapping_add(1);
        }
        save_transcript(&state.items);
    }
}

fn transcript_file() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("horizon").join("agent");
    // One conversation per vault: the agent's session is per working directory,
    // and a transcript from another vault would be a different conversation.
    let vault = storage::store_dir()?.display().to_string();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in vault.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(dir.join(format!("transcript-{hash:016x}.jsonl")))
}

fn load_transcript() -> Vec<ChatItem> {
    let Some(file) = transcript_file() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<ChatItem>(line).ok())
        .collect()
}

pub(crate) fn save_transcript(items: &[ChatItem]) {
    let Some(file) = transcript_file() else {
        return;
    };
    let Some(parent) = file.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let start = items.len().saturating_sub(TRANSCRIPT_LIMIT);
    let mut text = String::new();
    for item in &items[start..] {
        if let Ok(line) = serde_json::to_string(item) {
            text.push_str(&line);
            text.push('\n');
        }
    }
    let temp = file.with_extension("jsonl.tmp");
    if std::fs::write(&temp, text).is_ok() {
        let _ = std::fs::rename(&temp, &file);
    }
}

/// The manual travels beside the transcript, never inside the vault: the store
/// is the owner's writing, and a git status that names an app-generated file
/// would be a lie about what changed.
fn manual_file() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("horizon").join("agent").join("horizon-agent.md"))
}

fn write_manual(config: &AgentConfig, cwd: &std::path::Path) -> Option<PathBuf> {
    let file = manual_file()?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let text = manual(config, cwd);
    if std::fs::write(&file, text).is_err() {
        return None;
    }
    Some(file)
}

/// The harness's operating manual. It is deliberately short enough to live in
/// the system prompt every turn, and it points at the files themselves for the
/// rest: the vault is the truth, not this document.
fn manual(config: &AgentConfig, cwd: &std::path::Path) -> String {
    let mode = if config.write {
        "You may add, edit and delete files in the store when the user asks for it. Write carefully and then stop; Horizon reloads the folder after your run."
    } else {
        "You are read-only in this session: the write, edit and shell tools are not available. Answer, draft and point at the exact files instead of changing them."
    };
    format!(
        r#"# Horizon — the agent's manual

You are the agent inside **Horizon**, the user's personal life OS. The user is talking to you
from a panel beside a ninety-day wall of marks. Horizon is local-first: the store is a folder of
plain Markdown files, there is no server or database, and the app is just another client of that
folder.

The store for this conversation is the current working directory:

    {cwd}

{mode}

## Golden rules

1. Read before you write. Every entry, note and day page is a real file; `ls`, `rg` and `read`
   are the whole API.
2. Never touch `state.md` (device UI state), `README.md`, `LICENSE`, `assets/` or `conflicts/`.
3. Never rename or delete an existing store file unless removing that thing is the goal. Names
   are stable by design. (If a title changes, edit the `title:` field and let the app re-file it.)
4. A malformed file makes the whole store fail to load — the app shows a boot error instead of
   skipping it. Follow the exact frontmatter order and quoting below, or do not write.
5. Nothing in Horizon may ask to be tidied. Never propose tags, folders, priorities, streaks,
   reviews, reminders or completion percentages. Marks and subjects are inferred from the words;
   a blank day is a legal state.
6. Do not run git commands unless the user asks. Horizon reconciles the folder after your run.
   The owner handles commits and sync.
7. Speak like the app looks: concise, editorial, no emoji, no dashboard language. Markdown is
   welcome; short paragraphs and small lists beat tables of scores.
8. Never claim a change you did not make. If you wrote files, name them.

## The layout

    <vault>/
    ├── README.md            # explains the folder; never touch
    ├── state.md             # device UI state; gitignored; NEVER touch
    ├── entries/<day>-<slug>-<shortid>.md
    ├── notes/<slug>-<shortid>.md
    └── days/<day>.md

`<day>` is `YYYY-MM-DD`. The wall shows `today .. today+89`; older and later files still exist but
are not drawn. Read order is `created_at` then id, never the filename.

## File shape

Frontmatter, a blank line, then a Markdown body. The opening line is `---`, the closing line is
`---`, one `key: value` per line, string values JSON-quoted, `null` and booleans unquoted. LF
endings, one trailing newline. Body = the writing; structured fields never appear in the body.

Entry — `entries/<day>-<slug>-<shortid>.md`:

```markdown
---
kind: "entry"
version: 2
id: "e-<uuid-v4>"
day: "2026-11-14"
title: "Mum's birthday"
mark: "people"
subject: "family"
weight: 2
done: false
mark_auto: true
subject_auto: true
repeat_weekly: false
created_at: "2026-11-01T09:30:00Z"
---

- [ ] bring the cake
```

`weight` is 1 ordinary · 2 full day · 3 landmark. If `mark_auto`/`subject_auto` is true, the app
re-infers that field from the title on every read, ignoring the stored value. To pin a mark or
subject by hand, set the field **and** its `*_auto: false`.

Loose (undated) or clipped note — `notes/<slug>-<shortid>.md`:

```markdown
---
kind: "note"
version: 2
id: "n-<uuid-v4>"
created_at: "2026-11-01T09:30:00Z"
mark: "note"
subject: "me"
day: null
---

Ask the bond consultant about January rates
```

`day: null` = in the margin (undated). `day: "YYYY-MM-DD"` = clipped to that day. The body is the
note text; the first line is what the margin shows.

Day page — `days/<YYYY-MM-DD>.md`:

```markdown
---
kind: "day"
version: 2
day: "2026-11-14"
first_written: "2026-11-01"
---

# Mum's birthday

Four sentences about the day.
```

`first_written` is the day the page was first written. Never let the body become empty — an empty
page is no file at all. A day page adds no ink weight.

Naming: `<slug>` is lowercase ASCII alphanumerics and dashes, other runs collapsed to one dash,
40 characters at most, fallback `entry`. `<shortid>` is the first 8 characters of the UUID's last
group (the 12-hex tail after the final dash). No sequence numbers, ever.

## The vocabulary

Eighteen marks: `travel health people money work home ceremony admin study body food gift place
call deadline rest note thing`. Seventeen subjects: `ai assets backpack everyone bosch sketches
family finances house journalling kids me payments prompts retro tech travels`. If you are not
sure where a title lands, leave `*_auto: true` with a sensible value; the words decide on read.
`thing`/`me` are the fallbacks.

## Recipes

- **Something on a day:** write one entry file, then stop. Keep the field order above.
  `created_at` is `date -u +%Y-%m-%dT%H:%M:%SZ`; the id is a fresh UUID (`uuidgen`).
- **A detail with no date yet:** a note file with `day: null` in `notes/`.
- **Clip a loose note to a day:** change its `day:` to that date.
- **Tick a task:** rewrite that exact `- [ ] …` body line to `- [x] …`; nothing else changes.
- **Write the day's page:** create or extend `days/<day>.md`; preserve `first_written` on an
  existing file. Day-page Markdown supports headings, bold/italic, code, lists, `- [ ]` tasks,
  quotes, tables, rules, links, `@Name` for a person, and `^[YYYY-MM-DD^label]` for a day chip.
- **Delete:** remove the file. That is the app's own delete.
- **Find:** `rg -l 'title: ".*mum' entries/`.

## Reading a message

A user message may begin with a `<horizon-context>` block: today's date, the window, the active
lens, the open day and a few counts. Treat it as ground truth about what the user is looking at;
do not echo it back. Anything outside that block is what the user actually said.

If the user asks what is coming up, read the files and answer with dates and titles — the wall is
a picture, your job is to make it speak.
"#,
        cwd = cwd.display(),
        mode = mode
    )
}

/// A short description of what is on screen, attached to every message when the
/// brief's "tell the agent what's on screen" is on.
pub fn context_block(doc: &Doc, quarter: &Quarter) -> String {
    use horizon_core::dates;

    let mut out = String::new();
    out.push_str(&format!(
        "Today: {} ({}). The wall shows {} .. {} (90 days, Monday-first).\n",
        dates::key(quarter.today),
        quarter.today.format("%A"),
        dates::key(quarter.start),
        dates::key(quarter.end),
    ));
    match doc.ui.lens {
        Some(subject) => out.push_str(&format!(
            "Reading: the quarter through {}.\n",
            subject.name()
        )),
        None => out.push_str("Reading: everything.\n"),
    }
    if let Some(open) = doc.ui.open_day.as_deref() {
        let things = doc.entries_on(open, doc.ui.lens).count();
        let notes = doc.clipped_notes_on(open, doc.ui.lens).count();
        out.push_str(&format!(
            "Open day: {open} — {things} entr{}, {notes} clipped note(s), page {}.\n",
            if things == 1 { "y" } else { "ies" },
            if doc.has_page(open) {
                "written"
            } else {
                "empty"
            },
        ));
        if things > 0 {
            let lines: Vec<String> = doc
                .entries_on(open, doc.ui.lens)
                .take(8)
                .map(|e| {
                    format!(
                        "  - {} [{}]{}",
                        e.text,
                        e.subject.handle(),
                        if e.done { " done" } else { "" }
                    )
                })
                .collect();
            out.push_str(&lines.join("\n"));
            out.push('\n');
        }
    }
    let in_window = doc
        .entries
        .iter()
        .filter(|e| quarter.contains_key(&e.day))
        .count();
    let loose = doc.loose_notes().count();
    let clipped = doc.notes.iter().filter(|n| n.day.is_some()).count();
    let pages = doc.pages.values().filter(|p| !p.trim().is_empty()).count();
    out.push_str(&format!(
        "Counts: {in_window} entries in the window, {loose} loose notes, {clipped} clipped notes, {pages} day pages.\n"
    ));

    let mut ahead = Vec::new();
    for day in &quarter.days {
        if ahead.len() >= 6 {
            break;
        }
        if !doc.has_content(&day.key, doc.ui.lens) {
            continue;
        }
        let titles: Vec<String> = doc
            .entries_on(&day.key, doc.ui.lens)
            .take(3)
            .map(|e| format!("{} [{}]", e.text, e.subject.handle()))
            .collect();
        let mut line = format!("- {} {}", day.key, titles.join("; "));
        if doc.has_page(&day.key) {
            line.push_str(" + page");
        }
        ahead.push(line);
    }
    if !ahead.is_empty() {
        out.push_str("The next days with something:\n");
        out.push_str(&ahead.join("\n"));
        out.push('\n');
    }
    let mut loose_lines = Vec::new();
    for note in doc.loose_notes().take(6) {
        let first = note.text.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            loose_lines.push(format!("- {}", first));
        }
    }
    if !loose_lines.is_empty() {
        out.push_str("Loose notes (undated):\n");
        out.push_str(&loose_lines.join("\n"));
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn an_empty_config_file_is_pi_with_pis_defaults() {
        let config = AgentConfig::from_settings(&BTreeMap::new());
        assert_eq!(config.harness, "pi");
        assert_eq!(config.command, "pi");
        assert!(config.model.is_empty());
        assert!(config.context);
        assert!(config.write);
        assert!(config.enabled());
    }

    #[test]
    fn the_agent_can_be_switched_off_and_pinned() {
        let mut settings = BTreeMap::new();
        settings.insert("agent".to_string(), "off".to_string());
        assert!(!AgentConfig::from_settings(&settings).enabled());

        settings.insert("agent".to_string(), "pi".to_string());
        settings.insert(
            "agent.model".to_string(),
            "anthropic/claude-sonnet-4-5".to_string(),
        );
        settings.insert("agent.context".to_string(), "false".to_string());
        settings.insert("agent.write".to_string(), "false".to_string());
        let config = AgentConfig::from_settings(&settings);
        assert_eq!(config.model, "anthropic/claude-sonnet-4-5");
        assert!(!config.context);
        assert!(!config.write);
    }

    #[test]
    fn settings_round_trip_through_the_config_keys() {
        let config = AgentConfig {
            harness: "pi".into(),
            command: "/usr/local/bin/pi".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            thinking: "high".into(),
            context: false,
            write: true,
        };
        let values: BTreeMap<String, String> = config
            .settings()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        assert_eq!(AgentConfig::from_settings(&values), config);
    }

    #[test]
    fn the_manual_names_the_vault_and_the_write_mode() {
        let config = AgentConfig::default();
        let text = manual(&config, std::path::Path::new("/home/you/Horizon"));
        assert!(text.contains("/home/you/Horizon"));
        assert!(text.contains("Never touch `state.md`"));
        assert!(text.contains("kind: \"entry\""));
        assert!(text.contains("Horizon reloads the folder after your run"));
        let read_only = manual(
            &AgentConfig {
                write: false,
                ..AgentConfig::default()
            },
            std::path::Path::new("/tmp/v"),
        );
        assert!(read_only.contains("read-only"));
    }

    #[test]
    fn the_context_block_names_today_the_lens_and_the_open_day() {
        use chrono::NaiveDate;
        use horizon_core::{Doc, Entry, Subject};

        let today = NaiveDate::from_ymd_opt(2026, 11, 17).expect("a real date");
        let quarter = Quarter::new(today);
        let mut doc = Doc::empty();
        doc.ui.open_day = Some("2026-11-18".into());
        doc.ui.lens = Some(Subject::Family);
        doc.entries.push(Entry::new("2026-11-18", "Call mum"));
        doc.notes
            .push(horizon_core::LooseNote::new("Ask about the bond", None));

        let text = context_block(&doc, &quarter);
        assert!(text.contains("Today: 2026-11-17"));
        assert!(text.contains("Reading: the quarter through Family."));
        assert!(text.contains("Open day: 2026-11-18"));
        assert!(text.contains("Loose notes (undated)"));
        assert!(
            text.len() < 2000,
            "the context stays a preamble, not a dump"
        );
    }

    /// A real turn through the real harness. It needs `pi` on PATH and a
    /// configured model, so it is ignored by default. It uses its own folder so
    /// the transcript does not mix with the owner's conversation:
    ///
    /// ```sh
    /// HORIZON_STORE=/tmp/horizon-live cargo test -p horizon-app live_turn \
    ///   -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "starts a real harness and calls a model"]
    fn live_turn_round_trips_through_pi() {
        use std::time::{Duration, Instant};

        init();
        send("Reply with the single word: ready", None).expect("a prompt is accepted");
        let deadline = Instant::now() + Duration::from_secs(120);
        let view = loop {
            let view = snapshot().expect("the service is up");
            if !view.busy {
                break view;
            }
            assert!(
                Instant::now() < deadline,
                "no answer in 120s (status {:?}, detail {})",
                view.status,
                view.detail
            );
            std::thread::sleep(Duration::from_millis(250));
        };
        let answer = view
            .items
            .iter()
            .find_map(|item| match item {
                ChatItem::Assistant { text } if !text.trim().is_empty() => Some(text.clone()),
                _ => None,
            })
            .expect("an answer");
        eprintln!("assistant said: {answer}");
        assert!(answer.to_lowercase().contains("ready"));
        // The run ended but the folder still belongs to the agent until the app
        // has merged it: that is the contract the saver relies on.
        assert!(busy(), "a settled run asks for a reconcile");
        reconciled();
        assert!(!busy(), "the saver may write again");
        shutdown();
    }
}
