use dioxus::prelude::*;
use dioxus_icons::lucide::Plus;
use horizon_core::{dates, DrawMode};

use crate::components::labels;
use crate::components::marks::MarkGlyph;
use crate::state::{parse_line, preview_line, AppCtx, View};

/// A horizon mark, reduced to a faceted letterpress vignette: a sun rising
/// over a flat horizon line, rays fanned upward. The app is a horizon of
/// days — where the ink meets the sky — so the mark shows the moment of
/// emergence. Decorative: the wordmark carries the meaning, the sun carries
/// the affection.
#[component]
pub fn HorizonMark() -> Element {
    rsx! {
        span {
            class: "horizon-wrap",
            aria_hidden: "true",
            svg {
                class: "horizon",
                view_box: "0 0 32 22",
                fill: "currentColor",
                // Sun: a disc rising, half revealed above the line.
                path { d: "M9.5 16 A6.5 6.5 0 0 1 22.5 16 Z" }
                // Horizon line: the flat edge of the world.
                rect { x: 1.5, y: 15.2, width: 29, height: 1.6 }
                // Rays: three short strokes fanning upward.
                path { d: "M22.505 10.485 L25.335 7.655 L24.345 6.665 L21.515 9.495 Z" }
                path { d: "M9.495 10.485 L6.665 7.655 L7.655 6.665 L10.485 9.495 Z" }
                rect { x: 15.3, y: 2.5, width: 1.4, height: 4.0 }
            }
        }
    }
}

#[component]
pub fn Masthead() -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let mode = ctx.doc.read().ui.draw_mode;
    let view = ctx.ui.read().view;

    rsx! {
        header { class: "masthead",
            div { class: "wordmark", HorizonMark {}, "Horizon", span { "." } }
            button {
                class: "skip-link",
                r#type: "button",
                onclick: move |_| ctx.set_view(View::Brief),
                "The brief — sync, history, portability"
            }
            div { class: "mast-range", "{quarter.range_label()}" }
            MastSync {}
            div { class: "mast-right",
                div { class: "modegroup", role: "group", aria_label: "How days are drawn",
                    span { class: "lbl", aria_hidden: "true", "Draw" }
                    button {
                        class: "modebtn",
                        r#type: "button",
                        aria_pressed: if mode == DrawMode::Marks { "true" } else { "false" },
                        onclick: move |_| ctx.set_draw_mode(DrawMode::Marks),
                        "Marks"
                    }
                    button {
                        class: "modebtn",
                        r#type: "button",
                        aria_pressed: if mode == DrawMode::Words { "true" } else { "false" },
                        onclick: move |_| ctx.set_draw_mode(DrawMode::Words),
                        "Words"
                    }
                }
                div { class: "viewtabs", role: "tablist", aria_label: "View",
                    button {
                        class: "viewtab",
                        role: "tab",
                        r#type: "button",
                        aria_selected: if view == View::Wall { "true" } else { "false" },
                        onclick: move |_| ctx.set_view(View::Wall),
                        "The wall"
                    }
                    button {
                        class: "viewtab",
                        role: "tab",
                        r#type: "button",
                        aria_selected: if view == View::Brief { "true" } else { "false" },
                        onclick: move |_| ctx.set_view(View::Brief),
                        "The brief"
                    }
                }
                Composer {}
            }
        }
    }
}

#[component]
fn Composer() -> Element {
    let ctx = use_context::<AppCtx>();
    let mut draft = use_signal(String::new);
    let lens = ctx.doc.read().ui.lens;
    let value = draft();
    let preview = preview_line(&value, lens);
    let placeholder = ctx.composer_placeholder();
    let subject_handle = preview.subject.handle();
    let subject_ok = preview.subject_ok;
    let peek_class = if value.trim().is_empty() {
        "peek empty"
    } else {
        "peek"
    };

    rsx! {
        form {
            class: "composer",
            onsubmit: move |evt| {
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
                        crate::storage::focus_id("capture");
                    }
                    Err(err) => ctx.flash(err.message()),
                }
            },
            span {
                class: "{peek_class}",
                aria_hidden: "true",
                if value.trim().is_empty() {
                    // Empty input: a plus says 'add something'. Once the line
                    // parses, the mark glyph shows what will be created.
                    span { class: "plus", Plus {} }
                } else {
                    MarkGlyph { mark: preview.mark, subject: Some(preview.subject) }
                }
            }
            label { r#for: "capture", class: "sr-only", "File something to a day" }
            input {
                id: "capture",
                name: "capture",
                r#type: "text",
                autocomplete: "off",
                spellcheck: "false",
                placeholder: "{placeholder}",
                value: "{value}",
                oninput: move |evt| draft.set(evt.value()),
            }
            span {
                class: if subject_ok { "subtok" } else { "subtok bad" },
                aria_hidden: "true",
                "· {subject_handle}"
            }
            button {
                r#type: "submit",
                span { class: "lbl", "File" },
                span { class: "arrow", "→" },
            }
        }
    }
}

#[component]
pub fn Subhead() -> Element {
    let ctx = use_context::<AppCtx>();
    let quarter = ctx.quarter.read().clone();
    let doc = ctx.doc.read();
    let today_key = dates::key(quarter.today);
    let count = labels::today_count(&doc, &quarter);
    let next = labels::next_thing(&doc, &quarter);
    let today_label = labels::day_label(&doc, &today_key, None, quarter.start);
    let today_short = labels::today_short(&quarter);
    drop(doc);

    rsx! {
        div { class: "subhead",
            h1 { "Ninety days, ", em { "in marks." } }
            button {
                id: "nowline",
                class: "nowline",
                r#type: "button",
                aria_label: "Open today — {today_label}",
                onclick: move |_| {
                    let today = dates::key(ctx.quarter.read().today);
                    if ctx.doc.read().ui.open_day.as_deref() == Some(today.as_str()) {
                        crate::storage::focus_id("fold");
                    } else {
                        ctx.open_day(&today);
                    }
                },
                span { class: "dot", aria_hidden: "true" }
                span { class: "t", "Today, ", b { "{today_short}" } }
                span { class: "meta",
                    if count == 0 {
                        "open"
                    } else if count == 1 {
                        "1 thing"
                    } else {
                        "{count} things"
                    }
                    if let Some((_, mark, text)) = next.clone() {
                        " · next "
                        MarkGlyph { mark, text: text.clone(), subject: None }
                        " {text}"
                    } else {
                        " · nothing ahead"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Footer() -> Element {
    let ctx = use_context::<AppCtx>();
    let doc = ctx.doc.read();
    let quarter = ctx.quarter.read();
    let (carrying, open) = doc.stats(&quarter, doc.ui.lens);
    let loose = doc.loose_notes().count();
    let status = ctx.ui.read().status.clone();

    rsx! {
        footer { class: "foot",
            span { class: "stat", b { "{carrying}" } " days carry something" }
            span { class: "stat", b { "{open}" } " still open" }
            span { class: "stat", b { "{loose}" } " loose notes" }
            span { class: "status", role: "status", aria_live: "polite", "{status}" }
            span { class: "note", "Nothing here needs tidying." }
        }
    }
}

#[component]
pub fn Tooltip() -> Element {
    let ctx = use_context::<AppCtx>();
    let tip = ctx.ui.read().tip.clone();
    let Some(tip) = tip else {
        return rsx! {};
    };
    let head = if tip.subject.is_empty() {
        tip.name.clone()
    } else {
        format!("{} · {}", tip.subject, tip.name)
    };
    let style = format!("left: {}px; top: {}px;", tip.x, tip.y - 8.0);
    rsx! {
        div {
            class: "tip",
            role: "tooltip",
            style: "{style}",
            b { "{head}" }
            span { "{tip.text}" }
        }
    }
}

/// Storage and sync in one line, always visible: the wall and the brief both
/// answer "is this stored?" here. The detail lives in the tooltip, not a panel.
#[cfg(feature = "sync")]
#[component]
fn MastSync() -> Element {
    let state = use_context::<Signal<crate::sync::SyncState>>();
    let status = crate::sync::status(&state.read());
    rsx! {
        span {
            class: "mast-sync {status.tone}",
            role: "status",
            aria_live: "polite",
            title: "{status.detail}",
            "{status.label}"
        }
    }
}

#[cfg(not(feature = "sync"))]
#[component]
fn MastSync() -> Element {
    rsx! {}
}
