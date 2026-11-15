use chrono::Datelike;
use dioxus::prelude::*;
use horizon_core::dates::{self, Week};

use crate::components::labels;
use crate::components::marks::MarkGlyph;
use crate::state::AppCtx;

#[derive(Clone)]
struct WeekView {
    week: Week,
    live: usize,
    holds_today: bool,
    open: bool,
}

#[component]
pub fn Agenda() -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let doc = ctx.doc.read();
    let lens = doc.ui.lens;
    let open_day = doc.ui.open_day.clone();
    let armed = ctx.ui.read().arm_note.is_some();

    let mut weeks = Vec::new();
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
        let open = open_day.as_deref().is_some_and(|key| {
            week.slots
                .iter()
                .flatten()
                .any(|i| quarter.days[*i].key == key)
        });
        weeks.push(WeekView {
            week: week.clone(),
            live,
            holds_today,
            open,
        });
    }
    drop(doc);

    rsx! {
        div { class: "agenda", aria_label: "The quarter, week by week",
            for view in weeks {
                if view.live > 0 || view.holds_today || view.open {
                    div { class: if view.holds_today { "ag-week now" } else { "ag-week" },
                        span { class: "m", "{week_range(&view.week)}" }
                        span { class: "w", "Week {view.week.number}" }
                    }
                    // The current week is always rendered in full, even when it is empty.
                    // Other weeks print only the days that carry something.
                    for slot in view.week.slots {
                        if let Some(index) = slot {
                            {
                                let day = &quarter.days[index];
                                let has = !ctx.doc.read().items_on(&day.key, lens, quarter.start).is_empty()
                                    || ctx.doc.read().has_content(&day.key, lens);
                                let is_open = open_day.as_deref() == Some(day.key.as_str());
                                let is_today = day.date == quarter.today;
                                if has || view.holds_today || is_open {
                                    rsx! {
                                        AgendaDay {
                                            key: "{day.key}",
                                            index,
                                            today: is_today,
                                            armed,
                                        }
                                        if is_open {
                                            if let Some(open) = open_day.as_deref() {
                                                div { class: "ag-fold",
                                                    crate::components::fold::Fold {
                                                        key: "{open}",
                                                        day: open.to_string(),
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn week_range(week: &Week) -> String {
    let start = week.first;
    let end = start + chrono::Duration::days(6);
    if start.month() == end.month() {
        format!(
            "{}–{} {}",
            start.day(),
            end.day(),
            dates::month_short(start)
        )
    } else {
        format!(
            "{} {} – {} {}",
            start.day(),
            dates::month_short(start),
            end.day(),
            dates::month_short(end)
        )
    }
}

#[component]
fn AgendaDay(index: usize, today: bool, armed: bool) -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let doc = ctx.doc.read();
    let day = &quarter.days[index];
    let key = day.key.clone();
    let lens = doc.ui.lens;
    let items = doc.items_on(&key, lens, quarter.start);
    let tier = doc.tier(&key, lens);
    let entries: Vec<String> = doc.entries_on(&key, lens).map(|e| e.text.clone()).collect();
    let clipped: Vec<String> = doc
        .clipped_notes_on(&key, lens)
        .map(|n| n.text.clone())
        .collect();
    let page = lens.is_none() && doc.has_page(&key);
    let aria = labels::cell_label(&doc, &key, lens, &dates::key(quarter.today), quarter.start);
    let marks: Vec<_> = items.iter().take(8).cloned().collect();
    let first_line = entries
        .first()
        .cloned()
        .or_else(|| clipped.first().cloned())
        .unwrap_or_else(|| {
            if page {
                "a written page".into()
            } else {
                "open".into()
            }
        });
    let more = (entries.len() + clipped.len()).saturating_sub(1);
    drop(doc);

    let mut classes = String::from("ag-day");
    if !tier.class().is_empty() {
        classes.push(' ');
        classes.push_str(tier.class());
    }
    if today {
        classes.push_str(" today");
    }
    if armed {
        classes.push_str(" armed");
    }

    rsx! {
        button {
            id: "agenda-{key}",
            class: "{classes}",
            r#type: "button",
            aria_label: "{aria}",
            aria_current: if today { "date" } else { "false" },
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
            span { class: "ag-date",
                span { class: "wd", "{dates::weekday_short(day.date)} " }
                if today {
                    span { class: "stamp", "{day.date.day()}" }
                } else {
                    b { "{day.date.day()}" }
                }
                span { " {dates::month_short(day.date)}" }
                if today {
                    span { class: "ag-today", "today" }
                }
            }
            span { class: "ag-body",
                if !marks.is_empty() {
                    span { class: "ag-marks",
                        for item in marks {
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
                span { class: "ag-words",
                    span { class: "t", "{first_line}" }
                    if more > 0 {
                        span { class: "ag-more", "+{more}" }
                    }
                    if page {
                        span { class: "badge", "MD" }
                    }
                }
            }
        }
    }
}
