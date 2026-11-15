---
type: "Reference"
title: "The composer grammar and capture path"
openwiki_generated: true
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T14:46:47.383Z
sources:
  - id: openwiki-source-e604d03416b1397f1a78e6dc
    resource: repo://horizon-app/src/components/chrome.rs
  - id: openwiki-source-647ad384296f313c2f9e829b
    resource: repo://horizon-app/src/components/fold.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-b108f17faab5d01148959127
    resource: repo://horizon-core/src/composer.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
generated: { by: "pi", at: "2026-11-04T14:46:47.383Z" }
---


# The composer grammar and capture path

The composer is the only capture surface. One line carries a date, a title, and an optional subject; everything else is inferred. The grammar is a single pure function in `horizon-core::composer`, used by both the masthead composer and the composer inside a day fold.

## Accepted forms

```text
dec 3 - flight
3 dec - flight
3/12 - flight
today - call mum
tomorrow - call mum
day after tomorrow - call mum
flight                     # no date: open day, else today
bond - january /finances   # trailing subject override
```

The three absolute forms are named-month-first, day-first and numeric. Month names accept a unique prefix of at least three letters (`dec`, `december`), with an optional period. A separator is whitespace or one of `—`, `–`, `-`, `·`, `:`, `,`, `+`; leading separators are stripped from the title. The title may itself contain separators; only the leading run is removed.

Relative dates are matched first, longest phrase first: `day after tomorrow`, `tomorrow`, `today`. If no date is present, the composer uses the open day when it is inside the quarter; otherwise it uses today. An open day outside the window is ignored rather than moved.

Absolute dates resolve against the current year, then next year, then previous year, and are accepted only if the resulting date is inside the 90-day quarter. That is why `jan 5` typed in November resolves to the next January, while `feb 10` is refused when it falls beyond the window. Invalid calendar dates such as `31/2` are refused. The date is never silently snapped to the nearest valid day.

## Subject and mark

The last whitespace-delimited token starting with `/` is the subject override. Its value must be a subject id, handle, or a unique prefix of either or of the printed name; the lookup is case-insensitive. An unknown token is an error, not a guess. `Subject::from_query` refuses ambiguity: `a` could be AI or Assets, so it returns nothing.

If there is no explicit token, the active subject lens is the default. If there is no lens either, the title is passed to the mark and subject keyword inference. The result carries `explicit_subject = true` when the subject came from `/subject` or the lens. When that entry is added, `subject_auto` is set to `!explicit`, so a later title edit will not overwrite a deliberate subject. The mark remains automatic: `mark_auto` stays true, and edits to the title re-infer the mark unless the user has changed it.

The mark is always inferred from the title text, even when the subject is explicit. A subject override does not change the mark.

## Refusals

The composer returns a `ComposerError` rather than creating a partial or relocated entry:

| Error | When |
| --- | --- |
| `Empty` | The line is blank after removing the subject token. |
| `UnknownSubject` | The trailing `/token` does not resolve uniquely. |
| `OutsideWindow` | A date was recognized but does not fall inside the quarter. |
| `MissingTitle` | A date was consumed but no title remains. |

The UI shows `ComposerError::message()` in the status line. It does not move the date, create an undated entry, or open a picker.

## Preview

`composer::preview` is the live feedback path. It returns the mark and the subject that would be used, plus `subject_ok`. It never fails: an unknown subject token sets `subject_ok = false` and falls back to the lens or inference for the preview, while the submit path still refuses the token. The masthead and fold composers both render a small mark glyph and the subject handle beside the input, so the user sees the inference before pressing Enter.

## The two composers

The masthead composer files into the open day, else today, and uses the active lens. The fold composer is bound to its day; it uses the same grammar and the same `AppCtx::add_entry` path. A successful submit:

1. calls `parse_line(raw, doc, quarter)`, which delegates to the pure parser with the open day and lens;
2. calls `ctx.add_entry(&p.day, &p.title, Some(p.subject), p.explicit_subject)`;
3. clears the draft;
4. returns focus to the input (`capture` or `fold-add`).

`add_entry` constructs `Entry::new_with`, sets `subject_auto` from the explicitness flag, appends it to the document, saves, and flashes a short confirmation with the day and subject. Weight starts at one; the entry is not done; weekly recurrence is off until the user turns it on.

The fold composer sends an entry to the day it is attached to, even if a date token in the line says otherwise. The masthead composer respects the date token and the open day. Both show the same preview and both refuse the same errors.

## Examples and edge cases

- `dec 3 - flight` and `3 dec - flight` and `3/12 - flight` all produce the same day.
- `dec 3 ---- flight` strips the separator run and keeps `flight` as the title.
- `dec 3 . flight` keeps the period because `.` is not a separator.
- `bond - january /finances` files under Finances and keeps `bond - january` as the title.
- `bond /nonsense` is refused; it is not filed under Me.
- `flight` with an open day files to that day; with no open day it files to today.
- `flight` with an open day outside the quarter ignores the open day and files to today.
- `feb 10 - flight` in a quarter ending in January is refused rather than moved.

The tests pin these cases; see [Tests, invariants and verification](../quality/testing-and-invariants.md).
