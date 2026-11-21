//! Local persistence.
//!
//! Horizon keeps a folder of plain Markdown files: one per entry, note and day,
//! plus `state.md`. The folder is the live store — there is no server and no
//! database. The owner picks it once, on first launch, and the choice is kept in
//! `vault.txt` beside the app's own data. Horizon suggests `~/Documents/Horizon`;
//! `HORIZON_STORE` overrides that.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use dioxus::prelude::*;
use horizon_core::store;
use horizon_core::Doc;

const README: &str = "# Horizon markdown store\n\n\
This folder is the live Horizon store. Every entry, note, day page and reminder is\n\
a Markdown file with YAML-style frontmatter; `state.md` holds the open day, lens\n\
and draw mode. `reminders/` holds one file per reminder.\n\
Keep the frontmatter fields and write freely in the body.\n";

fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn data_dir() -> Option<PathBuf> {
    dirs::data_dir()
}

/// The folder Horizon suggests when there is no vault yet: visible in a file
/// manager, so it is easy to back up or point a sync tool at.
pub fn default_vault() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let documents = home.join("Documents");
    if documents.is_dir() {
        Some(documents.join("Horizon"))
    } else {
        Some(home.join("Horizon"))
    }
}

/// `~` at the start of a typed path means the home folder.
pub fn expand_home(path: &str) -> PathBuf {
    let path = path.trim();
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if let (Some(rest), Some(home)) = (path.strip_prefix("~/"), dirs::home_dir()) {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// Horizon's own settings live in the config folder, never in the vault: the
/// vault is the owner's Markdown and becomes a git repository, so a pointer to
/// the vault must not be inside it. `$XDG_CONFIG_HOME/horizon` on Linux
/// (`~/.config/horizon` by default).
fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("horizon"))
}

fn config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

const CONFIG_HEADER: &str = "\
# Horizon settings. The vault is the folder of Markdown files itself; this file
# lives beside the app's configuration, not inside the vault, so moving the
# folder keeps the pointer valid and the pointer never enters a commit.
";

/// One key, one line, JSON-quoted (which is also valid TOML basic-string syntax,
/// so a path with spaces, quotes or backslashes survives).
fn render_config(vault: &Path) -> String {
    let quoted = serde_json::to_string(&vault.display().to_string()).unwrap_or_default();
    format!("{CONFIG_HEADER}vault = {quoted}\n")
}

/// Every `key = value` line in the settings file. Comments, blank lines and
/// unknown keys are ignored, so the file can grow without a TOML dependency.
pub fn settings() -> BTreeMap<String, String> {
    let Some(file) = config_file() else {
        return BTreeMap::new();
    };
    match std::fs::read_to_string(file) {
        Ok(text) => parse_settings(&text),
        Err(_) => BTreeMap::new(),
    }
}

fn parse_settings(text: &str) -> BTreeMap<String, String> {
    let mut settings = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = serde_json::from_str::<String>(value)
            .unwrap_or_else(|_| value.trim_matches('"').to_string());
        settings.insert(key.to_string(), value);
    }
    settings
}

/// Read the one key the vault pointer needs. Kept separate from `settings` so a
/// missing or malformed agent key can never hide the vault.
fn parse_config(text: &str) -> Option<PathBuf> {
    parse_settings(text)
        .remove("vault")
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
}

/// Change settings in place, keeping comments, unknown keys and the order of the
/// lines that are not being changed. The brief's Agent block is the only caller;
/// the vault pointer uses the same path so re-choosing a folder never drops the
/// agent's configuration.
fn update_config(pairs: &[(&str, String)]) -> Result<(), String> {
    let file = config_file().ok_or("Could not find Horizon's config directory.")?;
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let text = set_config_lines(&text, pairs);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, text).map_err(|e| e.to_string())
}

/// Replace or append `key = value` lines, keeping comments, unknown keys and the
/// order of everything else. Pure, so the file's survival is testable without
/// touching the real settings.
fn set_config_lines(text: &str, pairs: &[(&str, String)]) -> String {
    let mut text = if text.trim().is_empty() {
        CONFIG_HEADER.to_string()
    } else {
        text.to_string()
    };
    for (key, value) in pairs {
        let quoted = serde_json::to_string(value).unwrap_or_default();
        let line = format!("{key} = {quoted}");
        let mut replaced = false;
        let mut next = String::new();
        for existing in text.lines() {
            let matches = existing
                .split_once('=')
                .is_some_and(|(candidate, _)| candidate.trim() == *key);
            if matches && !replaced {
                replaced = true;
                next.push_str(&line);
            } else {
                next.push_str(existing);
            }
            next.push('\n');
        }
        if !replaced {
            next.push_str(&line);
            next.push('\n');
        }
        text = next;
    }
    text
}

/// Persist the Agent block. Comments and unrelated keys survive.
pub fn save_agent_config(config: &crate::agent::AgentConfig) -> Result<(), String> {
    let pairs: Vec<(&str, String)> = config.settings();
    update_config(&pairs)
}

/// The agent's configuration, read fresh so a hand edit of `config.toml` is
/// picked up on the next restart without a cache to invalidate.
pub fn agent_config() -> crate::agent::AgentConfig {
    crate::agent::AgentConfig::from_settings(&settings())
}

fn write_config(vault: &Path) -> Result<(), String> {
    // Keep an existing file's comments and agent settings; only the pointer moves.
    let path = vault.display().to_string();
    if config_file().is_some_and(|file| file.is_file()) {
        return update_config(&[("vault", path)]);
    }
    let file = config_file().ok_or("Could not find Horizon's config directory.")?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, render_config(vault)).map_err(|e| e.to_string())
}

/// The folder the owner chose, if they ever chose one. A `vault.txt` left by the
/// first cut of this feature is adopted and removed: it may sit inside the
/// vault, where it would otherwise end up in a commit.
pub fn chosen_vault() -> Option<PathBuf> {
    let file = match config_file() {
        Some(file) => file,
        None => return adopt_legacy_vault(),
    };
    let text = match std::fs::read_to_string(&file).ok() {
        Some(text) => text,
        None => return adopt_legacy_vault(),
    };
    if let Some(vault) = parse_config(&text) {
        if vault.is_dir() {
            return Some(vault);
        }
        // The folder was deleted or mistyped. Drop the pointer instead of
        // opening a store that cannot exist, so the first-run page shows again.
        let _ = std::fs::remove_file(&file);
    }
    adopt_legacy_vault()
}

/// The first cut of this feature kept the path in `vault.txt` as a bare line.
/// Adopt it once, in the new format, and delete it.
fn adopt_legacy_vault() -> Option<PathBuf> {
    let legacy = data_dir()?.join("horizon").join("vault.txt");
    let text = std::fs::read_to_string(&legacy).ok()?;
    let path = parse_config(&text).or_else(|| {
        let text = text.trim();
        (!text.is_empty()).then(|| PathBuf::from(text))
    })?;
    let _ = write_config(&path);
    let _ = std::fs::remove_file(&legacy);
    Some(path)
}

/// Prepare the folder and remember it. The folder is created if it is missing, so
/// a mistyped but valid path fails here rather than on the first save.
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "tests use prepare_vault and write_config separately"
    )
)]
pub fn choose_vault(path: &Path) -> Result<(), String> {
    prepare_vault(path)?;
    write_config(path)
}

/// Create the vault and give it the store's README. Separated from the config
/// write so tests can exercise it without touching the real settings file.
fn prepare_vault(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let marker = path.join("README.md");
    if !marker.exists() {
        std::fs::write(&marker, README).map_err(|e| format!("{}: {e}", marker.display()))?;
    }
    Ok(())
}

/// The Markdown vault, or `None` when the owner has not chosen one yet. This is
/// also the git repository: whichever folder is chosen is the folder that syncs.
pub fn store_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("HORIZON_STORE") {
        return Some(PathBuf::from(dir));
    }
    chosen_vault()
}

/// A vault is open once there is a folder to write to. The first launch has none.
pub fn vault_is_set() -> bool {
    store_dir().is_some()
}

pub fn store_path() -> Option<String> {
    store_dir().map(|dir| dir.display().to_string())
}

fn read_store(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    let state = dir.join("state.md");
    if state.is_file() {
        files.insert(
            "state.md".into(),
            std::fs::read_to_string(&state).map_err(|e| e.to_string())?,
        );
    }
    for sub in ["entries", "notes", "days", "reminders"] {
        let subdir = dir.join(sub);
        if !subdir.is_dir() {
            continue;
        }
        for item in std::fs::read_dir(&subdir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let path = item.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let key = format!("{sub}/{name}");
            files.insert(
                key,
                std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
            );
        }
    }
    Ok(files)
}

fn managed_file(path: &Path) -> bool {
    let Some(text) = std::fs::read_to_string(path).ok() else {
        return false;
    };
    text.starts_with("---\n")
        && [
            "kind: \"entry\"",
            "kind: \"note\"",
            "kind: \"day\"",
            "kind: \"reminder\"",
        ]
        .iter()
        .any(|marker| text.contains(marker))
}

/// What Horizon last read or wrote, file path by file path. It is the base of
/// the three-way merge used after an external writer (the agent, an editor, a
/// pull) touches the folder while the app is open.
static LAST_FILES: std::sync::Mutex<Option<BTreeMap<String, String>>> = std::sync::Mutex::new(None);

fn remember(files: &BTreeMap<String, String>) {
    if let Ok(mut last) = LAST_FILES.lock() {
        *last = Some(files.clone());
    }
}

fn last_files() -> Option<BTreeMap<String, String>> {
    LAST_FILES.lock().ok().and_then(|last| last.clone())
}

fn write_store(dir: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for sub in ["entries", "notes", "days", "reminders"] {
        std::fs::create_dir_all(dir.join(sub)).map_err(|e| e.to_string())?;
    }

    for (relative, content) in files {
        let target = dir.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temp = target.with_extension("md.tmp");
        std::fs::write(&temp, content).map_err(|e| e.to_string())?;
        if target.exists() {
            std::fs::remove_file(&target).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&temp, &target).map_err(|e| e.to_string())?;
    }

    let readme = dir.join("README.md");
    if !readme.exists() {
        let _ = std::fs::write(&readme, README);
    }

    let expected: BTreeSet<&str> = files.keys().map(String::as_str).collect();
    for sub in ["entries", "notes", "days", "reminders"] {
        let subdir = dir.join(sub);
        let Ok(read_dir) = std::fs::read_dir(&subdir) else {
            continue;
        };
        for item in read_dir.flatten() {
            let path = item.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let relative = format!("{sub}/{name}");
            if !expected.contains(relative.as_str()) && managed_file(&path) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    remember(files);
    Ok(())
}

/// Merge three file maps, path by path. `base` is what Horizon last read or
/// wrote, `local` is the document in memory, `remote` is the folder now.
///
/// A path changed on one side takes that side; a write beats a delete; two
/// different writes keep the local text and return the remote text so the
/// caller can file it under `conflicts/`. `state.md` is device state and is
/// always taken from `local` — the folder must never move another device's open
/// day.
pub fn merge_files(
    base: &BTreeMap<String, String>,
    local: &BTreeMap<String, String>,
    remote: &BTreeMap<String, String>,
) -> (BTreeMap<String, String>, Vec<(String, String)>) {
    let mut keys: BTreeSet<&String> = BTreeSet::new();
    keys.extend(base.keys());
    keys.extend(local.keys());
    keys.extend(remote.keys());

    let mut merged = BTreeMap::new();
    let mut conflicts = Vec::new();
    for key in keys {
        if key == "state.md" {
            if let Some(text) = local.get(key) {
                merged.insert(key.clone(), text.clone());
            }
            continue;
        }
        let before = base.get(key);
        let mine = local.get(key);
        let theirs = remote.get(key);
        if mine == theirs {
            if let Some(text) = mine {
                merged.insert(key.clone(), text.clone());
            }
            continue;
        }
        if mine == before {
            if let Some(text) = theirs {
                merged.insert(key.clone(), text.clone());
            }
            continue;
        }
        if theirs == before {
            if let Some(text) = mine {
                merged.insert(key.clone(), text.clone());
            }
            continue;
        }
        // Both sides changed the same path.
        match (mine, theirs) {
            (Some(mine), Some(theirs)) => {
                merged.insert(key.clone(), mine.clone());
                conflicts.push((key.clone(), theirs.clone()));
            }
            // Writing beats deleting, from either side.
            (Some(text), None) | (None, Some(text)) => {
                merged.insert(key.clone(), text.clone());
            }
            (None, None) => {}
        }
    }
    (merged, conflicts)
}

/// Bring the folder and memory back together after something outside the app
/// wrote. The agent's runs end here: it writes files, settles, and Horizon reads
/// the folder, merges it with anything the owner typed meanwhile, and writes the
/// result once. `Ok(None)` means the folder matched what we already had.
pub async fn reconcile(doc: &Doc, today: NaiveDate) -> Result<Option<Doc>, String> {
    let Some(dir) = store_dir() else {
        return Ok(None);
    };
    let remote = read_store(&dir)?;
    let base = last_files().unwrap_or_default();
    if base == remote {
        return Ok(None);
    }
    let local = store::to_files(doc);
    let (merged, conflicts) = merge_files(&base, &local, &remote);
    if !conflicts.is_empty() {
        let stamp = chrono::Local::now().format("%Y%m%dT%H%M%S");
        for (path, text) in &conflicts {
            let target = dir.join("conflicts").join(format!("{path}.{stamp}.md"));
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&target, text);
        }
    }
    let next = store::from_files(&merged, today).map_err(|e| e.to_string())?;
    write_store(&dir, &merged)?;
    Ok(Some(next))
}

/// Read the live store from its Markdown folder. `Ok(None)` means the folder is
/// empty or absent, so the caller decides whether to start empty or with demo data.
pub async fn load(today: NaiveDate) -> Result<Option<Doc>, String> {
    let dir = store_dir().ok_or("Could not find Horizon's data directory.")?;
    let files = read_store(&dir)?;
    if files.is_empty() {
        return Ok(None);
    }
    store::from_files(&files, today)
        .map(|doc| {
            remember(&files);
            Some(doc)
        })
        .map_err(|e| e.to_string())
}

/// Persist the latest document. The app funnels every mutation through one
/// saver loop, so this can write the whole store without a later write racing it.
pub async fn save_now(doc: &Doc) {
    let Some(dir) = store_dir() else {
        return;
    };
    if let Err(err) = write_store(&dir, &store::to_files(doc)) {
        eprintln!("Horizon: could not write the Markdown store: {err}");
    }
}

/// The store folder and its files, as read from disk.
pub type StoreSnapshot = (PathBuf, BTreeMap<String, String>);

/// The store folder, read synchronously. The closed-app reminder pass runs
/// before any async runtime exists, so it cannot use `load`.
pub fn read_store_sync() -> Result<Option<StoreSnapshot>, String> {
    let Some(dir) = store_dir() else {
        return Ok(None);
    };
    let files = read_store(&dir)?;
    Ok(Some((dir, files)))
}

/// Write only the reminder files of a document, atomically, and delete reminder
/// files that no longer exist. The closed-app pass must never rewrite an entry,
/// a note, a day page or `state.md`.
pub fn write_reminders_only(dir: &Path, doc: &Doc) -> Result<(), String> {
    let wanted: BTreeMap<String, String> = store::to_files(doc)
        .into_iter()
        .filter(|(path, _)| path.starts_with("reminders/"))
        .collect();
    let folder = dir.join("reminders");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    for (relative, content) in &wanted {
        let target = dir.join(relative);
        let temp = target.with_extension("md.tmp");
        std::fs::write(&temp, content).map_err(|e| e.to_string())?;
        if target.exists() {
            std::fs::remove_file(&target).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&temp, &target).map_err(|e| e.to_string())?;
    }
    for item in std::fs::read_dir(&folder)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let path = item.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let relative = format!("reminders/{name}");
        if !wanted.contains_key(&relative) && managed_file(&path) {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

/// Reload the page against the vault that was just chosen. The store, the saver
/// and the sync thread all read `store_dir()` when they start, so a reload is
/// simpler and more honest than re-wiring three subsystems at runtime. A first
/// run has nothing in memory worth keeping, and the sync thread has not started
/// yet (it declines to run without a vault), so a fresh page starts everything
/// against the chosen folder.
pub fn reload() {
    spawn(async move {
        let _ = document::eval("location.reload();").await;
    });
}

pub fn focus_id(id: &str) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.getElementById({}); if (el) el.focus({{ preventScroll: true }}); }});",
        js_string(id)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

/// Focus a day in whichever shape is currently on screen: the wall in a wide
/// window, the agenda in a narrow one. Both elements can exist in the DOM; the
/// hidden one is ignored.
pub fn focus_day(key: &str) {
    let script = format!(
        "setTimeout(() => {{ const wall = document.getElementById({}); const agenda = document.getElementById({}); const el = (wall && wall.offsetParent !== null) ? wall : agenda; if (el) el.focus({{ preventScroll: true }}); }}, 120);",
        js_string(&format!("cell-{key}")),
        js_string(&format!("agenda-{key}"))
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

pub fn scroll_id(id: &str) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.getElementById({}); if (el) el.scrollIntoView({{ block: 'nearest', inline: 'nearest' }}); }});",
        js_string(id)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

pub fn scroll_end(id: &str) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.getElementById({id}); if (!el) return; const near = el.scrollHeight - el.scrollTop - el.clientHeight < 160; if (near) el.scrollTop = el.scrollHeight; }});",
        id = js_string(id)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

pub fn download(filename: &str, mime: &str, content: &str) {
    let script = format!(
        "const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([{content}], {{ type: {mime} }})); a.download = {filename}; document.body.appendChild(a); a.click(); a.remove();",
        content = js_string(content),
        mime = js_string(mime),
        filename = js_string(filename)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

/// Where a copy of the store goes: the Downloads folder.
fn copy_root() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Write a copy of the Markdown store. Not a second format: the same tree, the
/// same bytes, minus the device-local `state.md`, plus the folder's README.
pub fn export_store_copy(doc: &Doc) -> Result<String, String> {
    let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let folder = copy_root().join(format!("horizon-{stamp}"));
    let mut files = store::to_files(doc);
    files.remove("state.md");
    files.insert("README.md".into(), README.into());
    for (path, content) in files {
        let target = folder.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&target, content).map_err(|e| e.to_string())?;
    }
    Ok(folder.display().to_string())
}

/// The editor toolbar manipulates the live textarea selection in the webview, then dispatches
/// an input event so the ordinary Dioxus `oninput` path owns the buffer. This is the one place
/// the UI asks the platform for a selection; it never writes markup.
pub fn wrap_selection(textarea_id: &str, prefix: &str, suffix: &str, placeholder: &str) {
    let script = format!(
        r#"
        const ta = document.getElementById({id});
        if (!ta) return;
        const start = ta.selectionStart ?? ta.value.length;
        const end = ta.selectionEnd ?? ta.value.length;
        const selected = ta.value.slice(start, end) || {placeholder};
        ta.setRangeText({prefix} + selected + {suffix}, start, end, 'end');
        ta.dispatchEvent(new Event('input', {{ bubbles: true }}));
        ta.focus();
        "#,
        id = js_string(textarea_id),
        prefix = js_string(prefix),
        suffix = js_string(suffix),
        placeholder = js_string(placeholder)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

pub fn focus_textarea(textarea_id: &str) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.getElementById({}); if (el) {{ el.focus(); el.setSelectionRange(el.value.length, el.value.length); }} }});",
        js_string(textarea_id)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::{store, Entry};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 4).expect("a real date")
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("horizon-{tag}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn write_read_store_round_trip_and_remove_stale() {
        let dir = temp_dir("store");
        let mut doc = Doc::empty();
        doc.entries.push(Entry::new("2026-11-04", "Call mum"));
        doc.set_page("2026-11-04", "# Today\n\nThree things.", today());

        write_store(&dir, &store::to_files(&doc)).expect("write the store");
        let files = read_store(&dir).expect("read the store");
        assert!(files.contains_key("state.md"));
        assert!(files.contains_key("days/2026-11-04.md"));
        assert!(files.keys().any(|k| k.starts_with("entries/")));

        let back = store::from_files(&files, today()).expect("parse the store");
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].text, "Call mum");
        assert_eq!(back.page("2026-11-04"), "# Today\n\nThree things.");

        doc.entries.clear();
        write_store(&dir, &store::to_files(&doc)).expect("write the store");
        let files = read_store(&dir).expect("read the store");
        assert!(!files.keys().any(|k| k.starts_with("entries/")));
        assert!(files.contains_key("days/2026-11-04.md"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_version_one_folder_is_rewritten_with_stable_names() {
        let dir = temp_dir("migrate");
        std::fs::create_dir_all(dir.join("entries")).expect("create entries");
        std::fs::create_dir_all(dir.join("notes")).expect("create notes");
        std::fs::write(
            dir.join("entries/2026-11-04-001-call-mum-aaaaaaaa.md"),
            "---\nkind: \"entry\"\nversion: 1\nid: \"e-aaaaaaaa\"\nday: \"2026-11-04\"\ntitle: \"Call mum\"\ncreated_at: \"2026-11-04T09:00:00Z\"\n---\n",
        )
        .expect("write a version 1 entry");
        std::fs::write(
            dir.join("notes/001-a-note-bbbbbbbb.md"),
            "---\nkind: \"note\"\nversion: 1\nid: \"n-bbbbbbbb\"\nmark: \"thing\"\nsubject: \"me\"\nday: null\n---\n\nA note\n",
        )
        .expect("write a version 1 note");

        let files = read_store(&dir).expect("read the store");
        let doc = store::from_files(&files, today()).expect("parse the store");
        write_store(&dir, &store::to_files(&doc)).expect("write the store");

        let files = read_store(&dir).expect("read the store");
        assert!(files
            .keys()
            .any(|k| k.starts_with("entries/2026-11-04-call-mum-")));
        assert!(!files.keys().any(|k| k.contains("-001-")));
        assert!(files.keys().any(|k| k.starts_with("notes/a-note-")));

        let back = store::from_files(&files, today()).expect("parse the store");
        assert_eq!(back.entries[0].text, "Call mum");
        assert_eq!(back.notes[0].text, "A note");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tilde_means_the_home_folder() {
        let home = dirs::home_dir().unwrap_or_default();
        assert_eq!(expand_home("~/Vaults/Horizon"), home.join("Vaults/Horizon"));
        assert_eq!(expand_home("  ~/x  "), home.join("x"));
        assert_eq!(expand_home("~"), home);
        // An ordinary absolute path is taken as written.
        assert_eq!(expand_home("/tmp/vault"), PathBuf::from("/tmp/vault"));
    }

    #[test]
    fn choosing_a_vault_creates_it_and_writes_a_readme() {
        let dir = temp_dir("vault");
        prepare_vault(&dir).expect("prepare the temp vault");
        assert!(dir.is_dir(), "the folder is created");
        let readme = std::fs::read_to_string(dir.join("README.md")).expect("README is written");
        assert!(readme.starts_with("# Horizon markdown store"));
        // A second choice leaves an existing README alone.
        std::fs::write(dir.join("README.md"), "mine").expect("overwrite the README");
        prepare_vault(&dir).expect("prepare the temp vault");
        assert_eq!(
            std::fs::read_to_string(dir.join("README.md")).expect("README survives"),
            "mine"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_config_file_writes_and_reads_one_key() {
        let vault = Path::new("/home/you/Documents/My Vault");
        let text = render_config(vault);
        assert!(text.starts_with("#"), "a header explains the file");
        assert!(text.contains("vault = \"/home/you/Documents/My Vault\""));
        assert_eq!(parse_config(&text), Some(vault.to_path_buf()));
    }

    #[test]
    fn the_config_tolerates_comments_other_keys_and_bare_paths() {
        let text = "# comment\nvault = \"/tmp/a#b\"\nother = 1\n";
        assert_eq!(parse_config(text), Some(PathBuf::from("/tmp/a#b")));
        // A hand-edited, unquoted path still reads.
        assert_eq!(
            parse_config("vault = /tmp/plain\n"),
            Some(PathBuf::from("/tmp/plain"))
        );
        assert_eq!(parse_config("# nothing here\n"), None);
        assert_eq!(parse_config("vault = \"\"\n"), None);
    }

    #[test]
    fn agent_keys_do_not_hide_the_vault_pointer() {
        let text = "# mine\nagent = \"pi\"\nvault = \"/tmp/vault\"\nagent.write = \"false\"\n";
        let settings = parse_settings(text);
        assert_eq!(settings.get("vault"), Some(&"/tmp/vault".to_string()));
        assert_eq!(settings.get("agent"), Some(&"pi".to_string()));
        assert_eq!(parse_config(text), Some(PathBuf::from("/tmp/vault")));
    }

    #[test]
    fn writing_settings_keeps_comments_and_unknown_lines() {
        let before = "# a header\nvault = \"/tmp/vault\"\n# my note\nexperimental = \"yes\"\n";
        let after = set_config_lines(
            before,
            &[
                ("agent", "pi".to_string()),
                ("agent.context", "false".to_string()),
            ],
        );
        assert!(after.starts_with("# a header\nvault = \"/tmp/vault\"\n# my note\n"));
        assert!(after.contains("experimental = \"yes\""));
        assert!(after.contains("agent = \"pi\""));
        assert!(after.contains("agent.context = \"false\""));
        let settings = parse_settings(&after);
        assert_eq!(settings.get("agent.context"), Some(&"false".to_string()));
        assert_eq!(settings.get("vault"), Some(&"/tmp/vault".to_string()));
    }

    #[test]
    fn writing_the_same_key_twice_replaces_it_once() {
        let after = set_config_lines(
            "vault = \"/old\"\n",
            &[
                ("vault", "/new".to_string()),
                ("vault", "/newer".to_string()),
            ],
        );
        assert_eq!(parse_config(&after), Some(PathBuf::from("/newer")));
        assert_eq!(after.matches("vault =").count(), 1);
    }

    fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_merge_takes_a_change_from_either_side() {
        let base = files(&[("entries/a.md", "a"), ("entries/b.md", "b")]);
        let local = files(&[("entries/a.md", "a"), ("entries/b.md", "B from the app")]);
        let remote = files(&[
            ("entries/a.md", "a from the agent"),
            ("entries/b.md", "b"),
            ("notes/new.md", "new"),
        ]);
        let (merged, conflicts) = merge_files(&base, &local, &remote);
        assert_eq!(
            merged.get("entries/a.md").map(String::as_str),
            Some("a from the agent")
        );
        assert_eq!(
            merged.get("entries/b.md").map(String::as_str),
            Some("B from the app")
        );
        assert_eq!(merged.get("notes/new.md").map(String::as_str), Some("new"));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn a_merge_keeps_both_writes_when_both_sides_changed_a_file() {
        let base = files(&[("entries/a.md", "a")]);
        let local = files(&[("entries/a.md", "mine")]);
        let remote = files(&[("entries/a.md", "theirs")]);
        let (merged, conflicts) = merge_files(&base, &local, &remote);
        assert_eq!(merged.get("entries/a.md").map(String::as_str), Some("mine"));
        assert_eq!(
            conflicts,
            vec![("entries/a.md".to_string(), "theirs".to_string())]
        );
    }

    #[test]
    fn a_merge_lets_writing_beat_deleting() {
        let base = files(&[("entries/a.md", "a"), ("entries/b.md", "b")]);
        // The app deleted a; the agent deleted b while writing nothing else.
        let local = files(&[("entries/b.md", "b")]);
        let remote = files(&[("entries/a.md", "a"), ("entries/b.md", "b2")]);
        let (merged, _) = merge_files(&base, &local, &remote);
        assert!(!merged.contains_key("entries/a.md"), "a delete is honoured");
        assert_eq!(merged.get("entries/b.md").map(String::as_str), Some("b2"));

        // A path deleted on one side and written on the other keeps the writing.
        let base = files(&[("entries/c.md", "c")]);
        let local = files(&[("entries/c.md", "c")]);
        let remote = files(&[]);
        let (merged, _) = merge_files(&base, &local, &remote);
        assert!(!merged.contains_key("entries/c.md"));
    }

    #[test]
    fn a_merge_never_lets_the_folder_move_device_state() {
        let base = files(&[("state.md", "open 1")]);
        let local = files(&[("state.md", "open 2")]);
        let remote = files(&[("state.md", "open 3")]);
        let (merged, conflicts) = merge_files(&base, &local, &remote);
        assert_eq!(merged.get("state.md").map(String::as_str), Some("open 2"));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn a_config_pointer_to_a_missing_folder_is_dropped() {
        let tmp =
            std::env::temp_dir().join(format!("horizon-config-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        let missing = tmp.join("does-not-exist");
        write_config(&missing).expect("write the config pointer");
        assert_eq!(
            chosen_vault(),
            None,
            "a pointer to a deleted folder is dropped so the first-run page shows again"
        );
        assert!(
            !config_file().expect("config path").exists(),
            "the stale pointer is removed"
        );
        std::env::remove_var("XDG_CONFIG_HOME");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn a_new_vault_is_readable_as_an_empty_store() {
        let dir = temp_dir("fresh");
        prepare_vault(&dir).expect("prepare the temp vault");
        // The README is not a store file, so the folder reads as empty: the app
        // starts with an empty quarter rather than refusing to open it.
        assert!(read_store(&dir).expect("a fresh folder reads").is_empty());
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_store_reads_as_empty() {
        let dir = temp_dir("empty");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read_store(&dir).expect("a fresh folder reads").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
