//! The agent, in the margin and in a wide band.
//!
//! The margin already holds the things that have no day yet. The agent is the
//! other kind of detail: a conversation that reads the whole quarter. It gets
//! the same address, not a new navigation — `Notes | Agent` in the column head —
//! and the same widening move a loose note has: a centred band above the wall.
//!
//! Everything here is a reading surface over `agent::AgentView`. The service
//! owns the process; this file owns the typography.

use dioxus::prelude::*;
use horizon_core::parse_markdown;

use crate::agent::{self, AgentView, ChatItem, Status};
use crate::components::markdown::MarkdownView;
use crate::state::AppCtx;
use crate::storage;

/// The margin's agent view: a status line, the transcript, and a composer.
#[component]
pub fn AgentColumn() -> Element {
    let ctx = use_context::<AppCtx>();
    let state = use_context::<Signal<AgentView>>();
    let wide = ctx.ui.read().agent_wide;

    // The band above owns the conversation while it is open. Keeping the
    // transcript out of the column means one composer, one scroll, one input id.
    if wide {
        return rsx! {
            div { class: "agent-col agent-parked",
                p { class: "agent-parked-note", "The conversation is open wide above the wall." }
                p { class: "agent-parked-note dim", "Esc brings it back here." }
            }
        };
    }

    let view = state.read().clone();
    rsx! {
        div { class: "agent-col",
            AgentHead { view: view.clone() }
            Transcript { scroller: "agent-log".to_string(), wide: false }
            Composer { id: "agent-input".to_string() }
        }
    }
}

/// The wide band: the same conversation, centred on a 760px measure with room to
/// breathe. It is not a dialog and it does not cover the quarter — the quarter
/// stays visible under it, which is the point of writing wide in this app.
#[component]
pub fn AgentBand() -> Element {
    let ctx = use_context::<AppCtx>();
    let state = use_context::<Signal<AgentView>>();
    if !ctx.ui.read().agent_wide {
        return rsx! {};
    }
    let view = state.read().clone();
    rsx! {
        section { class: "agentband", id: "agentband", aria_label: "The agent",
            div { class: "agentband-head",
                span { class: "t", "The agent" }
                span { class: "k",
                    "Pi · "
                    StatusWord { text: status_text(&view).to_string(), working: is_working(&view) }
                }
                span { class: "grow" }
                NewConversation {}
                button {
                    class: "chip",
                    r#type: "button",
                    onclick: move |_| ctx.close_agent_wide(),
                    "Back to the column ↵"
                }
            }
            div { class: "agentband-body",
                div { class: "agentband-meta",
                    if view.model.is_empty() {
                        span { "Reads the same Markdown the wall does" }
                    } else {
                        span { "{view.model_label()}" }
                        span { " · reads the same Markdown the wall does" }
                    }
                }
                Transcript { scroller: "agentband-log".to_string(), wide: true }
                Composer { id: "agent-input-wide".to_string() }
            }
        }
    }
}

/// Status, the model, and the two actions the column owns: widen, and start a
/// new conversation. `New` arms first, like the clip control does, because
/// clearing the transcript should cost two clicks and never a dialog.
#[component]
fn AgentHead(view: AgentView) -> Element {
    let ctx = use_context::<AppCtx>();
    let status = view.status;

    rsx! {
        div { class: "agent-head",
            span { class: "a-who", "PI" }
            span { class: "a-state {status_class(status)}", title: "{view.detail}",
                span { class: "a-dot", aria_hidden: "true" }
                StatusWord { text: status_text(&view).to_string(), working: is_working(&view) }
            }
            span { class: "a-model", title: "{view.model_label()}", "{view.model}" }
            div { class: "a-acts",
                button {
                    class: "a-btn",
                    r#type: "button",
                    title: "Open the agent wide",
                    aria_label: "Open the agent wide",
                    onclick: move |_| ctx.open_agent_wide(),
                    "Wide ↗"
                }
                NewConversation {}
            }
        }
    }
}

/// Two-click clear, shared by the column and the band.
#[component]
fn NewConversation() -> Element {
    let ctx = use_context::<AppCtx>();
    let state = use_context::<Signal<AgentView>>();
    let busy = state.read().busy;
    let mut armed = use_signal(|| false);
    rsx! {
        button {
            class: if armed() { "a-btn danger" } else { "a-btn" },
            r#type: "button",
            aria_label: if armed() { "Confirm: clear the conversation" } else { "Start a new conversation" },
            disabled: busy,
            onclick: move |_| {
                if armed() {
                    armed.set(false);
                    if let Err(err) = agent::new_conversation() {
                        ctx.flash(err);
                    } else {
                        ctx.flash("A new conversation.");
                    }
                } else {
                    armed.set(true);
                }
            },
            if armed() { "Clear?" } else { "New" }
        }
    }
}

/// Whether the harness is doing something the owner should see moving.
fn is_working(view: &AgentView) -> bool {
    view.enabled && matches!(view.status, Status::Working | Status::Starting)
}

/// A status word that travels while the harness works: each letter brightens
/// and fades on its own delay, so a wave crosses the word. Per-letter spans
/// rather than a gradient clip, because the clip is not portable and the text
/// must stay legible when it is not there. The animated copy is hidden from
/// assistive tech; a plain copy sits beside it.
#[component]
fn StatusWord(text: String, working: bool) -> Element {
    if !working {
        return rsx! {
            span { class: "a-label", "{text}" }
        };
    }
    rsx! {
        span { class: "a-label is-working", aria_hidden: "true",
            for (index, letter) in text.chars().enumerate() {
                span { class: "a-letter", style: "animation-delay:-{index * 70}ms", "{letter}" }
            }
        }
        span { class: "sr-only", "{text}" }
    }
}

fn status_class(status: Status) -> &'static str {
    match status {
        Status::Failed => "is-error",
        Status::Working | Status::Starting => "is-busy",
        Status::Stopped => "is-stopped",
        _ => "is-ready",
    }
}

/// The brief can switch the harness off; the column says so instead of inviting
/// a message that would be refused.
fn status_text(view: &AgentView) -> &'static str {
    if view.enabled {
        view.status.label()
    } else {
        "off"
    }
}

/// The transcript. It reads the signal itself so the auto-scroll effect reruns on
/// new items and on every new streamed character.
#[component]
fn Transcript(scroller: String, wide: bool) -> Element {
    let state = use_context::<Signal<AgentView>>();
    let view = state.read().clone();
    let has_items = !view.items.is_empty() || !view.streaming.trim().is_empty();

    use_effect({
        let scroller = scroller.clone();
        move || {
            let _ = (
                state.read().items.len(),
                state.read().streaming.len(),
                state.read().thinking.len(),
            );
            storage::scroll_end(&scroller);
        }
    });

    rsx! {
        div { class: "agent-log", id: "{scroller}", role: "log", aria_live: "polite",
            if !has_items && view.thinking.trim().is_empty() {
                EmptyAgent { wide }
            }
            for (index, item) in view.items.iter().enumerate() {
                AgentItem { key: "{index}", item: item.clone() }
            }
            if !view.thinking.trim().is_empty() {
                ThinkingBlock { text: view.thinking.clone(), live: true }
            }
            if !view.streaming.trim().is_empty() {
                div { class: "amsg pi streaming",
                    div { class: "a-body astream", "{view.streaming}" }
                }
            }
        }
    }
}

#[component]
fn AgentItem(item: ChatItem) -> Element {
    match item {
        ChatItem::User { text, context } => rsx! {
            div { class: "amsg user",
                div { class: "a-top",
                    span { class: "a-who", "You" }
                    if context {
                        span { class: "a-ctx", "with the current view" }
                    }
                }
                p { class: "a-body", "{text}" }
            }
        },
        ChatItem::Assistant { text } => {
            let nodes = parse_markdown(&text);
            rsx! {
                div { class: "amsg pi",
                    div { class: "a-body", MarkdownView { nodes } }
                }
            }
        }
        ChatItem::Thinking { text } => rsx! {
            ThinkingBlock { text, live: false }
        },
        ChatItem::Tool {
            name,
            detail,
            out,
            done,
            failed,
            ..
        } => {
            let class = match (done, failed) {
                (_, true) => "atool failed",
                (true, false) => "atool done",
                (false, false) => "atool running",
            };
            rsx! {
                div { class: "{class}",
                    span { class: "a-tool-name", "{name}" }
                    span { class: "a-tool-detail", "{detail}" }
                    if !out.is_empty() {
                        span { class: "a-tool-out", title: "{out}" }
                    }
                }
            }
        }
        ChatItem::Notice { text, error } => rsx! {
            p { class: if error { "anotice is-error" } else { "anotice" }, "{text}" }
        },
    }
}

/// A thought, folded. Pi can think for a while before it answers, and that is
/// worth watching but not worth keeping on screen: the live block opens so the
/// working is visible, the archived one starts folded so the transcript stays a
/// conversation. Both toggle by hand, which is the whole point.
#[component]
fn ThinkingBlock(text: String, live: bool) -> Element {
    let mut open = use_signal(|| live);
    let words = text.split_whitespace().count();
    let class = match (open(), live) {
        (true, true) => "athink open live",
        (true, false) => "athink open",
        (false, true) => "athink live",
        (false, false) => "athink",
    };
    rsx! {
        div { class,
            button {
                class: "athink-toggle",
                r#type: "button",
                aria_expanded: if open() { "true" } else { "false" },
                title: if open() { "Fold the thinking" } else { "Read the thinking" },
                onclick: move |_| open.toggle(),
                span { class: "a-chev", aria_hidden: "true" }
                span { class: "a-who", "thinking" }
                if !open() {
                    span { class: "a-peek", "{peek(&text)}" }
                }
                span { class: "a-count", "{words} words" }
            }
            if open() {
                div { class: "athink-body", MarkdownView { nodes: parse_markdown(&text) } }
            }
        }
    }
}

/// The first line of a thought, with runs of whitespace collapsed, for the
/// folded row. It is a hint of what is inside, not a second rendering.
fn peek(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut out = String::new();
    let mut cut = false;
    for word in line.split_whitespace() {
        if out.is_empty() {
            out.push_str(word);
            continue;
        }
        if out.chars().count() + 1 + word.chars().count() > 64 {
            cut = true;
            break;
        }
        out.push(' ');
        out.push_str(word);
    }
    if out.chars().count() > 64 {
        out = out.chars().take(63).collect();
        cut = true;
    }
    if cut {
        out.push('…');
    }
    out
}

/// The blank conversation is the one screen allowed to teach. Three prompts, one
/// click each; they are examples of the grammar, not a menu.
#[component]
fn EmptyAgent(wide: bool) -> Element {
    let ctx = use_context::<AppCtx>();
    let state = use_context::<Signal<AgentView>>();
    let view = state.read().clone();
    let suggestions = [
        "What do the next two weeks look like?",
        "What is still loose?",
        "Write today's page from what is on the wall.",
    ];
    rsx! {
        div { class: "agent-empty",
            if !view.enabled {
                p { class: "empty-lede",
                    "The agent is switched off. Turn it on under "
                    em { "The agent" }
                    " in the brief, then come back here."
                }
                p { class: "empty-note", "Nothing is sent anywhere until you do." }
            } else {
                p { class: "empty-lede",
                    "Ask about the quarter. Pi reads the same Markdown files the wall does, "
                    "so it can answer, draft, and — if writing is on — file things for you."
                }
                if view.status == Status::Stopped || view.status == Status::Failed {
                    p { class: "empty-note", "The harness is not running. Send a message to start it again." }
                }
                div { class: "asugg", role: "group", aria_label: "Example prompts",
                    for suggestion in suggestions {
                        button {
                            class: "asugg-btn",
                            r#type: "button",
                            onclick: move |_| match send_line(ctx, suggestion) {
                                Ok(()) => {}
                                Err(err) => ctx.flash(err),
                            },
                            "{suggestion}"
                        }
                    }
                }
                if wide {
                    p { class: "empty-note", "Esc returns the conversation to the column." }
                } else {
                    p { class: "empty-note", "Alt+A opens this column. Esc widens it back." }
                }
            }
        }
    }
}

/// One composer, used by both surfaces. Enter sends; Shift+Enter breaks the
/// line; while Pi works the same button steers instead of starting a second run.
#[component]
fn Composer(id: String) -> Element {
    let ctx = use_context::<AppCtx>();
    let state = use_context::<Signal<AgentView>>();
    let busy = state.read().busy;
    let enabled = state.read().enabled;
    let mut draft = use_signal(String::new);
    let value = draft();
    let disabled = value.trim().is_empty() || !enabled;

    let mut submit = move || {
        if !enabled {
            return;
        }
        let text = draft();
        if text.trim().is_empty() {
            return;
        }
        match send_line(ctx, &text) {
            Ok(()) => draft.set(String::new()),
            Err(err) => ctx.flash(err),
        }
    };

    rsx! {
        form {
            class: "agent-add",
            onsubmit: move |evt| {
                evt.prevent_default();
                submit();
            },
            textarea {
                id: "{id}",
                class: "ain",
                rows: "2",
                spellcheck: "true",
                disabled: !enabled,
                placeholder: if !enabled {
                    "The agent is switched off in the brief"
                } else if busy {
                    "Steer it — or wait for the turn to finish"
                } else {
                    "Ask, draft, file something…"
                },
                aria_label: "Message the agent",
                value: "{value}",
                oninput: move |evt| draft.set(evt.value()),
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter && !evt.modifiers().shift() {
                        evt.prevent_default();
                        submit();
                    }
                },
            }
            div { class: "agent-add-bar",
                span { class: "a-hint",
                    if busy {
                        "Working · a message steers after this turn"
                    } else {
                        "Enter sends · Shift+Enter breaks the line"
                    }
                }
                span { class: "grow" }
                if busy {
                    button {
                        class: "a-stop",
                        r#type: "button",
                        onclick: move |_| agent::abort(),
                        "Stop"
                    }
                }
                button {
                    class: "a-send",
                    r#type: "submit",
                    disabled,
                    if busy { "Steer →" } else { "Send →" }
                }
            }
        }
    }
}

/// Build the on-screen context and send one line. Shared by the composer and the
/// three example prompts so they cannot drift apart.
fn send_line(ctx: AppCtx, text: &str) -> Result<(), String> {
    let doc = ctx.doc.read().clone();
    let quarter = ctx.quarter.read().clone();
    let context = agent::context_block(&doc, &quarter);
    agent::send(text, Some(context))
}

/// The brief's Agent block. One config form, the same fields that start the
/// harness; saving restarts a live process so the change is not a lie.
#[component]
pub fn AgentSettings() -> Element {
    let ctx = use_context::<AppCtx>();
    let mut config = use_signal(storage::agent_config);
    let mut note = use_signal(String::new);
    // (ok, text): a version is calm, a failure is an error.
    let mut check = use_signal(|| (false, String::new()));
    let current = config();
    let enabled = current.harness != "off";
    let levels = [
        "", "off", "minimal", "low", "medium", "high", "xhigh", "max",
    ];

    rsx! {
        section { class: "agent-settings", aria_labelledby: "agentTitle",
            h3 { id: "agentTitle", "The agent" }
            p {
                "Pi is the first supported agentic harness. It runs in the store folder, so it reads "
                "the same Markdown the wall does — no export, no second copy, no account. "
                "The manual it starts with lives in Horizon's config folder, never in the vault."
            }
            div { class: "agent-row",
                label { class: "agent-check",
                    input {
                        r#type: "checkbox",
                        checked: enabled,
                        onchange: move |evt| {
                            let next = if evt.checked() { "pi" } else { "off" };
                            config.write().harness = next.to_string();
                        },
                    }
                    span { "Enable the agent" }
                }
                span { class: "agent-harness", "Harness · Pi" }
            }
            label { class: "agent-label", r#for: "agent-command", "Command" }
            input {
                id: "agent-command",
                class: "sync-input",
                r#type: "text",
                autocomplete: "off",
                spellcheck: "false",
                placeholder: "pi",
                value: "{current.command}",
                oninput: move |evt| config.write().command = evt.value(),
            }
            div { class: "agent-pair",
                div {
                    label { class: "agent-label", r#for: "agent-model", "Model" }
                    input {
                        id: "agent-model",
                        class: "sync-input",
                        r#type: "text",
                        autocomplete: "off",
                        spellcheck: "false",
                        placeholder: "Pi's own default",
                        value: "{current.model}",
                        oninput: move |evt| config.write().model = evt.value(),
                    }
                }
                div {
                    label { class: "agent-label", r#for: "agent-provider", "Provider" }
                    input {
                        id: "agent-provider",
                        class: "sync-input",
                        r#type: "text",
                        autocomplete: "off",
                        spellcheck: "false",
                        placeholder: "auto",
                        value: "{current.provider}",
                        oninput: move |evt| config.write().provider = evt.value(),
                    }
                }
            }
            span { class: "agent-label", "Thinking" }
            div { class: "agent-levels", role: "group", aria_label: "Thinking level",
                for level in levels {
                    button {
                        class: "mchip",
                        r#type: "button",
                        aria_pressed: if current.thinking == level { "true" } else { "false" },
                        onclick: move |_| config.write().thinking = level.to_string(),
                        if level.is_empty() { "Pi's default" } else { "{level}" }
                    }
                }
            }
            div { class: "agent-row",
                label { class: "agent-check",
                    input {
                        r#type: "checkbox",
                        checked: current.context,
                        onchange: move |evt| config.write().context = evt.checked(),
                    }
                    span { "Tell it what is on screen" }
                }
            }
            p { class: "sync-note",
                "With this on, each message carries today's date, the window, the lens and the open day. "
                "It is a short preamble, not a copy of the notes — the files are the copy."
            }
            div { class: "agent-row",
                label { class: "agent-check",
                    input {
                        r#type: "checkbox",
                        checked: current.write,
                        onchange: move |evt| config.write().write = evt.checked(),
                    }
                    span { "Let it change the store" }
                }
            }
            p { class: "sync-note",
                "On, Pi has read, write and edit and can file things for you; every file it touches is "
                "printed in the conversation, and Horizon merges the folder after each run. Off, it is "
                "read-only: the write, edit and shell tools are not even declared."
            }
            div { class: "export-row",
                button {
                    class: "chip",
                    r#type: "button",
                    onclick: move |_| match storage::save_agent_config(&config()) {
                        Ok(()) => {
                            agent::restart(config());
                            ctx.flash("The agent will start with these settings.");
                        }
                        Err(err) => note.set(err),
                    },
                    "Save the agent's settings"
                }
                button {
                    class: "chip",
                    r#type: "button",
                    onclick: move |_| match agent::check(&config().command) {
                        Ok(version) => check.set((true, version)),
                        Err(err) => check.set((false, err)),
                    },
                    "Check the harness"
                }
            }
            p { class: "brief-status", role: "status", aria_live: "polite", "{note}" }
            if !check().1.is_empty() {
                p { class: if check().0 { "sync-note" } else { "sync-error" }, "{check().1}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::peek;

    #[test]
    fn the_folded_thought_hints_at_its_first_line() {
        assert_eq!(
            peek("Check the entries for mum."),
            "Check the entries for mum."
        );
        assert_eq!(peek("\n\n  spaced   out\nsecond line"), "spaced out");
        assert_eq!(peek("   "), "");
    }

    #[test]
    fn the_hint_stops_at_a_word_boundary() {
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen";
        let hint = peek(long);
        assert!(hint.ends_with('…'), "{hint}");
        assert!(hint.chars().count() <= 65, "{hint}");
        assert!(!hint.trim_end_matches('…').ends_with(' '), "{hint}");
        assert!(long.starts_with(hint.trim_end_matches('…')), "{hint}");
    }

    #[test]
    fn one_unbroken_word_is_cut_not_dropped() {
        let hint = peek(&"x".repeat(200));
        assert_eq!(hint.chars().count(), 64);
        assert!(hint.ends_with('…'));
    }
}
