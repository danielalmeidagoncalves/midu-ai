//! The one screen before there is a vault.
//!
//! Horizon has no account and no database, so the first launch has nothing to
//! open. The owner names the folder instead of the app guessing one, because that
//! folder is also the git repository that will sync. Nothing renders behind this
//! page: there is no doc to draw and no save to lose.

use dioxus::prelude::*;

use crate::storage;

/// The folder field and its two ways out: use the typed path, or ask the
/// platform for a folder. `typed` is bound to the field, so a dialog choice can
/// still be corrected by hand before it is used.
#[component]
pub fn VaultChoice(mut typed: Signal<String>, mut error: Signal<Option<String>>) -> Element {
    let suggested = storage::default_vault()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let typed_value = typed();
    let shown = if typed_value.is_empty() {
        suggested.clone()
    } else {
        typed_value
    };

    rsx! {
        section { class: "boot-vault", aria_labelledby: "vaultTitle",
            div { class: "boot-vault-inner",
                p { class: "kicker", "Horizon · first run" }
                h1 { id: "vaultTitle", "Where should the Markdown live?" }
                p { class: "lede",
                    "Every entry, note and day page is a Markdown file in one folder. Choose the "
                    "folder once; it also becomes the git repository that syncs between this "
                    "machine and another one."
                }
                label { r#for: "vault-path", class: "import-label", "Folder" }
                input {
                    id: "vault-path",
                    class: "import-box sync-input",
                    r#type: "text",
                    spellcheck: "false",
                    autocomplete: "off",
                    value: "{shown}",
                    oninput: move |evt| typed.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter {
                            evt.prevent_default();
                            let path = typed();
                            let path = if path.is_empty() { suggested.clone() } else { path };
                            use_folder(&path, &mut error);
                        }
                    },
                }
                div { class: "export-row",
                    {
                        let use_path = shown.clone();
                        rsx! {
                            button {
                                class: "chip",
                                r#type: "button",
                                onclick: move |_| use_folder(&use_path, &mut error),
                                "Use this folder"
                            }
                        }
                    }
                    {
                        let ask_path = shown.clone();
                        rsx! {
                            button {
                                class: "chip",
                                r#type: "button",
                                onclick: move |_| ask(&ask_path, &mut typed, &mut error),
                                "Choose a folder"
                            }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { class: "sync-error", role: "alert", "{message}" }
                }
                p { class: "sync-where",
                    "An existing Horizon folder opens as it is; an empty or new one starts an "
                    "empty quarter."
                }
            }
        }
    }
}

/// Open the system folder picker.
fn ask(shown: &str, typed: &mut Signal<String>, error: &mut Signal<Option<String>>) {
    if let Some(dir) = crate::desktop::ask_folder(shown) {
        typed.set(dir);
        error.set(None);
    }
}

/// Remember the folder and restart the page against it. A reload is the simplest
/// way to start the store, the saver and the sync thread on the new vault, and a
/// first run has nothing worth keeping in memory.
fn use_folder(typed: &str, error: &mut Signal<Option<String>>) {
    let typed = if typed.trim().is_empty() {
        match storage::default_vault() {
            Some(path) => path.display().to_string(),
            None => {
                error.set(Some("Type a folder to use.".into()));
                return;
            }
        }
    } else {
        typed.to_string()
    };
    match storage::choose_vault(&storage::expand_home(&typed)) {
        Ok(()) => storage::reload(),
        Err(message) => error.set(Some(message)),
    }
}

/// The page shown while there is no vault. It has no masthead on purpose: the
/// composer above an empty document would be the one control that cannot work.
#[component]
pub fn VaultBoot() -> Element {
    let typed = use_signal(String::new);
    let error = use_signal(|| None::<String>);
    rsx! {
        div { class: "app",
            main { class: "boot-page",
                VaultChoice { typed, error }
            }
        }
    }
}
