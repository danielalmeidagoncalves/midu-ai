//! The one-line composer. One pure function, every grammar case.
//!
//! `dec 3 - flight` · `3 dec - flight` · `3/12 - flight`
//! `today - …` · `tomorrow - …` · `day after tomorrow - …`
//! no date → the open day, else today
//! a trailing `/subject` overrides the inferred subject; an unknown one is refused.
//! dates outside the 90-day window are refused, never silently moved.

use chrono::{Datelike, NaiveDate};

use crate::dates::{self, month_prefix_index, Quarter};
use crate::inference::{infer_mark, infer_subject, Mark, Subject};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposerResult {
    pub day: String,
    pub title: String,
    pub mark: Mark,
    pub subject: Subject,
    /// True when the subject came from `/subject` or the open lens, not from the words.
    pub explicit_subject: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerError {
    Empty,
    UnknownSubject(String),
    OutsideWindow,
    MissingTitle,
}

impl ComposerError {
    pub fn message(&self) -> String {
        match self {
            ComposerError::Empty => "Type something first.".into(),
            ComposerError::UnknownSubject(s) => format!("No subject called “{s}”."),
            ComposerError::OutsideWindow => {
                "That date is outside this quarter (the next ninety days).".into()
            }
            ComposerError::MissingTitle => "Add a word or two after the date.".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposerPreview {
    pub mark: Mark,
    pub subject: Subject,
    pub subject_ok: bool,
}

/// Splits a trailing `/subject` off a line: the text without the token, and the
/// subject it names. `Err` carries the unknown query so the caller can refuse it
/// with the same words the composer uses. Entry parsing and the margin share it.
pub fn split_subject(raw: &str) -> (String, Option<Result<Subject, String>>) {
    let trimmed = raw.trim_end();
    let (before, token) = match trimmed.rsplit_once(char::is_whitespace) {
        Some((b, t)) => (b, t),
        None => ("", trimmed),
    };
    if let Some(q) = token.strip_prefix('/') {
        if !q.is_empty() && q.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return (
                before.trim().to_string(),
                Some(Subject::from_query(q).ok_or_else(|| q.to_string())),
            );
        }
    }
    (raw.to_string(), None)
}

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '—' | '–' | '-' | '·' | ':' | ',' | '+')
}

fn boundary(s: &str, at: usize) -> bool {
    s.get(at..)
        .and_then(|rest| rest.chars().next())
        .is_none_or(is_separator)
}

fn named_month_first(s: &str) -> Option<(u32, u32, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i < 3 {
        return None;
    }
    let month = month_prefix_index(&s[..i])?;
    let mut j = i;
    if bytes.get(j) == Some(&b'.') {
        j += 1;
    }
    if !boundary(s, j) {
        return None;
    }
    while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
        j += 1;
    }
    let start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == start {
        return None;
    }
    let day: u32 = s[start..j].parse().ok()?;
    if !boundary(s, j) {
        return None;
    }
    Some((month, day, j))
}

fn day_first(s: &str) -> Option<(u32, u32, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i > 2 {
        return None;
    }
    let day: u32 = s[..i].parse().ok()?;
    if !s[i..].starts_with(char::is_whitespace) {
        return None;
    }
    while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i - start < 3 {
        return None;
    }
    let month = month_prefix_index(&s[start..i])?;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
    }
    if !boundary(s, i) {
        return None;
    }
    Some((month, day, i))
}

fn numeric(s: &str) -> Option<(u32, u32, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i > 2 {
        return None;
    }
    let day: u32 = s[..i].parse().ok()?;
    let mut j = i;
    while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
        j += 1;
    }
    if bytes.get(j) != Some(&b'/') {
        return None;
    }
    j += 1;
    while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
        j += 1;
    }
    let start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == start || j - start > 2 {
        return None;
    }
    let month: u32 = s[start..j].parse().ok()?;
    if !boundary(s, j) {
        return None;
    }
    Some((month, day, j))
}

fn date_in_window(year: i32, month: u32, day: u32, quarter: &Quarter) -> Option<NaiveDate> {
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    if date.month() != month || date.day() != day {
        return None;
    }
    quarter.contains(date).then_some(date)
}

fn resolve(month: u32, day: u32, quarter: &Quarter) -> Option<NaiveDate> {
    let year = quarter.today.year();
    date_in_window(year, month, day, quarter)
        .or_else(|| date_in_window(year + 1, month, day, quarter))
        .or_else(|| date_in_window(year - 1, month, day, quarter))
}

fn strip_title(raw: &str) -> &str {
    raw.trim_start_matches(|c: char| is_separator(c)).trim()
}

/// Parse a complete composer line.
pub fn parse(
    raw: &str,
    open_day: Option<&str>,
    quarter: &Quarter,
    lens: Option<Subject>,
) -> Result<ComposerResult, ComposerError> {
    let (without_subject, subject_token) = split_subject(raw);
    let subject_override = match subject_token {
        Some(Ok(s)) => Some(s),
        Some(Err(bad)) => return Err(ComposerError::UnknownSubject(bad)),
        None => None,
    };
    let s = without_subject.trim();
    if s.is_empty() {
        return Err(ComposerError::Empty);
    }

    let lower = s.to_lowercase();
    let mut dated: Option<NaiveDate> = None;
    let mut consumed = 0usize;

    for (word, offset) in [("day after tomorrow", 2i64), ("tomorrow", 1), ("today", 0)] {
        if lower.starts_with(word) && boundary(&lower, word.len()) {
            dated = Some(quarter.today + chrono::Duration::days(offset));
            consumed = word.len();
            break;
        }
    }

    if dated.is_none() {
        if let Some((month, day, end)) = named_month_first(&lower) {
            dated = resolve(month, day, quarter);
            consumed = end;
        }
    }
    if dated.is_none() {
        if let Some((month, day, end)) = day_first(&lower) {
            dated = resolve(month, day, quarter);
            consumed = end;
        }
    }
    if dated.is_none() {
        if let Some((month, day, end)) = numeric(&lower) {
            dated = resolve(month, day, quarter);
            consumed = end;
        }
    }

    let key = if consumed > 0 {
        let Some(date) = dated else {
            return Err(ComposerError::OutsideWindow);
        };
        if !quarter.contains(date) {
            return Err(ComposerError::OutsideWindow);
        }
        dates::key(date)
    } else if let Some(open) = open_day.filter(|d| quarter.contains_key(d)) {
        open.to_string()
    } else {
        dates::key(quarter.today)
    };

    let title = strip_title(&s[consumed.min(s.len())..]).to_string();
    if title.is_empty() {
        return Err(ComposerError::MissingTitle);
    }

    let subject = subject_override
        .or(lens)
        .unwrap_or_else(|| infer_subject(&title));

    Ok(ComposerResult {
        day: key,
        title: title.clone(),
        mark: infer_mark(&title),
        subject,
        explicit_subject: subject_override.is_some() || lens.is_some(),
    })
}

/// Live feedback for the composer: the mark and subject that will be used.
pub fn preview(raw: &str, lens: Option<Subject>) -> ComposerPreview {
    let (without_subject, subject_token) = split_subject(raw);
    let subject_ok = !matches!(subject_token, Some(Err(_)));
    let subject = match subject_token {
        Some(Ok(s)) => s,
        _ => lens.unwrap_or_else(|| infer_subject(&without_subject)),
    };
    ComposerPreview {
        mark: infer_mark(&without_subject),
        subject,
        subject_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Quarter;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn q() -> Quarter {
        Quarter::new(d(2026, 11, 2))
    }

    fn ok(raw: &str) -> ComposerResult {
        parse(raw, None, &q(), None).unwrap()
    }

    #[test]
    fn three_date_forms_and_separators() {
        assert_eq!(ok("dec 3 - flight").day, "2026-12-03");
        assert_eq!(ok("3 dec - flight").day, "2026-12-03");
        assert_eq!(ok("3/12 - flight").day, "2026-12-03");
        assert_eq!(ok("Dec 3 — flight").title, "flight");
        assert_eq!(ok("dec 3 · flight").title, "flight");
        assert_eq!(ok("dec 3: flight").title, "flight");
        assert_eq!(ok("dec 3, flight").title, "flight");
        assert_eq!(ok("dec 3 + flight").title, "flight");
    }

    #[test]
    fn relative_dates() {
        assert_eq!(ok("today - call mum").day, "2026-11-02");
        assert_eq!(ok("tomorrow - call mum").day, "2026-11-03");
        assert_eq!(ok("day after tomorrow - call mum").day, "2026-11-04");
    }

    #[test]
    fn no_date_uses_open_day_then_today() {
        assert_eq!(ok("flight").day, "2026-11-02");
        assert_eq!(
            parse("flight", Some("2026-12-03"), &q(), None).unwrap().day,
            "2026-12-03"
        );
        // An open day outside the window is ignored.
        assert_eq!(
            parse("flight", Some("2027-06-01"), &q(), None).unwrap().day,
            "2026-11-02"
        );
    }

    #[test]
    fn year_rolls_into_the_window() {
        assert_eq!(ok("jan 5 - role").day, "2027-01-05");
    }

    #[test]
    fn outside_window_is_refused() {
        // Feb 10 is outside the 90 days from 2 Nov.
        assert_eq!(
            parse("feb 10 - flight", None, &q(), None).unwrap_err(),
            ComposerError::OutsideWindow
        );
        assert_eq!(
            parse("oct 1 - flight", None, &q(), None).unwrap_err(),
            ComposerError::OutsideWindow
        );
        assert_eq!(
            parse("31/2 - flight", None, &q(), None).unwrap_err(),
            ComposerError::OutsideWindow
        );
        assert_eq!(ok("nov 3 - flight").day, "2026-11-03");
    }

    #[test]
    fn subject_token_overrides_and_unknown_is_refused() {
        let r = ok("bond - january /finances");
        assert_eq!(r.subject, Subject::Finances);
        assert!(r.explicit_subject);
        assert_eq!(r.title, "bond - january");
        assert_eq!(
            parse("bond /nonsense", None, &q(), None).unwrap_err(),
            ComposerError::UnknownSubject("nonsense".into())
        );
        // A lens is a default, not an override of an explicit token.
        let r = parse("bond /travels", None, &q(), Some(Subject::Finances)).unwrap();
        assert_eq!(r.subject, Subject::Travels);
        let r = parse("bond", None, &q(), Some(Subject::Finances)).unwrap();
        assert_eq!(r.subject, Subject::Finances);
        assert!(r.explicit_subject);
    }

    #[test]
    fn split_subject_strips_the_token_for_other_surfaces() {
        let (text, token) = split_subject("Bike service /tech");
        assert_eq!(text, "Bike service");
        assert_eq!(token, Some(Ok(Subject::Tech)));
        let (_, bad) = split_subject("Bike service /nonsense");
        assert_eq!(bad, Some(Err("nonsense".into())));
        let (text, token) = split_subject("No token here");
        assert_eq!(text, "No token here");
        assert_eq!(token, None);
    }

    #[test]
    fn empty_and_titleless_are_refused() {
        assert_eq!(
            parse("   ", None, &q(), None).unwrap_err(),
            ComposerError::Empty
        );
        assert_eq!(
            parse("dec 3 - ", None, &q(), None).unwrap_err(),
            ComposerError::MissingTitle
        );
    }

    #[test]
    fn preview_shows_the_mark_and_subject() {
        let p = preview("dec 3 - flight", None);
        assert_eq!(p.mark, Mark::Travel);
        assert_eq!(p.subject, Subject::Travels);
        assert!(p.subject_ok);
        let p = preview("bond /nonsense", None);
        assert!(!p.subject_ok);
        let p = preview("bond", Some(Subject::Finances));
        assert_eq!(p.subject, Subject::Finances);
    }

    #[test]
    fn leading_separator_is_stripped() {
        assert_eq!(ok("dec 3 ---- flight").title, "flight");
        assert_eq!(ok("dec 3 . flight").title, ". flight");
    }
}
