//! Versioned JSON: the whole-document portability path. The Markdown store is
//! `store.rs`; there is no second Markdown shape.

use std::fmt;

use chrono::NaiveDate;
use serde_json::Value;

use crate::dates;
use crate::inference::{infer_mark, infer_subject, Mark, Subject};
use crate::model::{Doc, DOC_VERSION};

#[derive(Debug)]
pub enum ImportError {
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    NotAnObject,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Json(e) => write!(f, "That is not valid JSON: {e}"),
            ImportError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "This file is version {v}; this Horizon understands version {DOC_VERSION}."
                )
            }
            ImportError::NotAnObject => write!(f, "The document must be a JSON object."),
        }
    }
}

pub fn to_json(doc: &Doc) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(doc)
}

pub fn from_json(raw: &str) -> Result<Doc, ImportError> {
    let mut value: Value = serde_json::from_str(raw).map_err(ImportError::Json)?;
    if !value.is_object() {
        return Err(ImportError::NotAnObject);
    }
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > DOC_VERSION {
        return Err(ImportError::UnsupportedVersion(version));
    }
    // Migration path. Version 0 is the unversioned concept export; it is structurally the
    // same shape, so stamping it is the whole migration. Future versions add arms here.
    if version < DOC_VERSION {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("version".into(), Value::from(DOC_VERSION));
        }
    }
    let mut doc: Doc = serde_json::from_value(value).map_err(ImportError::Json)?;
    normalize(&mut doc);
    Ok(doc)
}

fn normalize(doc: &mut Doc) {
    doc.version = DOC_VERSION;
    for entry in &mut doc.entries {
        if entry.id.is_empty() {
            entry.id = crate::model::new_id("e");
        }
        entry.weight = entry.weight.clamp(1, 3);
        if entry.mark_auto {
            entry.mark = infer_mark(&entry.text);
        }
        if entry.subject_auto {
            entry.subject = infer_subject(&entry.text);
        }
        if entry.day.is_empty() {
            entry.day = dates::key(chrono::Local::now().date_naive());
        }
    }
    for note in &mut doc.notes {
        if note.id.is_empty() {
            note.id = crate::model::new_id("n");
        }
        if let Some(day) = note.day.as_deref() {
            if dates::parse_key(day).is_none() {
                note.day = None;
            }
        }
    }
    doc.pages.retain(|_, md| !md.trim().is_empty());
    for day in doc.pages.keys() {
        doc.page_meta
            .entry(day.clone())
            .or_insert_with(|| crate::model::PageMeta {
                first_written: day.clone(),
            });
    }
    doc.page_meta.retain(|day, _| doc.pages.contains_key(day));
}

/// The key printed in the secondary tab. It is generated from the same enums as the app.
pub fn key_lines() -> Vec<(Mark, &'static str)> {
    Mark::ALL.iter().map(|m| (*m, m.examples())).collect()
}

pub fn subject_lines() -> Vec<(Subject, &'static str)> {
    Subject::ALL.iter().map(|s| (*s, s.handle())).collect()
}

/// A page's first-write date, if any. Kept public so the UI can phrase the marker honestly.
pub fn first_written(doc: &Doc, day: &str) -> Option<NaiveDate> {
    doc.page_meta
        .get(day)
        .and_then(|m| dates::parse_key(&m.first_written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Entry, LooseNote};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn doc() -> Doc {
        let mut doc = Doc::empty();
        let mut e = Entry::new("2026-11-03", "Dentist, 09:20");
        e.set_note("- [ ] print the referral".into());
        doc.entries.push(e);
        doc.notes
            .push(LooseNote::new("Ask the bond consultant", None));
        doc.notes.push(LooseNote {
            day: Some("2026-11-03".into()),
            ..LooseNote::new("Passport expires 2029", None)
        });
        doc.set_page("2026-11-03", "# Today\n\nThree things.", d(2026, 11, 2));
        doc
    }

    #[test]
    fn json_round_trip_keeps_every_thing() {
        let doc = doc();
        let json = to_json(&doc).unwrap();
        let back = from_json(&json).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.notes.len(), 2);
        assert_eq!(back.page("2026-11-03"), "# Today\n\nThree things.");
        assert_eq!(back.entries[0].note_text(), "- [ ] print the referral");
        assert!(back.page_written_ahead("2026-11-03"));
    }

    #[test]
    fn unknown_versions_are_refused() {
        let err = from_json(r#"{"version":99,"entries":[]}"#).unwrap_err();
        assert!(matches!(err, ImportError::UnsupportedVersion(99)));
    }

    #[test]
    fn unversioned_import_is_migrated() {
        let doc = from_json(r#"{"entries":[{"day":"2026-11-03","text":"Dentist"}]}"#).unwrap();
        assert_eq!(doc.version, DOC_VERSION);
        assert_eq!(doc.entries[0].mark, Mark::Health);
    }

    #[test]
    fn unknown_mark_and_subject_fall_back_without_failing() {
        let doc = from_json(
            r#"{"version":1,"entries":[{"day":"2026-11-03","text":"x","mark":"nonsense","subject":"nonsense"}]}"#,
        )
        .unwrap();
        assert_eq!(doc.entries[0].mark, Mark::Thing);
        assert_eq!(doc.entries[0].subject, Subject::Me);
    }

    #[test]
    fn empty_pages_are_dropped_on_import() {
        let doc = from_json(r#"{"version":1,"pages":{"2026-11-03":"   "}}"#).unwrap();
        assert!(!doc.has_page("2026-11-03"));
    }
}
