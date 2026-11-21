# Horizon — architecture

This is the build note for the Rust implementation. The product spec (`horizon-product-spec.md`)
is the source of truth; the prototype (`horizon-life-os-concept-v6.html`) is the visual and
interaction reference.

## Shape

```
horizon-core/        pure Rust. No DOM, no renderer, no I/O. Every product opinion lives here.
  model.rs           Doc / Entry / LooseNote / PageMeta / UiState, enums, tiers, derived views
  inference.rs       the eighteen marks, the seventeen subjects, keyword scoring
  dates.rs           quarter maths: 90 days, Monday-first week rows, ISO weeks, month changes
  composer.rs        the one-line grammar (dates, separators, /subject)
  markdown.rs        small markdown AST + inline parser + task-line rewriting
  export.rs          JSON round-trip only (the whole-document portability path)
  store.rs           the one Markdown format: one file per entry/note/day, plus state.md
  sync.rs            the pure merge: path by path, keep-both conflicts, commit messages
  seed.rs            empty document + an opt-in demo quarter (tests / HORIZON_DEMO=1)

horizon-app/         Dioxus 0.7 desktop UI.
  app.rs             the root component: save loop, keyboard, quit, first-run switch
  boot.rs            the first-run page: choose the vault folder
  state.rs           AppCtx: signals for the document, ephemeral UI, quarter, status
  storage.rs         the Markdown file store, the vault choice, atomic writes, the token file, DOM focus helpers,
                     the config keys (vault, sync, agent), and the three-way merge used by sync and the agent
  git.rs             libgit2: open/commit/fetch/merge/push/log (feature `sync`)
  sync.rs            the background sync thread and the snapshot the brief renders
  agent/
    mod.rs           the agent service: harness process, state, transcript, manual, context, reconcile gate
    pi.rs            the Pi RPC adapter: JSONL, event translation, extension dialogs declined
  menu.rs            the store path and the menu hint
  desktop.rs         the folder dialog, quit, and "open the folder" (desktop only)
  components/
    chrome.rs        masthead, composer, subhead, footer, tooltip, status, eagle mark
    index.rs         the seventeen-subject index + lens meta
    wall.rs          weekday band, week gutters, cells, words mode, empty-week compression
    fold.rs          the in-place day page: aside, marks, entries, clipped notes, composer, page
    margin.rs        loose notes grouped by subject, filter chips, per-note markdown
                     editor + subject picker, clip-to-a-day, detach, remove, composer,
                     the full-width writing band a note borrows when it grows, and the
                     `Notes | Agent` reading switch in the column head
    agent.rs         the margin's agent reading: status, transcript, composer; and the
                     wide conversation band above the wall
    agenda.rs        the narrow week-by-week shape below 700px
    notes.rs         markdown editor (write/read, toolbar, task ticking, commit on read)
    markdown.rs      AST → Dioxus nodes. No innerHTML anywhere.
    marks.rs         one Lucide SVG glyph per semantic mark, with the shared stroke treatment
    brief.rs         the secondary tab: the key, the principles, the Markdown copy, JSON import/export
    sync.rs          the brief's Sync block: one config form (connect and
                     username/password update — credentials stored outside
                     the store), state, versions, a version's text
    menu.rs          the in-window menu strip (desktop only, `MenuStrip`)
```

## State and persistence

- `Doc` is the in-memory model. The live durable store is a folder of plain
  Markdown files: `entries/<day>-<slug>-<shortid>.md`, `notes/<slug>-<shortid>.md`,
  `days/<day>.md`, `reminders/<slug>-<shortid>.md`, and `state.md`. Each file is frontmatter, a
  blank line, then a Markdown body. There is no server, no database and no cloud copy; the folder
  is the store.
- **One format.** Every Markdown copy writes the same tree with
  the same frontmatter key order. There is no rendered digest and no per-platform variant: the old
  `export::markdown_folder`/`markdown_bundle` digest was removed. Names carry no position number,
  so adding or deleting one thing never renames another; read order is `created_at` then id (with
  the legacy `NNN` filename index as the tiebreak for pre-existing folders). The rules are written
  down in `openwiki/systems/document-model-and-persistence.md`; change them only with a version
  bump.
- `Doc.ui` stores the four things the spec says must survive reload: open day, subject lens, draw
  mode, and per-note editor mode. These live in `state.md`; `state.md` is still Markdown.
- Ephemeral UI (pickers, drafts, tooltip, status, quiet-index toggle, brief tab) lives in
  `AppCtx.ui` and is not persisted.
- **Reminders are a separate record, not a field.** `Doc.reminders` holds `Reminder` values, each
  written to `reminders/<slug>-<shortid>.md` with `kind: reminder`; removing a note or entry
  cascades to the reminders sourced from it, and a spent one-off is not written at all while a
  repeat stays and moves to its next occurrence. The grammar and arithmetic live in
  `horizon-core/src/reminders.rs` (`parse_when`, `next_after`, `advance`, `snooze`, `humanize`), so
  the app and `--fire-due` share one implementation. Delivery is hybrid: `App` polls once a second
  while open and touches `reminders.heartbeat`, which makes the opt-in systemd user timer
  (`horizon-app --fire-due`, once a minute) stand down; the timer writes only `reminders/` and never
  touches `state.md`. Catch-up is a single quiet sentence, never a burst.
- Persistence is silent. Every mutation bumps one `dirty` counter; a single saver loop in `App`
  coalesces changes and awaits each write. `storage::save_now` serializes the document with
  `store::to_files` and writes the whole Markdown folder; stale managed files are removed. There is
  no save indicator and no toast.
- **The vault is chosen, not assumed.** With no `config.toml` yet, `App` renders `boot::VaultBoot`
  instead of the wall: one page, a suggested path (`~/Documents/Horizon`, or `~/Horizon` when there
  is no Documents folder), a typed field and the system folder dialog through `rfd` (its backend is
  the xdg portal, which dioxus-desktop already enables). `storage::choose_vault` creates the folder,
  seeds `README.md` and writes the path to
  `dirs::config_dir()/horizon/config.toml` as `vault = "…"` - with the app's config, never inside
  the vault, so moving the folder keeps the pointer valid and the pointer never enters a commit.
  The file is one `key = value` per line with JSON-quoted strings (valid TOML), read by a 20-line
  parser so the single key needs no TOML dependency; comments and unknown keys are ignored, so it
  can grow. The page then reloads, so the store, the saver and the sync thread all
  start against the chosen folder. `HORIZON_STORE` still wins when set, for tests and scripts.
- That folder is also the git repository, which is why it is the owner's to place: sync happens
  wherever the vault lives, and an existing vault can be pointed at a remote without moving files.
- `export::to_json` / `export::from_json` remain the whole-document portability boundary. The
  Markdown copy is not a second format: `storage` writes `store::to_files` minus `state.md`, plus
  the same `README.md` the live folder carries. The copy is a timestamped folder under Downloads.
- Demo data is opt-in: `HORIZON_DEMO=1` on an empty store, or the `demo()` constructor in tests.
  A real first launch is a quiet, empty quarter.

## The wall fills its window

`.body` and `.wall-wrap` are flex columns and `.wall` grows; `align-content:stretch` plus
`grid-auto-rows:minmax(46px,auto)` share the leftover height across the week rows. Rows are still
content-sized when content needs more (an open fold pushes the wall down instead of clipping).
Each cell reprints the day number as a faint bottom-right numeral sized in container units of the
row (`5cqi`, capped at 44px) - decorative, `aria-hidden`, and the only thing that grows when a day
is empty.

## Sync

The store folder is also a git repository. Horizon is the only client: no staging area, no branch
picker, no diff view, no merge editor, one remote and one branch (`main`).

- **One writer.** The sync thread owns the repository (`horizon-app/src/sync.rs`). The UI sends
  commands and reads a snapshot; it never touches git and never blocks on the network.
- **Checkpoints.** Every store write marks activity. After twenty quiet seconds the burst becomes
  one commit (`Wrote 2 entries, 1 note`), then a pull, then a push. A launch pulls once, and a
  quiet five-minute timer does the same. Quit flushes and waits up to eight seconds.
- **One place decides the words.** `sync::status(&SyncState)` is a pure function returning a short
  label, a one-sentence detail and a tone (`calm`, `wait`, `alert`). The masthead prints it on every
  screen; the brief prints the same label. Precedence: error, busy, no remote (`Local only`),
  uncommitted files, behind, ahead, `In sync`. Eight unit tests pin that order, so the UI cannot
  drift into a second opinion about whether the writing is stored.
- **Merge is pure.** `horizon-core/src/sync.rs` takes the base, local and remote file maps plus the
  two commit identities. A path changed on one side takes that side. A path deleted on one side and
  written on the other keeps the writing. Two different writes keep the later commit in place and
  write the other to `conflicts/<path>.<short-commit>.md`. The aside name comes from the losing
  commit, so both devices compute the same winner and the same file: they converge instead of
  trading copies.
- **Unrelated histories** (both devices started before the first sync) merge against an empty base,
  so neither side is lost.
- **Reload after pull.** A pull that changes the tree bumps a revision. The app reloads the document
  from the folder before the next save, so memory never overwrites the other device's writing.
- **`state.md` is gitignored.** Open day, lens and draw mode are device state; syncing them would
  move your desktop's open day whenever you touched the other machine.
- **Credentials.** HTTPS + a personal access token. The token is written to
  `data_dir()/horizon-sync/token` with mode 0600 on Unix — beside the store, never inside it.
- **Why libgit2.** gitoxide fetches and clones but does not push: `gix` 0.90 has no
  `git-receive-pack` client and no push entry point, and the `gitoxide` CLI has no push subcommand
  either. Sync without push is not sync, so `git2` is the engine.
- **Feature gate.** `sync` (default on) pulls in `git2`. Without the feature the app compiles and
  runs exactly as before; the brief has no Sync block.

## The agent

The margin has a second reading. `Notes | Agent` in the column head (`Alt+N` / `Alt+A`) keeps the
same address and the same widening move a loose note already has: `Wide ↗` lifts the conversation
into a centred band above the wall, and `Esc` returns it to the column. The quarter stays visible
under the band; it is a reading surface, not a modal, and it adds nothing to the navigation.

- **One process, one service.** `agent::Service` owns a single `pi --mode rpc` child for the app
  run, started lazily on the first message. The UI polls `agent::snapshot()` and never blocks on the
  harness. The service owns the state machine (`Idle → Starting → Ready → Working → Stopped/Failed`),
  the transcript, the manual and the context preamble; the components own only the typography.
- **The harness reads the store, not a copy.** The child runs with the vault as its working folder,
  so `read`, `grep`, `find` and `ls` see the same files the wall does. A generated manual
  (`~/.config/horizon/agent/horizon-agent.md`) states the folder layout, frontmatter, the vocabulary
  and the boundaries; it is passed with `--append-system-prompt`. Each message carries a short
  `<horizon-context>` preamble (today, the window, the lens, the open day, counts, upcoming entries,
  loose notes) so the agent starts from the current reading without a rebuild step.
- **RPC, not one-shot.** `--mode rpc` is JSONL on stdin/stdout: it streams text, thinking and tool
  calls, accepts a steer while a turn is running, and ends with `agent_settled`. That is what makes
  the transcript show the work instead of a spinner. Extension dialogs are declined
  (`extension_ui_response` with `cancelled: true`) and printed as a notice, because Horizon has no
  modal; notifications are ignored. The adapter lives in `agent/pi.rs` and is the only file that
  knows Pi's wire format.
- **Writing is a second writer, gated by a merge.** With `agent.write = true` the child gets
  `read,write,edit,bash,grep,find,ls`; otherwise only `read,grep,find,ls`. After a run ends the
  saver stays paused (`agent::busy()` is true while a run is in flight *and* until the app has
  merged), then `storage::reconcile` reads the folder and runs the same three-way merge as sync:
  disjoint edits take both sides, writing beats deleting, and two different writes keep the later
  one with the other under `conflicts/<path>.<stamp>.md`. The app never commits for the agent; the
  owner's sync owns the commit. A file the agent writes with no frontmatter is not a managed file
  and is left alone.
- **Nothing durable lives in the vault.** The transcript is
  `~/.config/horizon/agent/transcript-<vault-hash>.jsonl` (capped at 400 items) and the manual sits
  beside it; both are regenerated, neither is synced, and neither is committed. The conversation is
  not part of the document model, so the store stays exactly the quarter.
- **Configuration is in the brief.** `The agent` block writes the keys below and restarts the
  harness; `Check harness` runs `<command> --version`. The model is inherited from Pi's own
  configuration unless `agent.model` (and optionally `agent.provider`) names one, so the same model
  the owner uses in the terminal is the one in the column.

| Key | Default | Meaning |
| --- | --- | --- |
| `agent` | `on` | `off` hides the agent reading; the column says so instead of inviting a message |
| `agent.command` | `pi` | the harness binary; must accept `--mode rpc` |
| `agent.provider` | (empty) | passed as `--provider` only when set |
| `agent.model` | (empty) | passed as `--model` only when set; empty inherits Pi's default |
| `agent.thinking` | (empty) | `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` |
| `agent.context` | `on` | include the `<horizon-context>` preamble with each message |
| `agent.write` | `off` | allow `write`, `edit` and `bash`; off is read-only |

- **Pi is the first adapter, not the only one.** `AgentConfig.command` is the seam: a different
  harness needs a new module beside `pi.rs` that speaks the same JSONL events, while the service,
  the components and the merge contract stay as they are.

## Releases

On this Arch desktop `tools/package.sh` builds a pacman package. It runs Cargo in the checkout,
then uses `makepkg` to install the release binary, desktop entry, icon and license into the archive.
`makepkg --nodeps` is used because libxdo is in `~/.local/lib` on this host, not installed as
`xdotool`; the resulting package declares `webkit2gtk-4.1`, `gtk3` and `xdotool`, which pacman checks
when the package is installed. The binary itself is 7.2 MB; the `.pkg.tar.zst` is 2.2 MB compressed.

The script is deliberately a local build, not a publishable AUR recipe: `makepkg` in its isolated
fakeroot shell failed to resolve the C symbols in vendored libgit2 here, while Cargo in the checkout
links it. `tools/package.sh` builds once in the normal shell and packages that exact binary.

`[bundle]` in `Dioxus.toml` supplies the identifier `app.horizon`, the product name, both
descriptions, the publisher and `formats = ["appimage"]`. The Debian `.deb` bundle is available via
`dx bundle --platform linux --package-types deb`, but it does not declare library dependencies.
- **AppImage** is blocked on a build machine without `libxdo.so.4`: linuxdeploy aborts deployment
  when a `NEEDED` library is missing. The link comes from `muda` (Dioxus's menu crate) on Linux;
  the system tray does not, and neither does `rfd`, so disabling the tray would not remove it.

## Platform

- `cargo run -p horizon-app` → desktop; `dx build --desktop` also works.
- There is no web target. Horizon is a desktop app with no server, no
  deployment and no browser build. It has no platform-specific `cfg` split.
- The desktop window has no usable system menubar: `muda` on GTK draws the submenu titles but no
  entries, and the default bar offers only hide/maximize anyway. So the menu is a Horizon component
  (`components/menu.rs`) — a menubar strip, a sheet, and keyboard paths (`Alt+H`, `F10`, arrows,
  `Enter`, `Esc`). Quit saves the store first, then closes the process.
- Dioxus 0.7.10 is the stable line. 0.8 is still alpha, so it is not used.
- `App` renders exactly one of the wall or the agenda based on a 700px viewport signal, so the fold
  and every entry id exist once. Focus follows the fold after the DOM update: a new/moved fold
  takes focus, a closed fold returns focus to the same cell/agenda row.

## On `dioxus_components`

The crate `dioxus_components` (0.1.2) is a Tailwind v4 / Radix-styled component collection built
for the browser (`js-sys`, `dioxus/web`). Its unconditional `js-sys` dependency and Tailwind/Radix
theme make it unsuitable for the desktop target here, and importing its styles would
replace the locked editorial visual system with a card/rounded/shadow theme. Horizon therefore uses
Dioxus's component model directly and keeps a small local `components/` library. This is a
deliberate platform + design-system decision, not an omission; it is recorded in the handoff.

## The wall's week count

The spec asks for both a 90-day window *starting today* and exactly 13 week rows. Those two
statements only coincide when today is Monday or Tuesday. The prototype's `weeksOf()` pads the
first week and shows all 90 days, which produces 13 rows on Monday/Tuesday and 14 otherwise.
Horizon follows the prototype: every one of the 90 days is shown, weekday columns stay true,
and no day is dropped. On the current date (Tuesday) the grid is 13 × 7.

## Open defaults

| Question | Default | Where |
| --- | --- | --- |
| Recurrence | One `repeat_weekly` toggle per entry. Future same-weekday occurrences are derived, drawn as dashed outline marks, never ink weight, and have no editor or exceptions. | `Entry.repeat_weekly`, `Doc::projected_on` |
| Closed subject | Quiet only. No close action, no hidden state. | constant `CLOSED_SUBJECTS = false` |
| Plan vs. record | One page object. `page_meta.first_written` enables a `written ahead` marker and a past-tense footer. No second type. | `Doc::page_written_ahead` |
| Look backward | Forward only. No past window, no infinite scroll. | constant `LOOK_BACK = false` |
