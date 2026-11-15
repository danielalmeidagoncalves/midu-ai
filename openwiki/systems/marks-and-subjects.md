---
type: domain-vocabulary
title: Marks, subjects and keyword inference
description: The eighteen marks and their fixed Lucide SVG glyphs, the seventeen subjects, the keyword scoring that infers both, and the override/auto/reset rules around a wrong guess.
tags: [inference, vocabulary, marks, subjects, keywords]
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T14:46:47.383Z
sources:
  - id: openwiki-source-fb12dd0e8559a6ff524f0457
    resource: repo://horizon-app/src/components/marks.rs
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-84a00d6eab52fe2e00796d90
    resource: repo://horizon-core/src/inference.rs
  - id: openwiki-source-1cfc5127771ad414abb946dc
    resource: repo://horizon-core/src/model.rs
generated: { by: "pi", at: "2026-11-04T14:46:47.383Z" }
---

# Marks, subjects and keyword inference

Horizon has exactly eighteen marks and seventeen subjects. They are product data, not an open taxonomy: adding one changes the vocabulary, the composer, the index, the key and the export. The order of the enums is part of the contract because ties in inference go to the earlier row.

## Marks

`Mark::ALL` is ordered:

| Mark | Id | Example |
| --- | --- | --- |
| Travel | `travel` | flight, visa |
| Health | `health` | dentist, bloods |
| People | `people` | mum, birthday |
| Money | `money` | bond, invoice |
| Work | `work` | offsite, client |
| Home | `home` | desk, lease |
| Ceremony | `ceremony` | wedding, lobola |
| Admin | `admin` | passport, form |
| Study | `study` | course, reading |
| Body | `body` | gym, run |
| Food | `food` | dinner, braai |
| Gift | `gift` | present, flowers |
| Place | `place` | hotel, venue |
| Call | `call` | call, zoom |
| Deadline | `deadline` | due, final |
| Rest | `rest` | leave, quiet |
| Note | `note` | idea, maybe |
| Thing | `thing` | everything else |

`Thing` is the default and the printed name is "Unsorted". Marks serialize by id; an unknown id deserializes to `Thing` rather than failing the import.

Each mark maps to one fixed [Lucide](https://lucide.dev) glyph through `horizon-app/src/components/marks.rs` (`Plane`, `HeartPulse`, `Users`, `Banknote`, `Briefcase`, `House`, `Church`, `FileText`, `BookOpen`, `Activity`, `Utensils`, `Gift`, `MapPin`, `Phone`, `Flag`, `Moon`, `StickyNote`, `Square`). The shared `.mk svg` CSS normalizes them to one 1.7-unit stroke, square caps, miter joins and no fills. Lucide is the single icon family; the app still exposes only the eighteen semantic marks and never an arbitrary icon picker. The change from the reference's hand-drawn paths is recorded in `HANDOFF.md`.

## Subjects

`Subject::ALL` is ordered and fixed:

| Subject | Id | Handle |
| --- | --- | --- |
| AI | `ai` | AI |
| Assets | `assets` | AST |
| Backpack | `backpack` | BKP |
| Everyone | `everyone` | EVRY |
| Bosch | `bosch` | BOS |
| Sketches | `sketches` | SKT |
| Family | `family` | FAM |
| Finances | `finances` | FIN |
| House | `house` | HSE |
| Journalling | `journalling` | JRN |
| Kids | `kids` | KID |
| Me | `me` | ME |
| Payments | `payments` | PAY |
| Prompts | `prompts` | PRM |
| Retro | `retro` | RTG |
| Tech | `tech` | TEC |
| Travels | `travels` | TRV |

`Me` is the default. `Subject::name` is the printed name; for `Everyone` it is the sentence "because everyone deserves to be happy", which is why the UI often uses the shorter handle. `Subject::from_query` resolves a composer `/token` by exact id, exact handle, then a unique prefix of id, handle or name. Ambiguity is refused: `a` could be AI or Assets, so it returns `None`; the composer reports an unknown subject instead of guessing.

## Scoring

Inference is lexical, not semantic. `normalize` lowercases the text and turns every character that is not ASCII alphanumeric or `-` into a space, with boundary spaces. `contains_word` then looks for the keyword with word boundaries, allowing one trailing `s` for plurals. Hyphens count as boundaries. `score` sums the lengths of every keyword that appears.

`infer_mark` and `infer_subject` run the same loop:

```text
best = fallback
best_score = 0
for (category, keywords) in table order:
    s = sum(len(keyword) for keyword in keywords if contains_word(text, keyword))
    if s > best_score:
        best = category
        best_score = s
return best
```

Only a strictly greater score replaces the current best, so ties go to the earlier row. Longer keywords contribute more, which lets "lobola meeting" beat a shorter overlapping word. The mark and subject tables are independent; a text can infer one mark and a different subject.

The fallbacks are `Mark::Thing` and `Subject::Me`. There is no "unknown" state in the UI and no confidence threshold. A wrong guess costs one click, which is the product's trade-off.

## What the inference actually does

The tests use the spec examples as fixtures. Some results are intentionally lexical rather than intuitive:

- `Dentist, 09:20` → Health / Me.
- `Visa interview` → Travel / Travels.
- `Mum's birthday` → People / Family.
- `Housing bond first payment` → Money / Payments.
- `Sketches — ink the Nelspruit series` → Place / Sketches (Nelspruit is a place keyword; the subject table finds Sketches).
- `Retrogaming night with Thabo` → Thing / Retro (the subject table matches the full `retrogaming` keyword; the mark table has no matching mark).
- `Thabo's school fees` → Study / Kids (school is a Study mark and a Kids subject; fees is a Payments subject but the total keyword score favors Kids).

This is why the UI always shows the inferred mark and subject before submission and offers one-click correction. The inference is a first guess, not a classification system.

## Auto flags and correction

`Entry` carries two independent flags:

- `mark_auto`: when true, the mark is inferred from the title; when false, the user chose it.
- `subject_auto`: when true, the subject is inferred from the title; when false, the user chose it, or the subject came from `/subject` or the active lens.

`Entry::new_with` sets the mark from `infer_mark`, the subject from the override or `infer_subject`, `mark_auto = true`, and `subject_auto = override.is_none()`. `LooseNote::new` does the same for a loose note, without the auto flags because it has no editor.

Editing an entry title re-infers only the automatic side: `set_entry_text` updates the mark when `mark_auto`, and the subject when `subject_auto`. Choosing a mark or subject in the picker sets the corresponding auto flag false. "Back to the automatic mark/subject" sets it true and recomputes immediately. Import normalization re-infers any automatic value, so an old export with stale inferred values is corrected on restore.

The composer's explicit subject is `subject_auto = false` by construction. A lens default also marks the subject explicit. This prevents a later title edit from silently moving a deliberately filed entry.

## Where the vocabulary is used

- The composer preview and the entry row show mark glyphs and subject handles.
- The wall/agenda marks use the same Lucide glyphs at several sizes.
- The index prints subject names and counts, with an en dash when a subject has no entries in the window.
- The brief prints the full mark key and subject key.
- JSON stores ids; Markdown export writes the printed names.
- Unknown ids on import fall back to Thing/Me rather than dropping the entry.

The source of truth is `horizon-core::inference`; the UI never defines its own vocabulary.
