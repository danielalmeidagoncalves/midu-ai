use chrono::Datelike;
use dioxus::prelude::*;
use horizon_core::{
    dates::{self},
    model::note_key_page,
    Entry, LooseNote, Mark, NoteTarget, Subject,
};

use crate::components::labels;
use crate::components::marks::MarkGlyph;
use crate::components::notes::NoteEditor;
use crate::state::{parse_line, preview_line, AppCtx};

#[component]
pub fn Fold(day: String) -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let Some(date) = dates::parse_key(&day) else {
        return rsx! {};
    };
    let doc = ctx.doc.read();
    let index = quarter.index_of(&day).unwrap_or(0);
    let tier = doc.tier(&day, None);
    let standfirst = labels::standfirst(&doc, &day);
    let items = doc.items_on(&day, None, quarter.start);
    let entries: Vec<Entry> = doc.entries_on(&day, None).cloned().collect();
    let clipped: Vec<LooseNote> = doc.clipped_notes_on(&day, None).cloned().collect();
    let page = doc.page(&day).to_string();
    let page_ahead = doc.page_written_ahead(&day);
    let page_past = doc.page_is_past(&day, quarter.today);
    let first_written = doc
        .page_meta
        .get(&day)
        .and_then(|m| dates::parse_key(&m.first_written))
        .map(dates::short_day);
    let page_key = note_key_page(&day);
    let page_open = doc.ui.note_edit.contains_key(&page_key);
    let today_key = dates::key(quarter.today);
    let aria = labels::day_label(&doc, &day, None, quarter.start);
    drop(doc);

    let prev = if index > 0 {
        Some(quarter.days[index - 1].clone())
    } else {
        None
    };
    let next = quarter.days.get(index + 1).cloned();
    let marks_count = items.len();
    let day_for_close = day.clone();

    rsx! {
        div {
            id: "fold",
            class: if tier == horizon_core::Tier::Landmark { "fold land anim" } else { "fold anim" },
            tabindex: "-1",
            role: "region",
            aria_label: "Day page — {aria}",
            onkeydown: {
                let day = day.clone();
                move |evt| match evt.key() {
                    Key::Escape => {
                        evt.prevent_default();
                        evt.stop_propagation();
                        ctx.close_fold(Some(&day));
                    }
                    Key::ArrowRight => { evt.prevent_default(); ctx.move_fold(1); }
                    Key::ArrowLeft => { evt.prevent_default(); ctx.move_fold(-1); }
                    Key::ArrowDown => { evt.prevent_default(); ctx.move_fold(7); }
                    Key::ArrowUp => { evt.prevent_default(); ctx.move_fold(-7); }
                    _ => {}
                }
            },
            div { class: "fold-aside",
                span { class: "fold-ink", aria_hidden: "true" }
                p { class: "fold-eyebrow",
                    "Week {date.iso_week().week()} · {tier.word()}"
                    if day == today_key {
                        span { class: "nowmark", " · today" }
                    }
                }
                div { class: "fold-day", "{date.day()}" }
                p { class: "fold-month",
                    "{dates::weekday_full(date)} · {date.day()} {dates::month_full(date)} {date.year()}"
                }
                p { class: "fold-stand", "{standfirst}" }
                div { class: "fold-nav",
                    if let Some(prev) = prev {
                        button {
                            class: "chip",
                            r#type: "button",
                            onclick: move |_| ctx.open_day(&prev.key),
                            "← {dates::short_day(prev.date)}"
                        }
                    }
                    if let Some(next) = next {
                        button {
                            class: "chip",
                            r#type: "button",
                            onclick: move |_| ctx.open_day(&next.key),
                            "{dates::short_day(next.date)} →"
                        }
                    }
                }
            }
            div { class: "fold-main",
                div { class: "fold-head",
                    span { "On this day" }
                    span { class: "r",
                        if marks_count == 1 { "1 thing" } else { "{marks_count} things" }
                    }
                    button {
                        class: "x",
                        r#type: "button",
                        onclick: move |_| ctx.close_fold(Some(&day_for_close)),
                        "Close · Esc"
                    }
                }
                div { class: "fold-marks",
                    if items.is_empty() {
                        span { class: "empty", "Nothing yet — the day is a clean page." }
                    } else {
                        for item in items {
                            MarkGlyph {
                                mark: item.mark,
                                text: item.text,
                                subject: Some(item.subject),
                                note: item.loose,
                                projected: item.projected,
                            }
                        }
                        span { class: "cap",
                            if marks_count == 1 { "1 mark" } else { "{marks_count} marks" }
                        }
                    }
                }
                ul { class: "evs",
                    if entries.is_empty() {
                        li { class: "emptyline",
                            "No appointments filed. Anything you write on the page below still belongs to this day."
                        }
                    }
                    for entry in entries {
                        EntryRow { key: "{entry.id}", entry }
                    }
                }
                if !clipped.is_empty() {
                    div { class: "fold-clips",
                        span { class: "lbl", "Clipped details" }
                        for note in clipped {
                            ClippedChip { note }
                        }
                    }
                }
                FoldComposer { day: day.clone() }
                DayPage {
                    day: day.clone(),
                    page,
                    page_ahead,
                    page_past,
                    first_written,
                    page_open,
                }
            }
        }
    }
}

#[component]
fn ClippedChip(note: LooseNote) -> Element {
    let ctx = use_context::<AppCtx>();
    let id = note.id.clone();
    let text = note.text.clone();
    rsx! {
        span { class: "c",
            MarkGlyph { mark: note.mark, text: text.clone(), subject: Some(note.subject), note: true }
            span { "{text}" }
            button {
                class: "clip-detach",
                r#type: "button",
                aria_label: "Detach {text} back to the margins",
                onclick: move |_| ctx.detach_note(&id),
                "Detach"
            }
        }
    }
}

#[component]
fn FoldComposer(day: String) -> Element {
    let ctx = use_context::<AppCtx>();
    let mut draft = use_signal(String::new);
    let value = draft();
    let lens = ctx.doc.read().ui.lens;
    let preview = preview_line(&value, lens);
    let peek_class = if value.trim().is_empty() {
        "peek empty"
    } else {
        "peek"
    };
    let subject_handle = preview.subject.handle();

    rsx! {
        form {
            class: "foldadd",
            onsubmit: {
                let day = day.clone();
                move |evt| {
                    evt.prevent_default();
                    let raw = draft();
                    let parsed = {
                        let doc = ctx.doc.read();
                        let quarter = ctx.quarter.read();
                        parse_line(&raw, &doc, &quarter)
                    };
                    match parsed {
                        Ok(p) => {
                            ctx.add_entry(&p.day, &p.title, Some(p.subject), p.explicit_subject);
                            draft.set(String::new());
                            crate::storage::focus_id("fold-add");
                            let _ = &day;
                        }
                        Err(err) => ctx.flash(err.message()),
                    }
                }
            },
            span { class: "{peek_class}", aria_hidden: "true",
                MarkGlyph { mark: preview.mark, subject: Some(preview.subject) }
            }
            input {
                id: "fold-add",
                r#type: "text",
                autocomplete: "off",
                placeholder: "Add to this day — dentist, 09:20",
                aria_label: "Add something to this day",
                value: "{value}",
                oninput: move |evt| draft.set(evt.value()),
                onkeydown: move |evt| {
                    if matches!(evt.key(), Key::Escape | Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown) {
                        evt.stop_propagation();
                    }
                },
            }
            span { class: "subtok", title: "{preview.subject.name()}", "{subject_handle}" }
            button { r#type: "submit", "Add" }
        }
    }
}

#[component]
fn DayPage(
    day: String,
    page: String,
    page_ahead: bool,
    page_past: bool,
    first_written: Option<String>,
    page_open: bool,
) -> Element {
    let ctx = use_context::<AppCtx>();
    let show_editor = !page.trim().is_empty() || page_open;
    let page_key = note_key_page(&day);
    let week = dates::parse_key(&day)
        .map(|d| d.iso_week().week())
        .unwrap_or(0);
    let title = labels::day_page_title(&day, &dates::key(ctx.today()));

    rsx! {
        if !show_editor {
            button {
                class: "sheet-open",
                r#type: "button",
                aria_label: "Open the markdown page for {day}",
                onclick: {
                    let page_key = page_key.clone();
                    move |_| {
                        ctx.toggle_note_editing(&page_key, true);
                        let id = format!("md-{}", page_key.replace(':', "-"));
                        crate::storage::focus_textarea(&id);
                    }
                },
                "+ Write the day's page"
            }
        } else {
            div { class: "dournal",
                div { class: "head",
                    span { class: "t", "{title}" }
                    span { class: "k", "Markdown · week {week}" }
                }
                if page_ahead {
                    p { class: "page-meta",
                        "written ahead"
                        if let Some(first) = first_written.clone() {
                            " · first written {first}"
                        }
                    }
                }
                NoteEditor {
                    target: NoteTarget::Page(day.clone()),
                    who: "Page".to_string(),
                    label: format!("Markdown page for {day}"),
                    placeholder: "Write the day itself…\n\n# Nelspruit\n- who is driving back on Sunday\n- [ ] confirm the venue with Sipho".to_string(),
                    empty_hint: "No page for this day yet.".to_string(),
                    whole: true,
                }
                if page_past {
                    p { class: "page-past", "The day has passed. This page is a record now." }
                }
            }
        }
    }
}

#[component]
fn EntryRow(entry: Entry) -> Element {
    let ctx = use_context::<AppCtx>();
    let id = entry.id.clone();
    let expanded = ctx.ui.read().expanded.as_deref() == Some(id.as_str());
    let mark_open = ctx.ui.read().mark_picker.as_deref() == Some(id.as_str());
    let subject_open = ctx.ui.read().subject_picker.as_deref() == Some(id.as_str());
    let mut draft = use_signal(|| entry.text.clone());
    let done = entry.done;
    let mark = entry.mark;
    let subject = entry.subject;
    let weight = entry.weight;
    let repeat = entry.repeat_weekly;
    let entry_text = entry.text.clone();
    let check_label = if done {
        format!("Mark not done — {entry_text}")
    } else {
        format!("Mark done — {entry_text}")
    };

    rsx! {
        li {
            id: "entry-{id}",
            class: if done { "evrow done" } else { "evrow" },
            button {
                id: "entry-{id}-check",
                class: "chk",
                r#type: "button",
                role: "checkbox",
                aria_checked: if done { "true" } else { "false" },
                aria_label: "{check_label}",
                onclick: {
                    let id = id.clone();
                    move |_| ctx.set_entry_done(&id, !done)
                },
            }
            button {
                id: "entry-{id}-mark",
                class: "mkbtn",
                r#type: "button",
                aria_label: "Change mark — currently {mark.name()}",
                aria_expanded: if mark_open { "true" } else { "false" },
                onclick: {
                    let id = id.clone();
                    move |_| ctx.toggle_mark_picker(&id)
                },
                MarkGlyph { mark }
            }
            input {
                id: "entry-{id}-text",
                class: "evt",
                r#type: "text",
                value: "{draft}",
                aria_label: "Entry text",
                oninput: move |evt| draft.set(evt.value()),
                onblur: {
                    let id = id.clone();
                    let original = entry_text.clone();
                    move |_| {
                        let next = draft();
                        if next.trim().is_empty() {
                            draft.set(original.clone());
                        } else {
                            ctx.set_entry_text(&id, &next);
                        }
                    }
                },
                onkeydown: {
                    let id = id.clone();
                    let original = entry_text.clone();
                    move |evt| {
                        if matches!(evt.key(), Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown) {
                            evt.stop_propagation();
                        }
                        if evt.key() == Key::Enter {
                            evt.prevent_default();
                            let next = draft();
                            if next.trim().is_empty() {
                                draft.set(original.clone());
                            } else {
                                ctx.set_entry_text(&id, &next);
                            }
                            evt.stop_propagation();
                        }
                        if evt.key() == Key::Escape {
                            evt.stop_propagation();
                            draft.set(original.clone());
                            crate::storage::focus_id(&format!("entry-{id}-text"));
                        }
                    }
                },
            }
            button {
                id: "entry-{id}-subject",
                class: "evsub",
                r#type: "button",
                title: "{subject.name()}",
                aria_label: "Subject — {subject.name()}. Change it.",
                aria_expanded: if subject_open { "true" } else { "false" },
                onclick: {
                    let id = id.clone();
                    move |_| ctx.toggle_subject_picker(&id)
                },
                "{subject.handle()}"
            }
            button {
                id: "entry-{id}-expand",
                class: "exp",
                r#type: "button",
                aria_expanded: if expanded { "true" } else { "false" },
                aria_label: "Details for {entry_text}",
                onclick: {
                    let id = id.clone();
                    move |_| ctx.toggle_expanded(&id)
                },
            }
            if expanded {
                div { class: "evdet",
                    NoteEditor {
                        target: NoteTarget::Entry(id.clone()),
                        who: "Note".to_string(),
                        label: format!("Markdown note for {entry_text}"),
                        placeholder: "**Call back** about the *notice period*\n- ask for it in writing\n- [ ] save the reply here".to_string(),
                        empty_hint: "Nothing written under this yet.".to_string(),
                    }
                    div { class: "evacts",
                        button {
                            class: "chip",
                            r#type: "button",
                            onclick: {
                                let id = id.clone();
                                move |_| ctx.toggle_subject_picker(&id)
                            },
                            "{subject.name()}"
                        }
                        div { class: "weightgroup", role: "group", aria_label: "How heavy this day is",
                            for (w, label) in [(1u8, "One thing"), (2, "Full day"), (3, "Landmark")] {
                                button {
                                    id: "entry-{id}-w{w}",
                                    class: "weightbtn",
                                    r#type: "button",
                                    aria_pressed: if weight == w { "true" } else { "false" },
                                    onclick: {
                                        let id = id.clone();
                                        move |_| ctx.set_entry_weight(&id, w)
                                    },
                                    "{label}"
                                }
                            }
                        }
                        if horizon_core::model::WEEKLY_RECURRENCE {
                            button {
                                id: "entry-{id}-repeat",
                                class: "chip",
                                r#type: "button",
                                aria_pressed: if repeat { "true" } else { "false" },
                                title: "Every week, on this weekday. No exceptions.",
                                onclick: {
                                    let id = id.clone();
                                    move |_| ctx.set_entry_repeat(&id, !repeat)
                                },
                                if repeat { "Repeats weekly ✓" } else { "Repeats weekly" }
                            }
                        }
                        button {
                            class: "chip",
                            r#type: "button",
                            aria_expanded: if mark_open { "true" } else { "false" },
                            onclick: {
                                let id = id.clone();
                                move |_| ctx.toggle_mark_picker(&id)
                            },
                            "Change mark"
                        }
                        if !entry.mark_auto {
                            button {
                                class: "chip",
                                r#type: "button",
                                onclick: {
                                    let id = id.clone();
                                    move |_| ctx.reset_entry_mark(&id)
                                },
                                "Back to the automatic mark"
                            }
                        }
                        if !entry.subject_auto {
                            button {
                                class: "chip",
                                r#type: "button",
                                onclick: {
                                    let id = id.clone();
                                    move |_| ctx.reset_entry_subject(&id)
                                },
                                "Back to the automatic subject"
                            }
                        }
                        button {
                            class: "chip danger",
                            r#type: "button",
                            onclick: {
                                let id = id.clone();
                                move |_| ctx.remove_entry(&id)
                            },
                            "Remove"
                        }
                    }
                    if mark_open {
                        MarkPicker { id: id.clone(), current: mark }
                    }
                    if subject_open {
                        SubjectPicker { id: id.clone(), current: subject }
                    }
                }
            }
        }
    }
}

#[component]
fn MarkPicker(id: String, current: Mark) -> Element {
    let ctx = use_context::<AppCtx>();
    rsx! {
        div { class: "markpick",
            div { class: "lbl", "Eighteen marks — pick the one that fits" }
            for mark in Mark::ALL {
                button {
                    r#type: "button",
                    title: "{mark.name()}",
                    aria_label: "{mark.name()}",
                    aria_pressed: if mark == current { "true" } else { "false" },
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.set_entry_mark(&id, mark)
                    },
                    MarkGlyph { mark }
                }
            }
        }
    }
}

#[component]
fn SubjectPicker(id: String, current: Subject) -> Element {
    let ctx = use_context::<AppCtx>();
    rsx! {
        div { class: "subpick",
            div { class: "lbl", "Seventeen subjects — the territory this belongs to" }
            for (index, subject) in Subject::ALL.iter().enumerate() {
                button {
                    r#type: "button",
                    aria_pressed: if *subject == current { "true" } else { "false" },
                    onclick: {
                        let id = id.clone();
                        let subject = *subject;
                        move |_| ctx.set_entry_subject(&id, subject)
                    },
                    span { class: "o", {format!("{:02}", index + 1)} }
                    span { "{subject.name()}" }
                }
            }
        }
    }
}
