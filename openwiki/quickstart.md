---
type: quickstart
title: Horizon quickstart
description: What Horizon is, how the two crates divide responsibility, the commands that build/test it, and which wiki page to read for a given maintenance task.
tags: [quickstart, orientation, build, testing, architecture]
sources:
  - id: openwiki-source-81811552667c74c81f90582e
    resource: repo://.zed/debug.json
  - id: openwiki-source-ff8e704eea33edb3b019211a
    resource: repo://.zed/tasks.json
  - id: openwiki-source-651d1fb6c9e49916a916ab51
    resource: repo://Cargo.toml
  - id: openwiki-source-d1eead8d200b68e1165d8f17
    resource: repo://HANDOFF.md
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-f78d1a767ae27f573e2eb73b
    resource: repo://horizon-app/src/main.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-76d2e29a57742ad34fd1b521
    resource: repo://horizon-core/src/dates.rs
  - id: openwiki-source-ddc32c34e6750e0ffd69251d
    resource: repo://horizon-core/src/lib.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-1edec297169af95a93474027
    resource: repo://tools/with-libxdo.sh
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Horizon quickstart

Horizon is a local-first personal life OS: a 90-day Monday-first wall of monochrome marks, a seventeen-subject index, and a Markdown page under every thing and every day. It is a Rust workspace with a pure product core and a Dioxus 0.7 renderer. Documentation is local Markdown; nothing is published.

## The two crates

```text
horizon-core/   pure Rust: vocabulary, inference, quarter maths, tiers,
                composer grammar, markdown AST, export, store, sync, seed
horizon-app/    Dioxus 0.7 UI: wall, fold, index, margins, agenda, notes, brief,
                git (libgit2), the sync thread
```

`horizon-core` has no DOM, renderer or I/O. It owns every product opinion and is where the 63 unit tests live. `horizon-app` is the renderer and the only crate that talks to the webview, the Markdown store on desktop/Android, the JSON import/export, the Markdown copy folder, git, and the window.

The document flows in one direction:

```text
composer → Entry → Doc signal → derived views → wall/fold/index
                         ↓
                  dirty counter → one saver → the Markdown store (desktop and Android)
                                                    ↓
                              quiet 20s → commit → pull/merge → push
```

Read [System overview and the pure-core boundary](architecture/system-overview.md) before changing the crate split or the data flow.

## Commands

```sh
# Core tests
cargo test -p horizon-core

# App store tests (needs the local libxdo path on this machine)
LIBRARY_PATH=$HOME/.local/lib cargo test -p horizon-app

# Desktop (empty first launch)
cargo run -p horizon-app

# Desktop with the sample quarter (only when the store is empty)
HORIZON_DEMO=1 cargo run -p horizon-app

# Desktop with hot reload
dx serve --package horizon-app --desktop
```

On Linux, Dioxus desktop may need `libxdo`. If it is in `~/.local/lib`, use the repository wrapper or set both paths:

```sh
LIBRARY_PATH=$HOME/.local/lib LD_LIBRARY_PATH=$HOME/.local/lib \
  cargo run -p horizon-app
```

`tools/with-libxdo.sh` does that automatically when `~/.local/lib` exists; it is a no-op otherwise.

The live store is `~/.local/share/horizon` on desktop and the app-files `horizon` folder on Android. `HORIZON_STORE` overrides it. Both platforms write the same Markdown format: see [the frozen format rules](systems/document-model-and-persistence.md#the-markdown-store).

Horizon has no browser build and no server component. The only targets are desktop and Android.

Android (the recorded environment did not have an SDK/NDK):

```sh
rustup target add aarch64-linux-android
cargo build -p horizon-app --no-default-features --features mobile --target aarch64-linux-android
# or, with an Android SDK/NDK:
dx build --package horizon-app --platform android
```

Zed tasks and CodeLLDB profiles are in `.zed/tasks.json` and `.zed/debug.json`. Use `task: spawn` for the run/build/test/check/clippy/fmt/contrast/wiki tasks, or `f4` for the demo/no-demo debug profiles.

Details: [Build, run, debug and platform operations](operations/build-run-and-platforms.md).

## Where to read for a task

| If you are changing… | Read |
| --- | --- |
| The quarter window, week rows, month/ISO labels, tiers, wall/agenda density | [The quarter, the wall and density tiers](systems/quarter-and-wall.md) |
| Mark/subject vocabulary, Lucide glyph mapping, keyword scoring, auto/override rules | [Marks, subjects and keyword inference](systems/marks-and-subjects.md) |
| Composer dates, `/subject`, open-day fallback, refusals, preview | [The composer grammar and capture path](systems/composer-and-capture.md) |
| `Doc`, entry/note/page shapes, the frozen Markdown format, stable file names, JSON import, saver, ticker | [Document model, versioning and local persistence](systems/document-model-and-persistence.md) |
| Markdown parsing/rendering, safe links, task ticking, note read/write, day pages | [Markdown, notes and day pages](systems/markdown-and-day-pages.md) |
| The end-to-end capture → fold → note → export → sync flow | [A day from capture to record](workflows/day-lifecycle.md) |
| Subject lens, index counts, quiet subjects, loose notes and clipping | [Subject lens, index and loose notes](workflows/lens-and-margin.md) |
| Masthead, wall, fold, agenda, margins, brief, CSS, focus, touch, contrast | [Surfaces, responsive layout and accessibility](surfaces/ui-and-accessibility.md) |
| Build features, desktop and Android, libxdo, Zed, the Markdown copy, sync builds | [Build, run, debug and platform operations](operations/build-run-and-platforms.md) |
| Test families, invariants, verification, OPEN defaults, omissions | [Tests, invariants and verification](quality/testing-and-invariants.md) |

## Rules not to break

- **The core stays pure.** No webview, DOM, file system or platform calls in `horizon-core`.
- **One document, one store, one shape.** `Doc` is in memory; Markdown files are the live source of truth. Desktop, Android and every Markdown copy write the same tree with the same frontmatter. Derived views are computed, not stored.
- **Sync is git, and the merge is pure.** The store folder is a repository; the merge rules are in `horizon-core/src/sync.rs` and never need a dialog. `state.md` is gitignored.
- **One shape at a time.** The wall and agenda are alternatives, so ids and focus stay unique.
- **The wall narrows, the page does not.** A lens filters marks and measures; the fold shows the whole day.
- **Recurrence is texture.** Projected weekly occurrences never add tier weight.
- **Markdown is typed nodes.** There is no raw HTML path and no `innerHTML`.
- **Terracotta means today and interaction**, not decoration.
- **Quiet is legal.** No streaks, overdue states, reviews, notifications or charts.

## Known caveats

- The spec asks for 90 days starting today and 13 week rows; those coincide only on Monday or Tuesday. Horizon shows all 90 days and 13/14 rows accordingly. No day is dropped.
- Android was cargo-checked and the mobile feature, `sysdirs` path and responsive shape are in place, but it was not built or device-verified in the recorded environment.
- Closed subjects, looking backward, past-window reading, sync, accounts and notifications are deliberate omissions.
- The four open defaults are recurrence, closed subjects, plan-vs-record and looking backward; see [Tests, invariants and verification](quality/testing-and-invariants.md) and the repository's `HANDOFF.md`.

## Repository entry points

- `README.md` — human-facing run/build summary.
- `ARCHITECTURE.md` — build note and the crate split.
- `HANDOFF.md` — what shipped, verification log, open defaults and deliberate omissions.
- `openwiki/` — this local Markdown wiki; start here and follow the task table.
- `horizon-product-spec.md` — the product contract.
- `horizon-life-os-concept-v6.html` — the visual/interaction reference.
- `openwiki/` — this generated local wiki.
