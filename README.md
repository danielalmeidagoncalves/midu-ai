# Horizon

A personal life OS for someone whose last ten organisation tools died of maintenance debt.

Ninety days, Monday-first, as a printed wall. Every thing on a day is one monochrome mark; every
mark and subject is inferred from the words and correctable in one click. The index has seventeen
subjects and prints an en dash when one is quiet. Clicking a day inflates that week's row into a
full day page in place. Under every thing and every day there is a markdown page. Today is a
terracotta postmark, not a highlight. Nothing is ever filed, tagged, reviewed, reminded or counted
as missed.

The product specification is [`horizon-product-spec.md`](horizon-product-spec.md); the reference
concept is [`horizon-life-os-concept-v6.html`](horizon-life-os-concept-v6.html); the build note is
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## Run it

Requirements: a stable Rust toolchain (Dioxus 0.7 needs 1.83+) and, for the `dx` commands, the
Dioxus CLI 0.7.10 (`curl -fsSL https://dioxuslabs.com/install.sh | bash` or
`cargo install dioxus-cli@0.7.10`).

```sh
# Desktop
cargo run -p horizon-app

# Desktop through the Dioxus CLI
dx serve --package horizon-app --desktop

# Tests
cargo test -p horizon-core
# Markdown-store filesystem tests (linker and loader both need the libxdo prefix here)
LIBRARY_PATH=$HOME/.local/lib LD_LIBRARY_PATH=$HOME/.local/lib cargo test -p horizon-app
```

Linux desktop needs the WebKitGTK development libraries and `libxdo` (on Arch:
`sudo pacman -S webkit2gtk-4.1 gtk3 xdotool`). If `libxdo` is installed in a non-standard prefix,
set `LIBRARY_PATH` when linking.

The first launch is an empty quarter. To see the concept's sample quarter (for screenshots or a
walkthrough), set `HORIZON_DEMO=1` on an empty store:

```sh
HORIZON_DEMO=1 cargo run -p horizon-app
```

**Leaving the app.** The desktop window carries its own menu — `Horizon` in a thin strip above the
masthead — because the webview's system menubar draws no usable entries under GTK. It opens with a
click, `Alt+H`, or `F10`, and holds the store path, "open the folder" (the file manager, or the path
in the footer if none answers), and Quit. `Ctrl+Q` (`Cmd+Q` on macOS) quits from anywhere and always
saves first.

Persistence is silent and local-first. The live store is a folder of plain Markdown files: one per
entry, one per loose note, one per day page, one per reminder, plus `state.md`. There is no account,
no server and no save indicator: the folder is the store, and any editor can read it. The optional
git sync in the next section is the only thing that leaves the device, and only when you point it at
a remote.

**The vault is yours to place.** On first launch - when no folder has been chosen yet - Horizon
shows one page instead of the wall and asks where the Markdown should live. It suggests
`~/Documents/Horizon` (or `~/Horizon` if there is no Documents folder) and accepts a typed path
(`~` works) or a folder picked in the system dialog. The choice is remembered in
`~/.config/horizon/config.toml` as one key, `vault = "…"`. It lives with the config, never inside
the vault, so moving the folder never loses the pointer and the pointer never enters a commit. That folder is also the git repository, so wherever the vault lives is where sync
happens; an existing Horizon folder opens as it is, and an empty or new one starts an empty quarter.
`HORIZON_STORE` overrides the remembered choice - the tests and the demo use it.

## The Markdown format

There is one format, and every copy writes exactly the same one. A folder copied to another
machine opens without conversion.

```text
<store>/
├── README.md
├── state.md
├── entries/<day>-<slug>-<shortid>.md
├── notes/<slug>-<shortid>.md
├── days/<day>.md
└── reminders/<slug>-<shortid>.md
```

- **One file per object.** An entry, a note and a day page each get one file. Nothing is aggregated:
  `days/<day>.md` is the day's own page, never a rendered listing of what is on that day.
- **Frontmatter, a blank line, then the writing.** Every field is `key: value`, string values
  JSON-quoted, keys always in the same order. An entry's body is that entry's Markdown note; a
  note file's body is the note; a day file's body is the page.
- **Names never move.** A name is the day, a slug of the text and a short id — no position number —
  so adding or deleting one thing never renames another.
- **LF endings, one trailing newline.** Unknown files are ignored and never deleted.

```markdown
---
kind: "entry"
version: 2
id: "e-6f1c…"
day: "2026-11-04"
title: "Call mum"
mark: "family"
subject: "family"
weight: 1
done: false
mark_auto: true
subject_auto: true
repeat_weekly: false
created_at: "2026-11-04T09:12:00Z"
---

- [ ] ask about Sunday
```

`state.md` holds this device's open day, lens and draw mode, so it is never part of a copy. The
brief's "write a copy of the Markdown store" writes this same tree. The full rules and the
frontmatter of every file type live in `openwiki/systems/document-model-and-persistence.md`.

## The wall

The wall fills the window: the week rows share whatever height is left over, as far as the content
allows, so a large screen prints large days instead of empty paper under the last week. Each cell
carries the day number again as a faint corner numeral that grows with the row - chrome, never
content, so a blank day stays blank.

`HORIZON_WINDOW=1500x1000` opens the desktop window at a given logical size.

## Sync

The store is a git repository, and Horizon is its only client. One HTTPS remote keeps two machines
on the same folder; the version history is the writing itself.

- **Automatic.** Twenty seconds after you stop writing, the burst becomes one commit
  (`Wrote 2 entries, 1 note`) and leaves the device. A launch pulls, and so does a quiet
  five-minute check.
- **One line says whether your writing is stored.** The masthead reads `Local only`, `3 uncommitted`,
  `1 to take`, `2 to send`, `In sync`, or `Sync failed`, on both screens, with a sentence in the
  tooltip. The brief prints the same words, so the label and the block cannot disagree.
- **Every note has a history.** The brief lists the last twelve versions; open one to see the files
  it touched, and open a file to read that version exactly as it was.
- **No conflict dialog.** When both devices changed the same file, the later commit stays and the
  other version is kept beside it under `conflicts/`, named after the commit it came from. Both
  devices compute the same winner, so they converge. Writing always beats deleting.
- **`state.md` never travels.** It is gitignored: open day, lens and draw mode are device state.

Set the remote once in the brief with a personal access token. The token is kept beside the store
(`horizon-sync/token`, mode 0600), never inside it and never in a commit.

Sync is on by default. libgit2 is the engine because gitoxide cannot push: `gix`
0.90 and `gix-protocol` 0.68 have no receive-pack client.

The merge rules are pure and tested in `horizon-core/src/sync.rs`; the libgit2 calls live in
`horizon-app/src/git.rs`; the background thread that commits and syncs is
`horizon-app/src/sync.rs`.

## The agent

The right margin has a second reading: `Notes | Agent` in the column head (`Alt+N` / `Alt+A`).
It talks to an agentic harness — Pi is the first — that runs in the store folder and reads the same
Markdown the wall does, so a question about the quarter needs no export and no second app.

- **Same shape as a loose note.** `Wide ↗` (or `Esc` on the column) lifts the conversation into a
  centred band above the wall; `Esc` returns it. The quarter stays visible beneath it.
- **The store is the context.** Pi starts with a generated manual describing the folder, the
  frontmatter, the marks and the subjects, plus a short preamble naming today, the window, the lens
  and the open day. Nothing is indexed or copied.
- **Writing is visible.** With writing on, Pi may add or edit entries, notes and pages. Horizon
  merges the folder after each run with the same three-way merge as sync, so a change made while it
  worked survives; a real conflict is kept aside under `conflicts/`. The app never commits for it.
- **Nothing durable in the vault.** The transcript and the manual live in
  `~/.config/horizon/agent/`, never in the store.

Pi must be on `PATH` (`pi --version`). Settings live in the brief under **The agent** and in
`~/.config/horizon/config.toml`:

| Key | Default | Meaning |
| --- | --- | --- |
| `agent` | `on` | `off` hides the agent reading |
| `agent.command` | `pi` | the harness binary |
| `agent.provider` | (empty) | passed as `--provider` when set |
| `agent.model` | (empty) | passed as `--model` when set; empty inherits Pi's default |
| `agent.thinking` | (empty) | `off` … `max` |
| `agent.context` | `on` | send the on-screen preamble with each message |
| `agent.write` | `off` | allow `write`, `edit` and `bash`; off is read-only |

## Documentation

The local codebase wiki lives in [`openwiki/`](openwiki/) and starts at
[`openwiki/quickstart.md`](openwiki/quickstart.md). It is plain Markdown with an evidence index,
written by OpenWiki; nothing is published. The visualizer is optional and local:

```sh
openwiki visualize
# http://127.0.0.1:4321
```

Refresh the wiki with `openwiki --update`, or through the OpenWiki Pi package. The human-facing
`README.md`, `ARCHITECTURE.md` and `HANDOFF.md` remain the short entry points; the wiki is the
deeper map for agents and maintainers.

## Zed

Project tasks live in [`.zed/tasks.json`](.zed/tasks.json); debug profiles live in
[`.zed/debug.json`](.zed/debug.json).

- Open the task picker with `ctrl-shift-p` → `task: spawn`, or `zed: open project tasks` to edit the
  file. `Horizon: run desktop (demo)` is the quickest first look; `Horizon: test core` and
  `Horizon: contrast` are the verification tasks.
- Open the local wiki with `Horizon: serve wiki`; it runs `tools/serve-wiki.sh` and serves the
  Markdown wiki at `http://127.0.0.1:4321`.
- Start debugging with `f4` (or `debugger: start`) and choose `Horizon: debug desktop (demo)`. Zed
  builds through the `Horizon: build desktop` task first; the first debug launch may download the
  CodeLLDB adapter.
- The desktop tasks call [`tools/with-libxdo.sh`](tools/with-libxdo.sh), which adds
  `~/.local/lib` to `LIBRARY_PATH`/`LD_LIBRARY_PATH` only when that directory exists. It is a no-op
  when `libxdo` is installed system-wide. The runtime dependency is real - Dioxus's menu crate
  `muda` links `libxdo.so.4` on Linux - so a packaged build needs it on the target machine too.

## Releases

On this Arch desktop, build the installable pacman package with:

```sh
tools/package.sh
sudo pacman -U dist/horizon-0.1.0-1-x86_64.pkg.tar.zst
```

It installs the binary, desktop launcher and Horizon icon, and declares runtime requirements
(`webkit2gtk-4.1`, `gtk3`, `xdotool`). Bump the workspace version in `Cargo.toml`, then rerun the
script for the next package. It builds the release binary with Cargo, then asks `makepkg` to package
it. `--nodeps` skips the local check because this machine's `libxdo` is under `~/.local/lib`, not
installed through pacman; pacman still checks the declared dependencies when installing.

For Debian/Ubuntu, `dx bundle --package horizon-app --release --platform linux --package-types deb
--out-dir dist` produces a `.deb`. The AppImage bundler needs `libxdo.so.4` available on the build
machine; on Arch, install `xdotool` or point `LD_LIBRARY_PATH` at a copy while bundling.

**Debian/Ubuntu package.** `dx bundle --package horizon-app --release --platform linux --package-types deb`
produces `horizon-app_<version>_amd64.deb`. Install with `sudo apt install ./….deb`. The `.deb` does
not declare library dependencies; the target machine needs `webkit2gtk-4.1`, `gtk3` and `libxdo` at
runtime, the same three a development machine needs.

The AppImage is the one-file option, but its bundler refuses to finish while any linked library is
missing on the build machine. Horizon links `libxdo.so.4` through `muda` (Dioxus's menu crate). Arch
ships that library inside `xdotool`, so build the AppImage on a machine where `xdotool` is installed
system-wide, or point `LD_LIBRARY_PATH` at a copy of `libxdo.so.4` while bundling.

**Version.** Both desktop artifacts take their version from `version.workspace = true` in the workspace
`Cargo.toml`; bump it there. The desktop package uses the crate name (`horizon-app`) in its
filename and installed path; the product name in menus and the window title is `Horizon` from
`Dioxus.toml`.

## Scope

Desktop only. Horizon has no server component, is not deployed anywhere, and has no browser build —
the app owns a local folder and nothing else.

## Layout

```text
.zed/           project tasks and debug profiles
horizon-core/   pure Rust: inference, dates, tiers, composer, markdown, export, store, sync, seed
horizon-app/    Dioxus 0.7 UI: wall, fold, index, margin, agenda, notes, brief, menu, git
tools/          contrast verification, libxdo and wiki helpers
```

`horizon-core` holds every product opinion and is unit-tested. `horizon-app` is the renderer and
the only place that touches the webview.

## The four open defaults

- **Recurrence:** one `Repeats weekly` toggle per entry. Future same-weekday occurrences are drawn
  as dashed outline marks, never add ink weight, and have no series management or exceptions.
- **Closed subject:** quiet only. There is no close action and no hidden state.
- **Plan vs. record:** one page object. A page whose first write predates its day prints
  `written ahead`; the past-tense footer is implemented but the forward-only window means it is
  only reachable if the window semantics change.
- **Looking backward:** forward only. No past window and no infinite scroll.

## Deliberately not built

No tags, folders, projects, priorities, colours, streaks, completion percentages, weekly reviews,
overdue badges, review queues, notification streams, charts, rich-text editors, or account
requirements. There is no branch picker, staging area, diff view or merge editor: sync has one
remote, one branch and one automatic rule. If a feature asks the user to tidy something, it is
wrong.

## Licence

MIT. See [`LICENSE`](LICENSE).

The day-card glyphs are from [Lucide](https://lucide.dev) via `dioxus-icons`; see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
