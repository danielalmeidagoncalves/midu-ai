# Horizon — product specification

> The build contract for Horizon: a personal life OS that shows the next ninety days as a
> printed wall of marks, one subject index, and a markdown page under every thing and every day.
>
> **For the coding agent reading this:** follow this document literally. Where it says *do not
> build*, do not build it, however obvious the feature looks. Sections marked **DECIDED** are
> settled by six design revisions; changing them silently is a defect. Sections marked **OPEN**
> are the user's decisions — implement a clear default, make it a single switch or constant, and
> note it in the handoff summary rather than asking mid-build.
>
> Reference implementation (read it, do not port it line for line):
> `horizon-life-os-concept-v6.html` — a single 2,500-line vanilla-JS concept with the full visual
> system, the mark vocabulary, the inference rules and a 120-line markdown renderer. The opinions
> in it are the product; the architecture is not.

---

## 0. The problem this app exists to solve

The user cannot keep their life organised or hold the important dates in their head. They have
already tried Obsidian, Logseq, Tolaria, OneNote, Todoist and others. **All of them died the same
way: the system needed maintenance the user never did.**

Two constraints follow from that, and they outrank every feature in this document:

1. **Nothing may ask to be tidied.** No inbox to process, no tags to manage, no weekly review, no
   streaks to break, no "3 overdue" guilt badges, no sync conflict to resolve, no taxonomy to
   maintain.
2. **The app must be beautiful enough that opening it is a pleasure**, because beauty is the
   actual retention mechanism. This is a designed object, not a CRUD UI.

Four discovery answers, treated as requirements:

| Answer | Requirement |
| --- | --- |
| *Died: maintenance debt* | Zero-maintenance capture. Filing is inferred, never requested. |
| *One thing perfect: see my next 30–90 days as a real picture* | The primary view is a fixed 90-day window, always the full picture, never a list of the future. |
| *Retention: it is beautiful* | Editorial print register. No dashboards, no cards, no shadows. |
| *Stage: idea, then a concept interface* | The visual concept is **locked**. Build it; do not redesign it. |

---

## 1. DECIDED — the shape of the product

### 1.1 One screen, not a navigation tree

There is no sidebar, no menu, no settings screen in the primary flow. The app opens on **the wall**.
Everything else is either a fold-out inside the wall or a secondary tab reached from the masthead.

### 1.2 The wall is a quarter spread

- A fixed **90-day forward window** starting today. Not a calendar, not a month view, not infinite
  scroll.
- Layout: **13 week rows × 7 weekday columns**, Monday first. A weekday band sits above the grid.
- Every cell shows the day numeral plus **one mark per thing on that day** (see §2). Up to 9 marks
  per cell, then `+n`.
- **Density is the visual weight.** A day's tier is `max(weight)` across its things, where weight is
  `1` ordinary, `2` full day, `3` landmark:
  - `0` — open: plain paper, hairline cell border.
  - `1` — one thing: pale ink wash (`--wash-1`).
  - `2` — full day: deeper wash (`--wash-2`).
  - `3` — landmark: solid ink block, paper-coloured text, terracotta numeral.
- **Empty weeks compress** to a thin band. The page spends height on weeks that hold something, so
  a blank stretch reads as calm instead of ninety equal boxes of guilt. The week containing today is
  never compressed.
- **Weekday orientation is a hard requirement.** Column position means *which weekday*; therefore no
  month rail or date scale may run across the top. Months are named in the left gutter, only when
  the month changes.

### 1.3 Today is unmistakable, by four marks

1. Numeral set in paper on a solid terracotta block (a postmark, not a highlight).
2. A hairline frame around the whole cell.
3. The word `today` at the foot of the cell — so the marker survives greyscale and colour blindness.
4. The current week's gutter carries a small `now` block and a terracotta month label.

The marker must never borrow the wash/ink density encoding: an empty today still reads as today.

### 1.4 The fold — detail opens *inside* the wall

Clicking a day does **not** navigate, open a modal, or push a panel. The day's own week row grows and
the cell inflates in place into a **day page**; every other day stays exactly where it was. This is
the core creative move and the reason the app beats a calendar app.

The day page contains, in order:

1. **Aside** — week number, tier word, huge numeral (≈140px), weekday + full date, a one-line
   standfirst ("This one changes something." / "Open. Nothing has to resolve here."), and previous /
   next day chips.
2. **The day's marks**, large, with a count.
3. **The entries** — checkbox, title (editable in place), mark, subject handle, weight control,
   landmark toggle, remove, and under each row an optional markdown note (§3).
4. **Clipped details** — loose notes that carry this date.
5. **A one-line composer** bound to this day, with a live mark + subject preview while typing.
6. **The day's own markdown page** (§3.2) — collapsed as `+ Write the day's page` when empty, opened
   automatically when it has content.

Only one day is folded at a time. `Esc` folds it shut and returns focus to the same cell.

### 1.5 The subject index

Seventeen subjects, in this exact order, printed as one wrapping line under the masthead:

```text
AI · Assets · Backpack · because everyone deserves to be happy · Bosch · Sketches · Family ·
Finances · House · Journalling · Kids · Me · Payments · Prompts · Retrogaming · Tech · Travels
```

- Each carries a **count of things in the window, or an en dash**. The dash is the instrument: it
  makes the *quiet* subjects visible without a badge, streak or warning. It is the same fact the
  wall tells you about a quiet week.
- Clicking a subject reads the whole quarter **through** that subject: every other mark leaves the
  wall, ink weight recomputes from what remains, and that subject's gaps become the shape. The meta
  line becomes `Name · n things · d days · next <mark> <title> · <date>`.
- Clicking again (or `Show the whole quarter`) returns to everything. A lens is a reading, never a
  filter that can strand the user.
- Quiet subjects collapse behind a `Quiet subjects (n)` toggle; the count line stays.
- The day page still opens **whole** under a lens — a day is never only one thing. Each entry row
  carries its subject handle (`FAM`, `TRV`) in the margin.

### 1.6 Loose notes — the margin, not an inbox

Details with no date yet ("ask the bond consultant about January rates") live in a right-hand margin
column, visible on the wall, with their own one-line composer. They are **clipped** to a day in one
click when they get a date. They are not a task inbox and must never accumulate a "process me"
affordance.

### 1.7 Navigation and input

| Input | Action |
| --- | --- |
| Click a day | Fold it open |
| `Esc` | Fold shut, restore focus to the cell; inside a note editor, read it back |
| `← / →` | Previous / next day (works on a focused cell *and* inside an open fold) |
| `↑ / ↓` | Previous / next week |
| `⌘/Ctrl + Enter` | Finish writing a note, return to reading |
| `draw: marks / words` | Redraw the quarter as marks, or as full sentences (globally persisted) |

Arrow-key movement must focus the target cell; focus must never be lost to `body`.

### 1.8 Mobile is a different shape, not a squeezed one

Below ~700px the wall is replaced by a **vertical week-by-week agenda**: week header, then day rows
with weekday, numeral, and their marks or words. Landmark days render as ink-filled rows. No
horizontal scrolling anywhere, ever. Current week always rendered, even if empty.

### 1.9 States that must persist

Open day, subject lens, draw mode, entries with notes, loose notes, day pages, per-entry and per-day
note editor mode. Persistence is silent — no "saved" toast, no autosave indicator.

---

## 2. DECIDED — the mark vocabulary

**The central visual idea: every thing prints as one mark, not a sentence.** Four things on a day is
four stamps; one thing is one large stamp. Texture becomes readable in a way a text list never is —
three health crosses in one week, a run of travel arrows before a wedding, two money marks at month
end.

### 2.1 The eighteen marks

All on a 24-unit grid, one stroke weight, square caps, no fills, all monochrome, rendered as **Lucide
SVG icons** and normalized by CSS to one stroke family. Never emoji, never mixed icon styles. Each
semantic mark maps to one fixed Lucide glyph.

| id | Name | Examples (keywords) |
| --- | --- | --- |
| `travel` | Travel | flight, visa |
| `health` | Health | dentist, bloods |
| `people` | People | mum, birthday |
| `money` | Money | bond, invoice |
| `work` | Work | offsite, client |
| `home` | Home | desk, lease |
| `ceremony` | Ceremony | wedding, lobola |
| `admin` | Admin | passport, form |
| `study` | Study | course, reading |
| `body` | Body | gym, run |
| `food` | Food | dinner, braai |
| `gift` | Gift | present, flowers |
| `place` | Place | hotel, venue |
| `call` | Call | call, zoom |
| `deadline` | Deadline | due, final |
| `rest` | Rest | leave, quiet |
| `note` | Note | idea, maybe |
| `thing` | Unsorted | everything else |

The semantic set remains the eighteen marks above. The original reference's hand-drawn 24-unit
paths were replaced with Lucide glyphs (`Plane`, `HeartPulse`, `Users`, `Banknote`, `Briefcase`,
`House`, `Church`, `FileText`, `BookOpen`, `Activity`, `Utensils`, `Gift`, `MapPin`, `Phone`, `Flag`,
`Moon`, `StickyNote`, `Square`) to make the icon source maintainable; the one-family, one-weight,
monochrome treatment is still part of the contract.

### 2.2 Rules that keep the vocabulary fixed

- **The user never has to pick a mark.** It is inferred from the words (longest keyword match wins).
- **Live feedback while typing** in the composer: show the mark and subject that will be used, so the
  vocabulary teaches itself in the act of writing.
- **A wrong guess costs one click**, never a filing decision: every entry has a mark picker showing
  all eighteen.
- **A mark always answers to its words**: hover/focus shows the exact sentence in a dark chip, in
  place. The key lives in the secondary tab, never on the wall.
- **Marks are never colour-coded.** Terracotta means *today* and nothing else.
- **The icon source is fixed.** Lucide supplies the glyphs, but the app exposes only the eighteen
  semantic marks; it does not become an arbitrary icon picker.
- **Loose notes use the same vocabulary with a different grammar** — a hairline under the glyph, so a
  detail never reads as an appointment.
- Mark size scales with how many are on the day: 30px single, 24, 18, 15.

### 2.3 Subject inference

Subjects are inferred exactly like marks, keyword-scored, longest match wins, default `Me`. Manual
override is a `/subject` token at the end of the composer line (`bond - january /finances`) or the
entry's subject handle in the day page. While a lens is open, new things default to that subject.

---

## 3. DECIDED — writing: markdown under everything

Revision six's answer to *"this has to let me write markdown notes under each task/event/idea."*
The notes are not a drawer — they are the reason the entry exists.

### 3.1 Two places to write

| Where | Owns | Reads as |
| --- | --- | --- |
| **Entry note** | one thing: what to bring, what was agreed, the checklist | a short printed block under the row |
| **Day page** | the day itself, even with nothing filed on it | a page at the foot of the unfolded day |

The day page exists so a day is not forced into a list: "write down what this year was, in four
sentences, before midnight" has no mark, no subject and no checkbox — and still belongs to 31 Dec.

### 3.2 Editor behaviour

- A monospaced textarea (**Write**) and a rendered page (**Read**). Reading is the **default state**
  for any note with content; an empty note opens as an empty textarea.
- `⌘/Ctrl+Enter` or `Esc` reads it back.
- A compact toolbar: bold, italic, heading, bulleted list, numbered list, task, quote, code, link,
  table, rule.
- **Tick tasks in the reading view** — a `- [ ]` line prints as a real checkbox whose click rewrites
  the markdown underneath. A note never becomes a second task list competing with the wall.
- **Writing never adds ink weight.** A day with only a page still draws as an open day.

### 3.3 Syntax — deliberately small

`#` / `##` headings, `**bold**`, `*italic*`, `` `code` ``, `-` lists, `1.` numbered, `- [ ]` tasks,
`>` quotes, `| tables |` with `---` separator rows, `---` rule, `[label](url)`.

Two gestures are Horizon's own:

- `@Name` prints a person in citation type.
- `^[YYYY-MM-DD^label]` prints a small chip pointing at another day in the quarter — a note that
  refers forward without becoming a dependency or a link that must be maintained.

### 3.4 Safety and completeness

- The renderer **builds DOM nodes one at a time**. No `innerHTML` anywhere in the note path. Pasted
  HTML prints as text.
- Links open with `rel="noopener noreferrer"`, target `_blank`, and only `http(s)`/`mailto` schemes.
- Honest limits to state in the UI, not to hide: no raw HTML, no embedded files, no pasted images.

### 3.5 What the wall gains

Two letters, in type rather than colour, so the ink system stays untouched: `MD` = the day has its
own page, `N` (or `N2`, `N3`) = that many entries carry a note. Both hidden while a lens is open.

---

## 4. DECIDED — the visual system

Editorial print register. Warm paper, high-contrast ink, one accent. **No** gradients, no card
shadows, no rounded corners, no emoji, no purple, no dashboard chrome.

Bind these tokens verbatim (from the reference file):

```css
:root{
  --bg:        oklch(98%   0.004 95);
  --surface:   oklch(100%  0.002 95);
  --fg:        oklch(20%   0.018 70);
  --muted:     oklch(44%   0.012 70);
  --border:    oklch(90%   0.006 95);
  --accent:    oklch(52%   0.10  28);   /* terracotta — TODAY ONLY */
  --ink-2:     oklch(34%   0.016 70);
  --ink-3:     oklch(46%   0.012 70);
  --rule-soft: oklch(94%   0.005 95);
  --wash-1:    oklch(94.5% 0.006 95);
  --wash-2:    oklch(91.5% 0.007 95);
  --hover:     oklch(95.5% 0.007 95);
  --accent-ink:    oklch(44% 0.11 28);
  --accent-on-ink: oklch(76% 0.09 28);
  --font-display: 'Iowan Old Style','Charter','Instrument Serif',Georgia,serif;
  --font-body:    'Newsreader',system-ui,sans-serif;
  --font-mono:    'IBM Plex Mono',ui-monospace,Menlo,monospace;
  --gut: 74px;   /* week gutter */
}
```

Rules:

- **Display and body type differ.** Serif display for headings and numerals; Newsreader for body and
  entry text; mono *only* for metadata, week labels, weekday band, handles and counts.
- **One accent, twice per screen at most**: today's postmark and the quiet/waiting marker.
- **Contrast**: normal text ≥ 4.5:1, large text and icons ≥ 3:1. Hover must never reduce contrast —
  move `--hover` (`+0.06–0.12` L), never the foreground. Solid controls invert both fg and bg.
- **Every focusable element** gets a visible `:focus-visible` ring (`2px solid var(--accent)`, offset).
- **Touch targets ≥ 44px** on touch surfaces (the concept's compact chrome may be denser on pointer
  devices; do not let it below 32px there).
- Section headings are uppercase mono at 9–10px with `0.13em` tracking; rules are 1px hairlines, not
  boxes.
- Long titles truncate with an ellipsis in the cell only; the day page and notes always show full
  text. No `white-space: nowrap` on user content.
- Avoid orphaned single words in headings and standfirsts; adjust the measure, not the overflow.

---

## 5. DECIDED — data model

The concept is stateless HTML + one `localStorage` key. The implementation replaces that with plain
Markdown files (§5, implementation note), keeping the invariants: a versioned representation, a migration path and an
export.

> **Implementation note (2026-11-05).** Superseded for storage. The live store is a folder of
> Markdown files, one per entry, note and day, plus `state.md`; there is no `localStorage` and no
> browser build. The JSON document shape below survives only as the whole-document import/export
> path. Both platforms write the same Markdown format, described in
> `openwiki/systems/document-model-and-persistence.md`.
>
> **Sync (2026-11-05).** Built as §7 asks: a background detail with no conflict dialog. The store
> folder is a git repository with one HTTPS remote; a quiet burst becomes a commit, and the merge
> keeps both versions of a file when two devices changed it. The rules live in
> `openwiki/systems/document-model-and-persistence.md#sync-and-versions`.

```ts
type Entry = {
  id: string;
  day: string;              // "YYYY-MM-DD"
  text: string;             // "Lobola meeting, Nelspruit"
  weight: 1 | 2 | 3;        // ordinary / full day / landmark
  done: boolean;
  mark: string;             // one of the eighteen, inferred then editable
  markAuto: boolean;        // false once the user has corrected it
  subject: string;          // one of the seventeen ids, inferred then editable
  subjectAuto: boolean;
  note?: string;            // markdown, optional
  createdAt: string;        // ISO
};

type LooseNote = {
  id: string;
  text: string;
  day?: string | null;      // null = in the margin; set = clipped
  mark: string;
  subject: string;
};

type DayPage = { [day: string]: string };   // markdown, '' when unwritten

type Doc = {
  version: 1;
  entries: Entry[];
  notes: LooseNote[];
  pages: DayPage;
  ui: { drawMode: "marks" | "words"; lens: string | null; openDay: string | null };
};
```

Derived, never stored: weight per day, counts per subject, "days carrying something", "days still
open", `MD` / `N` badges, week tier.

**Do not add** fields for: tags, folders, projects, priorities, colours, streaks, reminders, recurrences
(see §7). Adding one of these is a design regression, not a feature.

---

## 6. OPEN — the four questions left to the owner

Implement a default, isolate it behind one constant or flag, and report the choice in the handoff.

1. **Recurring things without a rule engine.** Default: a lightweight *"every week, on this weekday"*
   stamp on an entry that silently pre-fills the wall for future weeks (rendered in **outline** marks
   to distinguish texture from commitment), with no editor, no exception handling and no series
   management. Escalate to the owner before building any recurrence UI beyond a single toggle.
2. **Can a subject be *closed*?** A subject whose work is finished ("the house is bought") currently
   only goes quiet. Default: keep quiet-only; if added later, closure must be a single reversible
   action in the index, visualised as a struck-through name rather than a hidden one.
3. **Plan vs. record.** A page written ahead is a plan; the same page read afterwards is a record.
   Default: keep one object, but show a `written ahead` marker on pages whose first write predates
   their day, and a gentle past-tense footer once the day has passed. Do not build a second type.
4. **Does the wall look backward?** Default: forward only, 90 days. If a past window is added it must
   be a discrete "the last ninety" reading of the same spread, never an infinite scroll.

---

## 7. Do not build

These are the failure modes of the tools this app replaces. Each would reintroduce maintenance debt.

- **No grading that can be missed.** No streaks, no completion percentages, no weekly review, no daily
  prompt, no "you have not opened this in 5 days". The only growing number is nothing.
- **No overdue state.** A past undated thing is simply in the past.
- **No filing.** Never ask for tags, folders, projects, contexts or priorities. Inference plus a
  one-click correction is the entire taxonomy UX.
- **No generic productivity chrome.** No sidebar, no cards grid, no kanban, no charts — the user
  rejected charts outright: *a chart is an abstraction you decode before you can read it.* If a view
  needs a legend, it is the wrong view.
- **No rich-text editor.** Markdown only (§3.3).
- **No notification system in v1.** If ever added, it is one quiet daily line, not a stream.
- **No account requirement.** Local-first. Sync, if built, is a background detail that never produces
  a conflict dialog.

---

## 8. Technical requirement for the build

The concept is a ~2,500-line single-file HTML document with vanilla JS and DOM helpers — fine as a
reference, **not** as the architecture to extend.

- **Extract a typed, pure core**: inference (mark, subject), the weight/tier derivation, the subject
  counts, the quarter date maths (week rows, ISO weeks, the 90-day window), and the markdown parser.
  Pure functions, no DOM, unit-tested — these carry every product opinion.
- **Weekly drag** (the range rolls forward each day). The window must roll without losing the open
  day, and a day that leaves the window must not lose its data.
- **Persistence**: a folder of plain Markdown files as the live store on desktop and Android, the
  same format on both, **plus export/import as JSON** — writing must always be readable by
  anything, including a plain text editor in ten years.
- **Accessibility**: real buttons, `aria-pressed` on lens and mode toggles, a live region for status
  messages, focus preservation across fold open/close and arrow navigation, full keyboard operation,
  screen-reader labels that read the day's full content (`aria-label` on cells).
- **Test the interactions, not just the render**: fold open/close, arrow navigation, lens filtering,
  composer parsing, markdown rendering (including XSS attempts), task ticking, persistence round-trip.
- **Performance**: 90 cells × up to 9 marks re-render instantly; do not rebuild the whole wall on
  every keystroke in a note — patch the affected cell only.
- **Composer grammar** (keep it in one pure function):
  - `dec 3 - flight` · `3 dec - flight` · `3/12 - flight` → that day in the window
  - `today - …` · `tomorrow - …` · `day after tomorrow - …`
  - anything without a date → the open day, else today
  - trailing `/subject` token overrides the inferred subject; unknown subject is refused with a
    message, never guessed
  - dates outside the 90-day window are **refused**, never silently moved
  - the leading separator (any of `— – - · : , +`) is stripped from the title

---

## 9. Definition of done

- [ ] Wall renders 13 weeks × 7 days, Monday-first, weekday band, months only in the gutter.
- [ ] Four today markers present and legible in greyscale.
- [ ] Tier rendering correct for 0/1/2/3 weights; empty weeks compress; today's week never compresses.
- [ ] Fold opens in place, day page complete (aside, marks, entries, clipped notes, composer, page).
- [ ] `Esc`, arrows, focus restoration all work from keyboard alone.
- [ ] Index: 17 subjects in fixed order, counts, en dash when quiet, lens filters the wall and
      recomputes weight, lens is reversible, day page stays whole under a lens.
- [ ] Composer parses every grammar case, previews mark and subject live, refuses out-of-window dates.
- [ ] Entry notes and day pages: write/read, toolbar, task ticking, `@Name`, day-reference chip.
- [ ] Markdown renderer is XSS-safe (no `innerHTML`), handles tables, quotes, lists, code, links.
- [ ] Loose notes live in the margin and clip to a day in one click.
- [ ] `MD` / `N` badges correct and hidden under a lens.
- [ ] Words mode renders full sentences; mode survives reload.
- [ ] Mobile agenda below 700px, no horizontal scroll, 44px targets, today marked in all three ways.
- [ ] Persistence: reload restores everything; export/import JSON; export markdown folder.
- [ ] Contrast, focus rings and hover states verified against §4; no reduced-contrast hover.
- [ ] No feature from §7 has crept in.

---

## 10. The one-paragraph brief

Horizon is a personal life OS for someone whose last ten organisation tools died of maintenance debt.
It replaces the to-do app with a printed quarter: ninety days laid out Monday-first, where every thing
on a day prints as a single monochrome mark, so density and repetition — the shape of a life — are
legible before you read a word. Seventeen subjects (AI, Assets, Backpack, because everyone deserves to
be happy, Bosch, Sketches, Family, Finances, House, Journalling, Kids, Me, Payments, Prompts,
Retrogaming, Tech, Travels) form an index whose quiet entries are marked with an en dash, not a
warning. Clicking a day inflates it in place into a full day page — entries, notes, a composer — and
under each entry and each day there is a markdown page that prints back formatted the moment you stop
writing. Today is a postmark, not a highlight. Nothing is ever filed, tagged, reviewed, reminded or
counted as missed. It exists to be beautiful enough to open every day, and honest enough to show the
truth about where the next ninety days go.
