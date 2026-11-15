use dioxus::prelude::*;
use dioxus_icons::lucide::{
    Activity, Banknote, BookOpen, Briefcase, Church, FileText, Flag, Gift, HeartPulse, House,
    MapPin, Moon, Phone, Plane, Square, StickyNote, Users, Utensils,
};
use horizon_core::{Mark, Subject};

use crate::state::AppCtx;

#[component]
pub fn MarkGlyph(
    mark: Mark,
    #[props(default)] text: String,
    #[props(default)] subject: Option<Subject>,
    #[props(default)] note: bool,
    #[props(default)] projected: bool,
) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let mut class = String::from("mk");
    if note {
        class.push_str(" is-note");
    }
    if projected {
        class.push_str(" is-projected");
    }
    let name = mark.name().to_string();
    let subject_name = subject.map(|s| s.name().to_string()).unwrap_or_default();

    rsx! {
        span {
            class: "{class}",
            aria_hidden: "true",
            onmouseenter: {
                let text = text.clone();
                let name = name.clone();
                let subject_name = subject_name.clone();
                move |evt: MouseEvent| {
                    if text.is_empty() {
                        return;
                    }
                    let p = evt.client_coordinates();
                    ctx.ui.write().tip = Some(crate::state::Tip {
                        name: name.clone(),
                        subject: subject_name.clone(),
                        text: text.clone(),
                        x: p.x,
                        y: p.y,
                    });
                }
            },
            onmouseleave: move |_| {
                ctx.ui.write().tip = None;
            },
            MarkIcon { mark }
        }
    }
}

/// One Lucide icon per semantic mark. The shared `.mk svg` CSS keeps the
/// stroke weight, square caps and no-fill treatment consistent with Horizon.
#[component]
fn MarkIcon(mark: Mark) -> Element {
    match mark {
        Mark::Travel => rsx! { Plane {} },
        Mark::Health => rsx! { HeartPulse {} },
        Mark::People => rsx! { Users {} },
        Mark::Money => rsx! { Banknote {} },
        Mark::Work => rsx! { Briefcase {} },
        Mark::Home => rsx! { House {} },
        Mark::Ceremony => rsx! { Church {} },
        Mark::Admin => rsx! { FileText {} },
        Mark::Study => rsx! { BookOpen {} },
        Mark::Body => rsx! { Activity {} },
        Mark::Food => rsx! { Utensils {} },
        Mark::Gift => rsx! { Gift {} },
        Mark::Place => rsx! { MapPin {} },
        Mark::Call => rsx! { Phone {} },
        Mark::Deadline => rsx! { Flag {} },
        Mark::Rest => rsx! { Moon {} },
        Mark::Note => rsx! { StickyNote {} },
        Mark::Thing => rsx! { Square {} },
    }
}

/// A mark used inside a button: the button carries the label, the glyph stays decorative.
#[component]
pub fn MarkButtonGlyph(
    mark: Mark,
    #[props(default)] note: bool,
    #[props(default)] projected: bool,
) -> Element {
    rsx! { MarkGlyph { mark, note, projected } }
}
