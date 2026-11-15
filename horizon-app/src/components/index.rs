use dioxus::prelude::*;
use horizon_core::{dates, Subject};

use crate::components::marks::MarkGlyph;
use crate::state::AppCtx;

#[component]
pub fn IndexBand() -> Element {
    let ctx = use_context::<AppCtx>();
    let doc = ctx.doc.read();
    let quarter = ctx.quarter.read().clone();
    let lens = doc.ui.lens;
    let show_quiet = ctx.ui.read().show_quiet;

    let mut used = 0;
    let mut quiet = 0;
    let mut rows = Vec::new();
    for subject in Subject::ALL {
        let count = doc.subject_count(subject, &quarter);
        if count > 0 {
            used += 1;
        } else {
            quiet += 1;
        }
        let count_label = if count == 0 {
            "nothing".to_string()
        } else {
            format!("{count} {}", if count == 1 { "thing" } else { "things" })
        };
        let aria = format!(
            "{} — {} in the next ninety days",
            subject.name(),
            count_label
        );
        rows.push((subject, count, aria));
    }

    let meta = if let Some(lens) = lens {
        let count = doc.subject_count(lens, &quarter);
        let days = doc.subject_days(lens, &quarter);
        let next = doc.subject_next(lens, &quarter);
        Some((lens, count, days, next))
    } else {
        None
    };

    rsx! {
        section {
            class: if show_quiet { "index show-quiet" } else { "index" },
            aria_label: "Subject index",
            div { class: "index-head",
                h2 { id: "indexHead", "The index" }
                span { class: "index-meta",
                    if let Some((lens, count, days, next)) = meta {
                        b { "{lens.name()}" }
                        " · {count} "
                        if count == 1 { "thing" } else { "things" }
                        " · {days} "
                        if days == 1 { "day" } else { "days" }
                        if let Some(next) = next {
                            " · next "
                            MarkGlyph { mark: next.mark, text: next.text.clone(), subject: Some(lens) }
                            " {next.text} · "
                            {
                                dates::parse_key(&next.day)
                                    .map(dates::short_day)
                                    .unwrap_or_else(|| next.day.clone())
                            }
                        } else {
                            " · nothing ahead"
                        }
                    } else {
                        "{Subject::ALL.len()} subjects · {used} in use · {quiet} quiet"
                    }
                }
                button {
                    id: "indexClear",
                    class: "index-clear",
                    r#type: "button",
                    hidden: lens.is_none(),
                    onclick: move |_| ctx.set_lens(None),
                    "Show the whole quarter"
                }
                button {
                    id: "indexMore",
                    class: "index-more",
                    r#type: "button",
                    aria_expanded: if show_quiet { "true" } else { "false" },
                    hidden: quiet == 0,
                    onclick: move |_| {
                        let mut ui = ctx.ui;
                        let next = !ui.read().show_quiet;
                        ui.write().show_quiet = next;
                    },
                    "Quiet subjects ({quiet})"
                }
            }
            ul { class: "index-list",
                for (subject, count, aria) in rows {
                    li {
                        class: if count == 0 { "ix quiet" } else { "ix" },
                        button {
                            id: "subject-{subject.id()}",
                            r#type: "button",
                            aria_pressed: if lens == Some(subject) { "true" } else { "false" },
                            aria_label: "{aria}",
                            onclick: move |_| {
                                if lens == Some(subject) {
                                    ctx.set_lens(None);
                                } else {
                                    ctx.set_lens(Some(subject));
                                }
                                crate::storage::focus_id(&format!("subject-{}", subject.id()));
                            },
                            span { class: "h", title: "{subject.name()}", "{subject.handle()}" }
                            if count > 0 {
                                span { class: "n", "{count}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
