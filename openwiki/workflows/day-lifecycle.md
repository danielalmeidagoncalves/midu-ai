---
type: workflow
title: A day from capture to record
description: Follow one thing from the composer through the wall and fold into an entry note, a day page, the Markdown store and an export, including focus, clipping and the single-shape rule.
tags: [workflow, capture, fold, notes, export]
sources:
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-c1bc075dfb9585127c44a844
    resource: repo://horizon-app/src/components/brief.rs
  - id: openwiki-source-647ad384296f313c2f9e829b
    resource: repo://horizon-app/src/components/fold.rs
  - id: openwiki-source-735e0cf4efc981d22067f979
    resource: repo://horizon-app/src/components/margin.rs
  - id: openwiki-source-c5dc7e77e438bece6f861302
    resource: repo://horizon-app/src/components/notes.rs
  - id: openwiki-source-bc5aa14c8f6fb3801d975a1a
    resource: repo://horizon-app/src/components/wall.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-f2a3b3f3537f8099cca7944f
    resource: repo://horizon-core/src/store.rs
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# A day from capture to record

This is the path a single thing takes through Horizon. It touches the composer, the wall, the fold, the note editor, the live Markdown store and the exports, but each mechanism has its own page.

## 1. Capture

The user types:

```text
tomorrow - call mum
```

The masthead composer sends the line to `parse_line`, which uses the pure composer with the open day and the active lens. The parser recognizes `tomorrow`, strips the separator, and infers a People mark and a Family subject from "call mum". The preview beside the input already shows that mark and the `FAM` handle.

On submit, `AppCtx::add_entry` constructs an `Entry` with the parsed day, title, mark, subject and `subject_auto` set from the explicit flag. It appends the entry to `Doc`, bumps the dirty counter, and flashes the day and subject. The input clears and keeps focus. Nothing else is required: there is no save button and no confirmation dialog. The saver loop will write the new entry as a Markdown file.

## 2. The wall

The wall recomputes the row. `Doc::items_on` returns the entry as a real item, `Doc::tier` derives One from the weight, and the cell prints the mark for that day. Because the entry has text, the cell gets the `has` treatment and the wall's live-day count for that week increases.

If the day is today, the cell already carries the stamp, frame, `today` word and `now` gutter. If the day is tomorrow, it is a normal cell until the window rolls. The wall never stores a copy of the entry; it is a projection of the current document.

## 3. Opening the fold

Clicking the cell (or pressing Enter on it) calls `open_day`. That writes `ui.open_day` into the persisted document UI, closes any open pickers, scrolls the cell into view, and bumps dirty. The wall inserts the `Fold` after the row that contains the open day.

`App` watches `open_day` and focuses the fold after the DOM update. The fold's aside shows the large numeral, the weekday/date, ISO week, tier word, standfirst and previous/next chips. The main column lists the large marks, then the entry rows, then any clipped details, the day-bound composer, and the day page.

Esc closes the fold and returns focus to the same cell. Left/right move one day; up/down move one week. The fold's arrow handling stops propagation so the text inputs inside it do not also move the fold unless the user is at the fold level.

## 4. Editing the entry

The entry row is the small editor. It has a checkbox, a mark button, an editable title, a subject handle and an expander.

- The checkbox toggles `done` and saves.
- Clicking the mark opens the eighteen-mark picker; choosing one sets `mark_auto = false` and focuses the mark button.
- Clicking the subject handle opens the seventeen-subject picker; choosing one sets `subject_auto = false` and saves.
- Editing the title commits on blur or Enter. If `mark_auto` is still true, the mark is re-inferred; if `subject_auto` is still true, the subject is re-inferred. Escape restores the original text.
- The expander opens the detail panel: the Markdown note, weight buttons (One thing / Full day / Landmark), the single `Repeats weekly` toggle when recurrence is enabled, mark/subject reset actions, and Remove.

A reset action sets the auto flag back to true and re-infers from the current title. Remove deletes the entry and its note-editor key. Every action goes through `AppCtx`, updates the document signal and bumps dirty.

## 5. The note

The note editor defaults to read view when the entry already has text, and to write view when it is empty. `Write` focuses the textarea; `Read` commits and shows the rendered AST. Blur, Escape and Cmd/Ctrl+Enter also commit. The textarea uses the small Markdown toolbar for bold, italic, heading, list, task, quote, code, link, table and rule.

If the note contains a task:

```markdown
- [ ] print the referral
```

the read view renders it as a checkbox-like list item. Clicking it (or pressing Enter/Space) calls `toggle_note_task` with the item's source line. The core rewrites `[ ]` to `[x]` in the stored Markdown, preserving indentation and the rest of the line. The visible list and the stored text stay in sync; there is no separate task record. The `N` badge on the wall appears because the entry now has a non-empty note.

## 6. The day page

The fold also has the day's own Markdown page. If it is empty, the fold shows `+ Write the day's page`; opening it focuses the editor. Writing the page stores it under `Doc.pages[day]` and, on the first write, records `first_written = today`. That page is `days/<day>.md`.

A page written before its day shows `written ahead · first written <date>`. Under the forward-only window, a page is not normally past, but the model also has a `page_is_past` footer for a future past-window feature. The page editor uses the same Markdown renderer and task ticking as an entry note; the `MD` badge appears on the wall.

## 7. A loose note

A detail without a day goes into the margins. The user types it in the margin composer; `add_note` infers mark and subject and pushes a `LooseNote` with `day = None`. It stays in the margin list.

To clip it, the user presses `Clip to a day`. The note id becomes `arm_note`; the next cell click (or agenda row click) calls `clip_note`, sets `day` to that key, and clears the armed state. Under a lens, the clipped note takes the lens subject. Detach sets `day = None` and returns it to the margins. This is a two-step flow: arm, then choose the day.

## 8. Store and export

The brief is the only secondary tab. It can download the whole document as JSON, restore from JSON, or write a copy of the Markdown store. The JSON download is the exact versioned document. The Markdown copy is not a second format: it is `store::to_files` minus `state.md`, plus the same `README.md` the live folder carries, so a copy and the store differ only by the device-state file. Desktop writes it under Downloads; Android writes it to the app's external files directory, falling back to the app-private folder.

The saver loop has already persisted every mutation to the live store: one Markdown file per entry, note and day. A copy is a snapshot beside it, not the save path.

## 9. Sync

The store is a git repository. Every save marks activity; twenty quiet seconds later the burst becomes one commit and leaves the device. A launch pulls, a five-minute quiet timer pulls and pushes, and quit flushes. The brief's Sync block shows the state, the remote and the last twelve versions.

When both devices changed the same file, the later commit stays and the other version is kept under `conflicts/`, named after the commit it came from. There is no dialog and no "resolve" state. `state.md` never travels, so opening a day on the phone does not move the desktop's open day.

A pull that changes the folder bumps a revision; the app reloads the document from disk before the next save. That is what keeps the in-memory quarter from overwriting the other device's writing.

## The two shapes

On a narrow viewport the same flow uses the agenda instead of the wall. The open day's fold is inserted inside the agenda; the margins sit below it. The app renders only one shape at a time, so the fold and entry ids remain unique. The focus effect and the close behavior are shared: opening focuses the fold, closing returns to the same day in the shape that is on screen.
