---
type: architecture-overview
title: System overview and the pure-core boundary
description: Horizon separates a pure Rust product core from a Dioxus renderer, with an in-memory document mapped to a plain-Markdown file store, derived views and a single local saver.
tags: [architecture, rust, dioxus, persistence]
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
sources:
  - id: openwiki-source-651d1fb6c9e49916a916ab51
    resource: repo://Cargo.toml
  - id: openwiki-source-397a4b0768b3f5703fe7a0d4
    resource: repo://horizon-app/assets/main.css
  - id: openwiki-source-fa908abff259e47afe8cb7fd
    resource: repo://horizon-app/Cargo.toml
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-f78d1a767ae27f573e2eb73b
    resource: repo://horizon-app/src/main.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-2d6f10c7cc311de168f4b53a
    resource: repo://horizon-core/Cargo.toml
  - id: openwiki-source-ddc32c34e6750e0ffd69251d
    resource: repo://horizon-core/src/lib.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-f2a3b3f3537f8099cca7944f
    resource: repo://horizon-core/src/store.rs
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
---

# System overview

Horizon is a two-crate Cargo workspace with a deliberate dependency direction: `horizon-core` holds the product and `horizon-app` renders it.

```text
horizon-core/   pure Rust: vocabulary, inference, quarter maths, tiers,
                composer grammar, markdown AST, export, store, sync, seed
horizon-app/    Dioxus 0.7: wall, fold, index, margins, agenda, notes, brief,
                git (libgit2), the sync thread
```

`horizon-core` declares that it has no DOM, no renderer and no I/O. It owns the eighteen marks, seventeen subjects, keyword inference, the 90-day quarter, tier derivation, the one-line composer, the markdown parser, the one Markdown store format, the pure sync merge and the versioned JSON shape. That makes the product's opinions testable without a webview, a display or a file system. `horizon-core` depends only on `serde`, `serde_json`, `chrono` and `uuid`; platform concerns do not leak into it.

`horizon-app` is the renderer. Its `main` wires `app`, `components`, `state`, `storage` and — when the `sync` feature is on — `git` and `sync`; `menu`, `desktop` and `android` are the platform edges. Then it launches the Dioxus `App`. Desktop and Android share the same crate and the same component tree; the Cargo features only choose the Dioxus platform backend, and `build.rs` emits one `desktop_shell` cfg so desktop-only items read `#[cfg(desktop_shell)]`.

## The document and the store

`Doc` is the in-memory object. It contains:

- `entries`: the things filed on days, each with a mark, subject, weight, done flag, optional note and optional weekly recurrence;
- `notes`: loose or clipped "details", with an optional day;
- `pages`: day-keyed Markdown for the day's own page;
- `page_meta`: the first-write date used to distinguish writing ahead;
- `ui`: the persisted slice of interface state — draw mode, subject lens, open day, and per-note editor mode.

Everything else is derived. `Doc::items_on` builds the marks printed on a day, `Doc::tier` derives density from the maximum real entry weight, `Doc::subject_count` and `subject_next` compute the index, and `Doc::page_written_ahead` reads `page_meta` rather than storing a second page object. Derived views are never serialized, so they cannot drift from the entries.

On desktop and Android the durable store is a folder of plain Markdown files. `store::to_files` maps a `Doc` to one file per entry, one per note, one per day page, plus `state.md`; `store::from_files` rebuilds the document. Each file has YAML-style frontmatter (`kind`, `version`, ids, day, mark, subject, weight and flags) and a Markdown body. `state.md` holds the persisted UI slice, including `note_edit` as a JSON string. There is no other store.

The JSON boundary still exists for portability. `export::to_json` serializes the document; `export::from_json` accepts the current version, migrates the unversioned concept shape, rejects a future version, normalizes ids and weights, re-infers automatic marks and subjects, drops empty pages, and repairs page metadata. A JSON document restored through the brief replaces the in-memory document, and the next save writes it to the Markdown folder, which remains the source of truth.

## The ephemeral layer

`AppCtx.ui` is a separate `Ui` value held in a Dioxus signal. It contains transient interaction state: the active tab, expanded entry, open mark or subject picker, armed clip, quiet-index toggle, tooltip, status line, import draft and boot error. None of it belongs in `Doc::ui`; reloading the app is allowed to forget a tooltip or a picker, while the open day, lens, draw mode and note editor mode survive.

## The render path

A capture line flows through the pure core and back out as UI:

```text
composer text
  → composer::parse (date, title, inferred mark/subject, refusals)
  → Entry appended to Doc
  → Doc::items_on / tier / entries_on / clipped_notes_on
  → wall cells, fold, day page, index, margins
  → dirty counter
  → one saver loop
  → Markdown files (the one store format on both platforms)
  → quiet 20s → commit → pull/merge → push
```

`App` owns the signals for `today`, `quarter`, `doc`, `ui`, `narrow` and `dirty`, and provides `AppCtx` to descendants. UI mutations go through `AppCtx` methods; those methods update the document signal and bump `dirty`. A single future loop coalesces dirty ticks and awaits each `storage::save_now`. `save_now` writes the whole Markdown folder. The loop is why a later save cannot be overtaken by an earlier one.

`App` also owns the one-shape rule. A viewport signal reports whether the window is at most 700px; the app renders either the desktop wall or the mobile agenda, never both. That prevents duplicate `fold` and entry ids and keeps focus restoration unambiguous. The same 700px threshold is the CSS breakpoint.

## The platform boundary

`horizon-app` is the only place that touches the platform. `storage` reads and writes the Markdown folder on desktop and Android, focuses and scrolls elements, triggers webview downloads, writes the Markdown copy, and keeps the sync token beside the store. `git` is the only place that talks to a remote, and `sync` is the only place that runs it off the UI thread. `desktop` owns quit and "open the folder", and `components/menu` draws the in-window menu; both are desktop-only. `components` translate core values into Dioxus nodes; `components/markdown` turns the parsed AST into typed elements rather than HTML. The copy is the same store tree on both platforms; only its destination differs — Downloads on desktop, the app's external files directory on Android with the app-private folder as fallback.

This boundary is the main design decision in the repository: the core can be read, changed and tested as product logic, while the app remains a thin renderer plus a small set of webview calls.

## Where to read next

- [Quickstart](../quickstart.md) for commands and task routing.
- [Document model and persistence](../systems/document-model-and-persistence.md) for the schema, file store and saver.
- [The quarter, the wall and density tiers](../systems/quarter-and-wall.md) for the time window.
- [Surfaces, responsive layout and accessibility](../surfaces/ui-and-accessibility.md) for the component tree.
