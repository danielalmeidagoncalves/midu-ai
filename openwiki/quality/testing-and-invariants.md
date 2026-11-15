---
type: quality-guide
title: Tests, invariants and verification
description: The core test families, the build and contrast checks, the product invariants they protect, and the four OPEN defaults that are deliberate rather than missing.
tags: [testing, quality, invariants, verification]
sources:
  - id: openwiki-source-d1eead8d200b68e1165d8f17
    resource: repo://HANDOFF.md
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-b108f17faab5d01148959127
    resource: repo://horizon-core/src/composer.rs
  - id: openwiki-source-76d2e29a57742ad34fd1b521
    resource: repo://horizon-core/src/dates.rs
  - id: openwiki-source-09ff909661761d256f1251fc
    resource: repo://horizon-core/src/export.rs
  - id: openwiki-source-84a00d6eab52fe2e00796d90
    resource: repo://horizon-core/src/inference.rs
  - id: openwiki-source-6bf207506d53d3f1c4483e6a
    resource: repo://horizon-core/src/markdown.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-d676c14942e4fd176f4c0593
    resource: repo://horizon-core/src/seed.rs
  - id: openwiki-source-f2a3b3f3537f8099cca7944f
    resource: repo://horizon-core/src/store.rs
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-1f6ffa9493e99001b1f2ab74
    resource: repo://tools/contrast.py
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Tests, invariants and verification

Horizon's confidence comes from keeping the product logic in a pure core and testing that core directly. The app is a renderer; the tests protect the rules the renderer depends on.

## Test families

`cargo test -p horizon-core` runs 63 tests. They are grouped by the module that owns the behavior:

| Module | What the tests protect |
| --- | --- |
| `dates` | 90 days starting today; Monday-first padding; every day kept; 13 rows Monday/Tuesday and 14 on other weekdays; ISO week and month labels. |
| `model` | Weight is the maximum entry weight; tiers follow; a lens recomputes wall weight but leaves the day page whole; counts and next are window-only; loose and clipped notes are distinct; recurrence projects without ink; page first-write metadata; note read/write default; task toggling. |
| `inference` | Spec examples infer the expected mark and subject; longest keyword match wins; plurals are quiet; subject queries are exact, then prefixed, and ambiguous ones are refused; all 18 marks have names and stable ids; 17 subjects. |
| `composer` | Three date forms; relative dates; open-day and today fallback; year roll; out-of-window refusal; subject override and unknown-subject refusal; empty/titleless refusal; preview; separator stripping. |
| `markdown` | Headings, paragraphs, rules, lists, tasks with source line numbers, quotes, tables, inline marks, safe links, raw HTML as text, escaped markers, and task-line rewriting. |
| `export` | JSON round-trip keeps entries, notes, pages and notes; future versions are refused; unversioned input migrates; unknown marks/subjects fall back; empty pages are dropped. |
| `store` | One file per entry/note/day plus state.md; frontmatter and body round-trip; stable, index-free paths; adding a thing never renames another; a shared id tail widens instead of overwriting; read order comes from `created_at`; a version-1 folder still reads in its old order; unrelated files are ignored; unsupported versions are refused; automatic fields re-infer. |
| `sync` | Disjoint edits take both sides; new files on both sides are kept; identical edits do not conflict; a real conflict keeps both, names the aside after the losing commit, and converges from either device's perspective; a time tie breaks on commit id; writing beats deleting; `state.md` never travels; commit-message wording. |
| `seed` | The demo has a little of everything: entries, weights, recurrence, notes, loose notes and pages. |

The focus is behavior, not symbol coverage. For example, the model tests assert that a lens changes `weight` but not `entries_on(day, None)`, because the wall narrows while the fold stays whole. They assert that a recurring entry keeps its weight on its real day but contributes zero weight to a projected occurrence. They assert that a page written before its day reports `written ahead`, and that clearing it removes both the page and its metadata.

## The 13-row caveat

The quarter is always 90 days starting today. The wall is Monday-first and keeps every weekday in its true column, so the first and last weeks are padded. Those two requirements produce **13 rows when today is Monday or Tuesday and 14 rows on every other weekday**. No day is dropped. The test suite pins this explicitly: a Monday and a Tuesday produce 13 weeks, a Wednesday produces 14.

This is a deliberate resolution of a spec ambiguity, not an accidental off-by-one. Any future change to force 13 rows on every start date would alter the window semantics and must update the dates tests and the wall documentation together.

## Composer and inference guarantees

The composer tests are the contract for the one-line grammar. They cover `dec 3`, `3 dec` and `3/12`; `today`, `tomorrow` and `day after tomorrow`; no-date behavior using the open day and then today; an open day outside the window being ignored; year rollover; out-of-window refusal; `/subject` override; unknown-subject refusal; and a lens acting as a default rather than overriding an explicit token. The preview test checks that the mark and subject shown before submission match the eventual result.

The inference tests use the product spec's examples as fixtures. They also check the scoring rules that are easy to break: normalized punctuation, word-boundary matching, plural tolerance, longest total keyword match, earlier-row tie-breaking, and the `Me`/`Thing` fallbacks. Subject queries are exact first, then unique prefixes; `a` is refused because it could be AI or Assets.

## Markdown safety

The parser tests are also security tests. Raw HTML must remain text; a `<script>` or `<img onerror>` is never a node. Links are filtered to `http(s)` and `mailto`; a `javascript:` link keeps its label but loses its `href`. Task lists carry their zero-based source line, and `toggle_task_line` rewrites only the marker while preserving indentation and the rest of the line. That is what makes a reading-view tick safe: the app rewrites the stored Markdown string rather than rendering a form control into it.

## Persistence, store and export

The store tests round-trip a document through `store::to_files` and `store::from_files`: entries, notes, day pages, page metadata and `state.md` all survive. They check that file paths are readable and stable — the day, a slug and a short id, never a position number — that an unrelated README or note without frontmatter is ignored, that an unsupported store version is refused, and that automatic mark/subject fields re-infer on read. The app tests add the filesystem side: write a store, read it back, then remove an entry and verify the stale file is deleted; a hand-written version-1 folder is read, rewritten under the stable names, and still reads back in the same order.

The git tests are integration tests with a real bare repository as the remote. Two folders act as two devices: the second fast-forwards onto the first, a disjoint edit merges without conflict, and a same-file edit on both sides produces one conflict with the same aside path and content on both sides. A clean tree commits nothing, `state.md` never enters a commit, and history reads back with the files each commit touched.

The export tests verify that the JSON boundary round-trips the whole document, that a future `version` is refused rather than silently downgraded, that the unversioned concept shape migrates to version 1, that unknown mark/subject ids fall back to `Thing`/`Me`, and that empty pages do not survive import. JSON is the whole-document portability path; the Markdown copy is the store itself, so there is no second Markdown shape to test.

## Verification beyond the unit tests

The recorded verification also included:

- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets` clean;
- `cargo fmt --all -- --check` clean;
- `cargo test -p horizon-app` — 6 tests passed (three store/migration, three git; needs the local `libxdo` path);
- a native desktop build and an Android `cargo check` (the no-sync `mobile` build);
- browser interaction checks for the fold, focus, lens, composer, note writing, task ticking, clipping, import/export and safe Markdown;
- a native end-to-end store run with `HORIZON_STORE=/tmp/horizon-e2e HORIZON_DEMO=1`: 31 entry files, 4 note files, 6 day files and `state.md`, with a second launch loading the same folder without duplication;
- a native sync run against a bare remote: the demo committed and pushed 43 files after twenty quiet seconds, and a second empty store pulled all 31 entries, merged the unrelated histories and pushed the merge;
- `python3 tools/contrast.py`, which checks every normal-text token pair used by the interface at 4.5:1. The lowest reported pair is terracotta-on-paper at 5.46:1; the tooltip text is 15.90:1 and its label 16.38:1. Hover backgrounds are included and do not reduce contrast.

Android was cargo-checked but not built or device-verified in the recorded environment (no Android SDK/NDK). The mobile feature, the `sysdirs` app-data path, the JNI external-files call and the responsive shape are present; the `mobile,sync-vendored` build and device verification remain open operational steps.

## Invariants to preserve

These are the rules a change should not quietly break:

1. **The core stays pure.** No DOM, webview or file-system calls in `horizon-core`.
2. **One document, one store.** `Doc` is the in-memory model; the Markdown files are the live source of truth. Derived views are computed, not stored.
3. **One Markdown shape.** Desktop, Android and every Markdown copy write the same tree with the same frontmatter key order; there is no rendered digest and no per-platform variant.
4. **One shape at a time.** The wall and agenda are alternatives, never both, so ids and focus stay unique.
5. **The wall narrows, the page does not.** A lens filters marks and measures, but a day page always shows the whole day.
6. **Recurrence is texture, not ink.** Projected occurrences never add tier weight.
7. **Markdown is rendered as typed nodes.** No raw HTML path, no `innerHTML`.
8. **Today is the only accent.** Terracotta marks today and interaction; it is not decoration.
9. **Quiet is legal.** No streaks, overdue states, review queues, notifications or charts.

## The four OPEN defaults

The four deliberate defaults are recorded as constants in `horizon-core/src/model.rs` and described in `HANDOFF.md`:

- **Recurrence:** `WEEKLY_RECURRENCE = true`, one `repeat_weekly` toggle, projected forward, no series editor or exceptions.
- **Closed subject:** `CLOSED_SUBJECTS = false`; a subject can only be quiet.
- **Plan vs. record:** `PLAN_MARKERS = true`; one page object, `first_written` drives `written ahead`, and the past footer exists but the forward-only window makes it unreachable.
- **Look backward:** `LOOK_BACK = false`; forward 90 days only.

Deliberate omissions are also part of the contract: no tags, folders, projects, priorities, colours, streaks, completion percentages, weekly reviews, overdue badges, review queues, notification streams, charts, rich-text editors or account requirements. They are not unfinished features waiting to be added back by default.
