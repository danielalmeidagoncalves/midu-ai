//! Migrate an Obsidian vault into a Horizon Markdown store.
//!
//! ```sh
//! cargo run -p horizon-core --example migrate_obsidian -- <vault> <store> [--dry-run]
//! ```
//!
//! Every note in the vault becomes one loose note in the store. The folder
//! names the subject — the vault's folders map one-to-one onto Horizon's
//! seventeen — the file's modification time becomes `created_at`, and the first
//! line of the note is its title: the `# Heading` when the file opens with one
//! and no other file claims the same heading, else the file name (Obsidian's
//! identity for a note, and the only thing distinguishing nine subscription
//! stubs that all open with the same `# z.ai Subscription` template). A file
//! with no writing left becomes a title-only placeholder, so an empty stub
//! still names a thing.
//!
//! Obsidian-only syntax is translated, never dropped silently:
//!
//! - `[[Note]]` links become `@name` citations (multi-word targets are
//!   kebab-cased, because Horizon's citation grammar takes one token);
//! - `[[2025-10-28]]` becomes a `^[2025-10-28]` day chip;
//! - `![[image.png]]` embeds copy the file into `<store>/assets/` and become
//!   `![name](assets/…)` links; embeds whose file is missing are removed and
//!   reported;
//! - callouts become bold blockquotes, `==highlights==` become bold, and
//!   `%%…%%` comments are removed;
//! - YAML frontmatter is dropped, but the wikilinks it carried survive as a
//!   trailing `Related: @a · @b` line.
//!
//! Journaling files are loose notes titled by their date (Horizon's wall looks
//! forward only, so a past day page would never be shown); empty ones are
//! skipped. A file is a duplicate of an earlier one when their bodies are
//! near-identical *and* their names match — the file stems, or one's heading
//! against the other's stem — keeping the first in subject order, so a stale
//! copy moved between folders disappears while `Bills` under each child does
//! not, and near-identical template stubs under different names stay.
//!
//! The store is written through `store::to_files`, so the bytes are exactly
//! what the app itself would write, then read back and re-parsed to verify.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use horizon_core::inference::Subject;
use horizon_core::model::{Doc, LooseNote};
use horizon_core::store;

/// Folder name → subject, for the vault's first path component.
fn folder_subject(folder: &str) -> Option<Subject> {
    let subject = match folder {
        "AI" | "ai-brain" => Subject::Ai,
        "Assets" => Subject::Assets,
        "Backpack" => Subject::Backpack,
        "Because Everyone Deserves to Be Happy" => Subject::Everyone,
        "Bosch" => Subject::Bosch,
        "Excalidraw" => Subject::Sketches,
        "Family" => Subject::Family,
        "Finances" => Subject::Finances,
        "House" => Subject::House,
        "Journaling" => Subject::Journalling,
        "Kids" => Subject::Kids,
        "Me" => Subject::Me,
        "Payments" => Subject::Payments,
        "Prompts" => Subject::Prompts,
        "Retrogaming" => Subject::Retro,
        "Tech" => Subject::Tech,
        "Travels" => Subject::Travels,
        _ => return None,
    };
    Some(subject)
}

/// The walking order decides which duplicate is kept: subjects in index order,
/// then paths alphabetically, so `ai-brain` wins over a copy in `Assets`.
fn subject_order(subject: Option<Subject>) -> usize {
    subject
        .and_then(|s| Subject::ALL.iter().position(|&c| c == s))
        .unwrap_or(usize::MAX)
}

/// kebab-case a name: the file-name shape Horizon uses, and the only shape its
/// `@citation` grammar accepts.
fn kebab(text: &str) -> String {
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
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// The word set behind a name, for similarity: kebab-case first so file names
/// and headings tokenize the same way.
fn name_words(text: &str) -> BTreeSet<String> {
    kebab(text)
        .split('-')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Do two word sets overlap at least `num`/`den` of their union, in exact
/// integer arithmetic (a float threshold rounds the boundary case).
fn sets_overlap(a: &BTreeSet<String>, b: &BTreeSet<String>, num: usize, den: usize) -> bool {
    let shared = a.intersection(b).count();
    let total = a.union(b).count();
    total > 0 && shared * den >= num * total
}

/// One `@name` citation, or `None` when the name cannot be cited at all.
fn citation(name: &str) -> Option<String> {
    let name = name.rsplit('/').next().unwrap_or(name).trim();
    if name.is_empty() {
        return None;
    }
    let token = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.len() >= 2;
    let cited = if token { name } else { &kebab(name) };
    (!cited.is_empty()).then(|| format!("@{cited}"))
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, &c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// A link target that looks like a file (has an extension) but is not in the
/// vault: a dead embed, removed and reported rather than kept as noise.
fn is_missing_file(base: &str, assets: &BTreeMap<String, PathBuf>) -> bool {
    base.rsplit('/').next().unwrap_or(base).contains('.') && !assets.contains_key(base)
}

/// Translate one wikilink target. `assets` maps every file name in the vault to
/// its path; matched images are recorded in `copies` for the caller to move.
fn convert_target(
    target: &str,
    assets: &BTreeMap<String, PathBuf>,
    copies: &mut BTreeMap<String, PathBuf>,
) -> Option<String> {
    let target = target.split('|').next().unwrap_or("").trim();
    let target = target.trim_matches('"');
    if target.is_empty() {
        return None;
    }
    if is_date(target) {
        return Some(format!("^[{target}]"));
    }
    let base = target.rsplit('/').next().unwrap_or(target);
    if let Some(src) = assets.get(base) {
        let stem = Path::new(base)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(base);
        let ext = Path::new(base)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let name = if ext.is_empty() {
            kebab(stem)
        } else {
            format!("{}.{}", kebab(stem), ext)
        };
        copies.entry(name.clone()).or_insert_with(|| src.clone());
        return Some(format!("![{base}](assets/{name})"));
    }
    if is_missing_file(base, assets) {
        return None;
    }
    citation(base)
}

/// Rewrite the `[[links]]` in one line of markdown. `dead` collects the
/// embeds whose files are gone, for the report.
fn convert_links(
    line: &str,
    assets: &BTreeMap<String, PathBuf>,
    copies: &mut BTreeMap<String, PathBuf>,
    dead: &mut Vec<String>,
) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find("[[") {
        let embed = open > 0 && bytes[open - 1] == b'!';
        let inside = &rest[open + 2..];
        let Some(close) = inside.find("]]") else {
            break;
        };
        let target = &inside[..close];
        out.push_str(&rest[..if embed { open - 1 } else { open }]);
        match convert_target(target, assets, copies) {
            Some(replacement) => out.push_str(&replacement),
            None => dead.push(target.trim().to_string()),
        }
        rest = &inside[close + 2..];
    }
    out.push_str(rest);
    out
}

/// `> [!abstract] **Summary**` becomes `> **Summary**`; the folding marker and
/// an already-bold label are not doubled.
fn convert_callout(line: &str) -> String {
    let trimmed = line.trim_start();
    let Some(after) = trimmed.strip_prefix("> [!") else {
        return line.to_string();
    };
    let Some((word, tail)) = after.split_once(']') else {
        return line.to_string();
    };
    if !word.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return line.to_string();
    }
    let inner = tail
        .trim_start()
        .trim_start_matches(['+', '-'])
        .trim_start()
        .trim();
    let quoted = if inner.starts_with("**") && inner.ends_with("**") && inner.len() > 4 {
        inner
    } else if inner.is_empty() {
        return "> ".to_string();
    } else {
        &format!("**{inner}**")
    };
    format!("> {quoted}")
}

/// `==highlight==` becomes `**highlight**`, pairwise per line.
fn convert_highlights(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(first) = rest.find("==") {
        let after = &rest[first + 2..];
        if let Some(second) = after.find("==") {
            out.push_str(&rest[..first]);
            out.push_str("**");
            out.push_str(&after[..second]);
            out.push_str("**");
            rest = &after[second + 2..];
        } else {
            out.push_str(&rest[..first + 2]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Obsidian escapes parentheses in headings; Horizon's parser leaves the
/// backslash in the text, so restore the character.
fn unescape_parens(text: &str) -> String {
    text.replace("\\(", "(").replace("\\)", ")")
}

/// Remove `%%…%%` comment spans, including multi-line ones.
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("%%") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        match after.find("%%") {
            Some(close) => rest = &after[close + 2..],
            // An unclosed marker comments to the end, as Obsidian treats it.
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Split an Obsidian file into its YAML frontmatter block and body. Both
/// delimiters must be alone on their line, as Obsidian writes them.
fn strip_frontmatter(text: &str) -> (Option<String>, String) {
    let text = text.trim_start_matches('\u{feff}');
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text.to_string());
    };
    let Some(end) = rest.find("\n---") else {
        return (None, text.to_string());
    };
    let after_dash = &rest[end + 4..];
    if !after_dash.is_empty() && !after_dash.starts_with('\n') {
        return (None, text.to_string());
    }
    (
        Some(rest[..end].to_string()),
        after_dash
            .strip_prefix('\n')
            .unwrap_or(after_dash)
            .to_string(),
    )
}

/// Every wikilink target in a frontmatter block, in order, deduplicated.
fn frontmatter_links(frontmatter: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut links = Vec::new();
    let mut rest = frontmatter;
    while let Some(open) = rest.find("[[") {
        let inside = &rest[open + 2..];
        let Some(close) = inside.find("]]") else {
            break;
        };
        let target = inside[..close]
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"');
        if !target.is_empty() && seen.insert(target.to_string()) {
            links.push(target.to_string());
        }
        rest = &inside[close + 2..];
    }
    links
}

/// The file's opening `# Heading`, when the writing starts with one: the
/// heading text and the body that follows it.
fn opening_heading(body: &str) -> Option<(String, String)> {
    let first = body.lines().find(|l| !l.trim().is_empty())?;
    let trimmed = first.trim();
    let heading = trimmed.strip_prefix("# ").map(str::trim)?;
    if heading.is_empty() || trimmed.starts_with("## ") {
        return None;
    }
    let rest = body.strip_prefix(first).unwrap_or(body);
    Some((
        unescape_parens(heading),
        rest.trim_start_matches('\n').to_string(),
    ))
}

/// Near-duplicate bodies: equal once whitespace collapses, or a word-set
/// overlap high enough that one is a stale copy of the other.
fn bodies_are_duplicates(a: &str, b: &str) -> bool {
    let collapse = |s: &str| {
        s.split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (na, nb) = (collapse(a), collapse(b));
    if na == nb {
        return true;
    }
    let words = |s: &str| -> BTreeSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    sets_overlap(&words(&na), &words(&nb), 4, 5)
}

/// One vault file, parsed far enough to decide duplication and titles.
#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    rel: String,
    subject: Option<Subject>,
    stem: String,
    /// The heading the body opens with, if any.
    heading: Option<String>,
    /// The body after frontmatter (comments included; they are stripped later).
    body: String,
    frontmatter: Option<String>,
    journaling: bool,
}

/// Two files are the same note when their bodies are near-identical, their
/// opening headings do not disagree, and their names match: stem against
/// stem, or one's heading against the other's stem. Identical template bodies
/// under unrelated names (nine subscription stubs) are distinct notes, and so
/// are same-named template tables whose headings name different children.
fn same_note(a: &Candidate, b: &Candidate) -> bool {
    if !bodies_are_duplicates(&a.body, &b.body) {
        return false;
    }
    let a_stem = name_words(&a.stem);
    let b_stem = name_words(&b.stem);
    let a_heading = a.heading.as_deref().map(name_words);
    let b_heading = b.heading.as_deref().map(name_words);
    let headings_agree = match (&a_heading, &b_heading) {
        (Some(a), Some(b)) => sets_overlap(a, b, 4, 5),
        _ => true,
    };
    if !headings_agree {
        return false;
    }
    sets_overlap(&a_stem, &b_stem, 1, 2)
        || a_heading
            .as_ref()
            .is_some_and(|h| sets_overlap(h, &b_stem, 4, 5))
        || b_heading
            .as_ref()
            .is_some_and(|h| sets_overlap(h, &a_stem, 4, 5))
}

struct Report {
    imported: BTreeMap<String, usize>,
    skipped: Vec<String>,
    duplicates: Vec<String>,
    dead_links: Vec<String>,
    notes: Vec<LooseNote>,
}

fn main() {
    let code = run();
    if code != 0 {
        std::process::exit(code);
    }
}

fn run() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let positional: Vec<&String> = args.iter().filter(|a| *a != "--dry-run").collect();
    let [src, dst] = match positional.as_slice() {
        [src, dst] => [src.as_str(), dst.as_str()],
        _ => {
            eprintln!("Usage: migrate_obsidian <obsidian-vault> <horizon-store> [--dry-run]");
            return 1;
        }
    };
    let (src, dst) = match (fs::canonicalize(src), fs::canonicalize(dst)) {
        (Ok(src), Ok(dst)) => (src, dst),
        _ => {
            eprintln!("Both folders must exist: {src} / {dst}");
            return 1;
        }
    };

    // Never mix a migration into a store that already holds notes.
    if let Ok(existing) = fs::read_dir(dst.join("notes")) {
        let count = existing.filter_map(|e| e.ok()).count();
        if count > 0 {
            eprintln!(
                "Refusing to write into {}/notes: {count} files already there.",
                dst.display()
            );
            return 1;
        }
    }

    let mut markdown = Vec::new();
    let mut assets: BTreeMap<String, PathBuf> = BTreeMap::new();
    collect(&src, &src, &mut markdown, &mut assets);

    // Parse every file, then order them: subjects in index order, then paths.
    // The order decides which of two duplicates is kept.
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for (path, rel) in markdown {
        let raw = match fs::read(&path) {
            Ok(raw) => raw,
            Err(e) => {
                eprintln!("Skipping {}: {e}", path.display());
                continue;
            }
        };
        let text = String::from_utf8_lossy(&raw)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let (frontmatter, body) = strip_frontmatter(&text);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("note")
            .to_string();
        let journaling = rel.starts_with("Journaling/");
        if journaling && body.trim().is_empty() {
            skipped.push(format!("{rel} (empty journal day)"));
            continue;
        }
        let subject = rel.split('/').next().and_then(folder_subject);
        let heading = if journaling {
            None
        } else {
            opening_heading(strip_comments(&body).trim()).map(|(h, _)| h)
        };
        candidates.push(Candidate {
            path,
            rel,
            subject,
            stem,
            heading,
            body,
            frontmatter,
            journaling,
        });
    }
    candidates.sort_by(|a, b| {
        subject_order(a.subject)
            .cmp(&subject_order(b.subject))
            .then_with(|| a.rel.cmp(&b.rel))
    });

    // Pass one: drop each candidate that is an earlier note's stale copy.
    let mut survivors: Vec<&Candidate> = Vec::new();
    let mut duplicates: Vec<String> = Vec::new();
    for candidate in &candidates {
        if let Some(kept) = survivors.iter().find(|&&kept| same_note(kept, candidate)) {
            duplicates.push(format!("{} (of “{}”)", candidate.rel, kept.stem));
            continue;
        }
        survivors.push(candidate);
    }

    // Pass two: titles. A heading names the note only when no other survivor
    // opens with the same one; otherwise the file name does, and the heading
    // stays as the body's first line.
    let mut heading_counts: BTreeMap<String, usize> = BTreeMap::new();
    for candidate in &survivors {
        if let Some(heading) = candidate.heading.as_deref() {
            *heading_counts.entry(kebab(heading)).or_default() += 1;
        }
    }

    let mut report = Report {
        imported: BTreeMap::new(),
        skipped,
        duplicates,
        dead_links: Vec::new(),
        notes: Vec::new(),
    };
    let mut copies: BTreeMap<String, PathBuf> = BTreeMap::new();

    for candidate in &survivors {
        // The comment pass runs before anything else: `%%…%%` never survives
        // into a note, not even as the stub after a drawing's code fence.
        let body = strip_comments(&candidate.body).trim().to_string();
        let (title, rest) = if candidate.journaling {
            (candidate.stem.clone(), body)
        } else {
            let claimed_heading = candidate
                .heading
                .as_deref()
                .is_some_and(|h| heading_counts[&kebab(h)] == 1);
            if claimed_heading {
                let (heading, rest) = opening_heading(&body).expect("counted a moment ago");
                (heading, rest)
            } else {
                (candidate.stem.clone(), body)
            }
        };

        // Translate the Obsidian syntax, line by line, after the comment pass.
        let mut converted = String::new();
        for line in rest.lines() {
            let line = unescape_parens(&convert_highlights(&convert_callout(line)));
            let line = convert_links(&line, &assets, &mut copies, &mut report.dead_links);
            converted.push_str(&line);
            converted.push('\n');
        }

        let mut text = if rest.trim().is_empty() {
            title
        } else {
            format!("{title}\n\n{}", converted.trim_end())
        };
        if let Some(frontmatter) = candidate.frontmatter.as_deref() {
            let related: Vec<String> = frontmatter_links(frontmatter)
                .iter()
                .filter_map(|target| {
                    if is_date(target) {
                        Some(format!("^[{target}]"))
                    } else {
                        citation(target)
                    }
                })
                .collect();
            if !related.is_empty() {
                text.push_str("\n\nRelated: ");
                text.push_str(&related.join(" · "));
            }
        }

        let created = fs::metadata(&candidate.path)
            .and_then(|m| m.modified())
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        let mut note = LooseNote::new_at(&text, candidate.subject, created);
        if let Some(subject) = candidate.subject {
            note.subject = subject;
        }
        *report
            .imported
            .entry(note.subject.name().to_string())
            .or_default() += 1;
        report.notes.push(note);
    }

    println!(
        "Migrating {} notes into {}",
        report.notes.len(),
        dst.display()
    );
    for (subject, count) in &report.imported {
        println!("  {subject:<40} {count}");
    }

    // Write through the store's own serializer, minus this device's state.
    let mut doc = Doc::empty();
    doc.notes = report.notes.clone();
    let files = store::to_files(&doc);
    if !dry_run {
        for (path, contents) in &files {
            if path == "state.md" {
                continue;
            }
            let target = dst.join(path);
            if let Some(parent) = target.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("Cannot create {}: {e}", parent.display());
                    return 1;
                }
            }
            if let Err(e) = fs::write(&target, contents) {
                eprintln!("Cannot write {}: {e}", target.display());
                return 1;
            }
        }
        for (name, source) in &copies {
            let target = dst.join("assets").join(name);
            fs::create_dir_all(target.parent().expect("assets has a parent")).ok();
            if let Err(e) = fs::copy(source, &target) {
                eprintln!("Cannot copy asset {}: {e}", source.display());
                return 1;
            }
            println!("  asset       {name}");
        }
    }

    // Read the store back and re-parse it, exactly as the app will.
    if !dry_run {
        let mut written_map = BTreeMap::new();
        let mut walk = vec![dst.clone()];
        while let Some(dir) = walk.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!("Cannot read {}: {e}", dir.display());
                    return 1;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().and_then(|n| n.to_str()) != Some(".git") {
                        walk.push(path);
                    }
                    continue;
                }
                let rel = path
                    .strip_prefix(&dst)
                    .expect("store-relative")
                    .to_string_lossy()
                    .replace('\\', "/");
                let contents = fs::read_to_string(&path).unwrap_or_default();
                written_map.insert(rel, contents);
            }
        }
        match store::from_files(&written_map, Utc::now().date_naive()) {
            Ok(loaded) if loaded.notes.len() == report.notes.len() => {
                println!(
                    "Verified: the store reloads with {} notes.",
                    loaded.notes.len()
                );
            }
            Ok(loaded) => {
                eprintln!(
                    "Verification failed: {} written, {} reload.",
                    report.notes.len(),
                    loaded.notes.len()
                );
                return 1;
            }
            Err(e) => {
                eprintln!("Verification failed: {e}");
                return 1;
            }
        }
    } else {
        println!(
            "Dry run: nothing written ({} files would be).",
            files.len() - 1
        );
    }

    if !report.duplicates.is_empty() {
        println!("\nDuplicates skipped (kept the first in subject order):");
        for line in &report.duplicates {
            println!("  {line}");
        }
    }
    if !report.skipped.is_empty() {
        println!("\nSkipped:");
        for line in &report.skipped {
            println!("  {line}");
        }
    }
    let mut dead: BTreeSet<String> = report.dead_links.iter().cloned().collect();
    dead.remove("");
    if !dead.is_empty() {
        println!("\nEmbeds removed (file missing from the vault):");
        for link in &dead {
            println!("  {link}");
        }
    }
    0
}

/// Collect the vault: markdown files as (path, vault-relative path), every
/// other file by name for embed resolution. `.obsidian`, `.git` and other
/// dot-folders are left alone, as are dot-files.
fn collect(
    root: &Path,
    dir: &Path,
    markdown: &mut Vec<(PathBuf, String)>,
    assets: &mut BTreeMap<String, PathBuf>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, markdown, assets);
        } else if name.ends_with(".md") {
            let rel = path
                .strip_prefix(root)
                .expect("below the vault")
                .to_string_lossy()
                .replace('\\', "/");
            markdown.push((path, rel));
        } else {
            assets.entry(name.to_string()).or_insert(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_map_to_their_subject() {
        assert_eq!(folder_subject("Journaling"), Some(Subject::Journalling));
        assert_eq!(folder_subject("Retrogaming"), Some(Subject::Retro));
        assert_eq!(
            folder_subject("Because Everyone Deserves to Be Happy"),
            Some(Subject::Everyone)
        );
        assert_eq!(folder_subject("ai-brain"), Some(Subject::Ai));
        assert_eq!(folder_subject("Elsewhere"), None);
    }

    #[test]
    fn multi_word_targets_kebab_into_one_citation_token() {
        assert_eq!(citation("tolaria").as_deref(), Some("@tolaria"));
        assert_eq!(
            citation("EV Battery Fundamentals").as_deref(),
            Some("@ev-battery-fundamentals")
        );
        assert_eq!(
            citation("LFP Battery: BYD Blade vs CATL").as_deref(),
            Some("@lfp-battery-byd-blade-vs-catl")
        );
        assert_eq!(citation("Prompts/Prompts").as_deref(), Some("@Prompts"));
    }

    #[test]
    fn links_translate_to_citations_chips_and_assets() {
        let mut assets = BTreeMap::new();
        let png = PathBuf::from("/vault/Pasted image 20251104223128.png");
        assets.insert("Pasted image 20251104223128.png".into(), png.clone());
        let mut copies = BTreeMap::new();
        let mut dead = Vec::new();

        let line = "see [[EV Battery Fundamentals]] and [[2025-10-28]]";
        let out = convert_links(line, &assets, &mut copies, &mut dead);
        assert_eq!(out, "see @ev-battery-fundamentals and ^[2025-10-28]");
        assert!(dead.is_empty());

        let out = convert_links(
            "![[Pasted image 20251104223128.png]]",
            &assets,
            &mut copies,
            &mut dead,
        );
        assert_eq!(
            out,
            "![Pasted image 20251104223128.png](assets/pasted-image-20251104223128.png)"
        );
        assert_eq!(copies.get("pasted-image-20251104223128.png"), Some(&png));

        convert_links(
            "![[CamScanner 27-01-2026 00.21.pdf]]",
            &assets,
            &mut copies,
            &mut dead,
        );
        assert_eq!(dead, vec!["CamScanner 27-01-2026 00.21.pdf"]);
    }

    #[test]
    fn callouts_and_highlights_and_escapes_become_horizon_markdown() {
        assert_eq!(
            convert_callout("> [!abstract] **Summary**"),
            "> **Summary**"
        );
        assert_eq!(
            convert_callout("> [!summary]+ Shortlist"),
            "> **Shortlist**"
        );
        assert_eq!(convert_highlights("a ==keep== b"), "a **keep** b");
        assert_eq!(unescape_parens(r"BYD \(All Blade\)"), "BYD (All Blade)");
    }

    #[test]
    fn comments_are_removed_and_frontmatter_is_split() {
        assert_eq!(strip_comments("a %% hidden %% b"), "a  b");
        // A comment block leaves the blank line that surrounded it.
        assert_eq!(strip_comments("one\n%%\nblock\n%%\ntwo"), "one\n\ntwo");
        assert_eq!(strip_comments("text\n%%"), "text\n");
        let (fm, body) = strip_frontmatter("---\ntype: Topic\n---\n# Title\n\nBody");
        assert_eq!(fm.as_deref(), Some("type: Topic"));
        assert_eq!(body, "# Title\n\nBody");
        let (fm, body) = strip_frontmatter("# Just a note");
        assert!(fm.is_none());
        assert_eq!(body, "# Just a note");
    }

    #[test]
    fn an_opening_heading_names_the_note() {
        let (heading, rest) = opening_heading("# BYD Lineup\n\nBody").unwrap();
        assert_eq!(heading, "BYD Lineup");
        assert_eq!(rest, "Body");
        assert!(opening_heading("Massa com atum").is_none());
        assert!(opening_heading("## Sub\n\nBody").is_none());
    }

    #[test]
    fn stale_copies_are_duplicates_but_template_stubs_are_not() {
        // One note copied between folders: same body (modulo a typo), stems
        // and heading that match.
        let older = Candidate {
            path: PathBuf::from("/v/Assets/LFP Battery BYD Blade vs CATL.md"),
            rel: "Assets/LFP Battery BYD Blade vs CATL.md".into(),
            subject: Some(Subject::Assets),
            stem: "LFP Battery BYD Blade vs CATL".into(),
            heading: Some("LFP Battery: BYD Blade vs CATL".into()),
            body: "# LFP Battery: BYD Blade vs CATL\n\nComparison of the two dominant LFP makers"
                .into(),
            frontmatter: None,
            journaling: false,
        };
        let newer = Candidate {
            path: PathBuf::from("/v/ai-brain/ai-brain/lfp-battery-byd-vs-catl.md"),
            rel: "ai-brain/ai-brain/lfp-battery-byd-vs-catl.md".into(),
            subject: Some(Subject::Ai),
            stem: "lfp-battery-byd-vs-catl".into(),
            heading: Some("LFP Battery: BYD Blade vs CATL".into()),
            body: "# LFP Battery: BYD Blade vs CATL\n\nComparison of the two dominant LFP makerz"
                .into(),
            frontmatter: Some("type: Topic".into()),
            journaling: false,
        };
        assert!(same_note(&older, &newer));

        // Nine subscription stubs share one template heading and body, but
        // their file names are unrelated: distinct notes.
        let netflix = Candidate {
            path: PathBuf::from("/v/Payments/Netflix.md"),
            rel: "Payments/Netflix.md".into(),
            subject: Some(Subject::Payments),
            stem: "Netflix".into(),
            heading: Some("z.ai Subscription".into()),
            body: "# z.ai Subscription\n\n- Provider: z.ai".into(),
            frontmatter: None,
            journaling: false,
        };
        let apple = Candidate {
            path: PathBuf::from("/v/Payments/Apple.md"),
            rel: "Payments/Apple.md".into(),
            subject: Some(Subject::Payments),
            stem: "Apple".into(),
            heading: Some("z.ai Subscription".into()),
            body: "# z.ai Subscription\n\n- Provider: z.ai".into(),
            frontmatter: None,
            journaling: false,
        };
        assert!(!same_note(&netflix, &apple));

        // Same file name, different writing: `Bills` under each child.
        let mut miguel = netflix.clone();
        miguel.stem = "Bills".into();
        miguel.body = "* Colégio\n* Natação\n* Beira Mar".into();
        miguel.heading = None;
        let mut duarte = netflix.clone();
        duarte.stem = "Bills".into();
        duarte.body = "* CIAQ\n* Natação".into();
        duarte.heading = None;
        assert!(!same_note(&duarte, &miguel));

        // Same file name, near-identical template tables, but the headings
        // name different children: `Schedule` under each stays whole.
        let mut duarte_schedule = netflix.clone();
        duarte_schedule.stem = "Schedule".into();
        duarte_schedule.heading = Some("📅 Duarte's Weekly Schedule".into());
        duarte_schedule.body =
            "# 📅 Duarte's Weekly Schedule\n| Time | Mon | Tue |\n| Natação | 17:00 | — |".into();
        let mut miguel_schedule = netflix.clone();
        miguel_schedule.stem = "Schedule".into();
        miguel_schedule.heading = Some("📅 Miguel's Weekly Schedule".into());
        miguel_schedule.body =
            "# 📅 Miguel's Weekly Schedule\n| Time | Mon | Tue |\n| Natação | 18:00 | — |".into();
        assert!(!same_note(&duarte_schedule, &miguel_schedule));

        // One note is the other's revision and the heading names the copy:
        // the heading-to-stem match keeps the pair deduplicated.
        let core = "The shortlist of LFP battery electric vehicles sold in Portugal, distilled from the full research, with battery sizes, ranges, prices, warranty terms and dealership notes.";
        let mut revision = netflix.clone();
        revision.stem = "ev-suv-shortlist-portugal".into();
        revision.heading = Some("My Next EV Car".into());
        revision.body = format!("# My Next EV Car\n\n{core}");
        let mut original = netflix.clone();
        original.stem = "My Next EV Car".into();
        original.heading = None;
        original.body = core.into();
        assert!(same_note(&revision, &original));

        // A template heading that happens to contain another note's file name
        // (`Z.ai` inside `# z.ai Subscription`) is not a match: the stubs stay.
        let mut z_ai = netflix.clone();
        z_ai.stem = "Z.ai".into();
        assert!(!same_note(&z_ai, &apple));
    }

    #[test]
    fn frontmatter_links_survive_extraction() {
        let fm = "type: Topic\nRelated to:\n  - \"[[Buy an Electric Car]]\"\n  - \"[[EV Battery Fundamentals]]\"";
        assert_eq!(
            frontmatter_links(fm),
            vec!["Buy an Electric Car", "EV Battery Fundamentals"]
        );
    }
}
