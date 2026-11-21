//! The document. One versioned JSON object, with every derived view kept out of storage.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::dates::{self, Quarter, SPAN_DAYS};
use crate::inference::{infer_mark, infer_subject, Mark, Subject};

pub const DOC_VERSION: u32 = 1;

/// §6 OPEN — recurrence. One toggle, no rule engine, no series management.
pub const WEEKLY_RECURRENCE: bool = true;
/// §6 OPEN — closed subjects. Quiet is the only state; no close action is built.
pub const CLOSED_SUBJECTS: bool = false;
/// §6 OPEN — plan vs. record. One page object, with a first-write marker.
pub const PLAN_MARKERS: bool = true;
/// §6 OPEN — looking backward. Forward only.
pub const LOOK_BACK: bool = false;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DrawMode {
    #[default]
    Marks,
    Words,
}

impl DrawMode {
    pub fn toggled(self) -> Self {
        match self {
            DrawMode::Marks => DrawMode::Words,
            DrawMode::Words => DrawMode::Marks,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Open,
    One,
    Full,
    Landmark,
}

impl Tier {
    pub fn from_weight(weight: u8) -> Self {
        match weight {
            0 => Tier::Open,
            1 => Tier::One,
            2 => Tier::Full,
            _ => Tier::Landmark,
        }
    }

    pub fn weight(self) -> u8 {
        match self {
            Tier::Open => 0,
            Tier::One => 1,
            Tier::Full => 2,
            Tier::Landmark => 3,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Tier::Open => "open",
            Tier::One => "one thing",
            Tier::Full => "full day",
            Tier::Landmark => "landmark",
        }
    }

    pub fn class(self) -> &'static str {
        match self {
            Tier::Open => "",
            Tier::One => "l1",
            Tier::Full => "l2",
            Tier::Landmark => "land",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub text: String,
    #[serde(default = "one")]
    pub weight: u8,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub mark: Mark,
    #[serde(default = "yes")]
    pub mark_auto: bool,
    #[serde(default)]
    pub subject: Subject,
    #[serde(default = "yes")]
    pub subject_auto: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: String,
    /// §6 OPEN recurrence default: every week, on this weekday. No exceptions, no editor.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeat_weekly: bool,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            id: new_id("e"),
            day: String::new(),
            text: String::new(),
            weight: 1,
            done: false,
            mark: Mark::Thing,
            mark_auto: true,
            subject: Subject::Me,
            subject_auto: true,
            note: None,
            created_at: String::new(),
            repeat_weekly: false,
        }
    }
}

impl Entry {
    pub fn new(day: &str, text: &str) -> Self {
        Self::new_with(day, text, None, None)
    }

    pub fn new_with(
        day: &str,
        text: &str,
        subject_override: Option<Subject>,
        now: Option<chrono::DateTime<Utc>>,
    ) -> Self {
        let subject = subject_override.unwrap_or_else(|| infer_subject(text));
        Self {
            id: new_id("e"),
            day: day.to_string(),
            text: text.to_string(),
            weight: 1,
            done: false,
            mark: infer_mark(text),
            mark_auto: true,
            subject,
            subject_auto: subject_override.is_none(),
            note: None,
            created_at: now
                .unwrap_or_else(Utc::now)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            repeat_weekly: false,
        }
    }

    pub fn weight(&self) -> u8 {
        self.weight.clamp(1, 3)
    }

    pub fn note_text(&self) -> &str {
        self.note.as_deref().unwrap_or("")
    }

    pub fn set_note(&mut self, value: String) {
        self.note = if value.trim().is_empty() {
            None
        } else {
            Some(value)
        };
    }

    pub fn has_note(&self) -> bool {
        self.note.as_deref().is_some_and(|n| !n.trim().is_empty())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LooseNote {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub mark: Mark,
    #[serde(default)]
    pub subject: Subject,
    /// RFC 3339, written once. Read order uses it; the filename never does.
    #[serde(default)]
    pub created_at: String,
}

impl Default for LooseNote {
    fn default() -> Self {
        Self {
            id: new_id("n"),
            text: String::new(),
            day: None,
            mark: Mark::Thing,
            subject: Subject::Me,
            created_at: String::new(),
        }
    }
}

impl LooseNote {
    pub fn new(text: &str, subject_override: Option<Subject>) -> Self {
        Self::new_at(text, subject_override, Utc::now())
    }

    pub fn new_at(
        text: &str,
        subject_override: Option<Subject>,
        now: chrono::DateTime<Utc>,
    ) -> Self {
        Self {
            id: new_id("n"),
            text: text.to_string(),
            day: None,
            mark: infer_mark(text),
            subject: subject_override.unwrap_or_else(|| infer_subject(text)),
            created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

/// How a reminder repeats. There is no rule engine and no end date: a repeat
/// runs until the owner switches it off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    Once,
    Daily,
    Weekly,
    /// The Nth of each month, clamped to the month's last day.
    MonthlyOnDay(u32),
}

impl Repeat {
    pub fn is_once(self) -> bool {
        matches!(self, Repeat::Once)
    }
}

/// What a reminder was made from. The text is copied at creation, so the link
/// is provenance, not a live join.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderSource {
    #[default]
    Bare,
    Note(String),
    Entry(String),
}

impl ReminderSource {
    pub fn chip(&self) -> &'static str {
        match self {
            ReminderSource::Bare => "reminder",
            ReminderSource::Note(_) => "note",
            ReminderSource::Entry(_) => "entry",
        }
    }

    /// The source id, if this reminder came from a note or an entry.
    pub fn target_id(&self) -> Option<&str> {
        match self {
            ReminderSource::Bare => None,
            ReminderSource::Note(id) | ReminderSource::Entry(id) => Some(id),
        }
    }
}

/// One reminder. It is its own Markdown file in `reminders/`, never a field on
/// an entry or a note, so a synced folder never rewrites a note to add a date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    /// RFC 3339 in UTC. Display is local; the monthly anchor is a local day.
    #[serde(default)]
    pub next_at: String,
    #[serde(default)]
    pub repeat: Repeat,
    /// The local day-of-month a monthly reminder keeps across short months.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_day: Option<u32>,
    #[serde(default)]
    pub source: ReminderSource,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fired: Option<String>,
}

impl Default for Reminder {
    fn default() -> Self {
        Self {
            id: new_id("r"),
            text: String::new(),
            next_at: String::new(),
            repeat: Repeat::Once,
            anchor_day: None,
            source: ReminderSource::Bare,
            done: false,
            created_at: String::new(),
            last_fired: None,
        }
    }
}

impl Reminder {
    /// The next instant, parsed. An unreadable stamp reads as `None` rather
    /// than pretending to be due.
    pub fn at_utc(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.next_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        !self.done && self.at_utc().is_some_and(|at| at <= now)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PageMeta {
    #[serde(default)]
    pub first_written: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub draw_mode: DrawMode,
    #[serde(default)]
    pub lens: Option<Subject>,
    #[serde(default)]
    pub open_day: Option<String>,
    /// `entry:<id>` or `page:<day>` → true when the editor (not the reading view) is showing.
    #[serde(default)]
    pub note_edit: BTreeMap<String, bool>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            draw_mode: DrawMode::Marks,
            lens: None,
            open_day: None,
            note_edit: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub notes: Vec<LooseNote>,
    #[serde(default)]
    pub pages: BTreeMap<String, String>,
    #[serde(default)]
    pub page_meta: BTreeMap<String, PageMeta>,
    #[serde(default)]
    pub reminders: Vec<Reminder>,
    #[serde(default)]
    pub ui: UiState,
}

fn version() -> u32 {
    DOC_VERSION
}
fn one() -> u8 {
    1
}
fn yes() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

pub fn note_key_entry(id: &str) -> String {
    format!("entry:{}", id)
}

pub fn note_key_page(day: &str) -> String {
    format!("page:{}", day)
}

pub fn note_key_loose(id: &str) -> String {
    format!("loose:{}", id)
}

impl Default for Doc {
    fn default() -> Self {
        Self::empty()
    }
}

impl Doc {
    pub fn empty() -> Self {
        Self {
            version: DOC_VERSION,
            entries: Vec::new(),
            notes: Vec::new(),
            pages: BTreeMap::new(),
            page_meta: BTreeMap::new(),
            reminders: Vec::new(),
            ui: UiState::default(),
        }
    }

    pub fn reminder(&self, id: &str) -> Option<&Reminder> {
        self.reminders.iter().find(|r| r.id == id)
    }

    pub fn reminder_mut(&mut self, id: &str) -> Option<&mut Reminder> {
        self.reminders.iter_mut().find(|r| r.id == id)
    }

    pub fn remove_reminder(&mut self, id: &str) {
        self.reminders.retain(|r| r.id != id);
    }

    /// Reminders made from a note or an entry. Used when that thing is removed.
    pub fn remove_reminders_for(&mut self, source: &ReminderSource) {
        self.reminders.retain(|r| &r.source != source);
    }

    /// Reminders whose next instant has arrived, soonest first.
    pub fn due_reminders(&self, now: DateTime<Utc>) -> Vec<&Reminder> {
        let mut due: Vec<&Reminder> = self.reminders.iter().filter(|r| r.is_due(now)).collect();
        due.sort_by(|a, b| a.next_at.cmp(&b.next_at));
        due
    }

    /// Every reminder still to come, soonest first. Done one-offs drop out.
    pub fn upcoming_reminders(&self) -> Vec<&Reminder> {
        let mut live: Vec<&Reminder> = self.reminders.iter().filter(|r| !r.done).collect();
        live.sort_by(|a, b| a.next_at.cmp(&b.next_at));
        live
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn entry_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn note(&self, id: &str) -> Option<&LooseNote> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn note_mut(&mut self, id: &str) -> Option<&mut LooseNote> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn remove_entry(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
        self.ui.note_edit.remove(&note_key_entry(id));
        // A reminder made from this entry has nothing left to point at.
        self.reminders
            .retain(|r| r.source != ReminderSource::Entry(id.to_string()));
    }

    pub fn remove_note(&mut self, id: &str) {
        self.notes.retain(|n| n.id != id);
        self.ui.note_edit.remove(&note_key_loose(id));
        self.reminders
            .retain(|r| r.source != ReminderSource::Note(id.to_string()));
    }

    /// Replaces an undated or clipped note's whole body. The text is stored as
    /// typed; an empty body is a legal empty note, not a deletion.
    pub fn set_note_text(&mut self, id: &str, text: String) {
        if let Some(n) = self.note_mut(id) {
            n.text = text;
        }
    }

    /// Moves a note to another subject. The mark is left alone: it describes the
    /// words, and a re-filed note keeps them.
    pub fn set_note_subject(&mut self, id: &str, subject: Subject) {
        if let Some(n) = self.note_mut(id) {
            n.subject = subject;
        }
    }

    pub fn entries_on<'a>(
        &'a self,
        day: &'a str,
        lens: Option<Subject>,
    ) -> impl Iterator<Item = &'a Entry> + 'a {
        self.entries
            .iter()
            .filter(move |e| e.day == day && lens.is_none_or(|s| e.subject == s))
    }

    pub fn clipped_notes_on<'a>(
        &'a self,
        day: &'a str,
        lens: Option<Subject>,
    ) -> impl Iterator<Item = &'a LooseNote> + 'a {
        self.notes
            .iter()
            .filter(move |n| n.day.as_deref() == Some(day) && lens.is_none_or(|s| n.subject == s))
    }

    pub fn loose_notes(&self) -> impl Iterator<Item = &LooseNote> {
        self.notes.iter().filter(|n| n.day.is_none())
    }

    pub fn weight(&self, day: &str, lens: Option<Subject>) -> u8 {
        self.entries_on(day, lens)
            .map(Entry::weight)
            .max()
            .unwrap_or(0)
    }

    pub fn tier(&self, day: &str, lens: Option<Subject>) -> Tier {
        Tier::from_weight(self.weight(day, lens))
    }

    pub fn page(&self, day: &str) -> &str {
        self.pages.get(day).map(String::as_str).unwrap_or("")
    }

    pub fn has_page(&self, day: &str) -> bool {
        !self.page(day).trim().is_empty()
    }

    pub fn set_page(&mut self, day: &str, markdown: &str, today: NaiveDate) {
        if markdown.trim().is_empty() {
            self.pages.remove(day);
            self.page_meta.remove(day);
            return;
        }
        self.pages.insert(day.to_string(), markdown.to_string());
        self.page_meta
            .entry(day.to_string())
            .or_insert_with(|| PageMeta {
                first_written: dates::key(today),
            });
    }

    pub fn page_written_ahead(&self, day: &str) -> bool {
        if !PLAN_MARKERS {
            return false;
        }
        let Some(meta) = self.page_meta.get(day) else {
            return false;
        };
        let Some(first) = dates::parse_key(&meta.first_written) else {
            return false;
        };
        let Some(d) = dates::parse_key(day) else {
            return false;
        };
        first < d && self.has_page(day)
    }

    pub fn page_is_past(&self, day: &str, today: NaiveDate) -> bool {
        dates::parse_key(day).is_some_and(|d| d < today) && self.has_page(day)
    }

    pub fn note_count(&self, day: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| e.day == day && e.has_note())
            .count()
    }

    pub fn has_content(&self, day: &str, lens: Option<Subject>) -> bool {
        let has_entries = self.entries_on(day, lens).next().is_some();
        let has_notes = self.clipped_notes_on(day, lens).next().is_some();
        if lens.is_some() {
            has_entries || has_notes
        } else {
            has_entries || has_notes || self.has_page(day)
        }
    }

    /// Everything printed in a day's marks: real entries, clipped details, then recurring
    /// outline texture. Weight is only ever carried by real entries.
    pub fn items_on(&self, day: &str, lens: Option<Subject>, window_start: NaiveDate) -> Vec<Item> {
        let mut items = Vec::new();
        for e in self.entries_on(day, lens) {
            items.push(Item {
                mark: e.mark,
                text: e.text.clone(),
                subject: e.subject,
                projected: false,
                loose: false,
                weight: e.weight(),
                entry_id: Some(e.id.clone()),
            });
        }
        for n in self.clipped_notes_on(day, lens) {
            items.push(Item {
                mark: n.mark,
                text: n.text.clone(),
                subject: n.subject,
                projected: false,
                loose: true,
                weight: 0,
                entry_id: None,
            });
        }
        for p in self.projected_on(day, lens, window_start) {
            items.push(Item {
                mark: p.mark,
                text: p.text,
                subject: p.subject,
                projected: true,
                loose: false,
                weight: 0,
                entry_id: p.entry_id,
            });
        }
        items.sort_by(|a, b| {
            b.weight
                .cmp(&a.weight)
                .then(a.projected.cmp(&b.projected))
                .then(a.loose.cmp(&b.loose))
        });
        items
    }

    fn projected_on(
        &self,
        day: &str,
        lens: Option<Subject>,
        window_start: NaiveDate,
    ) -> Vec<Projected> {
        if !WEEKLY_RECURRENCE {
            return Vec::new();
        }
        let Some(target) = dates::parse_key(day) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in &self.entries {
            if !e.repeat_weekly || lens.is_some_and(|s| e.subject != s) {
                continue;
            }
            let Some(source) = dates::parse_key(&e.day) else {
                continue;
            };
            if source < window_start || target <= source {
                continue;
            }
            let delta = (target - source).num_days();
            if delta % 7 != 0 {
                continue;
            }
            if source.weekday() != target.weekday() {
                continue;
            }
            out.push(Projected {
                mark: e.mark,
                text: e.text.clone(),
                subject: e.subject,
                entry_id: Some(e.id.clone()),
            });
        }
        out
    }

    pub fn subject_count(&self, subject: Subject, quarter: &Quarter) -> usize {
        self.entries
            .iter()
            .filter(|e| e.subject == subject && quarter.contains_key(&e.day))
            .count()
    }

    pub fn subject_days(&self, subject: Subject, quarter: &Quarter) -> usize {
        self.entries
            .iter()
            .filter(|e| e.subject == subject && quarter.contains_key(&e.day))
            .map(|e| e.day.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn subject_next(&self, subject: Subject, quarter: &Quarter) -> Option<NextThing> {
        for day in &quarter.days {
            if let Some(e) = self
                .entries
                .iter()
                .find(|e| e.subject == subject && e.day == day.key)
            {
                return Some(NextThing {
                    day: day.key.clone(),
                    mark: e.mark,
                    text: e.text.clone(),
                });
            }
        }
        None
    }

    pub fn stats(&self, quarter: &Quarter, lens: Option<Subject>) -> (usize, usize) {
        let carrying = quarter
            .days
            .iter()
            .filter(|d| self.has_content(&d.key, lens))
            .count();
        (carrying, SPAN_DAYS as usize - carrying)
    }

    pub fn is_note_editing(&self, key: &str, has_text: bool) -> bool {
        self.ui.note_edit.get(key).copied().unwrap_or(!has_text)
    }

    pub fn set_note_editing(&mut self, key: &str, editing: bool) {
        self.ui.note_edit.insert(key.to_string(), editing);
    }

    pub fn toggle_task(&mut self, target: NoteTarget, line: usize, today: NaiveDate) -> bool {
        let source = match target {
            NoteTarget::Entry(ref id) => self
                .entry(id)
                .map(|e| e.note_text().to_string())
                .unwrap_or_default(),
            NoteTarget::Page(ref day) => self.page(day).to_string(),
            NoteTarget::Loose(ref id) => self.note(id).map(|n| n.text.clone()).unwrap_or_default(),
        };
        let Some(next) = crate::markdown::toggle_task_line(&source, line) else {
            return false;
        };
        match target {
            NoteTarget::Entry(id) => {
                if let Some(e) = self.entry_mut(&id) {
                    e.set_note(next);
                }
            }
            NoteTarget::Page(day) => {
                self.set_page(&day, &next, today);
            }
            NoteTarget::Loose(id) => {
                if let Some(n) = self.note_mut(&id) {
                    n.text = next;
                }
            }
        }
        true
    }

    pub fn day_items_has_page(&self, day: &str, lens: Option<Subject>) -> bool {
        lens.is_none() && self.has_page(day)
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub mark: Mark,
    pub text: String,
    pub subject: Subject,
    pub projected: bool,
    pub loose: bool,
    pub weight: u8,
    pub entry_id: Option<String>,
}

#[derive(Clone, Debug)]
struct Projected {
    mark: Mark,
    text: String,
    subject: Subject,
    entry_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NextThing {
    pub day: String,
    pub mark: Mark,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteTarget {
    Entry(String),
    Page(String),
    Loose(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn doc_with_demo() -> (Doc, Quarter) {
        let today = d(2026, 11, 2);
        let quarter = Quarter::new(today);
        let mut doc = Doc::empty();
        doc.entries.push(Entry::new("2026-11-03", "Dentist, 09:20"));
        doc.entries
            .push(Entry::new("2026-11-03", "Call Rui about the flat"));
        doc.entries.push(Entry::new("2026-11-07", "Mum's birthday"));
        doc.entries[2].weight = 2;
        doc.entries
            .push(Entry::new("2026-11-14", "Lobola meeting, Nelspruit"));
        doc.entries[3].weight = 3;
        doc.notes.push(LooseNote {
            day: Some("2026-11-03".into()),
            ..LooseNote::new("Passport expires 2029", None)
        });
        doc.notes
            .push(LooseNote::new("Ask the bond consultant", None));
        (doc, quarter)
    }

    #[test]
    fn weight_is_the_max_and_tier_follows() {
        let (doc, _q) = doc_with_demo();
        assert_eq!(doc.weight("2026-11-03", None), 1);
        assert_eq!(doc.weight("2026-11-07", None), 2);
        assert_eq!(doc.weight("2026-11-14", None), 3);
        assert_eq!(doc.tier("2026-11-14", None), Tier::Landmark);
        assert_eq!(doc.tier("2026-11-02", None), Tier::Open);
    }

    #[test]
    fn lens_recomputes_weight_and_keeps_the_page_whole() {
        let (doc, q) = doc_with_demo();
        assert_eq!(doc.weight("2026-11-03", Some(Subject::Me)), 1);
        assert_eq!(doc.weight("2026-11-03", Some(Subject::House)), 1);
        assert_eq!(doc.weight("2026-11-03", Some(Subject::Family)), 0);
        assert_eq!(doc.weight("2026-11-14", Some(Subject::Family)), 3);
        assert_eq!(doc.weight("2026-11-14", Some(Subject::Travels)), 0);
        assert_eq!(
            doc.items_on("2026-11-03", Some(Subject::Me), q.start).len(),
            1
        );
        // The lens narrows the wall, never the day page.
        assert_eq!(doc.entries_on("2026-11-03", None).count(), 2);
    }

    #[test]
    fn counts_and_next_are_window_only() {
        let (mut doc, q) = doc_with_demo();
        assert_eq!(doc.subject_count(Subject::Family, &q), 2);
        assert_eq!(doc.subject_days(Subject::Family, &q), 2);
        let next = doc.subject_next(Subject::Family, &q).unwrap();
        assert_eq!(next.day, "2026-11-07");
        assert_eq!(next.text, "Mum's birthday");

        doc.entries
            .push(Entry::new("2027-02-10", "Outside the window"));
        assert_eq!(doc.subject_count(Subject::Me, &q), 1);
    }

    #[test]
    fn loose_and_clipped_notes_are_distinct() {
        let (doc, q) = doc_with_demo();
        assert_eq!(doc.loose_notes().count(), 1);
        assert_eq!(doc.clipped_notes_on("2026-11-03", None).count(), 1);
        let items = doc.items_on("2026-11-03", None, q.start);
        assert!(items.iter().any(|i| i.loose));
    }

    #[test]
    fn recurring_entries_project_forward_without_ink() {
        let today = d(2026, 11, 2);
        let q = Quarter::new(today);
        let mut doc = Doc::empty();
        let mut e = Entry::new("2026-11-03", "Gym");
        e.repeat_weekly = true;
        e.weight = 3;
        doc.entries.push(e);
        // Real occurrence keeps its weight.
        assert_eq!(doc.weight("2026-11-03", None), 3);
        // Projections are outline texture, never weight.
        assert_eq!(doc.weight("2026-11-10", None), 0);
        let items = doc.items_on("2026-11-10", None, q.start);
        assert_eq!(items.len(), 1);
        assert!(items[0].projected);
        assert_eq!(items[0].mark, Mark::Body);
    }

    #[test]
    fn page_first_write_marks_plan_and_past() {
        let mut doc = Doc::empty();
        doc.set_page("2026-11-20", "# Plan", d(2026, 11, 3));
        assert!(doc.page_written_ahead("2026-11-20"));
        assert!(!doc.page_is_past("2026-11-20", d(2026, 11, 3)));
        assert!(doc.page_is_past("2026-11-20", d(2026, 11, 21)));
        doc.set_page("2026-11-20", "", d(2026, 11, 3));
        assert!(!doc.has_page("2026-11-20"));
        assert!(!doc.page_written_ahead("2026-11-20"));
    }

    #[test]
    fn note_editing_defaults_to_read_when_written() {
        let mut doc = Doc::empty();
        assert!(doc.is_note_editing("page:2026-11-03", false));
        assert!(!doc.is_note_editing("page:2026-11-03", true));
        doc.set_note_editing("page:2026-11-03", true);
        assert!(doc.is_note_editing("page:2026-11-03", true));
    }

    #[test]
    fn task_toggle_rewrites_the_markdown() {
        let mut doc = Doc::empty();
        doc.set_page("2026-11-03", "- [ ] book\n- [x] photos", d(2026, 11, 3));
        assert!(doc.toggle_task(NoteTarget::Page("2026-11-03".into()), 0, d(2026, 11, 3)));
        assert_eq!(doc.page("2026-11-03"), "- [x] book\n- [x] photos");
        assert!(doc.toggle_task(NoteTarget::Page("2026-11-03".into()), 1, d(2026, 11, 3)));
        assert_eq!(doc.page("2026-11-03"), "- [x] book\n- [ ] photos");
    }

    #[test]
    fn loose_note_body_edits_and_task_toggles() {
        let (mut doc, _) = doc_with_demo();
        let id = doc.loose_notes().next().unwrap().id.clone();

        doc.set_note_text(
            &id,
            "Bike service:\n- [ ] check chain\n- [ ] order brake pads".into(),
        );
        assert!(doc.note(&id).unwrap().text.contains("- [ ] check chain"));
        assert!(doc.toggle_task(NoteTarget::Loose(id.clone()), 1, d(2026, 11, 2)));
        assert_eq!(
            doc.note(&id).unwrap().text,
            "Bike service:\n- [x] check chain\n- [ ] order brake pads"
        );

        doc.set_note_subject(&id, Subject::Tech);
        assert_eq!(doc.note(&id).unwrap().subject, Subject::Tech);
    }

    #[test]
    fn removing_a_loose_note_clears_its_editing_state() {
        let (mut doc, _) = doc_with_demo();
        let id = doc.loose_notes().next().unwrap().id.clone();
        doc.set_note_editing(&note_key_loose(&id), true);
        doc.remove_note(&id);
        assert!(doc.note(&id).is_none());
        assert!(!doc.ui.note_edit.contains_key(&note_key_loose(&id)));
    }

    #[test]
    fn stats_count_real_content_only() {
        let (doc, q) = doc_with_demo();
        let (carrying, open) = doc.stats(&q, None);
        assert_eq!(carrying, 3);
        assert_eq!(open, 87);
    }
}
