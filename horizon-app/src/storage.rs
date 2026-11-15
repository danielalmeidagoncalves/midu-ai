//! Local persistence.
//!
//! Horizon keeps a folder of plain Markdown files: one per entry, note and day,
//! plus `state.md`. The folder is the live store — there is no server and no
//! database. The owner picks it once, on first launch, and the choice is kept in
//! `vault.txt` beside the app's own data. Desktop suggests `~/Documents/Horizon`,
//! Android the app files directory; `HORIZON_STORE` overrides both.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use dioxus::prelude::*;
use horizon_core::store;
use horizon_core::Doc;

const README: &str = "# Horizon markdown store\n\n\
This folder is the live Horizon store. Every entry, note and day page is a Markdown\n\
file with YAML-style frontmatter; `state.md` holds the open day, lens and draw mode.\n\
Keep the frontmatter fields and write freely in the body.\n";

fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

#[cfg(desktop_shell)]
fn data_dir() -> Option<PathBuf> {
    dirs::data_dir()
}

#[cfg(not(desktop_shell))]
fn data_dir() -> Option<PathBuf> {
    sysdirs::data_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| Some(std::env::temp_dir()))
}

/// The folder Horizon suggests when there is no vault yet: visible in a file
/// manager, so it is easy to back up or point a sync tool at. On Android the app
/// files directory is the only place that needs no permission.
pub fn default_vault() -> Option<PathBuf> {
    #[cfg(desktop_shell)]
    {
        let home = dirs::home_dir()?;
        let documents = home.join("Documents");
        if documents.is_dir() {
            Some(documents.join("Horizon"))
        } else {
            Some(home.join("Horizon"))
        }
    }
    #[cfg(not(desktop_shell))]
    {
        data_dir().map(|dir| dir.join("horizon"))
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
    #[cfg(desktop_shell)]
    {
        dirs::config_dir().map(|dir| dir.join("horizon"))
    }
    #[cfg(not(desktop_shell))]
    {
        sysdirs::config_dir()
            .or_else(data_dir)
            .map(|dir| dir.join("horizon"))
    }
}

fn config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

const CONFIG_HEADER: &str = "\
# Horizon settings. The vault is the folder of Markdown files itself; this file
# lives beside the app's configuration, not inside the vault, so moving the
# folder keeps the pointer valid and the pointer never enters a commit.
";

/// One key, one line. Written with JSON quoting because that is also a valid
/// TOML basic string, so a path with spaces, quotes or backslashes survives.
fn render_config(vault: &Path) -> String {
    let quoted = serde_json::to_string(&vault.display().to_string()).unwrap_or_default();
    format!("{CONFIG_HEADER}vault = {quoted}\n")
}

/// Read the one key this file carries. Comments and unknown keys are ignored,
/// so the file can gain a second setting without a TOML dependency.
fn parse_config(text: &str) -> Option<PathBuf> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "vault" {
            continue;
        }
        let value = value.trim();
        let path = serde_json::from_str::<String>(value)
            .unwrap_or_else(|_| value.trim_matches('"').to_string());
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

fn write_config(vault: &Path) -> Result<(), String> {
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
    for sub in ["entries", "notes", "days"] {
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
        && ["kind: \"entry\"", "kind: \"note\"", "kind: \"day\""]
            .iter()
            .any(|marker| text.contains(marker))
}

fn write_store(dir: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for sub in ["entries", "notes", "days"] {
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
    for sub in ["entries", "notes", "days"] {
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
    Ok(())
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
        .map(Some)
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

/// Focus a day in whichever shape is currently on screen: the wall on desktop,
/// the agenda on mobile. Both elements can exist in the DOM; the hidden one is ignored.
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

/// Where a copy of the store goes: Downloads on desktop, the app's external
/// files directory on Android (reachable over USB), falling back to app-private.
fn copy_root() -> PathBuf {
    #[cfg(desktop_shell)]
    {
        dirs::download_dir()
            .or_else(dirs::data_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(not(desktop_shell))]
    {
        crate::android::external_files_dir()
            .or_else(data_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }
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
