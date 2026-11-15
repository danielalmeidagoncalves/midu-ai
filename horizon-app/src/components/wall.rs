use chrono::Datelike;
use dioxus::prelude::*;
use horizon_core::{
    dates::{self, Week},
    DrawMode, Subject,
};

use crate::components::{labels, marks::MarkGlyph};
use crate::state::AppCtx;

#[derive(Clone)]
struct WeekView {
    week: Week,
    live: usize,
    holds_today: bool,
    quiet: bool,
    month_label: Option<&'static str>,
}

#[component]
pub fn Wall() -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let doc = ctx.doc.read();
    let lens = doc.ui.lens;
    let mode = doc.ui.draw_mode;
    let open_day = doc.ui.open_day.clone();
    let armed = ctx.ui.read().arm_note.is_some();
    let lens_name = lens.map(|s| s.name().to_string());

    let mut weeks = Vec::new();
    let mut last_month = None;
    for week in &quarter.weeks {
        let live = week
            .slots
            .iter()
            .flatten()
            .filter(|index| {
                let day = &quarter.days[**index];
                !doc.items_on(&day.key, lens, quarter.start).is_empty()
                    || doc.has_content(&day.key, lens)
            })
            .count();
        let holds_today = week.slots.iter().flatten().any(|i| {
            let day = &quarter.days[*i];
            day.date == quarter.today
        });
        let quiet = live == 0 && !holds_today;
        let month_label = if last_month != Some(week.month) {
            last_month = Some(week.month);
            Some(dates::MONTH_SHORT[(week.month - 1) as usize])
        } else {
            None
        };
        weeks.push(WeekView {
            week: week.clone(),
            live,
            holds_today,
            quiet,
            month_label,
        });
    }

    let aria = match lens_name {
        Some(name) => format!("Quarter wall through {name}, {}", quarter.range_label()),
        None => format!("Quarter wall, {}", quarter.range_label()),
    };
    drop(doc);

    rsx! {
        div { class: "wall", role: "group", aria_label: "{aria}",
            div { class: "wkhead", aria_hidden: "true",
                div { class: "wkhead-gut" }
                div { class: "wkhead-row",
                    for label in dates::WEEKDAY_SHORT {
                        span { "{label}" }
                    }
                }
            }
            for view in weeks {
                div {
                    class: if view.quiet {
                        if view.holds_today { "gut now quiet" } else { "gut quiet" }
                    } else if view.holds_today {
                        "gut now"
                    } else {
                        "gut"
                    },
                    span { class: "mon", {view.month_label.unwrap_or("")} }
                    span { class: "wk", "W{view.week.number}" }
                    if view.holds_today {
                        span { class: "gnow", "now" }
                    }
                    span {
                        class: "gcount",
                        if view.live == 0 {
                            ""
                        } else if view.live == 1 {
                            "1 day"
                        } else {
                            "{view.live} days"
                        }
                    }
                }
                div {
                    class: if view.quiet { "row quiet" } else { "row" },
                    for slot in view.week.slots {
                        if let Some(index) = slot {
                            Cell {
                                index,
                                lens,
                                mode,
                                today: quarter.days[index].date == quarter.today,
                                armed,
                                open: open_day.as_deref() == Some(quarter.days[index].key.as_str()),
                            }
                        } else {
                            span { class: "cell off", aria_hidden: "true" }
                        }
                    }
                }
                if let Some(open) = open_day.as_deref() {
                    if view.week.slots.iter().flatten().any(|i| quarter.days[*i].key == open) {
                        crate::components::fold::Fold { key: "{open}", day: open.to_string() }
                    }
                }
            }
        }
    }
}

#[component]
fn Cell(
    index: usize,
    lens: Option<Subject>,
    mode: DrawMode,
    today: bool,
    armed: bool,
    open: bool,
) -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let doc = ctx.doc.read();
    let day = &quarter.days[index];
    let key = day.key.clone();
    let items = doc.items_on(&key, lens, quarter.start);
    let tier = doc.tier(&key, lens);
    let weekend = dates::is_weekend(day.date);
    let today_key = dates::key(quarter.today);
    let aria = labels::cell_label(&doc, &key, lens, &today_key, quarter.start);
    let page_badge = lens.is_none() && doc.has_page(&key);
    let note_badge = doc.note_count(&key);
    let entries: Vec<(String, String)> = doc
        .entries_on(&key, lens)
        .map(|e| (e.text.clone(), e.mark.name().to_string()))
        .collect();
    let first_clipped = doc
        .clipped_notes_on(&key, lens)
        .next()
        .map(|n| n.text.clone());
    let shown_marks = items.iter().take(9).cloned().collect::<Vec<_>>();
    let more = items.len().saturating_sub(shown_marks.len());
    let words_shown = entries.iter().take(3).cloned().collect::<Vec<_>>();
    let words_more = entries.len().saturating_sub(words_shown.len());
    let number = day.date.day();
    drop(doc);

    let mut classes = String::from("cell");
    if !items.is_empty() {
        classes.push_str(" has");
    }
    if !tier.class().is_empty() {
        classes.push(' ');
        classes.push_str(tier.class());
    }
    if weekend {
        classes.push_str(" we");
    }
    if today {
        classes.push_str(" today");
    }
    if armed {
        classes.push_str(" armed");
    }
    if open {
        classes.push_str(" is-open");
    }
    let marks_class = if shown_marks.len() == 1 {
        "marks one"
    } else if shown_marks.len() == 2 {
        "marks two"
    } else if shown_marks.len() >= 5 {
        "marks dense"
    } else {
        "marks"
    };

    let move_focus = move |delta: i64| {
        let target = (index as i64 + delta).clamp(0, quarter.days.len() as i64 - 1) as usize;
        let key = quarter.days[target].key.clone();
        crate::storage::focus_id(&format!("cell-{key}"));
    };

    rsx! {
        button {
            id: "cell-{key}",
            class: "{classes}",
            r#type: "button",
            aria_label: "{aria}",
            aria_current: if today { "date" } else { "false" },
            tabindex: if today { "0" } else { "-1" },
            onclick: move |_| {
                if armed {
                    let note = ctx.ui.read().arm_note.clone();
                    if let Some(note) = note {
                        ctx.clip_note(&note, &key);
                    }
                } else {
                    ctx.open_day(&key);
                }
            },
            onkeydown: move |evt| {
                match evt.key() {
                    Key::ArrowRight => { evt.prevent_default(); move_focus(1); }
                    Key::ArrowLeft => { evt.prevent_default(); move_focus(-1); }
                    Key::ArrowDown => { evt.prevent_default(); move_focus(7); }
                    Key::ArrowUp => { evt.prevent_default(); move_focus(-7); }
                    _ => {}
                }
            },
            span { class: "num",
                if today {
                    span { class: "stamp", "{number}" }
                } else {
                    span { "{number}" }
                }
                if page_badge || note_badge > 0 {
                    span { class: "ctags",
                        if note_badge > 0 {
                            span {
                                class: "badge",
                                title: "{note_badge} entries carry a markdown note",
                                if note_badge == 1 { "N" } else { "N{note_badge}" }
                            }
                        }
                        if page_badge {
                            span { class: "badge", title: "This day has its own markdown page", "MD" }
                        }
                    }
                }
                if mode == DrawMode::Marks && more > 0 {
                    span { class: "more", "+{more}" }
                }
                if mode == DrawMode::Words && words_more > 0 {
                    span { class: "more", "+{words_more}" }
                }
            }
            if mode == DrawMode::Words {
                if words_shown.is_empty() {
                    if let Some(note) = first_clipped {
                        span { class: "ev", "{note}" }
                    }
                } else {
                    for (text, _) in words_shown {
                        span { class: "ev", "{text}" }
                    }
                }
            } else if !shown_marks.is_empty() {
                div { class: "{marks_class}",
                    for item in shown_marks {
                        MarkGlyph {
                            mark: item.mark,
                            text: item.text,
                            subject: Some(item.subject),
                            note: item.loose,
                            projected: item.projected,
                        }
                    }
                }
            }
            if today {
                span { class: "today-tag", "today" }
            }
            // Faint numeral in the bottom-right corner, sized to whatever room the
            // row has: chrome, never content. The row still gets taller when the
            // marks need it; the corner numeral only spends space that is left over.
            span { class: "ghost", aria_hidden: "true", "{number}" }
        }
    }
}
