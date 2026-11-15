use dioxus::prelude::*;
use horizon_core::{Block, Inline, ListItem};

/// Render the parser's AST. Every node is typed; text is a text node. There is no innerHTML
/// anywhere in this path, so a note can never inject markup.
#[component]
pub fn MarkdownView(
    nodes: Vec<Block>,
    #[props(default)] on_task: Option<EventHandler<usize>>,
) -> Element {
    rsx! {
        div { class: "md",
            for block in nodes {
                {render_block(&block, on_task)}
            }
        }
    }
}

fn render_block(block: &Block, on_task: Option<EventHandler<usize>>) -> Element {
    match block {
        Block::Paragraph(content) => rsx! {
            p { for inline in content { {render_inline(inline)} } }
        },
        Block::Heading { level, content } => match level {
            1 => rsx! { h1 { for inline in content { {render_inline(inline)} } } },
            2 => rsx! { h2 { for inline in content { {render_inline(inline)} } } },
            _ => rsx! { h3 { for inline in content { {render_inline(inline)} } } },
        },
        Block::List { ordered, items } => {
            if *ordered {
                rsx! {
                    ol {
                        for item in items { {render_list_item(item, on_task)} }
                    }
                }
            } else {
                rsx! {
                    ul {
                        for item in items { {render_list_item(item, on_task)} }
                    }
                }
            }
        }
        Block::Quote(content) => rsx! {
            blockquote { for inline in content { {render_inline(inline)} } }
        },
        Block::Table { head, rows } => rsx! {
            table {
                thead {
                    tr {
                        for cell in head {
                            th { for inline in cell { {render_inline(inline)} } }
                        }
                    }
                }
                tbody {
                    for row in rows {
                        tr {
                            for cell in row {
                                td { for inline in cell { {render_inline(inline)} } }
                            }
                        }
                    }
                }
            }
        },
        Block::Rule => rsx! { hr {} },
    }
}

fn render_list_item(item: &ListItem, on_task: Option<EventHandler<usize>>) -> Element {
    if let Some(done) = item.task {
        let class = if done { "cb on task" } else { "cb task" };
        let line = item.line;
        let label = format!(
            "{} — {}",
            if done { "Done" } else { "Not done" },
            plain_text(&item.content)
        );
        rsx! {
            li {
                class: "{class}",
                role: "checkbox",
                tabindex: "0",
                aria_checked: if done { "true" } else { "false" },
                aria_label: "{label}",
                onclick: move |_| {
                    if let (Some(handler), Some(line)) = (on_task, line) {
                        handler.call(line);
                    }
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter || evt.key().to_string() == " " {
                        evt.prevent_default();
                        if let (Some(handler), Some(line)) = (on_task, line) {
                            handler.call(line);
                        }
                    }
                },
                span { class: "box", aria_hidden: "true" }
                span { for inline in &item.content { {render_inline(inline)} } }
            }
        }
    } else {
        rsx! {
            li { for inline in &item.content { {render_inline(inline)} } }
        }
    }
}

fn render_inline(inline: &Inline) -> Element {
    match inline {
        Inline::Text(text) => rsx! { "{text}" },
        Inline::Bold(content) => rsx! {
            b { for child in content { {render_inline(child)} } }
        },
        Inline::Italic(content) => rsx! {
            i { for child in content { {render_inline(child)} } }
        },
        Inline::Code(code) => rsx! { code { "{code}" } },
        Inline::Link { label, href } => {
            if let Some(href) = href {
                rsx! {
                    a {
                        href: "{href}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        for child in label { {render_inline(child)} }
                    }
                }
            } else {
                rsx! {
                    span { class: "tipmark",
                        for child in label { {render_inline(child)} }
                    }
                }
            }
        }
        Inline::Person(name) => rsx! { cite { "@{name}" } },
        Inline::DayRef { day, label, num } => {
            let text = match label {
                Some(label) => format!("{num} {label}"),
                None => format!("{num}"),
            };
            let title = match label {
                Some(label) => format!("Refers to {day} — {label}"),
                None => format!("Refers to {day}"),
            };
            rsx! {
                span { class: "badge", title: "{title}", "{text}" }
            }
        }
    }
}

/// A plain-text approximation for the checkbox's accessible name.
pub fn plain_text(nodes: &[Inline]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Inline::Text(t) => out.push_str(t),
            Inline::Bold(c) | Inline::Italic(c) => out.push_str(&plain_text(c)),
            Inline::Code(c) => out.push_str(c),
            Inline::Link { label, .. } => out.push_str(&plain_text(label)),
            Inline::Person(name) => {
                out.push('@');
                out.push_str(name);
            }
            Inline::DayRef { label, num, .. } => {
                out.push_str(&num.to_string());
                if let Some(label) = label {
                    out.push(' ');
                    out.push_str(label);
                }
            }
        }
    }
    out
}
