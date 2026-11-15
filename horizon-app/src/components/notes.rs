use std::rc::Rc;

use dioxus::prelude::*;
use horizon_core::{parse_markdown, NoteTarget};

use crate::components::markdown::MarkdownView;
use crate::state::AppCtx;
use crate::storage;

#[component]
pub fn NoteEditor(
    target: NoteTarget,
    who: String,
    label: String,
    placeholder: String,
    empty_hint: String,
    #[props(default)] whole: bool,
) -> Element {
    let ctx = use_context::<AppCtx>();
    let target = Rc::new(target);
    let key = AppCtx::editing_key(&target);
    let textarea_id = format!("md-{}", key.replace(':', "-"));
    let committed = ctx.note_markdown(&target);
    let editing = ctx.note_editing(&target);
    let mut buffer = use_signal(|| committed.clone());

    // When the note is being read, keep the buffer in step with the document (imports,
    // task ticks, a commit from another surface). While writing, the buffer is the source.
    {
        let target = target.clone();
        use_effect(move || {
            if !ctx.note_editing(&target) {
                let next = ctx.note_markdown(&target);
                let current = buffer.peek().clone();
                if current != next {
                    buffer.set(next);
                }
            }
        });
    }

    let nodes = parse_markdown(&committed);
    let write_class = if editing { "mode-on" } else { "" };
    let read_class = if editing { "" } else { "mode-on" };

    let task_handler = {
        let target = target.clone();
        EventHandler::new(move |line: usize| {
            ctx.toggle_note_task(&target, line);
            let next = ctx.note_markdown(&target);
            buffer.set(next);
        })
    };

    let actions: [(&str, &str, &str, &str, &str, &str); 11] = [
        ("B", "b", "Bold", "**", "**", "bold text"),
        ("I", "i", "Italic", "*", "*", "italic text"),
        ("H", "d", "Heading", "## ", "", "Heading"),
        ("•", "q", "Bulleted list", "- ", "", "list item"),
        ("1.", "q", "Numbered list", "1. ", "", "item"),
        ("☐", "q", "Task", "- [ ] ", "", "task"),
        ("“", "q", "Quote", "> ", "", "quote"),
        ("{}", "q", "Code", "`", "`", "code"),
        ("a", "q", "Link", "[", "]()", "link text"),
        (
            "|—",
            "q",
            "Table",
            "\n| what | when |\n| --- | --- |\n| ",
            " |  |",
            "cell",
        ),
        ("—", "q", "Rule", "\n---\n", "", ""),
    ];

    rsx! {
        div { class: if whole { "evd whole" } else { "evd" },
            div { class: "evd-note",
                span { class: "who", "{who}" }
                span { class: "mode",
                    button {
                        id: "{textarea_id}-write",
                        r#type: "button",
                        class: "{write_class}",
                        aria_pressed: if editing { "true" } else { "false" },
                        aria_label: "Write the note as markdown",
                        onclick: {
                            let target = target.clone();
                            let key = key.clone();
                            let textarea_id = textarea_id.clone();
                            move |_| {
                                buffer.set(ctx.note_markdown(&target));
                                ctx.toggle_note_editing(&key, true);
                                storage::focus_textarea(&textarea_id);
                            }
                        },
                        "Write"
                    }
                    button {
                        r#type: "button",
                        class: "{read_class}",
                        aria_pressed: if editing { "false" } else { "true" },
                        aria_label: "Read the note as a page",
                        onclick: {
                            let target = target.clone();
                            let key = key.clone();
                            let textarea_id = textarea_id.clone();
                            move |_| {
                                let text = buffer();
                                ctx.set_note(&target, &text);
                                ctx.toggle_note_editing(&key, false);
                                storage::focus_id(&format!("{textarea_id}-write"));
                            }
                        },
                        "Read"
                    }
                }
            }
            div { class: "mdbar", hidden: !editing,
                for (text, cls, title, prefix, suffix, sample) in actions {
                    ToolbarButton {
                        textarea_id: textarea_id.clone(),
                        text: text.to_string(),
                        class: cls.to_string(),
                        title: title.to_string(),
                        prefix: prefix.to_string(),
                        suffix: suffix.to_string(),
                        sample: sample.to_string(),
                    }
                }
                span { class: "sep" }
            }
            div { class: "mdbox",
                textarea {
                    id: "{textarea_id}",
                    class: "mdin",
                    hidden: !editing,
                    spellcheck: "true",
                    placeholder: "{placeholder}",
                    aria_label: "{label}",
                    value: "{buffer}",
                    onmounted: {
                        let textarea_id = textarea_id.clone();
                        move |_| {
                            if editing {
                                storage::focus_textarea(&textarea_id);
                            }
                        }
                    },
                    oninput: move |evt| buffer.set(evt.value()),
                    onblur: {
                        let target = target.clone();
                        move |_| {
                            let text = buffer();
                            ctx.set_note(&target, &text);
                        }
                    },
                    onkeydown: {
                        let target = target.clone();
                        let key = key.clone();
                        let textarea_id = textarea_id.clone();
                        move |evt| {
                            if matches!(evt.key(), Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown) {
                                evt.stop_propagation();
                            }
                            let commit = evt.key() == Key::Escape
                                || (evt.key() == Key::Enter
                                    && (evt.modifiers().meta() || evt.modifiers().ctrl()));
                            if commit {
                                evt.prevent_default();
                                evt.stop_propagation();
                                let text = buffer();
                                ctx.set_note(&target, &text);
                                ctx.toggle_note_editing(&key, false);
                                storage::focus_id(&format!("{textarea_id}-write"));
                            }
                        }
                    },
                }
                div {
                    class: "mdout",
                    hidden: editing,
                    role: "document",
                    ondoubleclick: {
                        let target = target.clone();
                        let key = key.clone();
                        let textarea_id = textarea_id.clone();
                        move |_| {
                            buffer.set(ctx.note_markdown(&target));
                            ctx.toggle_note_editing(&key, true);
                            storage::focus_textarea(&textarea_id);
                        }
                    },
                    if committed.trim().is_empty() {
                        p { class: "hint", "{empty_hint}" }
                    } else {
                        MarkdownView { nodes, on_task: Some(task_handler) }
                    }
                }
            }
            if whole {
                div { class: "evd-foot",
                    span { class: "hint", "# heading, **bold**, - list, - [ ] task, > quote — Esc reads it back" }
                    span { class: "grow" }
                }
            }
        }
    }
}

#[component]
fn ToolbarButton(
    textarea_id: String,
    text: String,
    class: String,
    title: String,
    prefix: String,
    suffix: String,
    sample: String,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            title: "{title}",
            aria_label: "{title}",
            onmousedown: move |evt| evt.prevent_default(),
            onclick: move |_| {
                storage::wrap_selection(&textarea_id, &prefix, &suffix, &sample);
            },
            "{text}"
        }
    }
}
