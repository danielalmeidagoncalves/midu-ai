---
type: data-model
title: Document model, versioning and local persistence
description: The Doc schema, Entry and LooseNote shapes, the one-file-per-entry/note/day Markdown store, state.md, JSON migration, the coalescing saver, and the forward-rolling window ticker.
tags: [data-model, persistence, markdown, migration, git, sync]
sources:
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-c1bc075dfb9585127c44a844
    resource: repo://horizon-app/src/components/brief.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-09ff909661761d256f1251fc
    resource: repo://horizon-core/src/export.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-f2a3b3f3537f8099cca7944f
    resource: repo://horizon-core/src/store.rs
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Document model, versioning and local persistence

Horizon keeps one in-memory object, `Doc`, and persists it as a folder of plain Markdown files: one per entry, one per note, one per day page, plus `state.md`. Desktop and Android differ only in where that folder lives — there is no browser build and no `localStorage` store. Nothing leaves the device unless you connect a git remote; that path is [Sync and versions](#sync-and-versions).

## The shape

```text
Doc
├── version: 1
├── entries: Vec<Entry>
├── notes: Vec<LooseNote>
├── pages: BTreeMap<day, markdown>
├── page_meta: BTreeMap<day, PageMeta>
└── ui: UiState
```

`Entry` is one thing filed on a day. It carries an id (`e-<uuid>`), day key (`YYYY-MM-DD`), text, weight (clamped to 1–3), done flag, mark, `mark_auto`, subject, `subject_auto`, optional Markdown note, `created_at` RFC 3339 timestamp, and the single `repeat_weekly` flag. A new entry starts at weight 1, not done, with inference enabled for both mark and subject, and recurrence off.

`LooseNote` is a detail rather than a thing. It has an id (`n-<uuid>`), text, an optional day, a mark and a subject. `day: None` means it is still in the margins; `Some(day)` means it is clipped to that day. The same type covers both, so detaching is just setting the day back to `None`.

`PageMeta` stores `first_written`, a day key. It is the only metadata needed to distinguish a page written ahead from a page written on its day. `UiState` stores the four persisted interface choices: `draw_mode`, `lens`, `open_day`, and `note_edit` (a map keyed by `entry:<id>` or `page:<day>`). Everything else is computed or ephemeral.

## The Markdown store

`horizon-core::store` owns the file format; `horizon-app::storage` owns the directory I/O.

There is exactly one Markdown shape in Horizon. The live store on desktop, the live store on Android and the copy the brief writes out are the same files, byte for byte. A folder can move between the two platforms without conversion.

```text
<store>/
├── README.md
├── state.md
├── entries/<day>-<slug>-<shortid>.md
├── notes/<slug>-<shortid>.md
└── days/<day>.md
```

Frozen rules, decided 2026-11-05. They are the contract step 2 makes `store.rs` enforce; until then the `notes/` row below is one step ahead of the code. Change them only with a version bump and a note here.

- **One file per object.** An entry, a loose or clipped note, and a day page each get one file. Nothing aggregates entries into a digest; `days/<day>.md` is always the day's own page and never a rendered listing.
- **Frontmatter, then a blank line, then the body.** The opening line is `---`, the closing line is `---`, and every field is `key: value` with string values JSON-quoted. Keys are written in a fixed order (below) so a diff between two saves is stable.
- **The body is the writing.** An entry file's body is that entry's Markdown note, a note file's body is the note text, and a day file's body is the day page. Structured fields never appear in the body.
- **Names are stable.** A file name is derived from content that does not move: the day, a slug of the text, and a short id. There is no position or sequence number in a name, so adding or deleting one thing never renames another.
- **Line endings are LF and every file ends with one trailing newline.** `from_files` still accepts CRLF when reading.
- **Unknown files are left alone.** Anything without Horizon frontmatter is ignored on read and never deleted on write. A stale file is removed only when it carries a recognised `kind` and is no longer part of the document.

Frontmatter keys, in written order:

| File | Keys, in order | Body |
| --- | --- | --- |
| `entries/<day>-<slug>-<shortid>.md` | `kind` (`entry`), `version`, `id`, `day`, `title`, `mark`, `subject`, `weight`, `done`, `mark_auto`, `subject_auto`, `repeat_weekly`, `created_at` | the entry's Markdown note |
| `notes/<slug>-<shortid>.md` | `kind` (`note`), `version`, `id`, `created_at`, `mark`, `subject`, `day` (`null` when loose) | the note text |
| `days/<day>.md` | `kind` (`day`), `version`, `day`, `first_written` | the day page |
| `state.md` | `kind` (`state`), `version`, `draw_mode`, `lens`, `open_day`, `note_edit` | empty |

`state.md` is the one file that is not part of the export copy: it records this device's open day, lens, draw mode and note-editor flags, which are not facts about the quarter. `note_edit` is a JSON string so the map survives without a second file.

Naming details:

- The slug is the first 40 ASCII alphanumeric characters of the text, lowercased, with runs of other characters collapsed to one dash, and a fallback of `entry`. The short id is the last 8 characters of the UUID.
- An entry whose day does not parse as `YYYY-MM-DD` is filed under `undated`, which the app does not currently produce but the format allows.
- If two objects would generate the same path, the short id is widened to the whole id tail before falling back to the full id. This is a guard, not the normal case.

`store::to_files` maps a `Doc` to this `BTreeMap<path, content>`; `store::from_files` parses the frontmatter and body back into a `Doc`. Read order does not come from the file names: entries and notes are sorted by `created_at`, then id, so the wall is identical no matter how the folder was written or synced.

The app writes every file on each save and removes stale managed files (recognized entry/note/day frontmatter) that are no longer in the document. That handles entry deletion, title-driven path changes and page removal. The folder is app-owned; `README.md` explains the format and is not rewritten if it already exists.

The default native location is `data_dir()/horizon` on desktop and the `sysdirs` app-files directory on Android. On Linux that is usually `~/.local/share/horizon`. `HORIZON_STORE` overrides the folder for tests or an explicit location. There is no database, index file or hidden binary state.

## Versioning and JSON import

Two version numbers exist: `DOC_VERSION = 1` for the JSON document and `STORE_VERSION = 2` for the Markdown files. Files carrying `version: 1` and `version: 2` are both read; a file newer than `STORE_VERSION` is refused rather than guessed at, so a newer build's folder is not silently damaged by an older one. Version 2 added `created_at` to a note's frontmatter; a version-1 note without it is still read.

The older indexed names (`entries/<day>-001-<slug>-<shortid>.md`, `notes/001-<slug>-<shortid>.md`) are read too, with the index used as the ordering tiebreak, so upgrading an existing store keeps its order. The first save rewrites the folder under the stable names and removes the indexed files as stale.

The JSON import path remains for portability and migration:

1. Parse JSON and require a top-level object.
2. Read `version`; a version greater than 1 is refused as `UnsupportedVersion`.
3. If the version is lower, stamp it to 1. The unversioned concept export is structurally compatible.
4. Deserialize into `Doc`, then normalize.

Normalization:

- missing entry ids get new `e-` ids; missing note ids get `n-` ids;
- entry weights are clamped to 1–3;
- if `mark_auto`, the mark is recomputed from the current text;
- if `subject_auto`, the subject is recomputed;
- an empty entry day is set to the local today;
- a note day that does not parse as `YYYY-MM-DD` becomes `None`;
- empty pages are removed;
- every remaining page gets `PageMeta`, defaulting `first_written` to the page's day;
- metadata for missing pages is removed.

Unknown mark or subject ids fall back to `Unsorted` (`Mark::Thing`) and `Me`.

## Load and migration

`storage::load(today)` returns `Result<Option<Doc>, String>`:

- **`storage::load`:** read `state.md`, `entries/`, `notes/` and `days/`; if any file exists, rebuild the document through `store::from_files`. An empty folder returns `None`, and the caller starts empty or, with `HORIZON_DEMO=1`, with the demo quarter.

`App` loads once. `None` means a quiet first launch, or the opt-in demo quarter when `HORIZON_DEMO=1`; the app writes that first snapshot. A parse or file error becomes a boot error instead of silently discarding data.

## Choosing the vault

The folder is the owner's to place, because it is also the git repository. `storage::store_dir()` returns, in order: `HORIZON_STORE` when set; the path remembered in `$XDG_CONFIG_HOME/horizon/config.toml`; otherwise `None`.

`None` means no vault has been chosen, and `App` renders `boot::VaultBoot` instead of the wall: a page with a suggested path, a text field and two buttons, "Use this folder" and "Choose a folder". The suggestion is `~/Documents/Horizon`, or `~/Horizon` when there is no Documents folder; on Android it is the app files directory, which needs no permission. A typed `~` expands to the home folder. On desktop the button opens the system folder dialog (`rfd`, whose backend is the xdg portal that dioxus-desktop already enables); Android has no picker, so it keeps the suggested folder.

`storage::choose_vault` creates the folder if it is missing, writes the standard `README.md` when there is none, and records the absolute path as `vault = "…"` in `$XDG_CONFIG_HOME/horizon/config.toml` - with the app's config, never inside the vault, so moving or syncing the folder does not break the pointer. The file is one key per line with JSON-quoted strings, which is also valid TOML; Horizon reads the single key it needs with a small line parser, ignoring comments and other keys, and a hand-edited unquoted path still reads. The page then reloads, which starts the store, the saver and the sync thread against the chosen folder instead of re-wiring three subsystems at runtime. An existing Horizon folder opens unchanged; an empty or new one shows an empty quarter. `config.toml` is app state, not vault state: it is never written into the Markdown tree and never travels. The first cut of this feature used a bare `<data dir>/horizon/vault.txt`; a leftover one is adopted into `config.toml` once and deleted, so no stray file can be committed from inside a vault.

The migration is one-way in practice: once the Markdown folder exists, it wins. The legacy JSON is not deleted, so it remains a backup.

## Why one saver loop

Every mutation bumps a `dirty: Signal<u64>` counter. A single future in `App` sleeps briefly, compares the current tick with the last seen tick, clones the document, and awaits `storage::save_now`. The loop is deliberately serial:

```text
mutation → dirty += 1
          ↓
saver loop wakes → sees new tick → snapshot → await write
          ↓
next tick → next write
```

`save_now` calls `store::to_files` and writes the whole folder, then removes stale managed files. The loop coalesces bursts: if several mutations happen within one 20ms window, it writes the latest document once. It is not a debounce that can drop the final state; a dirty tick remains visible until the next iteration. Save errors are logged rather than shown, matching the product's no-save-indicator rule.

## The window ticker

A second future wakes every 60 seconds, reads the local date, and compares it with `today`. When the date changes, it updates `today`, rebuilds `Quarter::new(now)`, and checks the open day. If the open day has fallen out of the new window, `open_day` is cleared so the fold closes. The document itself is never pruned: entries, notes and pages outside the new window remain in `Doc`, in the Markdown files and in any JSON backup.

This is a forward-only window. There is no past window and no infinite scroll; `LOOK_BACK` is false. The app does not delete data when the window moves.

## Persisted versus ephemeral UI

`Doc::ui` is persisted because the spec requires the open day, lens, draw mode and note editor modes to survive reload. It lives in `state.md`. `AppCtx` also holds a separate `Ui` value that is not persisted:

- active tab (`Wall` or `Brief`);
- expanded entry id;
- open mark or subject picker;
- armed clip id;
- quiet-index toggle;
- tooltip;
- status message;
- import draft;
- boot error.

This split keeps interaction furniture out of the document. Reloading may forget a tooltip or a picker; it must not forget which day was open or which subject was being read.

## Pages and first-write metadata

`Doc::set_page` is the only page writer. Empty Markdown removes the page and its metadata. A non-empty write stores the Markdown and, on the first write, inserts `first_written = today`. `page_written_ahead` compares that first-write day with the page's day: if the page was written earlier, the UI prints `written ahead`. `page_is_past` compares the page day with today and is used for the record footer.

`toggle_task` on a page or entry note reads the source Markdown, calls `toggle_task_line`, writes the rewritten string back, and refreshes `first_written` through `set_page` when the target is a page. The Markdown itself is the source of truth; there is no separate task state.

## Sync and versions

The store folder is also a git repository (`main`, one HTTPS remote). Horizon is the only client: no staging area, no branch picker, no diff view, no merge editor.

`horizon-app/src/sync.rs` owns one background thread and the repository. The UI sends commands and reads a snapshot, so no network call blocks rendering. The thread commits after twenty quiet seconds of writing, pulls, and pushes; it pulls once at launch, and again every five quiet minutes. Quit flushes and waits up to eight seconds. The commit message is deterministic: `Wrote 2 entries, 1 note` for all-new changes, `Updated …` otherwise.

`horizon-core/src/sync.rs` is the merge, and it is pure: base, local and remote file maps plus the two commit identities in, a merged tree out.

- a path changed on one side takes that side;
- a path deleted on one side and written on the other keeps the writing;
- two different writes keep the later commit in place and write the other version to `conflicts/<path>.<short-commit>.md`;
- the aside name is derived from the losing commit, so both devices compute the same winner and the same file and converge;
- `state.md` is skipped: device state never travels;
- unrelated histories (both devices started before the first sync) merge against an empty base.

`Git::pull` applies the merged files, deletes paths that disappeared, and commits with both parents. A pull that changes the tree bumps a revision in the sync snapshot; the app reloads the document from the folder, so memory never overwrites the other device's writing on the next save.

Credentials are HTTPS plus a personal access token, written to `data_dir()/horizon-sync/token` (mode 0600 on Unix) — beside the store, never inside it, so it cannot be committed.

The sync code is behind the `sync` feature (on by default on desktop). Android has no system OpenSSL, so a device build uses `sync-vendored`, which compiles OpenSSL with the NDK. Without the feature the app compiles as before and the brief has no Sync block.

## Export boundary

The JSON is the portability boundary for a whole document. The brief downloads it, and "Restore from JSON" replaces the document through `from_json` (including normalization); the next save writes the Markdown folder.

The Markdown copy is not a second format. "Write a copy of the Markdown store" writes exactly what `store::to_files` produces, minus `state.md`, plus the same `README.md` the live folder carries. On desktop it lands in a timestamped folder under Downloads; on Android it lands in the app's external files directory, reachable over USB and from a file manager, with the app-private folder as the fallback destination. Same tree, same bytes, both platforms. There is no account, no telemetry and no conflict dialog.
