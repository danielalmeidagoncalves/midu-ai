---
type: markdown-system
title: Markdown, notes and day pages
description: The markdown AST and typed renderer, safe links and raw-HTML-as-text, note write/read modes, task toggling by source line, and day-page first-write metadata.
tags: [markdown, notes, day-pages, security, rendering]
sources:
  - id: openwiki-source-58bcf5dc38c4149c385b183f
    resource: repo://horizon-app/src/components/agenda.rs
  - id: openwiki-source-647ad384296f313c2f9e829b
    resource: repo://horizon-app/src/components/fold.rs
  - id: openwiki-source-dbdd2b2999563b1a4f943fbb
    resource: repo://horizon-app/src/components/markdown.rs
  - id: openwiki-source-c5dc7e77e438bece6f861302
    resource: repo://horizon-app/src/components/notes.rs
  - id: openwiki-source-bc5aa14c8f6fb3801d975a1a
    resource: repo://horizon-app/src/components/wall.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-6bf207506d53d3f1c4483e6a
    resource: repo://horizon-core/src/markdown.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-f2a3b3f3537f8099cca7944f
    resource: repo://horizon-core/src/store.rs
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Markdown, notes and day pages

Every entry and every day can carry Markdown. Horizon does not use a rich-text editor or a third-party Markdown renderer. It parses a deliberately small Markdown subset into an AST, then renders that AST as typed Dioxus nodes. There is no `innerHTML` path. On desktop and Android the bodies are individual `.md` files in the Markdown store; in memory the same content lives in `Doc`.

## The AST

`horizon-core::markdown` defines the block and inline vocabulary:

```text
Block:  Paragraph | Heading | List | Quote | Table | Rule
Inline: Text | Bold | Italic | Code | Link | Person | DayRef
ListItem: content, task: Option<bool>, line: Option<usize>
```

A task item carries the zero-based source line. That line is what lets the reading view tick a checkbox without re-parsing or rewriting the whole note. Ordinary list items have `task: None` and `line: None`.

The parser normalizes CRLF and tabs, then recognizes headings (`#` through `######`), unordered and ordered lists, task items (`- [ ]`, `- [x]`, `*`, `+`), quotes, tables with a separator row, horizontal rules, and paragraphs. A line that looks like a block but is not consumed is printed as a paragraph rather than dropped.

Inline parsing handles escapes, code spans, bold/italic, links, `@Name` citations, and `^[YYYY-MM-DD^label]` day references. A day reference is rendered as a small day chip using the day's date number and optional label.

## Where the Markdown lives

On both platforms, the body of an entry file is that entry's note; the body of `days/<day>.md` is the day page; the body of a note file is the loose or clipped note text. The frontmatter holds the structured fields (day, mark, subject, weight, flags). In memory the same strings are `Entry.note`, `Doc.pages[day]` and `LooseNote.text`; `store::to_files` writes them and `store::from_files` reads them back.

This is the only shape Horizon writes down. The frozen rules and the exact frontmatter order live in [Document model, versioning and local persistence](document-model-and-persistence.md#the-markdown-store). There is no rendered digest beside the store: the same tree, copied, is what leaves the device.

## Safety

Two rules carry most of the safety:

1. **Raw HTML is text.** `<script>`, `<img onerror>`, and any other markup are ordinary characters in a `Text` node. The component tree never creates an HTML element from note content.
2. **Links are filtered.** `safe_href` accepts only `http://`, `https://` and `mailto:`. A `javascript:` link keeps its label but receives `href: None`; the renderer prints it as inert text. Safe links render with `target="_blank"` and `rel="noopener noreferrer"`.

The renderer walks the AST and produces `p`, `h1`/`h2`/`h3`, `ul`/`ol`/`li`, `blockquote`, `table`/`thead`/`tbody`/`tr`/`th`/`td`, and `hr` nodes. Inline nodes become text, `b`, `i`, `code`, `a`, `cite` and a day-chip span. This is why the tests can assert that a raw `<script>` produces no script node.

## Task ticking

A rendered task item is not a form checkbox. It is an `li` with `role="checkbox"`, `tabindex="0"`, `aria-checked`, and an accessible label. Click, Enter or Space calls the task handler with the item's source line.

The model path is:

```text
MarkdownView task click
  → AppCtx::toggle_note_task(target, line)
  → Doc::toggle_task(target, line, today)
  → markdown::toggle_task_line(source, line)
  → set_note / set_page
```

`toggle_task_line` finds the line, preserves its leading indentation and list prefix, flips only `[ ]` ↔ `[x]` (or `[X]`), and leaves the rest of the line untouched. It returns `None` for a line that is not a task or an out-of-range line. For a day page, the rewritten string goes through `set_page`, so page metadata stays consistent. For an entry note, it goes through `set_note`, which stores `None` when the note becomes blank.

## Note write and read modes

`NoteEditor` is shared by entry notes and the day's page. The committed Markdown is the source of truth. The component keeps a local textarea buffer and a persisted editing flag.

- A note with text defaults to the **read** view. An empty note defaults to the **write** view.
- `Write` switches to the textarea and focuses it; `Read` commits the buffer, sets the editing flag false, and focuses the Write button.
- Clicking the rendered page also switches to write mode.
- Blur commits the buffer. Escape and Cmd/Ctrl+Enter commit and return to read mode. Arrow keys inside the textarea are stopped from propagating to the fold's arrow navigation.
- While reading, an effect keeps the buffer in step with the document, so imports, task ticks or commits from another surface appear. While writing, the buffer is the source.

The toolbar wraps the current textarea selection with Markdown syntax (bold, italic, heading, list, task, quote, code, link, table, rule). It uses a small webview selection call and dispatches an `input` event, so the ordinary Dioxus input handler owns the buffer. The toolbar never writes HTML.

## Day pages

The day's own page is a `Doc.pages` entry keyed by the day in memory, and `days/<day>.md` on native. When empty, the fold shows a `+ Write the day's page` button. Opening it focuses the textarea. The page editor is the same `NoteEditor` with `whole = true`, so it has the full toolbar and hint line.

`Doc::set_page` is the only writer. A non-empty page stores the Markdown and records `first_written = today` on the first write. An empty write removes both the page and its metadata. `page_written_ahead` compares the first-write date with the page day; if the page was written earlier, the UI prints `written ahead · first written <date>`. `page_is_past` drives the record footer. Under the forward-only window that footer is effectively unreachable, but the code path remains.

The wall and agenda show two badges: `N` when one or more entries on the day have notes, and `MD` when the day itself has a page. Both are hidden under a subject lens, because the wall is then a subject measure rather than a document map.

## What the Markdown can contain

Headings, paragraphs, bold/italic, inline code, links, ordered/unordered lists, tasks, quotes, tables and rules. It cannot embed raw HTML, run scripts, load images, or create arbitrary attachments. The same content is what the JSON export stores, what the live Markdown files hold and what the Markdown copy writes; there is no separate rich-text representation and no second Markdown dialect.
