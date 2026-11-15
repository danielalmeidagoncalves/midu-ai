//! Plain-Markdown file store.
//!
//! One file per entry, note and day, plus `state.md`. The serialized shape is
//! deliberately small: YAML-style frontmatter with quoted strings and a Markdown
//! body. The core stays pure; the app supplies the directory and does the I/O.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use chrono::NaiveDate;

use crate::dates;
use crate::inference::{infer_mark, infer_subject, Mark, Subject};
use crate::model::{new_id, Doc, DrawMode, Entry, LooseNote, PageMeta, UiState, DOC_VERSION};

pub const STORE_VERSION: u32 = 2;

#[derive(Debug)]
pub enum StoreError {
    BadFrontmatter(String),
    BadValue(String),
    UnsupportedVersion(u32),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::BadFrontmatter(file) => write!(f, "Bad Markdown frontmatter in {file}."),
            StoreError::BadValue(file) => write!(f, "Bad value in {file}."),
            StoreError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "This store uses version {v}; this Horizon understands {STORE_VERSION}."
                )
            }
        }
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
        if out.len() >= 40 {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "entry".into()
    } else {
        out
    }
}

/// The tail of an id, used to keep file names short. `len` widens it when two
/// objects would otherwise land on the same path.
fn short_id(id: &str, len: usize) -> &str {
    let tail = id.rsplit_once('-').map(|(_, tail)| tail).unwrap_or(id);
    &tail[..tail.len().min(len)]
}

/// A legacy v1 file carried its position in the name: `entries/<day>-<NNN>-…`
/// and `notes/<NNN>-…`. The position is only used to keep the old read order
/// when an object has no `created_at`.
fn legacy_index(path: &str) -> Option<u32> {
    let name = path.rsplit('/').next()?.strip_suffix(".md")?;
    let rest = if path.starts_with("notes/") {
        name
    } else if path.starts_with("entries/") {
        // The day prefix is `YYYY-MM-DD` or `undated`; the index followed it.
        let day_len = if name.as_bytes().get(4) == Some(&b'-') {
            10
        } else {
            7
        };
        name.get(day_len..)?.strip_prefix('-')?
    } else {
        return None;
    };
    let (digits, tail) = rest.split_at_checked(3)?;
    if !tail.starts_with('-') || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn frontmatter(fields: &[(&str, String)]) -> String {
    let mut out = String::from("---\n");
    for (key, value) in fields {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
    out.push_str("---\n");
    out
}

fn state_file(ui: &UiState) -> String {
    let note_edit = serde_json::to_string(&ui.note_edit).unwrap_or_else(|_| "{}".into());
    let draw_mode = match ui.draw_mode {
        DrawMode::Marks => "marks",
        DrawMode::Words => "words",
    };
    frontmatter(&[
        ("kind", quote("state")),
        ("version", STORE_VERSION.to_string()),
        ("draw_mode", quote(draw_mode)),
        (
            "lens",
            ui.lens
                .map(|s| quote(s.id()))
                .unwrap_or_else(|| "null".into()),
        ),
        (
            "open_day",
            ui.open_day
                .as_deref()
                .map(quote)
                .unwrap_or_else(|| "null".into()),
        ),
        ("note_edit", quote(&note_edit)),
    ])
}

fn entry_file(entry: &Entry) -> String {
    let mut md = frontmatter(&[
        ("kind", quote("entry")),
        ("version", STORE_VERSION.to_string()),
        ("id", quote(&entry.id)),
        ("day", quote(&entry.day)),
        ("title", quote(&entry.text)),
        ("mark", quote(entry.mark.id())),
        ("subject", quote(entry.subject.id())),
        ("weight", entry.weight().to_string()),
        ("done", entry.done.to_string()),
        ("mark_auto", entry.mark_auto.to_string()),
        ("subject_auto", entry.subject_auto.to_string()),
        ("repeat_weekly", entry.repeat_weekly.to_string()),
        ("created_at", quote(&entry.created_at)),
    ]);
    if entry.has_note() {
        md.push('\n');
        md.push_str(entry.note_text());
        if !md.ends_with('\n') {
            md.push('\n');
        }
    }
    md
}

fn note_file(note: &LooseNote) -> String {
    let mut md = frontmatter(&[
        ("kind", quote("note")),
        ("version", STORE_VERSION.to_string()),
        ("id", quote(&note.id)),
        ("created_at", quote(&note.created_at)),
        ("mark", quote(note.mark.id())),
        ("subject", quote(note.subject.id())),
        (
            "day",
            note.day
                .as_deref()
                .map(quote)
                .unwrap_or_else(|| "null".into()),
        ),
    ]);
    md.push('\n');
    md.push_str(&note.text);
    if !md.ends_with('\n') {
        md.push('\n');
    }
    md
}

fn day_file(day: &str, meta: Option<&PageMeta>, page: &str) -> String {
    let first_written = meta
        .map(|m| m.first_written.as_str())
        .filter(|v| !v.is_empty())
        .unwrap_or(day);
    let mut md = frontmatter(&[
        ("kind", quote("day")),
        ("version", STORE_VERSION.to_string()),
        ("day", quote(day)),
        ("first_written", quote(first_written)),
    ]);
    md.push('\n');
    md.push_str(page);
    if !md.ends_with('\n') {
        md.push('\n');
    }
    md
}

fn path_day(day: &str) -> String {
    if dates::parse_key(day).is_some() {
        day.to_string()
    } else {
        "undated".to_string()
    }
}

/// Names carry no position: adding or removing one thing never renames another,
/// so a synced folder (and its history) stays quiet.
fn entry_path(entry: &Entry, id_len: usize) -> String {
    format!(
        "entries/{}-{}-{}.md",
        path_day(&entry.day),
        slug(&entry.text),
        short_id(&entry.id, id_len)
    )
}

fn note_path(note: &LooseNote, id_len: usize) -> String {
    // A note's body can be many lines now; the name comes from the first, the
    // same line the margin shows, so a long body cannot grow a hyphen chain.
    let title = note.text.lines().next().unwrap_or("");
    format!("notes/{}-{}.md", slug(title), short_id(&note.id, id_len))
}

/// Widen the short id until the path is free. Two objects can only collide when
/// their ids share a tail; the full id is always unique.
fn unique_path(make: impl Fn(usize) -> String, id: &str, taken: &mut BTreeSet<String>) -> String {
    let mut len = 8;
    while len < id.len() {
        let path = make(len);
        if !taken.contains(&path) {
            taken.insert(path.clone());
            return path;
        }
        len += 4;
    }
    // Identical ids should never happen; if they do, keep both files rather
    // than letting the map silently drop one.
    let base = make(id.len());
    let stem = base.strip_suffix(".md").unwrap_or(&base);
    let mut path = base.clone();
    let mut n = 2;
    while taken.contains(&path) {
        path = format!("{stem}-{n}.md");
        n += 1;
    }
    taken.insert(path.clone());
    path
}

/// The complete set of files for a document. Paths are relative and safe.
pub fn to_files(doc: &Doc) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut taken = BTreeSet::new();
    files.insert("state.md".into(), state_file(&doc.ui));
    for entry in &doc.entries {
        let path = unique_path(|len| entry_path(entry, len), &entry.id, &mut taken);
        files.insert(path, entry_file(entry));
    }
    for note in &doc.notes {
        let path = unique_path(|len| note_path(note, len), &note.id, &mut taken);
        files.insert(path, note_file(note));
    }
    for (day, page) in &doc.pages {
        if page.trim().is_empty() || dates::parse_key(day).is_none() {
            continue;
        }
        files.insert(
            format!("days/{day}.md"),
            day_file(day, doc.page_meta.get(day), page),
        );
    }
    files
}

fn parse_frontmatter(src: &str) -> Result<(BTreeMap<String, String>, String), StoreError> {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let rest = src
        .strip_prefix("---\n")
        .ok_or_else(|| StoreError::BadFrontmatter("missing opening ---".into()))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| StoreError::BadFrontmatter("missing closing ---".into()))?;
    let head = &rest[..end];
    let mut body = &rest[end + 5..];
    body = body.strip_prefix('\n').unwrap_or(body);

    let mut fields = BTreeMap::new();
    for line in head.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (key, raw) = line
            .split_once(": ")
            .ok_or_else(|| StoreError::BadFrontmatter(line.into()))?;
        let value = if raw.starts_with('"') {
            serde_json::from_str::<String>(raw)
                .map_err(|_| StoreError::BadFrontmatter(key.into()))?
        } else {
            raw.to_string()
        };
        fields.insert(key.to_string(), value);
    }
    Ok((fields, body.to_string()))
}

fn string(fields: &BTreeMap<String, String>, key: &str) -> Option<String> {
    fields
        .get(key)
        .filter(|v| !v.is_empty() && v.as_str() != "null")
        .cloned()
}

fn boolean(fields: &BTreeMap<String, String>, key: &str, fallback: bool) -> bool {
    match fields.get(key).map(String::as_str) {
        Some("true") => true,
        Some("false") => false,
        _ => fallback,
    }
}

fn number(fields: &BTreeMap<String, String>, key: &str, fallback: u8) -> u8 {
    fields
        .get(key)
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(fallback)
}

fn kind(fields: &BTreeMap<String, String>) -> Option<&str> {
    fields.get("kind").map(String::as_str)
}

fn check_version(fields: &BTreeMap<String, String>, file: &str) -> Result<(), StoreError> {
    let version = fields
        .get("version")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(STORE_VERSION);
    if version > STORE_VERSION {
        return Err(StoreError::UnsupportedVersion(version));
    }
    if version == 0 {
        return Err(StoreError::BadValue(file.into()));
    }
    Ok(())
}

fn parse_state(fields: &BTreeMap<String, String>, file: &str) -> Result<UiState, StoreError> {
    check_version(fields, file)?;
    let draw_mode = match fields.get("draw_mode").map(String::as_str) {
        Some("words") => DrawMode::Words,
        _ => DrawMode::Marks,
    };
    let lens = string(fields, "lens").and_then(|v| Subject::from_id(&v));
    let open_day = string(fields, "open_day").filter(|d| dates::parse_key(d).is_some());
    let note_edit = string(fields, "note_edit")
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();
    Ok(UiState {
        draw_mode,
        lens,
        open_day,
        note_edit,
    })
}

fn parse_entry(
    fields: &BTreeMap<String, String>,
    body: &str,
    file: &str,
    today: NaiveDate,
) -> Result<Entry, StoreError> {
    check_version(fields, file)?;
    let mut body = body.to_string();
    let title = match string(fields, "title") {
        Some(title) => title,
        None => {
            if let Some(rest) = body.strip_prefix("# ") {
                let (title, rest) = rest
                    .split_once('\n')
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .unwrap_or_else(|| (rest.to_string(), String::new()));
                body = rest;
                title
            } else {
                return Err(StoreError::BadValue(file.into()));
            }
        }
    };
    if title.trim().is_empty() {
        return Err(StoreError::BadValue(file.into()));
    }
    let id = string(fields, "id").unwrap_or_else(|| new_id("e"));
    let day = string(fields, "day")
        .filter(|d| dates::parse_key(d).is_some())
        .unwrap_or_else(|| dates::key(today));
    let mark = Mark::parse_or_thing(fields.get("mark").map(String::as_str).unwrap_or(""));
    let subject = Subject::parse_or_me(fields.get("subject").map(String::as_str).unwrap_or(""));
    let mark_auto = boolean(fields, "mark_auto", true);
    let subject_auto = boolean(fields, "subject_auto", true);
    let note = {
        let trimmed = body.trim_end();
        (!trimmed.trim().is_empty()).then(|| trimmed.to_string())
    };
    let inferred_mark = infer_mark(&title);
    let inferred_subject = infer_subject(&title);
    Ok(Entry {
        id,
        day,
        text: title,
        weight: number(fields, "weight", 1).clamp(1, 3),
        done: boolean(fields, "done", false),
        mark: if mark_auto { inferred_mark } else { mark },
        mark_auto,
        subject: if subject_auto {
            inferred_subject
        } else {
            subject
        },
        subject_auto,
        note,
        created_at: string(fields, "created_at").unwrap_or_default(),
        repeat_weekly: boolean(fields, "repeat_weekly", false),
    })
}

fn parse_note(
    fields: &BTreeMap<String, String>,
    body: &str,
    file: &str,
) -> Result<LooseNote, StoreError> {
    check_version(fields, file)?;
    let text = body.trim_end().to_string();
    if text.trim().is_empty() {
        return Err(StoreError::BadValue(file.into()));
    }
    let day = string(fields, "day").filter(|d| dates::parse_key(d).is_some());
    Ok(LooseNote {
        id: string(fields, "id").unwrap_or_else(|| new_id("n")),
        text,
        day,
        mark: Mark::parse_or_thing(fields.get("mark").map(String::as_str).unwrap_or("")),
        subject: Subject::parse_or_me(fields.get("subject").map(String::as_str).unwrap_or("")),
        created_at: string(fields, "created_at").unwrap_or_default(),
    })
}

fn parse_day(
    fields: &BTreeMap<String, String>,
    body: &str,
    file: &str,
    fallback_day: &str,
) -> Result<(String, String, PageMeta), StoreError> {
    check_version(fields, file)?;
    let day = string(fields, "day")
        .filter(|d| dates::parse_key(d).is_some())
        .or_else(|| dates::parse_key(fallback_day).map(|_| fallback_day.to_string()))
        .ok_or_else(|| StoreError::BadValue(file.into()))?;
    let first_written = string(fields, "first_written")
        .filter(|d| dates::parse_key(d).is_some())
        .unwrap_or_else(|| day.clone());
    let page = body.trim_end().to_string();
    if page.trim().is_empty() {
        return Err(StoreError::BadValue(file.into()));
    }
    Ok((day, page, PageMeta { first_written }))
}

/// Rebuild a document from the file map. Files without frontmatter are ignored
/// so a README or an unrelated note can live beside the store.
pub fn from_files(files: &BTreeMap<String, String>, today: NaiveDate) -> Result<Doc, StoreError> {
    let mut doc = Doc::empty();
    let mut entries: Vec<(String, u32, String, Entry)> = Vec::new();
    let mut notes: Vec<(String, u32, String, LooseNote)> = Vec::new();
    for (path, contents) in files {
        if !contents.starts_with("---\n") {
            continue;
        }
        let (fields, body) = parse_frontmatter(contents)?;
        let Some(kind) = kind(&fields) else {
            continue;
        };
        match kind {
            "state" if path == "state.md" => {
                doc.ui = parse_state(&fields, path)?;
            }
            "entry" if path.starts_with("entries/") => {
                let entry = parse_entry(&fields, &body, path, today)?;
                let index = legacy_index(path).unwrap_or(u32::MAX);
                entries.push((entry.created_at.clone(), index, entry.id.clone(), entry));
            }
            "note" if path.starts_with("notes/") => {
                let note = parse_note(&fields, &body, path)?;
                // v1 notes had no `created_at`; their name index was newest-first,
                // so inverting it keeps the old reading order.
                let index = legacy_index(path).map_or(0, |i| u32::MAX - i);
                notes.push((note.created_at.clone(), index, note.id.clone(), note));
            }
            "day" if path.starts_with("days/") => {
                let fallback = path
                    .strip_prefix("days/")
                    .and_then(|p| p.strip_suffix(".md"))
                    .unwrap_or("");
                let (day, page, meta) = parse_day(&fields, &body, path, fallback)?;
                doc.pages.insert(day.clone(), page);
                doc.page_meta.insert(day, meta);
            }
            _ => {}
        }
    }
    // Reading order never depends on the filename. Entries read oldest-first
    // within their day; notes read newest-first, as the margins always have.
    entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    notes.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)));
    doc.entries = entries.into_iter().map(|(_, _, _, e)| e).collect();
    doc.notes = notes.into_iter().map(|(_, _, _, n)| n).collect();
    doc.version = DOC_VERSION;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::note_key_entry;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn doc() -> Doc {
        let mut doc = Doc::empty();
        let mut entry = Entry::new_with("2026-11-04", "Call mum", Some(Subject::Family), None);
        entry.set_note("- [ ] ask about Sunday".into());
        entry.weight = 2;
        entry.repeat_weekly = true;
        let entry_id = entry.id.clone();
        doc.entries.push(entry);
        doc.notes
            .push(LooseNote::new("Ask the bond consultant", None));
        doc.notes.push(LooseNote {
            day: Some("2026-11-04".into()),
            ..LooseNote::new("Passport expires 2029", None)
        });
        doc.set_page("2026-11-04", "# Today\n\nThree things.", d(2026, 11, 2));
        doc.ui.draw_mode = DrawMode::Words;
        doc.ui.lens = Some(Subject::Family);
        doc.ui.open_day = Some("2026-11-04".into());
        doc.ui.note_edit.insert(note_key_entry(&entry_id), true);
        doc
    }

    #[test]
    fn round_trip_preserves_every_thing() {
        let doc = doc();
        let files = to_files(&doc);
        let back = from_files(&files, d(2026, 11, 4)).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].text, "Call mum");
        assert_eq!(back.entries[0].note_text(), "- [ ] ask about Sunday");
        assert_eq!(back.entries[0].weight, 2);
        assert!(back.entries[0].repeat_weekly);
        assert_eq!(back.notes.len(), 2);
        assert_eq!(back.page("2026-11-04"), "# Today\n\nThree things.");
        assert_eq!(back.page_meta["2026-11-04"].first_written, "2026-11-02");
        assert_eq!(back.ui.draw_mode, DrawMode::Words);
        assert_eq!(back.ui.lens, Some(Subject::Family));
        assert_eq!(back.ui.open_day.as_deref(), Some("2026-11-04"));
        assert_eq!(back.ui.note_edit.len(), 1);
    }

    #[test]
    fn an_undated_note_keeps_its_whole_body_and_first_line_name() {
        let mut doc = Doc::empty();
        let mut note = LooseNote::new("Bike service", Some(Subject::Tech));
        note.text = "Bike service:\n- [ ] check chain\n- [ ] order brake pads".into();
        doc.notes.push(note);
        let files = to_files(&doc);
        assert!(files
            .keys()
            .any(|k| k.starts_with("notes/bike-service-") && !k.contains("-check-chain")));
        let back = from_files(&files, d(2026, 11, 4)).unwrap();
        let note = back.loose_notes().next().unwrap();
        assert_eq!(
            note.text,
            "Bike service:\n- [ ] check chain\n- [ ] order brake pads"
        );
        assert_eq!(note.subject, Subject::Tech);
        assert!(note.day.is_none());
    }

    #[test]
    fn paths_are_readable_and_stable() {
        let files = to_files(&doc());
        assert!(files.contains_key("state.md"));
        assert!(files.contains_key("days/2026-11-04.md"));
        assert!(files
            .keys()
            .any(|k| k.starts_with("entries/2026-11-04-call-mum-")));
        assert!(files.keys().any(|k| k.starts_with("notes/ask-the-bond-")));
        assert!(files
            .keys()
            .any(|k| k.starts_with("notes/passport-expires-")));
        assert!(!files.keys().any(|k| k.contains("-001-")));
    }

    #[test]
    fn adding_a_thing_does_not_rename_another() {
        let mut doc = doc();
        let before: Vec<String> = to_files(&doc)
            .into_keys()
            .filter(|k| k.starts_with("entries/"))
            .collect();
        doc.entries.insert(0, Entry::new("2026-11-04", "Buy milk"));
        let after: Vec<String> = to_files(&doc)
            .into_keys()
            .filter(|k| k.starts_with("entries/"))
            .collect();
        assert!(before.iter().all(|path| after.contains(path)));
    }

    #[test]
    fn a_shared_id_tail_widens_instead_of_overwriting() {
        let mut doc = Doc::empty();
        let mut a = Entry::new("2026-11-04", "Call mum");
        a.id = "e-11111111".into();
        let mut b = Entry::new("2026-11-04", "Call mum");
        b.id = "e-11111111".into();
        doc.entries.push(a);
        doc.entries.push(b);
        let files = to_files(&doc);
        assert_eq!(
            files.keys().filter(|k| k.starts_with("entries/")).count(),
            2
        );
        assert!(files.keys().any(|k| k.ends_with("-2.md")));
    }

    #[test]
    fn order_comes_from_created_at_not_the_filename() {
        let mut doc = Doc::empty();
        let mut late = Entry::new_with(
            "2026-11-04",
            "Second",
            None,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            ),
        );
        late.id = "e-zzzzzzzz".into();
        let mut early = Entry::new_with(
            "2026-11-04",
            "First",
            None,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-04T09:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            ),
        );
        early.id = "e-aaaaaaaa".into();
        doc.entries.push(late);
        doc.entries.push(early);
        let back = from_files(&to_files(&doc), d(2026, 11, 4)).unwrap();
        assert_eq!(back.entries[0].text, "First");
        assert_eq!(back.entries[1].text, "Second");
    }

    #[test]
    fn a_version_one_store_still_reads_in_its_old_order() {
        let mut files = BTreeMap::new();
        for (index, text) in [(1, "Newest"), (2, "Older"), (3, "Oldest")] {
            files.insert(
                format!("notes/{index:03}-note-{index}.md"),
                format!(
                    "---\nkind: \"note\"\nversion: 1\nid: \"n-{index}\"\nmark: \"thing\"\nsubject: \"me\"\nday: null\n---\n\n{text}\n"
                ),
            );
        }
        files.insert(
            "entries/2026-11-04-001-call-mum-aaaaaaaa.md".into(),
            "---\nkind: \"entry\"\nversion: 1\nid: \"e-aaaaaaaa\"\nday: \"2026-11-04\"\ntitle: \"Call mum\"\n---\n".into(),
        );
        let doc = from_files(&files, d(2026, 11, 4)).unwrap();
        assert_eq!(
            doc.notes
                .iter()
                .map(|n| n.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Newest", "Older", "Oldest"]
        );
        assert_eq!(doc.entries[0].text, "Call mum");
    }

    #[test]
    fn unrelated_files_and_readmes_are_ignored() {
        let mut files = BTreeMap::new();
        files.insert("README.md".into(), "# Not ours\n".into());
        files.insert("notes/reading-list.md".into(), "A book list\n".into());
        let doc = from_files(&files, d(2026, 11, 4)).unwrap();
        assert!(doc.entries.is_empty());
        assert!(doc.notes.is_empty());
    }

    #[test]
    fn unsupported_version_is_refused() {
        let mut files = BTreeMap::new();
        files.insert(
            "entries/x.md".into(),
            "---\nkind: \"entry\"\nversion: 99\nid: \"e-1\"\nday: \"2026-11-04\"\ntitle: \"x\"\n---\n".into(),
        );
        assert!(matches!(
            from_files(&files, d(2026, 11, 4)),
            Err(StoreError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn automatic_fields_reinfer_on_read() {
        let mut files = BTreeMap::new();
        files.insert(
            "entries/x.md".into(),
            "---\nkind: \"entry\"\nversion: 1\nid: \"e-1\"\nday: \"2026-11-04\"\ntitle: \"Dentist\"\nmark: \"money\"\nsubject: \"house\"\nweight: 1\ndone: false\nmark_auto: true\nsubject_auto: true\nrepeat_weekly: false\ncreated_at: \"\"\n---\n".into(),
        );
        let doc = from_files(&files, d(2026, 11, 4)).unwrap();
        assert_eq!(doc.entries[0].mark, Mark::Health);
        assert_eq!(doc.entries[0].subject, Subject::Me);
    }
}
