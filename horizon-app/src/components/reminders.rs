//! The reminder strip, its rows, the panel that sets one, and the toast.
//!
//! The strip is the only place reminders are drawn: one quiet line each, soonest
//! first, nothing on the wall. The panel is the same inline shape the subject and
//! mark pickers use, so a reminder never opens a modal.

use chrono::{Duration, Local, Utc};
use dioxus::prelude::*;
use horizon_core::reminders as core;
use horizon_core::{Reminder, ReminderSource};

use crate::state::{AppCtx, ReminderTarget};

/// One quiet band above the wall (or the agenda), soonest first. It renders
/// nothing at all when there is nothing to remind.
#[component]
pub fn ReminderStrip() -> Element {
    let ctx = use_context::<AppCtx>();
    let doc = ctx.doc.read();
    let live: Vec<Reminder> = doc.upcoming_reminders().into_iter().cloned().collect();
    drop(doc);
    if live.is_empty() {
        return rsx! {};
    }
    let open = ctx.ui.read().reminder_open;
    let panel = ctx.ui.read().reminder_panel.clone();
    let now = Local::now();
    let next = live
        .first()
        .map(|reminder| core::humanize(reminder.at_utc(), now))
        .unwrap_or_default();
    let count = live.len();
    let label = if count == 1 {
        "Reminders · 1".to_string()
    } else {
        format!("Reminders · {count}")
    };

    rsx! {
        section { class: "rstrip", role: "region", aria_label: "Reminders",
            button {
                class: "rstrip-head",
                r#type: "button",
                aria_expanded: if open { "true" } else { "false" },
                onclick: move |_| ctx.toggle_reminders(),
                span { class: "t", "{label}" }
                if !open {
                    span { class: "k", "next {next}" }
                }
                span { class: "chev", aria_hidden: "true", if open { "−" } else { "+" } }
            }
            if open {
                ul { class: "rlist",
                    for reminder in live {
                        ReminderRow { key: "{reminder.id}", reminder }
                    }
                }
                div { class: "rfoot",
                    if panel.as_deref() == Some("bare") {
                        RemindPanel {
                            target: ReminderTarget::Bare,
                            hint: String::new(),
                        }
                    } else {
                        button {
                            class: "rnew",
                            r#type: "button",
                            onclick: move |_| ctx.toggle_reminder_panel("bare"),
                            "Remind me…"
                        }
                    }
                    ClosedAppToggle {}
                }
            }
        }
    }
}

#[component]
fn ReminderRow(reminder: Reminder) -> Element {
    let ctx = use_context::<AppCtx>();
    let id = reminder.id.clone();
    let now = Local::now();
    let when = core::humanize(reminder.at_utc(), now);
    let repeat = core::repeat_phrase(reminder.repeat, reminder.anchor_day);
    let due = reminder.is_due(Utc::now());
    let text = reminder.text.clone();
    let source = reminder.source.chip();
    let is_note = matches!(
        reminder.source,
        ReminderSource::Note(_) | ReminderSource::Entry(_)
    );

    rsx! {
        li { class: if due { "rrow due" } else { "rrow" },
            span { class: "rwhen", "{when}" }
            span { class: "rtext", "{text}" }
            if let Some(phrase) = repeat {
                span { class: "rrep", "{phrase}" }
            }
            if is_note {
                span { class: "rsrc", "{source}" }
            }
            div { class: "racts",
                button {
                    class: "rbtn",
                    r#type: "button",
                    title: "Snooze ten minutes",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.snooze_reminder(&id, Duration::minutes(10))
                    },
                    "10m"
                }
                button {
                    class: "rbtn",
                    r#type: "button",
                    title: "Snooze one hour",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.snooze_reminder(&id, Duration::hours(1))
                    },
                    "1h"
                }
                button {
                    class: "rbtn",
                    r#type: "button",
                    title: "Snooze until tomorrow at nine",
                    onclick: {
                        let id = id.clone();
                        move |_| {
                            let target = core::tomorrow_nine(Local::now());
                            ctx.snooze_reminder(&id, target - Utc::now());
                        }
                    },
                    "tmrw"
                }
                button {
                    class: "rbtn done",
                    r#type: "button",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.finish_reminder(&id)
                    },
                    "Done"
                }
                button {
                    class: "rbtn danger",
                    r#type: "button",
                    aria_label: "Remove the reminder for {text}",
                    onclick: {
                        let id = id.clone();
                        move |_| ctx.remove_reminder(&id)
                    },
                    "×"
                }
            }
        }
    }
}

/// The inline panel that sets a reminder: quick chips, one grammar line, a live
/// preview, and Set. Esc closes it.
#[component]
pub fn RemindPanel(target: ReminderTarget, hint: String) -> Element {
    let ctx = use_context::<AppCtx>();
    let mut text = use_signal(|| hint.clone());
    let mut when = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let now = Local::now();
    let value = when();
    let preview = (!value.trim().is_empty()).then(|| core::parse_when(&value, now));
    let key = target.key();
    let cancel_key = key.clone();
    let set_target = target.clone();

    rsx! {
        div {
            class: "remind-panel",
            onkeydown: {
                let cancel_key = cancel_key.clone();
                move |evt: KeyboardEvent| {
                    if evt.key() == Key::Escape {
                        evt.prevent_default();
                        ctx.toggle_reminder_panel(&cancel_key);
                    }
                }
            },
            div { class: "remind-quick",
                for (label, fill) in [
                    ("In 30m", "in 30m"),
                    ("In 1h", "in 1h"),
                    ("In 3h", "in 3h"),
                    ("Tomorrow 9:00", "tomorrow 9:00"),
                ] {
                    button {
                        class: "rbtn",
                        r#type: "button",
                        onclick: move |_| when.set(fill.to_string()),
                        "{label}"
                    }
                }
            }
            input {
                id: "remind-when-{key}",
                class: "remind-when",
                r#type: "text",
                value: "{when}",
                placeholder: "in 1h30m · at 10:30 · every month on the 30th",
                aria_label: "When to remind",
                oninput: move |evt| {
                    when.set(evt.value());
                    error.set(None);
                },
            }
            if let Some(result) = preview {
                match result {
                    Ok(parsed) => rsx! {
                        div { class: "remind-preview", "→ {core::humanize(Some(parsed.at), now)}" }
                    },
                    Err(err) => rsx! {
                        div { class: "remind-preview bad", "{err.message()}" }
                    },
                }
            }
            input {
                class: "remind-text",
                r#type: "text",
                value: "{text}",
                placeholder: "The words to show",
                aria_label: "Reminder text",
                oninput: move |evt| text.set(evt.value()),
            }
            if let Some(message) = error() {
                p { class: "remind-err", role: "alert", "{message}" }
            }
            div { class: "remind-acts",
                button {
                    class: "rbtn primary",
                    r#type: "button",
                    onclick: move |_| match ctx.set_reminder(set_target.clone(), &text(), &when()) {
                        Ok(()) => {}
                        Err(message) => error.set(Some(message)),
                    },
                    "Set"
                }
                button {
                    class: "rbtn",
                    r#type: "button",
                    onclick: {
                        let cancel_key = cancel_key.clone();
                        move |_| ctx.toggle_reminder_panel(&cancel_key)
                    },
                    "Cancel"
                }
            }
        }
    }
}

/// The one-line notice under the masthead when a reminder fires while the window
/// is open. It never takes focus and dismisses itself.
#[component]
pub fn ReminderToast() -> Element {
    let ctx = use_context::<AppCtx>();
    let Some(toast) = ctx.ui.read().toast.clone() else {
        return rsx! {};
    };
    let id = toast.reminder.clone();

    rsx! {
        div { class: "rtoast", role: "status", aria_live: "polite",
            span { class: "rt-text", "{toast.text}" }
            div { class: "rt-acts",
                if let Some(id) = id.clone() {
                    button {
                        class: "rbtn",
                        r#type: "button",
                        onclick: {
                            let id = id.clone();
                            move |_| ctx.snooze_reminder(&id, Duration::minutes(10))
                        },
                        "Snooze 10m"
                    }
                    button {
                        class: "rbtn",
                        r#type: "button",
                        onclick: {
                            let id = id.clone();
                            move |_| ctx.snooze_reminder(&id, Duration::hours(1))
                        },
                        "Snooze 1h"
                    }
                    button {
                        class: "rbtn done",
                        r#type: "button",
                        onclick: {
                            let id = id.clone();
                            move |_| ctx.finish_reminder(&id)
                        },
                        "Done"
                    }
                }
                button {
                    class: "rbtn danger",
                    r#type: "button",
                    aria_label: "Dismiss",
                    onclick: move |_| ctx.dismiss_toast(),
                    "×"
                }
            }
        }
    }
}

/// The opt-in that lets reminders fire with the window closed. It only appears
/// where a systemd user session can answer.
#[component]
fn ClosedAppToggle() -> Element {
    let ctx = use_context::<AppCtx>();
    if !crate::reminders::notifications_available() {
        return rsx! {
            span { class: "rnote", "No notification daemon — reminders wait in the strip." }
        };
    }
    if !crate::reminders::systemd_available() {
        return rsx! {
            span { class: "rnote", "Reminders fire while Horizon is open." }
        };
    }
    let enabled = crate::reminders::timer_enabled();
    let label = if enabled {
        "Reminding when closed ✓"
    } else {
        "Keep reminding when Horizon is closed"
    };
    rsx! {
        button {
            class: "rclosed",
            r#type: "button",
            aria_pressed: if enabled { "true" } else { "false" },
            onclick: move |_| {
                let outcome = if crate::reminders::timer_enabled() {
                    crate::reminders::disable_timer()
                } else {
                    crate::reminders::enable_timer()
                };
                match outcome {
                    Ok(()) => ctx.flash("Reminders now follow the window."),
                    Err(err) => ctx.flash(err),
                }
            },
            "{label}"
        }
    }
}
