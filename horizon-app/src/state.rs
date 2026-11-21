//! Shared UI state. Durable state lives in `Doc`; this is the ephemeral layer.

use chrono::{Duration, Local, NaiveDate, Utc};
use dioxus::prelude::*;
use horizon_core::{
    dates::{self, Quarter},
    export, model, reminders as core_reminders, Doc, DrawMode, NoteTarget, Reminder,
    ReminderSource, Subject,
};

use crate::storage;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Wall,
    Brief,
}

/// What the right-hand column is showing. Notes and the agent share the same
/// address: the margin is where details wait, and the agent is another kind of
/// detail — a conversation that reads the whole quarter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarginMode {
    #[default]
    Notes,
    Agent,
}

#[derive(Clone, Debug)]
pub struct Tip {
    pub name: String,
    pub subject: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// The one-line notice a fired reminder shows under the masthead. It carries
/// the reminder's id so its own buttons can act on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub text: String,
    pub reminder: Option<String>,
}

/// What a reminder is being made from. The panel and the strip share it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReminderTarget {
    Bare,
    Note(String),
    Entry(String),
}

impl ReminderTarget {
    pub fn key(&self) -> String {
        match self {
            ReminderTarget::Bare => "bare".to_string(),
            ReminderTarget::Note(id) => format!("note:{id}"),
            ReminderTarget::Entry(id) => format!("entry:{id}"),
        }
    }

    pub fn source(&self) -> ReminderSource {
        match self {
            ReminderTarget::Bare => ReminderSource::Bare,
            ReminderTarget::Note(id) => ReminderSource::Note(id.clone()),
            ReminderTarget::Entry(id) => ReminderSource::Entry(id.clone()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ui {
    pub view: View,
    pub expanded: Option<String>,
    pub mark_picker: Option<String>,
    pub subject_picker: Option<String>,
    pub arm_note: Option<String>,
    /// The margin's subject filter; `None` shows every subject's groups.
    pub margin_filter: Option<Subject>,
    /// The loose note whose editor is open, if any.
    pub margin_open: Option<String>,
    /// The loose note opened wide as a full-width band above the wall.
    pub wide_note: Option<String>,
    /// Which reading the margin column carries: loose notes or the agent.
    pub margin_mode: MarginMode,
    /// The agent opened as the full-width, centred conversation band.
    pub agent_wide: bool,
    /// Whether the reminder strip is showing its rows.
    pub reminder_open: bool,
    /// The note, entry or bare row whose reminder panel is open, by `target.key()`.
    pub reminder_panel: Option<String>,
    /// A reminder that just fired, until it is dismissed or times out.
    pub toast: Option<Toast>,
    pub show_quiet: bool,
    pub tip: Option<Tip>,
    pub status: String,
    pub import_text: String,
    pub boot_error: Option<String>,
}

#[derive(Clone, Copy)]
pub struct AppCtx {
    pub doc: Signal<Doc>,
    pub ui: Signal<Ui>,
    pub quarter: Signal<Quarter>,
    pub today: Signal<NaiveDate>,
    /// Bumped by every mutation. One saver loop in `App` writes the latest document.
    pub dirty: Signal<u64>,
}

impl AppCtx {
    pub fn today(&self) -> NaiveDate {
        *self.today.read()
    }

    pub fn save(&self) {
        let mut dirty = self.dirty;
        dirty += 1;
    }

    pub fn flash(&self, message: impl Into<String>) {
        let message = message.into();
        let mut ui = self.ui;
        ui.write().status = message.clone();
        spawn(async move {
            sleep_ms(4600).await;
            if ui.peek().status == message {
                ui.write().status.clear();
            }
        });
    }

    pub fn open_day(&self, key: &str) {
        let mut doc = self.doc;
        {
            let mut d = doc.write();
            d.ui.open_day = Some(key.to_string());
        }
        let mut ui = self.ui;
        ui.write().expanded = None;
        ui.write().mark_picker = None;
        ui.write().subject_picker = None;
        self.save();
        storage::scroll_id(&format!("cell-{key}"));
    }

    pub fn close_fold(&self, focus_key: Option<&str>) {
        let key = self.doc.read().ui.open_day.clone();
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            d.ui.open_day = None;
        }
        let mut ui = self.ui;
        ui.write().expanded = None;
        ui.write().mark_picker = None;
        ui.write().subject_picker = None;
        self.save();
        let _ = focus_key.or(key.as_deref());
    }

    pub fn move_fold(&self, delta: i64) {
        let current = self.doc.read().ui.open_day.clone();
        let Some(current) = current else {
            return;
        };
        let quarter = self.quarter.read();
        let Some(index) = quarter.index_of(&current) else {
            return;
        };
        let next = (index as i64 + delta).clamp(0, quarter.days.len() as i64 - 1) as usize;
        let key = quarter.days[next].key.clone();
        drop(quarter);
        self.open_day(&key);
    }

    pub fn set_lens(&self, lens: Option<Subject>) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            d.ui.lens = lens;
            d.ui.open_day = None;
        }
        let mut ui = self.ui;
        ui.write().expanded = None;
        ui.write().mark_picker = None;
        ui.write().subject_picker = None;
        self.save();
        match lens {
            Some(s) => self.flash(format!("Reading the quarter through {}.", s.name())),
            None => self.flash("Reading the whole quarter."),
        }
    }

    pub fn set_draw_mode(&self, mode: DrawMode) {
        {
            let mut doc = self.doc;
            doc.write().ui.draw_mode = mode;
        }
        self.save();
        self.flash(match mode {
            DrawMode::Marks => "The quarter in marks.",
            DrawMode::Words => "The quarter in words.",
        });
    }

    pub fn set_view(&self, view: View) {
        let mut ui = self.ui;
        ui.write().view = view;
        ui.write().tip = None;
    }

    pub fn toggle_expanded(&self, id: &str) {
        let mut ui = self.ui;
        let current = ui.read().expanded.clone();
        let next = if current.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
        ui.write().expanded = next;
        ui.write().mark_picker = None;
        ui.write().subject_picker = None;
    }

    pub fn toggle_margin_open(&self, note_id: &str) {
        let mut ui = self.ui;
        let current = ui.read().margin_open.clone();
        let next = if current.as_deref() == Some(note_id) {
            None
        } else {
            Some(note_id.to_string())
        };
        ui.write().margin_open = next;
        ui.write().subject_picker = None;
    }

    pub fn toggle_mark_picker(&self, id: &str) {
        let mut ui = self.ui;
        let current = ui.read().mark_picker.clone();
        ui.write().mark_picker = if current.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
        ui.write().subject_picker = None;
        ui.write().expanded = Some(id.to_string());
    }

    pub fn toggle_subject_picker(&self, id: &str) {
        let mut ui = self.ui;
        let current = ui.read().subject_picker.clone();
        ui.write().subject_picker = if current.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
        ui.write().mark_picker = None;
        ui.write().expanded = Some(id.to_string());
    }

    pub fn set_entry_mark(&self, id: &str, mark: horizon_core::Mark) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.mark = mark;
                e.mark_auto = false;
            }
        }
        let mut ui = self.ui;
        ui.write().mark_picker = None;
        self.save();
        storage::focus_id(&format!("entry-{id}-mark"));
    }

    pub fn set_entry_subject(&self, id: &str, subject: Subject) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.subject = subject;
                e.subject_auto = false;
            }
        }
        let mut ui = self.ui;
        ui.write().subject_picker = None;
        self.save();
        storage::focus_id(&format!("entry-{id}-subject"));
        self.flash(format!("Filed under {}.", subject.name()));
    }

    pub fn reset_entry_mark(&self, id: &str) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.mark_auto = true;
                e.mark = horizon_core::infer_mark(&e.text);
            }
        }
        self.save();
        storage::focus_id(&format!("entry-{id}-mark"));
    }

    pub fn reset_entry_subject(&self, id: &str) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.subject_auto = true;
                e.subject = horizon_core::infer_subject(&e.text);
            }
        }
        self.save();
        storage::focus_id(&format!("entry-{id}-subject"));
    }

    pub fn set_entry_repeat(&self, id: &str, repeat: bool) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.repeat_weekly = repeat;
            }
        }
        self.save();
        storage::focus_id(&format!("entry-{id}-repeat"));
    }

    pub fn set_entry_weight(&self, id: &str, weight: u8) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.weight = weight.clamp(1, 3);
            }
        }
        self.save();
        storage::focus_id(&format!("entry-{id}-w{weight}"));
    }

    pub fn set_entry_done(&self, id: &str, done: bool) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.done = done;
            }
        }
        self.save();
        storage::focus_id(&format!("entry-{id}-check"));
    }

    pub fn remove_entry(&self, id: &str) {
        let day = self
            .doc
            .read()
            .entry(id)
            .map(|e| e.day.clone())
            .unwrap_or_default();
        {
            let mut doc = self.doc;
            doc.write().remove_entry(id);
        }
        let mut ui = self.ui;
        ui.write().expanded = None;
        self.save();
        if let Some(d) = dates::parse_key(&day) {
            self.flash(format!("Removed from {}.", dates::short_day(d)));
        }
    }

    pub fn set_entry_text(&self, id: &str, text: &str) {
        let mut doc = self.doc;
        {
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.text = text.to_string();
                if e.mark_auto {
                    e.mark = horizon_core::infer_mark(text);
                }
                if e.subject_auto {
                    e.subject = horizon_core::infer_subject(text);
                }
            }
        }
        self.save();
    }

    pub fn set_entry_note(&self, id: &str, note: &str) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(e) = d.entry_mut(id) {
                e.set_note(note.to_string());
            }
        }
        self.save();
    }

    pub fn set_page(&self, day: &str, markdown: &str) {
        let today = self.today();
        {
            let mut doc = self.doc;
            doc.write().set_page(day, markdown, today);
        }
        self.save();
    }

    pub fn toggle_note_editing(&self, key: &str, editing: bool) {
        {
            let mut doc = self.doc;
            doc.write().set_note_editing(key, editing);
        }
        self.save();
    }

    pub fn toggle_task(&self, target: NoteTarget, line: usize) {
        {
            let mut doc = self.doc;
            doc.write().toggle_task(target, line, self.today());
        }
        self.save();
    }

    pub fn clip_note(&self, note_id: &str, day: &str) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            let lens = d.ui.lens;
            if let Some(n) = d.notes.iter_mut().find(|n| n.id == note_id) {
                n.day = Some(day.to_string());
                if let Some(lens) = lens {
                    n.subject = lens;
                }
            }
        }
        let mut ui = self.ui;
        ui.write().arm_note = None;
        // A clipped note leaves the margins; if it was open wide, the band closes
        // with it so the wall comes back.
        if ui.read().wide_note.as_deref() == Some(note_id) {
            ui.write().wide_note = None;
        }
        self.save();
        if let Some(d) = dates::parse_key(day) {
            self.flash(format!("Clipped to {}.", dates::short_day(d)));
        }
    }

    pub fn detach_note(&self, note_id: &str) {
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(n) = d.notes.iter_mut().find(|n| n.id == note_id) {
                n.day = None;
            }
        }
        self.save();
        self.flash("Detached — back in the margins.");
    }

    pub fn remove_note(&self, note_id: &str) {
        {
            let mut doc = self.doc;
            doc.write().remove_note(note_id);
        }
        let mut ui = self.ui;
        if ui.read().margin_open.as_deref() == Some(note_id) {
            ui.write().margin_open = None;
        }
        if ui.read().wide_note.as_deref() == Some(note_id) {
            ui.write().wide_note = None;
        }
        if ui.read().arm_note.as_deref() == Some(note_id) {
            ui.write().arm_note = None;
        }
        self.save();
    }

    /// Opens a loose note as the full-width band above the wall, already in
    /// write mode — the wide space is for writing. The margin's in-place view
    /// of the same note steps aside so the editor id exists once.
    pub fn open_wide_note(&self, note_id: &str) {
        {
            let mut doc = self.doc;
            doc.write()
                .set_note_editing(&model::note_key_loose(note_id), true);
        }
        let mut ui = self.ui;
        // The margin's in-place view of the same note steps aside: the
        // editor and its textarea id exist once, in the band.
        if ui.read().margin_open.as_deref() == Some(note_id) {
            ui.write().margin_open = None;
        }
        ui.write().wide_note = Some(note_id.to_string());
        self.save();
    }

    /// Closes the wide band and returns focus to the note it came from.
    pub fn close_wide_note(&self) {
        let mut ui = self.ui;
        let was = ui.read().wide_note.clone();
        ui.write().wide_note = None;
        if let Some(id) = was {
            crate::storage::focus_id(&format!("note-{id}-sum"));
        }
    }

    pub fn add_note(&self, raw: &str, subject_override: Option<Subject>) {
        // The margin shares the composer's grammar: a trailing `/subject` wins
        // over the picker, then the picker, then the lens, then the words.
        let (text, token) = horizon_core::composer::split_subject(raw);
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let subject = match token {
            Some(Ok(s)) => Some(s),
            Some(Err(bad)) => {
                self.flash(format!("No subject called “{bad}”."));
                return;
            }
            None => subject_override.or(self.doc.read().ui.lens),
        };
        {
            let mut doc = self.doc;
            doc.write()
                .notes
                .insert(0, horizon_core::LooseNote::new(text, subject));
        }
        self.save();
        self.flash("Added to the margins — no day needed.");
    }

    pub fn set_note_subject(&self, note_id: &str, subject: Subject) {
        {
            let mut doc = self.doc;
            doc.write().set_note_subject(note_id, subject);
        }
        self.save();
        self.flash(format!("Filed under {}.", subject.name()));
    }

    pub fn set_margin_filter(&self, subject: Option<Subject>) {
        let mut ui = self.ui;
        ui.write().margin_filter = subject;
    }

    /// Open or close the reminder strip's rows.
    pub fn toggle_reminders(&self) {
        let mut ui = self.ui;
        let open = ui.read().reminder_open;
        ui.write().reminder_open = !open;
        if open {
            ui.write().reminder_panel = None;
        }
    }

    /// Open or close the reminder panel for one note, entry or bare row.
    pub fn toggle_reminder_panel(&self, key: &str) {
        let mut ui = self.ui;
        let current = ui.read().reminder_panel.clone();
        let next = if current.as_deref() == Some(key) {
            None
        } else {
            Some(key.to_string())
        };
        ui.write().reminder_panel = next;
    }

    /// The text a reminder inherits when the panel opens: the note's first line
    /// or the entry's words. A bare reminder starts empty.
    pub fn reminder_hint(&self, target: &ReminderTarget) -> String {
        let doc = self.doc.read();
        match target {
            ReminderTarget::Bare => String::new(),
            ReminderTarget::Note(id) => doc
                .note(id)
                .map(|note| note.text.lines().next().unwrap_or("").trim().to_string())
                .unwrap_or_default(),
            ReminderTarget::Entry(id) => doc
                .entry(id)
                .map(|entry| entry.text.clone())
                .unwrap_or_default(),
        }
    }

    /// Make a reminder. `when` is the one-line grammar; `text` is the words to
    /// show, falling back to the note or entry it came from.
    pub fn set_reminder(
        &self,
        target: ReminderTarget,
        text: &str,
        when: &str,
    ) -> Result<(), String> {
        let now = Local::now();
        let parsed = core_reminders::parse_when(when, now).map_err(|e| e.message())?;
        let text = if text.trim().is_empty() {
            self.reminder_hint(&target)
        } else {
            text.trim().to_string()
        };
        if text.is_empty() {
            return Err("Add a word or two for the reminder.".into());
        }
        let reminder = Reminder {
            text,
            next_at: parsed.at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            repeat: parsed.repeat,
            anchor_day: parsed.anchor_day,
            source: target.source(),
            created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ..Reminder::default()
        };
        {
            let mut doc = self.doc;
            doc.write().reminders.push(reminder);
        }
        {
            let mut ui = self.ui;
            ui.write().reminder_panel = None;
        }
        self.save();
        self.flash(format!(
            "Reminder set — {}.",
            core_reminders::humanize(Some(parsed.at), now)
        ));
        Ok(())
    }

    /// Move a reminder by a fixed amount. The repeat is untouched.
    pub fn snooze_reminder(&self, id: &str, by: Duration) {
        let now = Utc::now();
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(reminder) = d.reminder_mut(id) {
                core_reminders::snooze(reminder, now, by);
            }
        }
        self.save();
        self.dismiss_toast();
        self.flash("Snoozed.");
    }

    /// The reminder's own `Done`: a one-off closes, a repeat moves on.
    pub fn finish_reminder(&self, id: &str) {
        let now = Utc::now();
        let mut finished = false;
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(reminder) = d.reminder_mut(id) {
                if reminder.repeat.is_once() {
                    reminder.done = true;
                    finished = true;
                } else {
                    core_reminders::advance(reminder, now);
                }
            }
        }
        self.save();
        self.dismiss_toast();
        if finished {
            self.flash("Reminder done.");
        }
    }

    pub fn remove_reminder(&self, id: &str) {
        {
            let mut doc = self.doc;
            doc.write().remove_reminder(id);
        }
        self.save();
        self.flash("Reminder removed.");
    }

    /// A reminder came due while the window was open: notify once, advance it,
    /// and leave the toast. Catch-up is handled by the caller, never here.
    pub fn fire_reminder(&self, id: &str) {
        let now = Utc::now();
        let mut fired = None;
        {
            let mut doc = self.doc;
            let mut d = doc.write();
            if let Some(reminder) = d.reminder_mut(id) {
                if reminder.is_due(now) {
                    fired = Some((
                        reminder.text.clone(),
                        core_reminders::notification_body(reminder),
                    ));
                    core_reminders::advance(reminder, now);
                }
            }
        }
        let Some((text, body)) = fired else {
            return;
        };
        self.save();
        crate::reminders::notify(&body);
        {
            let mut ui = self.ui;
            ui.write().toast = Some(Toast {
                text,
                reminder: Some(id.to_string()),
            });
        }
        let mut ui = self.ui;
        let id = id.to_string();
        spawn(async move {
            sleep_ms(20_000).await;
            if ui.peek().toast.as_ref().and_then(|t| t.reminder.as_deref()) == Some(id.as_str()) {
                ui.write().toast = None;
            }
        });
    }

    pub fn dismiss_toast(&self) {
        let mut ui = self.ui;
        ui.write().toast = None;
    }

    /// Switch the right-hand column. The agent's input exists only after the
    /// column renders, so focus waits a frame rather than asking the DOM now.
    pub fn set_margin_mode(&self, mode: MarginMode) {
        {
            let mut ui = self.ui;
            if ui.read().margin_mode != mode {
                ui.write().margin_mode = mode;
                ui.write().subject_picker = None;
                // The band belongs to the agent reading; leaving the reading
                // closes it rather than leaving a conversation with no column.
                if mode != MarginMode::Agent {
                    ui.write().agent_wide = false;
                }
            }
        }
        if mode == MarginMode::Agent {
            crate::storage::focus_id("agent-input");
        }
    }

    /// Open the agent wide: the same move as a loose note's `Write wide`, and the
    /// same shape (a band above the wall), but the band is a centred column of
    /// conversation rather than a note editor.
    pub fn open_agent_wide(&self) {
        {
            let mut ui = self.ui;
            ui.write().margin_mode = MarginMode::Agent;
            ui.write().agent_wide = true;
        }
        crate::storage::focus_id("agent-input-wide");
    }

    /// Close the band and return focus to the column's Agent tab, so the
    /// keyboard path never lands on the body.
    pub fn close_agent_wide(&self) {
        let mut ui = self.ui;
        ui.write().agent_wide = false;
        crate::storage::focus_id("margin-mode-agent");
    }

    pub fn add_entry(&self, day: &str, title: &str, subject: Option<Subject>, explicit: bool) {
        let mut entry = horizon_core::Entry::new_with(day, title, subject, None);
        entry.subject_auto = !explicit;
        {
            let mut doc = self.doc;
            doc.write().entries.push(entry.clone());
        }
        self.save();
        if let Some(d) = dates::parse_key(day) {
            self.flash(format!(
                "Filed to {} · {}",
                dates::short_day(d),
                entry.subject.name()
            ));
        }
    }

    pub fn import_json(&self, text: &str) -> Result<(), String> {
        match export::from_json(text) {
            Ok(doc) => {
                let mut signal = self.doc;
                signal.set(doc);
                let mut ui = self.ui;
                ui.write().import_text.clear();
                self.save();
                self.flash("The quarter was restored from that document.");
                Ok(())
            }
            Err(err) => {
                self.flash(err.to_string());
                Err(err.to_string())
            }
        }
    }

    pub fn export_json(&self) {
        let doc = self.doc.read().clone();
        match export::to_json(&doc) {
            Ok(json) => {
                storage::download("horizon.json", "application/json", &json);
                self.flash("JSON download started.");
            }
            Err(err) => self.flash(format!("Could not write the JSON: {err}")),
        }
    }

    pub fn export_markdown(&self) {
        let doc = self.doc.read().clone();
        match storage::export_store_copy(&doc) {
            Ok(path) => self.flash(format!("Copy written to {path}")),
            Err(err) => self.flash(format!("Could not write the copy: {err}")),
        }
    }

    pub fn note_markdown(&self, target: &NoteTarget) -> String {
        match target {
            NoteTarget::Entry(id) => self
                .doc
                .read()
                .entry(id)
                .map(|e| e.note_text().to_string())
                .unwrap_or_default(),
            NoteTarget::Page(day) => self.doc.read().page(day).to_string(),
            NoteTarget::Loose(id) => self
                .doc
                .read()
                .note(id)
                .map(|n| n.text.clone())
                .unwrap_or_default(),
        }
    }

    pub fn note_editing(&self, target: &NoteTarget) -> bool {
        let doc = self.doc.read();
        let key = match target {
            NoteTarget::Entry(id) => model::note_key_entry(id),
            NoteTarget::Page(day) => model::note_key_page(day),
            NoteTarget::Loose(id) => model::note_key_loose(id),
        };
        doc.is_note_editing(&key, !self.note_markdown(target).trim().is_empty())
    }

    pub fn set_note(&self, target: &NoteTarget, text: &str) {
        match target {
            NoteTarget::Entry(id) => self.set_entry_note(id, text),
            NoteTarget::Page(day) => self.set_page(day, text),
            NoteTarget::Loose(id) => {
                {
                    let mut doc = self.doc;
                    doc.write().set_note_text(id, text.to_string());
                }
                self.save();
            }
        }
    }

    pub fn toggle_note_task(&self, target: &NoteTarget, line: usize) {
        self.toggle_task(target.clone(), line);
    }

    pub fn editing_key(target: &NoteTarget) -> String {
        match target {
            NoteTarget::Entry(id) => model::note_key_entry(id),
            NoteTarget::Page(day) => model::note_key_page(day),
            NoteTarget::Loose(id) => model::note_key_loose(id),
        }
    }

    pub fn composer_placeholder(&self) -> String {
        match self.doc.read().ui.lens {
            Some(s) => format!("Into {}", s.name()),
            None => "dec 3 - flight · tomorrow - call mum".into(),
        }
    }
}

/// A sleep for the app's async loops.
pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

/// Parse a composer line against the current window and lens.
pub fn parse_line(
    raw: &str,
    doc: &Doc,
    quarter: &Quarter,
) -> Result<horizon_core::ComposerResult, horizon_core::ComposerError> {
    horizon_core::parse_composer(raw, doc.ui.open_day.as_deref(), quarter, doc.ui.lens)
}

/// A live preview that never panics on an unknown subject token.
pub fn preview_line(raw: &str, lens: Option<Subject>) -> horizon_core::ComposerPreview {
    horizon_core::composer::preview(raw, lens)
}
