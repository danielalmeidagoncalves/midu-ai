use chrono::Local;
use dioxus::prelude::*;
use horizon_core::{dates::Quarter, Doc};

#[cfg(desktop_shell)]
use crate::components::MenuStrip;
use crate::components::{agenda, brief, chrome, index, margin, wall};
use crate::state::{AppCtx, Ui, View};
use crate::storage;
#[cfg(feature = "sync")]
use crate::sync::SyncState;

const CSS: &str = include_str!("../assets/main.css");

/// Ctrl+Q on Linux, Cmd+Q on macOS: the shortcut the platform taught people
/// for "leave". Returns what to do *after* the store is flushed. The phone has
/// no process to close and no keyboard shortcut surface, so it saves and stops.
fn quit_shortcut(evt: &KeyboardEvent) -> Option<impl Fn() + 'static + use<>> {
    let command = if cfg!(target_os = "macos") {
        evt.modifiers().meta()
    } else {
        evt.modifiers().ctrl()
    };
    let pressed =
        command && matches!(evt.key(), Key::Character(ref c) if c.eq_ignore_ascii_case("q"));
    pressed.then(leave)
}

#[cfg(desktop_shell)]
fn leave() -> impl Fn() + 'static + use<> {
    crate::desktop::quit
}

#[cfg(not(desktop_shell))]
fn leave() -> impl Fn() + 'static + use<> {
    || {}
}

/// The in-window menu. Only the desktop shell renders it; the phone does not.
#[cfg(desktop_shell)]
fn menubar() -> Element {
    rsx! { MenuStrip {} }
}

#[cfg(not(desktop_shell))]
fn menubar() -> Element {
    rsx! {}
}

/// Whether the very first Tab press should land on the brief rather than the
/// composer: a small navigation aid, and the only way a keyboard reaches a
/// block below the wall. It never takes focus away from the composer again.
fn shift_tab_first() -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
    static FIRST: AtomicBool = AtomicBool::new(true);
    FIRST.swap(false, Ordering::Relaxed)
}

/// The one-line shortcut reminder under the page, where a system menubar would
/// otherwise teach it.
#[cfg(desktop_shell)]
fn menu_hint() -> Element {
    rsx! {
        span { class: "menuhint", "{crate::menu::QUIT_HINT}" }
    }
}

#[cfg(not(desktop_shell))]
fn menu_hint() -> Element {
    rsx! {}
}

#[component]
pub fn App() -> Element {
    let mut today = use_signal(|| Local::now().date_naive());
    let mut quarter = use_signal(|| Quarter::new(*today.read()));
    let mut doc = use_signal(Doc::empty);
    let mut ui = use_signal(Ui::default);
    let mut loaded = use_signal(|| false);
    let mut narrow = use_signal(|| false);
    let dirty = use_signal(|| 0u64);
    let ctx = AppCtx {
        doc,
        ui,
        quarter,
        today,
        dirty,
    };
    use_context_provider(|| ctx);

    // Sync runs on its own thread; the UI only reads its snapshot.
    #[cfg(feature = "sync")]
    let mut sync_state = use_signal(SyncState::default);
    #[cfg(feature = "sync")]
    {
        use_hook(crate::sync::start);
        use_context_provider(|| sync_state);
        use_future(move || async move {
            loop {
                crate::state::sleep_ms(700).await;
                if let Some(service) = crate::sync::handle() {
                    let next = service.snapshot();
                    if *sync_state.peek() != next {
                        sync_state.set(next);
                    }
                }
            }
        });

        // A pull rewrote the folder. Reload before the next save, so the
        // in-memory document never overwrites the other device's writing.
        let mut seen_revision = use_signal(|| 0u64);
        use_effect(move || {
            if !loaded() {
                return;
            }
            let revision = sync_state.read().revision;
            if revision == seen_revision() {
                return;
            }
            seen_revision.set(revision);
            let today = *today.peek();
            spawn(async move {
                if let Ok(Some(fresh)) = storage::load(today).await {
                    doc.set(fresh);
                }
            });
        });
    }

    // The stylesheet is injected into the document head rather than rendered as a child of
    // the app root: the desktop webview and the mobile webview both treat a head stylesheet
    // as the page's own, and it survives the app's root diffing.
    use_future(move || async move {
        let css = serde_json::to_string(CSS).unwrap_or_else(|_| "\"\"".into());
        let script = format!(
            "(function(){{ const s = document.createElement('style'); s.setAttribute('data-horizon',''); s.textContent = {css}; document.head.appendChild(s); document.title = 'Horizon'; }})();"
        );
        let _ = document::eval(&script).await;
    });

    // One saver. Mutations bump `dirty`; this loop coalesces them and awaits each write, so
    // the Markdown store can never be left with an older document because two writes raced.
    use_future(move || async move {
        let mut seen = 0u64;
        loop {
            crate::state::sleep_ms(20).await;
            let tick = *dirty.peek();
            if tick != seen {
                seen = tick;
                let snapshot = doc.read().clone();
                storage::save_now(&snapshot).await;
                #[cfg(feature = "sync")]
                if let Some(service) = crate::sync::handle() {
                    service.touch();
                }
            }
        }
    });

    // One shape at a time. Rendering both the wall and the agenda would duplicate the fold
    // and every entry id; the CSS breakpoint and this signal share the same 700px threshold.
    use_future(move || async move {
        let mut eval = document::eval(
            "dioxus.send(window.innerWidth <= 700);\
             window.addEventListener('resize', () => dioxus.send(window.innerWidth <= 700));",
        );
        while let Ok(value) = eval.recv::<bool>().await {
            narrow.set(value);
        }
    });

    // Load once. The live store is a Markdown folder; an old JSON document is
    // migrated into it on first run.
    use_future(move || async move {
        let today = *today.read();
        match storage::load(today).await {
            Ok(Some(loaded_doc)) => doc.set(loaded_doc),
            Ok(None) => {
                let demo = std::env::var("HORIZON_DEMO").ok().as_deref() == Some("1");
                let first = if demo {
                    horizon_core::seed::demo(today)
                } else {
                    Doc::empty()
                };
                doc.set(first);
                let snapshot = doc.read().clone();
                storage::save_now(&snapshot).await;
                #[cfg(feature = "sync")]
                if let Some(service) = crate::sync::handle() {
                    service.touch();
                }
            }
            Err(err) => {
                ui.write().boot_error = Some(err);
            }
        }
        loaded.set(true);
    });

    // The window rolls forward. The open day survives if it is still inside; if it has
    // left the window the fold closes, but its data is never removed.
    use_future(move || async move {
        loop {
            crate::state::sleep_ms(60_000).await;
            let now = Local::now().date_naive();
            if now != *today.read() {
                today.set(now);
                let next = Quarter::new(now);
                quarter.set(next.clone());
                {
                    let mut d = doc.write();
                    if let Some(open) = d.ui.open_day.clone() {
                        if !next.contains_key(&open) {
                            d.ui.open_day = None;
                        }
                    }
                }
                let snapshot = doc.read().clone();
                storage::save_now(&snapshot).await;
                #[cfg(feature = "sync")]
                if let Some(service) = crate::sync::handle() {
                    service.touch();
                }
            }
        }
    });

    // Focus follows the fold after the DOM has updated: a newly opened or moved fold takes
    // focus; a closed fold returns focus to the same day in the shape that is on screen.
    let mut last_open = use_signal(|| None::<String>);
    use_effect(move || {
        let open = doc.read().ui.open_day.clone();
        let previous = last_open();
        if open != previous {
            match (&open, &previous) {
                (Some(_), _) => storage::focus_id("fold"),
                (None, Some(key)) => storage::focus_day(key),
                _ => {}
            }
            last_open.set(open);
        }
    });

    // The window title is set once; the webview's default is the crate name.
    use_future(move || async move {
        let _ = document::eval("document.title = 'Horizon';").await;
    });

    // No vault yet: there is no document to draw and nowhere to save it, so the
    // app shows the one screen that can fix that and nothing else.
    if !crate::storage::vault_is_set() {
        return rsx! {
            crate::boot::VaultBoot {}
        };
    }

    let view = ui.read().view;
    let app_class = if view == View::Brief {
        "app view-brief"
    } else {
        "app"
    };

    let _shift_tab = shift_tab_first();

    rsx! {
        div {
            class: "{app_class}",
            onkeydown: move |evt| {
                if let Some(leave) = quit_shortcut(&evt) {
                    let snapshot = doc.peek().clone();
                    spawn(async move {
                        storage::save_now(&snapshot).await;
                        leave();
                    });
                    evt.prevent_default();
                } else if evt.key() == Key::Escape {
                    let armed = ui.read().arm_note.is_some();
                    if armed {
                        ui.write().arm_note = None;
                        ctx.flash("Clipped nothing.");
                        evt.prevent_default();
                    } else if ui.read().wide_note.is_some() {
                        ctx.close_wide_note();
                        evt.prevent_default();
                    } else if doc.read().ui.open_day.is_some() {
                        ctx.close_fold(None);
                        evt.prevent_default();
                    }
                }
            },
            if !loaded() {
                div { class: "boot", chrome::HorizonMark {}, "Horizon" }
            } else {
                {menubar()}
                chrome::Masthead {}
                chrome::Subhead {}
                if view == View::Brief {
                    brief::Brief {}
                } else {
                    index::IndexBand {}
                    if narrow() {
                        margin::WideNoteBand {}
                        agenda::Agenda {}
                        div { class: "body", margin::Margins {} }
                    } else {
                        div { class: "body",
                            div { class: "wall-wrap",
                                margin::WideNoteBand {}
                                wall::Wall {}
                            }
                            margin::Margins {}
                        }
                    }
                }
                chrome::Footer {}
                chrome::Tooltip {}
            }
            {menu_hint()}
        }
    }
}
