//! The git status line in the brief.
//!
//! Horizon does not run git — the owner commits, pushes and pulls by hand in
//! the terminal. The brief shows one line: what state the store repository is
//! in right now, so the owner knows whether there is anything to do.

use dioxus::prelude::*;

use crate::sync::SyncState;

#[component]
pub fn SyncBlock() -> Element {
    let sync = use_context::<Signal<SyncState>>();
    let state = sync();
    let line = crate::sync::status(&state);

    rsx! {
        h3 { "Sync" }
        div { class: "sync-status",
            span { class: "sync-line", title: "{line.detail}", "{line.label}" }
            if let Some(remote) = state.remote.clone() {
                span { class: "sync-meta", "{remote}" }
            }
        }
        p { class: "store-path",
            "The Markdown files are in "
            code { "{state.store}" }
        }
    }
}
