---
type: interface-guide
title: Surfaces, responsive layout and accessibility
description: How Horizon arranges its masthead, index, wall, fold, margins, agenda and brief; why only one shape renders at a time; and the keyboard, focus, touch, motion and contrast rules.
tags: [ui, dioxus, accessibility, responsive, design-system]
sources:
  - id: openwiki-source-397a4b0768b3f5703fe7a0d4
    resource: repo://horizon-app/assets/main.css
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-58bcf5dc38c4149c385b183f
    resource: repo://horizon-app/src/components/agenda.rs
  - id: openwiki-source-c1bc075dfb9585127c44a844
    resource: repo://horizon-app/src/components/brief.rs
  - id: openwiki-source-e604d03416b1397f1a78e6dc
    resource: repo://horizon-app/src/components/chrome.rs
  - id: openwiki-source-647ad384296f313c2f9e829b
    resource: repo://horizon-app/src/components/fold.rs
  - id: openwiki-source-c5dc7e77e438bece6f861302
    resource: repo://horizon-app/src/components/notes.rs
  - id: openwiki-source-bc5aa14c8f6fb3801d975a1a
    resource: repo://horizon-app/src/components/wall.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-1f6ffa9493e99001b1f2ab74
    resource: repo://tools/contrast.py
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Surfaces, responsive layout and accessibility

Horizon has one component tree with two shapes: a desktop wall and a mobile agenda. The product rule is that the quarter stays visible, so the interface is organized as surfaces around one day rather than as routes or modals.

## Surface map

| Surface | Responsibility |
| --- | --- |
| Menu | Native desktop only: a thin menubar strip above the masthead whose "Horizon" sheet holds the store path, "open the folder" and Quit. Opens by click, `Alt+H` or `F10`; `Ctrl+Q` (`Cmd+Q`) quits from anywhere. |
| Masthead | Wordmark, quarter range, Marks/Words switch, The wall/The brief tabs, and the capture composer with a live mark/subject preview. |
| Subhead | The "Ninety days, in marks" line and the today/now line with count and next thing. |
| Index | Seventeen subjects, counts or en dashes, lens metadata, quiet-subject toggle and lens reset. |
| Wall | Desktop: weekday band, month/ISO-week gutter, seven columns, marks or words, today markers, empty-week compression, and the in-place fold. |
| Fold | The day page: aside with numeral and standfirst, large marks, entries, clipped details, a bound composer, and the day's Markdown page. |
| Margins | Loose notes with clip/detach controls and a composer that follows the last note; it pins to the column bottom when the notes fill it. On mobile it sits below the agenda. |
| Agenda | Mobile: week-by-week rows, current week in full, empty weeks skipped, today stamp/frame/word, and the fold embedded after the open day. |
| Brief | The key, principles, live store location, four open defaults, exports and JSON restore. |
| Footer | Days carrying something, days still open, loose-note count and the quiet status line. |

The menu sheet is the one overlay, and it is native-only: the GTK/muda system menubar draws no entries under this toolkit, so Horizon draws its own. It is a bordered sheet with no shadow, closes on `Esc` back to its trigger, and is never shown on the phone build.

There are no other modals, side panels, routes or overlays. The fold opens inside the week row on desktop and inside the agenda on mobile. The brief is the only secondary tab.

## One shape at a time

`App` reads a `narrow` signal fed by `window.innerWidth <= 700`. When it is true, the app renders `agenda` and the margins; otherwise it renders the wall and the margins. It never renders both. The CSS breakpoint at 700px hides `.wall` and shows `.agenda` as a second guard.

That rule matters beyond layout: rendering both would duplicate the `fold` element and every entry id, break focus restoration, and make `document.getElementById` ambiguous. The 700px threshold is intentionally shared by the signal and the stylesheet.

## Wall

The wall is a CSS grid with a fixed gutter and seven equal weekday columns. Each week has a gutter row with month, ISO week, a `now` marker and a live-day count, followed by seven cells. Empty weeks (no content and no today) get a compressed row; the current week is never compressed.

A cell prints up to nine marks and a `+n` overflow. In Words mode it prints up to three entry sentences and a `+n` count. The page and note badges (`MD`, `N`) are hidden under a lens, because the wall is then a subject measure rather than a document map. The four today markers are the date stamp, the 1px inset frame, the `today` word, and the `now` gutter with the terracotta month.

The cell is a real button. Arrow keys move focus by one day left/right and one week up/down; Enter opens the fold or clips an armed note. The cell carries an accessible label assembled from the day's entries and clipped notes plus "Today" when relevant.

## Fold

The fold is a region inserted after the open week row. Its aside carries the large numeral, weekday/date, ISO week and tier word, standfirst, and previous/next day chips. The main column carries the large mark row, entry rows, clipped-note chips, a composer bound to that day, and the day page.

Esc closes the fold; left/right/up/down move the fold by a day or a week. The close chip also returns focus to the same day. Entry rows expose a checkbox with `role="checkbox"`, a mark button, an editable title, a subject handle, and an expander. The expanded panel holds the Markdown note, subject chip, three-way weight control, the one weekly-repeat toggle when enabled, mark/subject reset actions, and Remove.

When the fold is empty, the day page is a `+ Write the day's page` button. Opening it focuses the textarea; writing it switches the page to the read view by default. A page whose first write predates its day prints `written ahead` with the first-write date. A page on a past day prints the record footer; under the forward-only window that footer is effectively unreachable, but the code path exists.

## Agenda

Below 700px the wall becomes a vertical list of weeks. A week is printed when it has content, holds today, or contains the open day. The current week is always printed in full, even when empty; other weeks print only days that carry something. The default mobile row shows the date, up to eight marks, the first sentence (or "a written page" / "open"), and a `+n` count. The today row adds the stamp, the inset frame and the `today` word.

The fold appears inside the agenda after the open day, with the same content and keyboard behavior but a single-column aside and a smaller numeral. Margins follow the agenda, so a loose note can be clipped to an agenda day with the same arm-then-click flow.

## Brief

The brief is the only secondary tab. It prints the mark key, the seventeen subjects, the principles, the four open defaults, and a **Where it lives** section. On desktop and Android that section names the Markdown store folder and says the files are the source of truth. The same tab carries JSON download/restore, the Markdown store copy, and the **Sync** block. It has no settings screen or charts.

The Sync block is the whole git surface. Before a remote is set it shows a remote field, a token field and Connect, then says where the files are on this device. After that it shows one state line, the remote, **Sync now**, **Refresh history**, a kept-both count when conflicts exist, and the last twelve versions. A version expands to the files it touched; a file opens that version's Markdown read-only in a `pre`. There is no branch picker, staging area, diff view or merge editor, and no conflict dialog.

## "Is my writing stored?"

One line in the masthead answers it, on every screen and every platform, and the Sync block prints the same words: a plain mono label with a one-sentence `title` explaining it. `sync::status` decides the label, and eight tests pin the order: a failure first (`Sync failed`), then `Syncing…`, then `Local only` when no remote is set (the files are still Markdown on disk), then `3 uncommitted`, then `1 to take`, then `2 to send`, then `In sync`, with a kept-conflict count riding along (`In sync · 1 kept`). Reading the remote outranks sending to it, because that is the step that needs a person. The state is one short label, never a panel, a badge with a number in a corner, or a dialog.

The first Tab press from the wall lands on a visually hidden "The brief — sync, history, portability" button inside the masthead; Enter opens the brief. Without it a keyboard could not reach a block below the wall, because the masthead's controls all precede the brief in DOM order.

## Visual system

The stylesheet binds a fixed OKLch token set: warm paper `--bg`, surface, ink `--fg`, muted/ink-2/ink-3, soft rules, washes, paper-dim, hover, and two terracotta tokens. The display face is a serif stack, body text is Newsreader, and metadata is IBM Plex Mono. There are no gradients, cards, shadows, rounded corners or emoji.

Terracotta is reserved for today and interaction: the today stamp/frame/word, the `now` gutter, the terracotta month, focus rings, armed clip cells, and the landmark accent. It is not a decorative category color. The marks themselves are monochrome strokes; density is carried by paper, wash, and solid ink.

## Accessibility and input

- **Semantics:** controls are buttons, not clickable divs; tabs use `role="tab"` and `aria-selected`; checkboxes use `role="checkbox"` and `aria-checked`; pickers and the fold use `aria-expanded`; weight and repeat controls use `aria-pressed`; the current day uses `aria-current="date"`.
- **Focus:** `:focus-visible` outlines are defined for every interactive surface. Opening or moving a fold focuses the fold after the DOM update; closing returns focus to the same wall cell or agenda row. Arrow keys move cells and the fold; Esc closes the fold or cancels an armed clip.
- **Touch:** at 700px and below, and under `pointer: coarse`, controls such as the index, composer, tabs, notes, pickers, chips, fold close, and checkboxes are raised to 44px or more. The compact desktop controls can be smaller; the touch rule is the one that matters for mobile.
- **Motion:** `prefers-reduced-motion: reduce` disables the fold animation and transitions.
- **Contrast:** every normal-text token pair is checked at 4.5:1, including hover and tooltip pairs. The lowest reported pair is terracotta-on-paper at 5.46:1. Hover backgrounds do not reduce contrast.
- **Menu:** the strip is a `role="menubar"` with one trigger; the sheet is a `role="menu"` that takes focus when it opens, moves the highlight with arrows/Home/End, runs the highlighted entry with Enter, and closes with Esc. Quit saves the store before the process exits.
- **Status:** the footer and brief expose a polite `role="status"` line for short confirmations. The app never uses a toast or a modal.
- **Sync history:** each version row is a real button with `aria-expanded`; the files inside are buttons; the version body is selectable text. Errors use `role="alert"`.

Mark glyphs are rendered as SVG paths/circles/rects and marked `aria-hidden`; the surrounding button or row carries the accessible name. The tooltip is supplementary mouse feedback, not the only way to read a label.

## What is deliberately not here

No charts, settings screen, notification stream, rich-text editor, image upload, raw HTML, tag tree or card layout. The brief states the live store, the omissions and the four open defaults, so the interface does not imply features that Horizon refuses to build.
