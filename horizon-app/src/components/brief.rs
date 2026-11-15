use dioxus::prelude::*;
use horizon_core::{Mark, Subject};

use crate::components::marks::MarkGlyph;
use crate::state::AppCtx;
use crate::storage;

#[cfg(feature = "sync")]
fn sync_block() -> Element {
    rsx! { crate::components::sync::SyncBlock {} }
}

#[cfg(not(feature = "sync"))]
fn sync_block() -> Element {
    rsx! {}
}

#[component]
pub fn Brief() -> Element {
    let ctx = use_context::<AppCtx>();
    let mut import = use_signal(String::new);
    let import_value = import();
    let status = ctx.ui.read().status.clone();

    rsx! {
        section { class: "brief", aria_labelledby: "briefTitle",
            div { class: "brief-inner",
                p { class: "kicker", "Horizon · a personal life OS" }
                h2 { id: "briefTitle", "Ninety days, ", br {}, em { "and the writing underneath." } }
                p { class: "lede",
                    "The wall is the map: ninety days, Monday-first, every thing printed as one mark. "
                    "Under every thing and every day there is a markdown page. Nothing is filed, tagged, "
                    "reviewed, reminded or counted as missed."
                }
                p {
                    "Marks and subjects are inferred from the words you write and can be corrected in one click. "
                    "Density is ink: a blank day is paper, a loaded day takes a wash, a landmark floods the block. "
                    "Terracotta means today and nothing else."
                }

                h3 { "The eighteen marks" }
                ul { class: "mkindex",
                    for mark in Mark::ALL {
                        li {
                            MarkGlyph { mark }
                            span { "{mark.name()}" }
                            span { class: "n", "{mark.examples()}" }
                        }
                    }
                }

                h3 { "The seventeen" }
                ul { class: "mkindex subindex",
                    for (index, subject) in Subject::ALL.iter().enumerate() {
                        li {
                            span { class: "o", {format!("{:02}", index + 1)} }
                            span { "{subject.name()}" }
                            span { class: "h", "{subject.handle()}" }
                        }
                    }
                }

                h3 { "The rules this app refuses to break" }
                ul { class: "principles",
                    li { b { "Nothing needs tending." } " There is no inbox, no tag tree, no streak. Marks and subjects are a byproduct of writing, not a filing decision you owe the app." }
                    li { b { "A blank day is a legal state." } " Open days are drawn as paper, not as unfinished work — and a subject with a dash is allowed to stay that way." }
                    li { b { "The full picture never leaves the screen." } " The quarter stays visible while a day is folded open, and a subject is read by narrowing the wall, never by leaving it." }
                    li { b { "One flourish." } " Quiet paper, ink, and one terracotta. The only theatrical moment is a day unfolding to full size." }
                }

                h3 { "Still open" }
                p { class: "open-q",
                    "Four questions are deliberately left as defaults, each behind one constant. "
                    "Recurrence is a single " b { "repeats weekly" } " toggle: future same-weekday occurrences are drawn as dashed outlines and never add ink weight, with no series management or exceptions. "
                    "A subject can only go quiet; it cannot be closed. "
                    "A page is one object: if its first write predates its day it prints " b { "written ahead" } ", and once the day has passed it reads as a record. "
                    "The wall looks forward only; there is no past window and no infinite scroll."
                }

                h3 { "Where it lives" }
                p {
                    "The live store is a folder of plain Markdown files: one file per entry, one per loose note, and one per day page. "
                    "The files are the source of truth; the wall and index are derived from them."
                }
                if let Some(path) = storage::store_path() {
                    p { class: "store-path", "{path}" }
                }

                h3 { "Keep a copy" }
                p {
                    "The quarter is local-first and has no account. Export it as JSON, or write a copy of the same Markdown store — the same files, the same bytes, ready for any editor in ten years."
                }
                p { class: "brief-status", role: "status", aria_live: "polite", "{status}" }
                div { class: "export-row",
                    button {
                        class: "chip",
                        r#type: "button",
                        onclick: move |_| ctx.export_json(),
                        "Download the JSON"
                    }
                    button {
                        class: "chip",
                        r#type: "button",
                        onclick: move |_| ctx.export_markdown(),
                        "Write a copy of the store"
                    }
                }
                label { r#for: "import-json", class: "import-label", "Restore from JSON" }
                textarea {
                    id: "import-json",
                    class: "import-box",
                    placeholder: "Paste a Horizon JSON document here",
                    value: "{import_value}",
                    oninput: move |evt| import.set(evt.value()),
                }
                button {
                    class: "chip danger",
                    r#type: "button",
                    disabled: import_value.trim().is_empty(),
                    onclick: move |_| {
                        if ctx.import_json(&import()).is_ok() {
                            import.set(String::new());
                        }
                    },
                    "Replace everything from this JSON"
                }

                {sync_block()}

                h3 { "Deliberately not supported" }
                p {
                    "No raw HTML, embedded files or pasted images. No notifications. No conflict dialogs. "
                    "No rich-text editor. No charts, because a chart is an abstraction you decode before you can read it."
                }

                p { class: "colophon",
                    "Horizon · local-first · plain Markdown files · markdown underneath everything"
                }
            }
        }
    }
}
