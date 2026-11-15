# Horizon — handoff

Built as a Rust workspace: a pure, tested core and a Dioxus 0.7 desktop/Android UI. The product
spec is the contract; the v6 concept is the visual reference.

## What shipped

**Pure core — `horizon-core`**

- The eighteen marks, each mapped to one fixed Lucide SVG glyph, the seventeen subjects in the
  fixed order, and keyword inference (sum of matched keyword lengths, ties to the earlier row; the reference's
  behaviour). Unknown marks/subjects in imported JSON fall back to Unsorted / Me.
- The 90-day quarter: Monday-first week rows, ISO week numbers, month-change labels, the padded
  first and last week, and the window checks used by the composer and index.
- Tier derivation from `max(weight)`, lens-aware weight, day content, subject counts/days/next,
  clipped vs loose notes, page first-write metadata, and the one recurrence projection.
- The composer grammar in one pure function: `dec 3`, `3 dec`, `3/12`, `today`, `tomorrow`,
  `day after tomorrow`, no-date → open day/today, trailing `/subject`, leading-separator stripping,
  out-of-window refusal, unknown-subject refusal, and a live preview.
- A small markdown parser and AST: headings, bold/italic, code, lists, tasks with source line
  numbers, quotes, tables, rules, safe links, `@Name`, `^[day^label]`, raw-HTML-as-text, and
  `toggle_task_line` for the reading-view checkbox.
- One plain-Markdown store format, identical on desktop and Android: one file per entry, note and
  day, plus `state.md`, with frontmatter, a blank line and a Markdown body. The core owns
  serialization/parsing; the app owns the folder I/O. The rules are frozen in
  `openwiki/systems/document-model-and-persistence.md`.
- Versioned JSON export/import with migration and normalisation. That JSON is the whole-document
  portability path; it is the only other representation.
- A pure sync merge: three file maps in, a merged tree out, path by path. Disjoint edits take both
  sides, writing beats deleting, and two different writes keep the later commit plus the other
  version under `conflicts/<path>.<short-commit>.md`. Both devices compute the same result.
- An empty first launch and an opt-in demo quarter (`HORIZON_DEMO=1`).

**Dioxus UI — `horizon-app`**

- The masthead (range, Marks/Words, The wall/The brief, composer with live mark + subject),
  subhead with the today line and next thing, the seventeen-subject index with counts/en dashes,
  the quiet-subject toggle, lens meta, and the lens reset.
- The wall: weekday band, month-only gutter, 7 columns, up to 9 marks then `+n`, words mode,
  four today markers (stamp, hairline frame, `today` word, `now` gutter + encarnado month),
  tier washes and solid-ink landmarks, and compressed empty weeks.
- The fold: in-place day page with aside, huge numeral, standfirst, previous/next chips, large
  marks, whole-day entries under a lens, clipped details with detach, a bound composer, and the
  day's own markdown page (collapsed when empty, opened when written).
- Entry detail: checkbox, editable title, mark picker (18), subject picker (17), three-way weight
  control, landmark, the single recurrence toggle, reset-to-automatic for mark and subject,
  remove, and a markdown note with write/read, toolbar, task ticking, `@Name`, day chips, links.
- Loose notes in the right margin, grouped by subject (index order) with a filter chip row,
  each note opening in place into the shared markdown editor with a subject picker, plus
  clip-to-a-day with armed cells, detach, remove, and a composer honouring a trailing `/subject`,
  an explicit subject pick, then the lens.
- `Write wide`: the full-width writing band above the wall for a loose note — the same shape as a
  day's page, opened in write mode, Esc or `Back to the column` to return, the margin row stepping
  aside so the editor exists once. Clipping or removing the note closes the band; focus returns to
  the note's row.
- The Benfica pass, all in tokens and one component: the accent family is encarnado
  (oklch 48% 0.20 28.5 ≈ #B30002, the club's red, deepened) where terracotta was — today
  markers, focus rings, armed cells, selection — on the same warm paper; a faceted eagle mark
  beside the wordmark and on the boot splash; a red nameplate rule slightly proud of the
  masthead's ink hairline; and `E pluribus unum` set small at the foot of the margins, where
  the club keeps its ribbon — out of many loose details, one day. Red stays
  interaction-and-today only; ink and paper still carry the structure.
- The mobile shape below 700px: vertical week-by-week agenda, current week always in full,
  landmark rows filled with ink, today stamp + frame + word, no horizontal scroll, 44px targets.
- The brief: the key, the principles, the four open defaults, JSON download, a copy of the
  Markdown store, JSON restore, and the Sync block with its configuration form. No charts,
  no settings screen, no app preferences.
- Sync: the store folder is a git repository with one HTTPS remote. A quiet twenty-second burst
  becomes one commit, then a pull and a push; a launch pulls; a five-minute timer does the same;
  quit flushes. The brief's Sync block is one configuration screen for the whole life of the store:
  the same fields connect first time and update later — the remote field carries the
  connected URL, the username and password replace the saved pair, and a failure reopens the form because
  the fix usually lives in it. On hosts that refuse real passwords (GitHub, GitLab, Codeberg) the
  password slot takes a token or app-password; the pair is the same over HTTP basic auth. The
  credentials are stored outside the store (app data dir, mode 600, re-read on every network
  operation) so they can never be committed; `sync_credentials_live_outside_the_store`
  pins that invariant. An SSH address still gets one password attempt, then falls back to keys;
  an SSH auth failure is explained in place (`explain`) and the form flags an SSH address at the
  Remote field, because that transport wants keys, not the credentials below it. The brief also shows the state, the remote and the last twelve versions, with each
  commit's files and any file's text at that commit. Conflicts are never a dialog.
- Persistence: the live store is a Markdown folder, on desktop and on Android alike. One saver loop
  coalesces mutations and awaits each write, so a later save cannot be overtaken by an earlier one.
  Open day, lens, draw mode and note editor modes survive reload. The window ticker rolls forward
  without deleting data.
- Focus: restored open days focus the fold; opening/moving a fold focuses it after the DOM update;
  closing returns focus to the same cell (desktop) or agenda row (mobile). Arrow navigation works
  on cells and inside the fold.
- Zed integration: project tasks for run/serve/build/test/check/clippy/fmt/contrast/wiki, and CodeLLDB
  debug profiles for the demo and no-demo desktop app. Desktop tasks go through
  `tools/with-libxdo.sh`, which adds the local `libxdo` prefix only when it exists.

## Verification performed

- `cargo test -p horizon-core` — **67 passed**.
- Loose-note bodies are unit-tested: task toggling inside an undated note's markdown, subject
  re-filing, removal clearing the note's editing state, the `/subject` token split, and a
  multi-line undated note round-tripping through the store with `day: null`.
- `cargo test -p horizon-app` — **20 passed** (Markdown store write/read, stale-file cleanup, v1→v2
  name migration, three git tests, eight `sync::status` tests pinning the masthead label, and three
  vault tests (`~` expansion, choosing a folder, a fresh vault reading as an empty store);
  needs `LIBRARY_PATH=$HOME/.local/lib` on this machine).
- The git tests use a real bare repository as the remote: two device folders, fast-forward,
  a disjoint merge, a same-file conflict with the aside file identical on both sides, and
  `state.md` absent from every commit.
- End-to-end desktop run against a bare remote: a fresh store plus `HORIZON_DEMO=1` produced
  `Wrote 31 entries, 4 notes, 6 day pages, 1 file` after twenty quiet seconds and pushed 43 files;
  a second empty store pointed at the same remote pulled all 31 entries, merged the unrelated
  histories, and pushed the merge commit.
- `cargo check -p horizon-app` — clean.
- `cargo check -p horizon-app --no-default-features --features mobile --target aarch64-linux-android`
  — clean (compiled, not device-verified).
- `dx build --desktop` (Dioxus CLI 0.7.10) — succeeded. Linux needed `libxdo`; it was built locally
  into `~/.local/lib` for this environment. A normal install is `xdotool`.
- Native Markdown store end-to-end: launched the app with `HORIZON_STORE=/tmp/horizon-e2e
  HORIZON_DEMO=1`, got 31 entry files, 4 note files, 6 day files and `state.md`; a second launch
  loaded the same folder without adding or duplicating files.
- Interaction checks were made on a browser build that has since been removed from the project.
  They exercised the same `main.css` and the same components the desktop app renders, so the
  findings below still describe Horizon's behaviour, but the checks cannot be re-run as written;
  they are kept as a record of what was verified, not as a live test path.
  - fold opens in place; empty day page collapses to `+ Write the day's page`; opening it focuses the
    textarea;
  - Esc closes the fold and returns focus to the same cell/agenda row; arrow keys move focus
    (right/left one day, up/down one week); arrow keys inside a fold move the fold and keep focus;
  - lens filters the wall, recomputes weight, hides `N`/`MD`, and the day page stays whole (a House
    entry remained visible under a Family lens);
  - composer filed `tomorrow - call mum` to the right day with the inferred People/Family, refused
    an out-of-window date, refused an unknown `/nonsense` subject, and honoured `/travels`;
  - mark and subject overrides persisted across reload; mark picker and subject picker update the
    row and the wall;
  - entry note write → Escape → read view, and the `N` badge appeared; task ticking rewrote both an
    entry note and a day page (`- [x]` in the stored markdown);
  - loose note clip → day, detach → margin;
  - JSON import replaced the document and the brief status confirmed it; JSON and markdown copies
    were produced;
  - raw `<img onerror>` / `<script>` printed as text (no `img`/`script` nodes); safe links got
    `target="_blank"` and `rel="noopener noreferrer"`, `javascript:` links stayed inert; `@Sipho`
    printed as a citation and `^[2026-11-15^drive]` as a day chip;
  - `written ahead` marker on a future page; recurrence projections rendered dashed and did not add
    a tier class;
  - mobile agenda current week in full, empty weeks skipped, today stamp/frame/word, no horizontal
    scroll at 390px; the wall remains at 800px with no horizontal scroll.
- `python3 tools/contrast.py` — every normal-text pair meets 4.5:1; the lowest reported pair is
  encarnado-on-paper at 6.78:1; the tooltip text is 15.90:1 and its label 16.38:1. Hover backgrounds were included
  in the check; none reduces contrast.
- `rg 'innerHTML|outerHTML|dangerous_inner_html' horizon-app/src horizon-core/src` — no such use in
  our source. (Dioxus's generated interpreter file under `target/` uses `innerHTML` for its own
  attribute plumbing; it is not in the note path and is not source.)

## The four OPEN defaults, as implemented

1. **Recurrence** — `Entry.repeat_weekly`, one toggle. Future same-weekday occurrences are derived
   by `Doc::projected_on`, rendered with `.mk.is-projected` (dashed, thinner), and never contribute
   to `weight`/tier. No series editor, no exceptions, no end date. The demo's `Gym` shows 12
   projections across the window.
2. **Closed subject** — not built. `CLOSED_SUBJECTS = false`; a subject can only be quiet. The
   index prints the en dash.
3. **Plan vs. record** — one `Doc.pages` object. `Doc.page_meta.first_written` is set on the first
   write; `Doc::page_written_ahead` drives the `written ahead` marker. `Doc::page_is_past` and the
   footer exist, but with a forward-only window that starts today, a day in the window is never
   past; the footer becomes reachable only if a past window is added. No second page type.
4. **Look backward** — not built. `LOOK_BACK = false`; forward 90 days only, no infinite scroll.

## Deliberate decisions and omissions

- **The marks use Lucide, not the reference's hand-drawn paths.** The eighteen semantic marks stay
  fixed and one-to-one with Lucide glyphs; CSS normalizes stroke weight, square caps, miter joins
  and no fill. This trades exact prototype geometry for a maintained 1,700-icon set and is the one
  deliberate departure from the paper's original §2.1 wording. The spec and
  `THIRD_PARTY_NOTICES.md` record it.
- **`dioxus_components` 0.1.2 was not used.** The crate is a Tailwind v4 / Radix-styled component
  collection built for the browser (`js-sys`, `dioxus/web`). Its unconditional `js-sys` dependency and
  Tailwind/Radix theme make it unsuitable for the desktop/Android targets here and would replace
  the locked editorial visual system. Horizon uses Dioxus's component model with a local
  `components/` library instead. This is the one place the architecture note and the implementation
  differ, and it is recorded here rather than hidden.
- **13 vs. 14 week rows.** The spec asks for a 90-day window *starting today* and exactly 13 week
  rows. Those coincide only when today is Monday or Tuesday. Horizon shows all 90 days and keeps
  weekday columns true; the grid is 13 rows on Monday/Tuesday and 14 otherwise. Today (Wednesday
  4 Nov 2026) is 14. No day is dropped. See `ARCHITECTURE.md`.
- **Persistence is a plain-Markdown folder on desktop and Android**, with `state.md` for the
  persisted UI slice. Sync is git against one HTTPS remote; there is no server of Horizon's own, no
  account and no conflict dialog.
- **The web target was removed.** Horizon is desktop + Android only; there is no browser build, no
  wasm dependency and no `localStorage` store. It existed for visual verification with in-page DOM
  measurement, and it is gone because the app is not deployed anywhere and the measurement crutch
  cost a `cfg` split through storage, timers, menus and `main`. Everything conditional in the code
  is now desktop-versus-Android.
- **The Markdown copy is the store itself.** Desktop writes a timestamped folder under Downloads;
  Android writes to the app's external files directory, falling back to the app-private folder.
  The bytes are the same, so a folder moves between devices without conversion. The live Android
  store is still app-private Markdown.
- **No notification system, no account, no settings screen, no charts, no tags/folders/projects/
  priorities/colours/streaks/reviews/overdue states.** The only secondary tab is the brief.
- **The demo quarter is opt-in**, never seeded silently. A real first launch is empty and quiet.
- **Android was cargo-checked, not built or device-verified** (no Android SDK/NDK). The `mobile`
  feature, the shared crate, the `sysdirs` app-data path and the responsive shape are in place; the
  command is in `README.md`.
- **Past-tense footer** is implemented but effectively unreachable under the forward-only window;
  noted above rather than pretending otherwise.
- **The date `today` is the machine's local date**, not a fixed demo date. The demo quarter is
  generated relative to today.

## Definition-of-done checklist

| Item | Status |
| --- | --- |
| 13×7 Monday-first wall, weekday band, months only in gutter | Yes on Monday/Tuesday; 14 rows otherwise to keep all 90 days (documented). Current date = 14. |
| Four today markers, greyscale-legible | Yes (stamp, frame, `today`, `now` gutter/encarnado month); mobile adds stamp + frame + word. |
| Tiers 0/1/2/3, empty weeks compress, today's week never | Yes. Empty-week test: 13 quiet rows of 14, current week full. |
| Fold opens in place with complete day page | Yes. |
| Esc, arrows, focus restoration | Yes, verified. |
| Index: 17, counts, en dash, lens filters/recomputes/reversible, day whole | Yes, verified. |
| Composer grammar + live preview + refusal | Yes, 9 composer tests + the interaction checks above. |
| Notes/day pages: write/read, toolbar, task ticking, @Name, day chip | Yes, verified. |
| Markdown XSS-safe, tables/quotes/lists/code/links | Yes; AST → typed nodes; raw HTML text; safe links only. |
| Loose notes in margin; arm and click a day to clip | Yes, plus detach, per-subject grouping with a filter chip row, and a per-note markdown editor with a subject picker. |
| `MD` / `N` badges correct, hidden under lens | Yes. |
| Words mode full sentences, persists | Yes. |
| Mobile agenda <700px, no horizontal scroll, 44px targets, today 3 ways | Yes; 390px and 800px checks. |
| Persistence: reload, Markdown live store, JSON import/export, Markdown copy | Yes; one format on desktop and Android; saver loop; copy destinations per platform. |
| Contrast, focus rings, hover | Contrast script all ≥4.5; focus rings in CSS; hover checked. |
| No §7 features | Yes. |

## Files

- `horizon-core/src/{inference,dates,model,composer,markdown,export,store,sync,seed}.rs`
- `horizon-app/src/{app,state,storage,git,sync,android,desktop,menu}.rs`,
  `horizon-app/build.rs` (the `desktop_shell` cfg)
- The frozen Markdown format: `README.md` (summary + example), `ARCHITECTURE.md`, and
  `openwiki/systems/document-model-and-persistence.md` (rules, frontmatter table, naming, versioning)
- `horizon-app/src/components/{chrome,index,wall,fold,margin,agenda,notes,markdown,marks,brief,sync,labels,menu}.rs`
- `horizon-app/assets/main.css`
- `THIRD_PARTY_NOTICES.md` (Lucide / dioxus-icons)
- `.zed/tasks.json`, `.zed/debug.json`
- `openwiki/` (local Markdown wiki; start at `openwiki/quickstart.md`), `AGENTS.md`
- `ARCHITECTURE.md`, `README.md`, `Dioxus.toml`, `tools/contrast.py`, `tools/with-libxdo.sh`, `tools/serve-wiki.sh`

## Next steps if continued

1. Add an Android build/CI job and verify, on a device: the Markdown app-data folder, the
   external-files copy, and the `mobile,sync-vendored` build (NDK + vendored OpenSSL).
2. If the owner wants a past window, add a discrete "the last ninety" reading behind `LOOK_BACK`
   and make the past-tense page footer reachable; do not turn it into infinite scroll.
3. If Android should write into a user-chosen folder (Documents, an Obsidian vault), add a
   Storage Access Framework picker; the external-files copy is the fallback until it exists.
