---
type: workflow
title: Subject lens, index and loose notes
description: How the seventeen-subject index narrows the wall, how counts/days/next are computed, why the day page stays whole, and how loose notes are added, armed, clipped and detached.
tags: [index, lens, subjects, loose-notes, margins]
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T14:46:47.383Z
sources:
  - id: openwiki-source-397a4b0768b3f5703fe7a0d4
    resource: repo://horizon-app/assets/main.css
  - id: openwiki-source-02c68ef233bef3e7684b42ce
    resource: repo://horizon-app/src/app.rs
  - id: openwiki-source-647ad384296f313c2f9e829b
    resource: repo://horizon-app/src/components/fold.rs
  - id: openwiki-source-44b44175429e88fd2994500f
    resource: repo://horizon-app/src/components/index.rs
  - id: openwiki-source-735e0cf4efc981d22067f979
    resource: repo://horizon-app/src/components/margin.rs
  - id: openwiki-source-bc5aa14c8f6fb3801d975a1a
    resource: repo://horizon-app/src/components/wall.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
generated: { by: "pi", at: "2026-11-04T14:46:47.383Z" }
---

# Subject lens, index and loose notes

The index and the margins are the two ways to read across days without leaving the wall. The index narrows the quarter to one subject. The margins hold details that do not have a day yet.

## The index

The index always lists all seventeen subjects in their fixed order. For each subject it computes `Doc::subject_count(subject, quarter)`: the number of entries in the current 90-day window whose subject matches. It does not count notes or pages.

A subject with a count of zero prints an en dash and is styled as quiet. Quiet subjects are hidden by default on narrow screens unless the user opens "Quiet subjects (n)"; on wider screens they remain visible but italic and muted. A quiet subject is still a real subject: clicking it applies the lens and the wall becomes empty for that subject. There is no close/archive action; quiet is the only state.

The index header shows either the global summary (`17 subjects · n in use · n quiet`) or, under a lens, the lens meta line:

```text
Family · 2 things · 2 days · next Mum's birthday · Sat 7 Nov
```

`subject_count` counts entries; `subject_days` counts distinct days; `subject_next` scans the quarter in date order and returns the first entry for that subject. All three are window-only: an entry outside the quarter is ignored.

## The lens

Clicking a subject calls `AppCtx::set_lens(Some(subject))`. That writes `Doc.ui.lens`, closes any open day, saves, and flashes a short message. Clicking the same subject again, or "Show the whole quarter", calls `set_lens(None)`.

The lens is a wall and measure filter, never a page filter:

- `Doc::entries_on(day, Some(subject))` and `clipped_notes_on` filter the wall items.
- `Doc::weight` and `tier` use only matching entries, so the wall's density is recomputed.
- The `N` and `MD` badges are hidden under a lens, because the wall is now a subject measure rather than a document map.
- `has_content` under a lens counts matching entries and clipped notes, not the day page.
- The fold always calls `items_on(day, None, ...)`, `entries_on(day, None)` and `tier(day, None)`. Once you open a day, you see the whole day, including entries from other subjects and the day page.

This is the "the full picture never leaves the screen" rule: the quarter stays visible, and reading a subject narrows the wall rather than opening a separate page.

The index itself remains the full seventeen-subject index while a lens is active. The header meta changes to the lens summary, but the rows still let the user switch subjects. The lens is persisted in `Doc.ui`, so it survives reload.

## Loose notes

A loose note is a `LooseNote` with `day = None`. It is a detail, not a thing: it has a mark and subject for recognition but does not add weight to any day.

The margin composer follows the last loose note; when the notes fill the column it sits at the bottom. It adds a note with `add_note`:

1. trim the text; ignore empty input;
2. infer the mark and subject, using the active lens as the subject default;
3. insert the note at the front of `Doc.notes`;
4. save and flash "Added to the margins — no day needed."

The margin list shows all `day = None` notes. Each has a mark glyph, its text, "Clip to a day" and "Remove". The header count is the number of loose notes. The empty state explains that a detail can stay undated until it has a day.

## Clipping

Clipping is deliberately two steps:

1. Press "Clip to a day" on a note. This sets `AppCtx.ui.arm_note = Some(note_id)`; the button changes to "Pick a day…" and the status says "Pick a day. Esc to cancel."
2. Click a wall cell or agenda row. `clip_note` sets the note's `day` to that key, clears `arm_note`, saves, and flashes "Clipped to <short day>."

If a lens is active, the clip also sets the note's subject to the lens. That keeps the clipped note visible when the day page is read under that lens. The clip is one click on the day after the arm, not a drag or a dialog.

Detach reverses the operation: `detach_note` sets `day = None`, saves, and flashes "Detached — back in the margins." Remove deletes the note entirely. A clipped note is rendered in the fold's "Clipped details" row with its own Detach button; it is not an entry and does not affect tier.

## Why the page stays whole

The wall and the fold take different lens arguments by design. The wall's `items_on` and `tier` receive the active subject; the fold passes `None`. That is why a House entry remains visible when the user opens a day under a Family lens, and why the day's own page is still shown. The lens is a reading measure across the quarter, not a filter on the day's record.

The model test pins this behavior: `items_on(day, Some(Me))` returns the matching subset, while `entries_on(day, None)` still returns the whole day. The fold relies on the latter.

## What is not here

There is no tag tree, no multi-subject filter, no saved lens, no subject close action, no margin-to-subject filing step and no notification when a note is clipped. The index and the margins are intentionally small: one subject at a time, one arm-then-click clip, and a detach path back.
