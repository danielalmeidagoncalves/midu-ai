---
type: time-and-layout
title: The quarter, the wall and density tiers
description: The 90-day Monday-first window, week padding and labels, the four density tiers derived from entry weight, and how the wall and mobile agenda render the same quarter.
tags: [dates, quarter, wall, tiers, grid]
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T14:46:47.383Z
sources:
  - id: openwiki-source-d1eead8d200b68e1165d8f17
    resource: repo://HANDOFF.md
  - id: openwiki-source-58bcf5dc38c4149c385b183f
    resource: repo://horizon-app/src/components/agenda.rs
  - id: openwiki-source-bc5aa14c8f6fb3801d975a1a
    resource: repo://horizon-app/src/components/wall.rs
  - id: openwiki-source-76d2e29a57742ad34fd1b521
    resource: repo://horizon-core/src/dates.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
generated: { by: "pi", at: "2026-11-04T14:46:47.383Z" }
---

# The quarter, the wall and density tiers

Horizon's time unit is a 90-day quarter starting today. The quarter is a pure data structure; the wall and the mobile agenda are two renderings of it.

## The quarter

`Quarter::new(today)` sets:

- `start = today`;
- `end = today + 89 days`;
- `days: Vec<Day>` with 90 entries, each carrying the date, `YYYY-MM-DD` key and index;
- `weeks: Vec<Week>` built Monday-first.

Each `Week` has an ISO week number, the month printed in the gutter, seven slots (`Option<index>`), and the first real day. Slots before today in the first week and after day 90 in the last week are `None`; the wall renders them as empty cells. The month and ISO week come from the first real day in the row, not from the padded Monday.

`Quarter` also provides `contains`, `contains_key`, `index_of`, `day`, `week_of`, `range_label` and the short/long date formatters. The composer uses `contains`/`contains_key` to refuse out-of-window dates; the index uses `subject_count`/`subject_days` against the same window.

## The 13-row caveat

The spec asks for both "90 days starting today" and "13 week rows". Those two requirements agree only when today is Monday or Tuesday. Horizon keeps every day and keeps weekday columns true, so the grid is **13 rows on Monday or Tuesday and 14 rows on Wednesday through Sunday**. The first and last rows are padded. No day is dropped. This is documented in `ARCHITECTURE.md` and pinned by the dates tests.

If a future change forces 13 rows on every start date, it must choose what to drop or how to shift the window; that is a product decision, not a layout tweak.

## Density tiers

Tier is a property of a day under a lens, derived from the maximum **real** entry weight:

| Weight | Tier | Class | Meaning |
| --- | --- | --- | --- |
| 0 | Open | none | paper, empty day |
| 1 | One | `l1` | one thing |
| 2 | Full | `l2` | full day |
| 3+ | Landmark | `land` | changes something |

`Entry::weight()` clamps to 1–3, and `Doc::weight` takes the maximum across entries on that day that match the lens. A day with only clipped notes, a page, or projected recurrences has weight 0 and stays Open. A blank day is legal; an open day is not unfinished work.

The wall uses the class for the wash or solid-ink treatment. The fold uses the same tier for its aside and its `land` styling. The mobile agenda uses `l1`, `l2` and `land` on the whole row. The tier is never stored; it is always recomputed from the current entries and lens.

## The wall

The wall is a two-column grid: a fixed gutter and seven equal weekday columns. The header band prints Mon–Sun. Each week row has a gutter with:

- the month only when it changes from the previous week;
- the ISO week number (`W45`);
- `now` when the week holds today;
- a count of days carrying something.

A week is **quiet** when it has no content and does not hold today. Quiet weeks compress to a shorter row; the current week never compresses. That is how the quarter keeps its shape without printing ninety identical empty rows.

A cell prints up to nine marks and a `+n` overflow. In Words mode it prints up to three entry sentences and a `+n` count. The page and note badges (`MD`, `N`) sit beside the date and are hidden under a lens. The four today markers are the stamp, the 1px inset frame, the `today` word and the `now` gutter with the terracotta month. The today cell is the only tab stop by default; arrow keys move focus by day or week.

## Filling the window

The wall takes the height the layout gives it: `.body` and `.wall-wrap` are flex columns, `.wall` grows, and `align-content:stretch` with `grid-auto-rows:minmax(46px,auto)` shares the spare height between the week rows. A taller window therefore prints taller days instead of leaving blank paper under the last row; a row whose fold is open is taller than its share and pushes the rest down rather than being clipped.

Each cell also carries a **corner numeral**: the day number again, bottom-right, at `clamp(13px, 5cqi, 44px)` against the row as a container-query context, in ink at 16–34% opacity (terracotta when the day is today, accent-on-ink on a landmark). It is `aria-hidden` and `pointer-events:none`: the date beside it already speaks to a screen reader, so this is chrome that spends only height the content does not need. It gives a sparse wall reading weight without inflating a quiet day's type or adding a second marking system.

## The mobile agenda

Below 700px the same quarter becomes a vertical list of weeks. A week is printed when it has content, holds today, or contains the open day. The current week is rendered in full, including empty days; other weeks print only days that carry something. Each day row shows the weekday/date, up to eight marks, the first sentence, and a `+n` count. Today adds the stamp, the inset frame and the `today` word. The open day's fold appears inside the agenda after its row.

The mobile shape is not a squeezed wall: it drops the seven-column grid, expands the touch targets, and keeps the same data and actions. The current week is always available even when it is empty, so the app never presents a blank screen on a quiet week.

## What counts as content

`Doc::items_on` assembles three kinds of item:

1. real entries on the day (carry weight);
2. clipped loose notes on the day (weight 0);
3. projected weekly recurrences (weight 0).

Only real entries contribute to `weight` and therefore to tier. A recurrence is outline texture: it is drawn dashed and thinner, and a day whose only item is a projection remains Open. This is the implemented recurrence default; there is no series editor, exception list or end date.

A day's page alone does not create a mark, but it does make the day carry content for the stats and for the mobile "a written page" line. `has_content` includes entries, clipped notes and (without a lens) the page. Under a lens, the page is deliberately excluded from the measure.

## Data versus rendering

The data layer owns `Quarter`, `Day`, `Week`, `Tier`, `weight` and the projections. The wall owns the weekday band, gutters, cells, mark/words choice, badges and empty-week compression. The agenda owns the vertical week grouping, current-week rule and row layout. Neither surface stores a copy of the quarter; both read the same signal and derive their display from it.
