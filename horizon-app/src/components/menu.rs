//! The in-window menu strip.
//!
//! Native only. The system menu bar on this GTK/muda build renders as a blank
//! strip, so the menu lives inside the window: one "Horizon" button that opens
//! a small sheet with the store path, a way to open the files, and Quit.
//!
//! It follows the wall's rules — no rounded corners, no shadow, monospace for
//! menu chrome, one accent for focus — and it is reachable three ways: a click,
//! `Alt+H`, and `F10`. `Esc` closes it and returns focus to the button.

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::state::{AppCtx, MarginMode, View};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    Store,
    OpenFolder,
    Agent,
    Quit,
}

impl Entry {
    const ALL: [Entry; 4] = [Entry::Store, Entry::OpenFolder, Entry::Agent, Entry::Quit];

    fn label(self) -> &'static str {
        match self {
            Entry::Store => "where the files are",
            Entry::OpenFolder => "open the folder",
            Entry::Agent => "the agent",
            Entry::Quit => "quit horizon",
        }
    }

    /// Performs the entry. Returns true when the window should close.
    fn run(self, ctx: AppCtx) -> bool {
        match self {
            Entry::Store => {
                ctx.flash(crate::menu::store_location());
                false
            }
            Entry::OpenFolder => {
                ctx.flash(match crate::desktop::open_store_folder() {
                    Ok(()) => "Opening the folder.".to_string(),
                    Err(why) => why,
                });
                false
            }
            Entry::Agent => {
                // The agent lives in the margin; from the brief, the wall has to
                // come back first for the column to exist.
                ctx.set_view(View::Wall);
                ctx.set_margin_mode(MarginMode::Agent);
                false
            }
            Entry::Quit => true,
        }
    }
}

#[component]
pub fn MenuStrip() -> Element {
    let ctx = use_context::<AppCtx>();
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| 0usize);
    let where_it_lives = crate::menu::store_location();

    let mut close = move || {
        open.set(false);
        active.set(0);
        crate::storage::focus_id("menu-button");
    };

    let mut choose = move |entry: Entry| {
        if entry.run(ctx) {
            // A native window has no other way out, so quit must flush the store
            // first: the saver loop may still hold the last keystroke.
            let snapshot = ctx.doc.peek().clone();
            spawn(async move {
                crate::storage::save_now(&snapshot).await;
                crate::desktop::quit();
            });
        } else {
            close();
        }
    };

    rsx! {
        nav { class: "menubar", role: "menubar", aria_label: "Horizon",
            button {
                id: "menu-button",
                class: "menu-trigger",
                r#type: "button",
                role: "menuitem",
                aria_haspopup: "menu",
                aria_expanded: if open() { "true" } else { "false" },
                onclick: move |_| {
                    if open() {
                        close();
                    } else {
                        open.set(true);
                    }
                },
                onkeydown: move |evt| match evt.key() {
                    Key::F10 => {
                        open.set(!open());
                        evt.prevent_default();
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("h") && evt.modifiers().alt() => {
                        open.set(true);
                        evt.prevent_default();
                    }
                    _ => {}
                },
                "Horizon"
            }
            if open() {
                div {
                    id: "menu-sheet",
                    class: "menu-sheet",
                    role: "menu",
                    tabindex: "-1",
                    onmounted: move |_| crate::storage::focus_id("menu-sheet"),
                    // The sheet is one focus stop; arrows move the highlight and
                    // Enter runs it, which keeps the native keyboard contract.
                    onkeydown: move |evt| {
                        let last = Entry::ALL.len() - 1;
                        match evt.key() {
                            Key::ArrowDown => {
                                active.set((active() + 1).min(last));
                                evt.prevent_default();
                            }
                            Key::ArrowUp => {
                                active.set(active().saturating_sub(1));
                                evt.prevent_default();
                            }
                            Key::Home => {
                                active.set(0);
                                evt.prevent_default();
                            }
                            Key::End => {
                                active.set(last);
                                evt.prevent_default();
                            }
                            Key::Enter => {
                                choose(Entry::ALL[active()]);
                                evt.prevent_default();
                            }
                            Key::Escape => {
                                close();
                                evt.prevent_default();
                            }
                            _ => {}
                        }
                    },
                    div { class: "menu-where",
                        lucide::FolderClosed { size: 13 }
                        span { "{where_it_lives}" }
                    }
                    for (index, entry) in Entry::ALL.iter().copied().enumerate() {
                        if entry == Entry::Quit {
                            hr { class: "menu-rule" }
                        }
                        button {
                            class: "menu-entry",
                            class: if index == active() { "is-active" },
                            role: "menuitem",
                            r#type: "button",
                            tabindex: "-1",
                            onmouseenter: move |_| active.set(index),
                            onclick: move |_| choose(entry),
                            span { class: "menu-name", "{entry.label()}" }
                        }
                    }
                }
            }
        }
    }
}
