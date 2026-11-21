use dioxus::prelude::*;
use horizon_core::{dates, LooseNote, NoteTarget, Subject};

use crate::components::marks::MarkGlyph;
use crate::components::notes::NoteEditor;
use crate::state::{AppCtx, MarginMode, ReminderTarget};

/// Loose notes: the details with no day yet, grouped by the subject they
/// belong to. A chip narrows the column to one subject; each note opens in
/// place into the same markdown editor entries use, and still clips to a day.
#[component]
pub fn Margins() -> Element {
    let ctx = use_context::<AppCtx>();
    let doc = ctx.doc.read();
    let filter = ctx.ui.read().margin_filter;
    let notes: Vec<LooseNote> = doc.loose_notes().cloned().collect();
    drop(doc);

    // The chip row: every subject that carries an undated note, index order.
    // A filter whose subject emptied out (its last note clipped) lets go.
    let chips: Vec<(Subject, usize)> = Subject::ALL
        .iter()
        .map(|s| (*s, notes.iter().filter(|n| n.subject == *s).count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    let filter = filter.filter(|s| chips.iter().any(|(subject, _)| *subject == *s));

    // Groups in index order; inside a group the store order is newest first.
    let mut groups: Vec<(Subject, Vec<LooseNote>)> = Vec::new();
    for note in &notes {
        if filter.is_some_and(|s| note.subject != s) {
            continue;
        }
        match groups.iter_mut().find(|(s, _)| *s == note.subject) {
            Some((_, list)) => list.push(note.clone()),
            None => groups.push((note.subject, vec![note.clone()])),
        }
    }
    groups.sort_by_key(|(s, _)| {
        Subject::ALL
            .iter()
            .position(|x| *x == *s)
            .unwrap_or(usize::MAX)
    });

    let count = notes.len();
    let armed = ctx.ui.read().arm_note.clone();
    let mode = ctx.ui.read().margin_mode;
    let empty_text = if filter.is_some() {
        "Nothing waiting under this subject."
    } else {
        "Nothing waiting. A detail can stay undated until it has a day."
    };

    rsx! {
        aside { class: "margins", aria_labelledby: "marginsHead",
            MarginHead { count, mode }
            if mode == MarginMode::Agent {
                crate::components::agent::AgentColumn {}
            } else {
                NotesColumn { count, chips, groups, filter, armed, empty_text }
            }
        }
    }
}

/// The column's head is the switch. One column, two readings: loose notes and
/// the agent, with the count or the status as the quiet instrument the rest of
/// the app uses.
#[component]
fn MarginHead(count: usize, mode: MarginMode) -> Element {
    let ctx = use_context::<AppCtx>();
    rsx! {
        div { class: "margins-head",
            h2 { id: "marginsHead",
                if mode == MarginMode::Agent { "Agent" } else { "Loose notes" }
            }
            if mode == MarginMode::Notes {
                span { class: "n", "{count}" }
            }
            div { class: "margin-modes", role: "tablist", aria_label: "The right-hand column",
                button {
                    id: "margin-mode-notes",
                    class: "margin-mode",
                    r#type: "button",
                    role: "tab",
                    aria_selected: if mode == MarginMode::Notes { "true" } else { "false" },
                    onclick: move |_| ctx.set_margin_mode(MarginMode::Notes),
                    "Notes"
                }
                button {
                    id: "margin-mode-agent",
                    class: "margin-mode",
                    r#type: "button",
                    role: "tab",
                    aria_selected: if mode == MarginMode::Agent { "true" } else { "false" },
                    onclick: move |_| ctx.set_margin_mode(MarginMode::Agent),
                    "Agent"
                }
            }
        }
    }
}

/// The loose-notes reading: filter chips, the groups, the motto and the composer.
#[component]
fn NotesColumn(
    count: usize,
    chips: Vec<(Subject, usize)>,
    groups: Vec<(Subject, Vec<LooseNote>)>,
    filter: Option<Subject>,
    armed: Option<String>,
    empty_text: String,
) -> Element {
    let ctx = use_context::<AppCtx>();
    let chips_len = chips.len();

    rsx! {
        div { class: "margins-notes",
            if chips_len > 1 {
                div { class: "margin-chips", role: "group", aria_label: "Filter loose notes by subject",
                    button {
                        class: "mchip",
                        r#type: "button",
                        aria_pressed: if filter.is_none() { "true" } else { "false" },
                        onclick: move |_| ctx.set_margin_filter(None),
                        "All"
                    }
                    for (subject, n) in chips {
                        button {
                            class: "mchip",
                            r#type: "button",
                            title: "{subject.name()}",
                            aria_pressed: if filter == Some(subject) { "true" } else { "false" },
                            onclick: {
                                let next = if filter == Some(subject) { None } else { Some(subject) };
                                move |_| ctx.set_margin_filter(next)
                            },
                            span { class: "h", "{subject.handle()}" }
                            span { class: "n", "{n}" }
                        }
                    }
                }
            }
            div { class: "margins-foot",
                "Details with no day yet. Clip one when it gets a date."
                // Benfica's ribbon, earned: out of many loose details, one
                // day — the same union the crest's motto names. It sits at
                // the bottom of the field, where the club keeps it.
                span { class: "motto", "E pluribus unum" }
            }
            ul { class: "margins-list",
                if groups.is_empty() {
                    li { class: "note-empty",
                        p { "{empty_text}" }
                    }
                }
                for (subject, group) in groups {
                    li { class: "note-group",
                        div { class: "note-group-h",
                            span { class: "t", "{subject.name()}" }
                            span { class: "n", "{group.len()}" }
                        }
                        ul { class: "note-group-list",
                            for note in group {
                                NoteItem { note, armed: armed.clone() }
                            }
                        }
                    }
                }
            }
            NoteComposer {}
        }
    }
}

#[component]
fn NoteItem(note: LooseNote, armed: Option<String>) -> Element {
    let ctx = use_context::<AppCtx>();
    let id = note.id.clone();
    let text = note.text.clone();
    let summary = text.lines().next().unwrap_or("").trim().to_string();
    let has_body = text.lines().count() > 1;
    let subject = note.subject;
    // While the wide band owns this note's editor, the margin's row stays
    // collapsed: the editor (and its textarea id) exists once, in the band.
    let wide = ctx.ui.read().wide_note.as_deref() == Some(id.as_str());
    let open = !wide && ctx.ui.read().margin_open.as_deref() == Some(id.as_str());
    let pick_key = format!("note:{id}");
    let subject_open = ctx.ui.read().subject_picker.as_deref() == Some(pick_key.as_str());
    let is_armed = armed.as_deref() == Some(id.as_str());
    let reminder_key = format!("note:{id}");
    let reminder_open = ctx.ui.read().reminder_panel.as_deref() == Some(reminder_key.as_str());
    let reminder_hint = summary.clone();

    rsx! {
        li {
            id: "note-{id}",
            class: if open { "note open" } else { "note" },
            div { class: "note-top",
                MarkGlyph {
                    mark: note.mark,
                    text: summary.clone(),
                    subject: Some(subject),
                    note: true,
                }
                button {
                    class: "note-sum",
                    id: "note-{id}-sum",
                    r#type: "button",
                    aria_expanded: if open { "true" } else { "false" },
                    aria_label: "Open the note — {summary}",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.toggle_margin_open(&id)
                    },
                    span { class: "t", "{summary}" }
                    if has_body {
                        span { class: "more", aria_hidden: "true", "…" }
                    }
                }
            }
            if !open {
                div { class: "note-acts",
                    ClipButton { id: id.clone(), is_armed }
                    button {
                        class: "note-open",
                        r#type: "button",
                        aria_expanded: if reminder_open { "true" } else { "false" },
                        onclick: {
                            let reminder_key = reminder_key.clone();
                            move |_| ctx.toggle_reminder_panel(&reminder_key)
                        },
                        if reminder_open { "Reminding…" } else { "Remind" }
                    }
                    button {
                        class: "note-open",
                        r#type: "button",
                        aria_label: "Write this note wide",
                        onclick: {
                            let id = id.clone();
                            move |_| ctx.open_wide_note(&id)
                        },
                        "Write wide"
                    }
                    button {
                        class: "note-rm",
                        r#type: "button",
                        onclick: {
                            let id = id.clone();
                            move |_| ctx.remove_note(&id)
                        },
                        "Remove"
                    }
                }
            } else {
                div { class: "note-body",
                    NoteEditor {
                        target: NoteTarget::Loose(id.clone()),
                        who: subject.name().to_string(),
                        label: format!("Markdown note for {summary}"),
                        placeholder: "**Ask about** the *notice period*\n- [ ] get it in writing\n- keep the reply here".to_string(),
                        empty_hint: "Nothing written under this yet.".to_string(),
                    }
                    div { class: "note-acts",
                        button {
                            class: "note-sub",
                            r#type: "button",
                            title: "{subject.name()} — change the subject",
                            aria_expanded: if subject_open { "true" } else { "false" },
                            onclick: {
                                let pick_key = pick_key.clone();
                                move |_| ctx.toggle_subject_picker(&pick_key)
                            },
                            "{subject.handle()}"
                        }
                        ClipButton { id: id.clone(), is_armed }
                        button {
                            class: "note-open",
                            r#type: "button",
                            aria_expanded: if reminder_open { "true" } else { "false" },
                            onclick: {
                                let reminder_key = reminder_key.clone();
                                move |_| ctx.toggle_reminder_panel(&reminder_key)
                            },
                            if reminder_open { "Reminding…" } else { "Remind" }
                        }
                        button {
                            class: "note-open",
                            r#type: "button",
                            aria_label: "Write this note wide",
                            onclick: {
                                let id = id.clone();
                                move |_| ctx.open_wide_note(&id)
                            },
                            "Write wide"
                        }
                        button {
                            class: "note-rm",
                            r#type: "button",
                            onclick: {
                                let id = id.clone();
                                move |_| ctx.remove_note(&id)
                            },
                            "Remove"
                        }
                    }
                    if subject_open {
                        NoteSubjectPicker { id: id.clone(), current: subject }
                    }
                }
            }
            if reminder_open {
                crate::components::reminders::RemindPanel {
                    target: ReminderTarget::Note(id.clone()),
                    hint: reminder_hint.clone(),
                }
            }
        }
    }
}

/// The arm-then-click clip control, shared by the collapsed and open rows.
#[component]
fn ClipButton(id: String, is_armed: bool) -> Element {
    let ctx = use_context::<AppCtx>();
    rsx! {
        button {
            class: "note-clip",
            r#type: "button",
            aria_pressed: if is_armed { "true" } else { "false" },
            onclick: {
                let id = id.clone();
                move |_| {
                    let mut ui = ctx.ui;
                    let next = if ui.read().arm_note.as_deref() == Some(id.as_str()) {
                        None
                    } else {
                        Some(id.clone())
                    };
                    ui.write().arm_note = next;
                    let armed = ui.read().arm_note.is_some();
                    if armed {
                        ctx.flash("Pick a day. Esc to cancel.");
                    }
                }
            },
            if is_armed { "Pick a day…" } else { "Clip to a day" }
        }
    }
}

/// The same seventeen subjects as the fold's picker, filing a note instead of
/// an entry.
#[component]
fn NoteSubjectPicker(id: String, current: Subject) -> Element {
    let ctx = use_context::<AppCtx>();
    rsx! {
        div { class: "subpick",
            div { class: "lbl", "File this note under" }
            for (index, subject) in Subject::ALL.iter().enumerate() {
                button {
                    r#type: "button",
                    aria_pressed: if *subject == current { "true" } else { "false" },
                    onclick: {
                        let id = id.clone();
                        let subject = *subject;
                        move |_| {
                            ctx.set_note_subject(&id, subject);
                            ctx.toggle_subject_picker(&format!("note:{id}"));
                        }
                    },
                    span { class: "o", {format!("{:02}", index + 1)} }
                    span { "{subject.name()}" }
                }
            }
        }
    }
}

#[component]
fn NoteComposer() -> Element {
    let ctx = use_context::<AppCtx>();
    let mut draft = use_signal(String::new);
    let mut picked = use_signal(|| None::<Subject>);
    let mut pick_open = use_signal(|| false);
    let value = draft();
    let lens = ctx.doc.read().ui.lens;
    // A typed `/subject` wins over the picker, the picker over the lens —
    // the same order `add_note` files with.
    let preview = crate::state::preview_line(&value, picked().or(lens));
    let chosen = picked().or(lens);
    let peek_class = if value.trim().is_empty() {
        "peek empty"
    } else {
        "peek"
    };

    rsx! {
        form {
            class: "noteadd",
            onsubmit: {
                move |evt| {
                    evt.prevent_default();
                    let text = draft();
                    if !text.trim().is_empty() {
                        ctx.add_note(&text, picked());
                        draft.set(String::new());
                    }
                }
            },
            span { class: "{peek_class}", aria_hidden: "true",
                MarkGlyph { mark: preview.mark, subject: Some(preview.subject) }
            }
            input {
                id: "note-capture",
                r#type: "text",
                autocomplete: "off",
                placeholder: "A detail with no day yet",
                aria_label: "Add a loose note",
                value: "{value}",
                oninput: move |evt| draft.set(evt.value()),
            }
            button {
                class: "note-subpick",
                r#type: "button",
                title: if let Some(s) = chosen { format!("{} — pick another", s.name()) } else { "Pick a subject" },
                aria_expanded: if pick_open() { "true" } else { "false" },
                aria_label: "Subject for the new note",
                onclick: move |evt| {
                    evt.prevent_default();
                    pick_open.toggle();
                },
                if let Some(s) = chosen { "{s.handle()}" } else { "AUTO" }
            }
            button { r#type: "submit", "Add" }
            if pick_open() {
                div { class: "composer-pick subpick",
                    div { class: "lbl", "File the new note under" }
                    for (index, subject) in Subject::ALL.iter().enumerate() {
                        button {
                            r#type: "button",
                            aria_pressed: if Some(*subject) == picked().or(lens) { "true" } else { "false" },
                            onclick: {
                                let subject = *subject;
                                move |_| {
                                    // Pressing the chosen subject again clears back to automatic.
                                    let next = if Some(subject) == picked() {
                                        None
                                    } else {
                                        Some(subject)
                                    };
                                    picked.set(next);
                                    pick_open.set(false);
                                    crate::storage::focus_id("note-capture");
                                }
                            },
                            span { class: "o", {format!("{:02}", index + 1)} }
                            span { "{subject.name()}" }
                        }
                    }
                }
            }
        }
    }
}

/// The wide writing space: a loose note opened as a full-width band above the
/// wall, in the same shape as a day's page. The margin's narrow column is no
/// place for a long body; `Write wide` borrows the wall's width instead of
/// introducing a dialog, because a note is not an interruption — it is the
/// thing being written. Esc returns.
#[component]
pub fn WideNoteBand() -> Element {
    let ctx = use_context::<AppCtx>();
    let id = ctx.ui.read().wide_note.clone();

    let Some(id) = id else {
        return rsx! {};
    };

    let doc = ctx.doc.read();
    let Some(note) = doc.note(&id).cloned() else {
        drop(doc);
        return rsx! {};
    };
    drop(doc);

    let summary = note.text.lines().next().unwrap_or("").trim().to_string();
    let has_body = note.text.lines().count() > 1;
    let created = chrono::DateTime::parse_from_rfc3339(&note.created_at)
        .map(|dt| dates::short_day(dt.with_timezone(&chrono::Utc).date_naive()))
        .unwrap_or_else(|_| "—".to_string());
    let pick_key = format!("note:{id}");
    let subject_open = ctx.ui.read().subject_picker.as_deref() == Some(pick_key.as_str());

    rsx! {
        div { class: "noteband", id: "noteband",
            div { class: "noteband-head",
                span { class: "t",
                    if summary.is_empty() { "An untitled note" } else { "{summary}" }
                    if has_body { span { class: "more", aria_hidden: "true", " …" } }
                }
                span { class: "k",
                    "{note.subject.name()} · loose · added {created} · markdown"
                }
            }
            NoteEditor {
                target: NoteTarget::Loose(id.clone()),
                who: "Loose note".to_string(),
                label: if summary.is_empty() {
                    "Markdown for an untitled loose note".to_string()
                } else {
                    format!("Markdown note for {summary}")
                },
                placeholder: "**Ask about** the *notice period*\n- [ ] get it in writing\n- keep the reply here".to_string(),
                empty_hint: "Nothing written under this yet.".to_string(),
                whole: true,
            }
            div { class: "noteband-acts",
                button {
                    class: "chip",
                    r#type: "button",
                    title: "{note.subject.name()} — change the subject",
                    aria_expanded: if subject_open { "true" } else { "false" },
                    onclick: {
                        let pick_key = pick_key.clone();
                        move |_| ctx.toggle_subject_picker(&pick_key)
                    },
                    "{note.subject.handle()}"
                }
                span { class: "grow" }
                button {
                    class: "chip",
                    r#type: "button",
                    onclick: move |_| ctx.close_wide_note(),
                    "Back to the column ↵"
                }
                button {
                    class: "chip danger",
                    r#type: "button",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.remove_note(&id)
                    },
                    "Remove"
                }
                if subject_open {
                    span { class: "grow" }
                }
            }
            if subject_open {
                div { class: "subpick",
                    div { class: "lbl", "File this note under" }
                    for (index, subject) in Subject::ALL.iter().enumerate() {
                        button {
                            r#type: "button",
                            aria_pressed: if *subject == note.subject { "true" } else { "false" },
                            onclick: {
                                let id = id.clone();
                                let subject = *subject;
                                let pick_key = pick_key.clone();
                                move |_| {
                                    ctx.set_note_subject(&id, subject);
                                    ctx.toggle_subject_picker(&pick_key);
                                }
                            },
                            span { class: "o", {format!("{:02}", index + 1)} }
                            span { "{subject.name()}" }
                        }
                    }
                }
            }
        }
    }
}
