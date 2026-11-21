//! Reminders: when a thing should come back, and how often.
//!
//! Pure date and duration maths, no document and no DOM. The grammar is the
//! composer's shape — one line, read left to right — so a reminder is typed the
//! way a day is filed.
//!
//! ```text
//! in 1h30m · in 90m · in 20m
//! at 10:30 · tomorrow 9:00 · monday 9:00
//! every day at 8:00 · every monday at 9:00 · every month on the 30th
//! ```
//!
//! A time that has already passed today rolls to tomorrow. A repeat runs until
//! it is switched off: there is no end date and no series management.

use std::fmt;

use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc,
};

use crate::dates;
use crate::model::{Reminder, Repeat};

/// What a typed line means: the first instant, and how it repeats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct When {
    pub at: DateTime<Utc>,
    pub repeat: Repeat,
    /// The local day-of-month a monthly reminder keeps across short months.
    pub anchor_day: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhenError {
    Empty,
    NotUnderstood,
    OutOfRange,
}

impl WhenError {
    pub fn message(&self) -> String {
        match self {
            WhenError::Empty => "Type a time first.".into(),
            WhenError::NotUnderstood => {
                "Try “in 1h30m”, “at 10:30”, “tomorrow 9:00” or “every month on the 30th”.".into()
            }
            WhenError::OutOfRange => "That is more than a year away.".into(),
        }
    }
}

impl fmt::Display for WhenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

/// Nothing may be scheduled further out than this; it catches a typo like
/// `in 900h` before it becomes a reminder nobody will ever see.
const MAX_AHEAD_DAYS: i64 = 366;

/// Parse one line into its first instant and its repeat.
pub fn parse_when(raw: &str, now: DateTime<Local>) -> Result<When, WhenError> {
    let text = raw.trim().to_lowercase();
    if text.is_empty() {
        return Err(WhenError::Empty);
    }
    let when = if let Some(rest) = text.strip_prefix("in ") {
        let at = now + duration(rest.trim()).ok_or(WhenError::NotUnderstood)?;
        When {
            at: at.with_timezone(&Utc),
            repeat: Repeat::Once,
            anchor_day: None,
        }
    } else if let Some(rest) = text.strip_prefix("every ") {
        repeating(rest.trim(), now)?
    } else {
        let (day, time) = parse_day_and_time(&text, now).ok_or(WhenError::NotUnderstood)?;
        When {
            at: local_at(day, time).ok_or(WhenError::NotUnderstood)?,
            repeat: Repeat::Once,
            anchor_day: None,
        }
    };
    if when.at > now.with_timezone(&Utc) + Duration::days(MAX_AHEAD_DAYS) {
        return Err(WhenError::OutOfRange);
    }
    Ok(when)
}

/// `every day at 8:00` · `every monday at 9:00` · `every month on the 30th`
fn repeating(rest: &str, now: DateTime<Local>) -> Result<When, WhenError> {
    if let Some(after) = rest.strip_prefix("month on the ") {
        let day = ordinal(after.trim()).ok_or(WhenError::NotUnderstood)?;
        let anchor = day.clamp(1, 31);
        let time = now.time();
        let (mut year, mut month) = (now.year(), now.month());
        let mut at =
            monthly_candidate(year, month, anchor, time).ok_or(WhenError::NotUnderstood)?;
        if at <= now.with_timezone(&Utc) {
            (year, month) = next_month_of(year, month);
            at = monthly_candidate(year, month, anchor, time).ok_or(WhenError::NotUnderstood)?;
        }
        return Ok(When {
            at,
            repeat: Repeat::MonthlyOnDay(anchor),
            anchor_day: Some(anchor),
        });
    }
    let (day, time, repeat) = if let Some(after) = rest.strip_prefix("day at ") {
        (
            now.date_naive(),
            time_of_day(after).ok_or(WhenError::NotUnderstood)?,
            Repeat::Daily,
        )
    } else {
        // A weekday name, then `at <time>`.
        let (name, after) = rest.split_once(' ').ok_or(WhenError::NotUnderstood)?;
        let weekday = weekday_index(name).ok_or(WhenError::NotUnderstood)?;
        let after = after.strip_prefix("at ").ok_or(WhenError::NotUnderstood)?;
        let time = time_of_day(after).ok_or(WhenError::NotUnderstood)?;
        (
            next_weekday(now.date_naive(), weekday),
            time,
            Repeat::Weekly,
        )
    };
    let mut at = local_at(day, time).ok_or(WhenError::NotUnderstood)?;
    // `every day at 8:00` typed at 09:00 starts tomorrow; a weekly one starts
    // next week, not tomorrow.
    if at <= now.with_timezone(&Utc) {
        at += match repeat {
            Repeat::Weekly => Duration::days(7),
            _ => Duration::days(1),
        };
    }
    Ok(When {
        at,
        repeat,
        anchor_day: None,
    })
}

/// The instant after `current`, stepping in local time so `08:00` stays `08:00`
/// across a clock change.
pub fn next_after(
    current: DateTime<Utc>,
    repeat: Repeat,
    anchor_day: Option<u32>,
) -> DateTime<Utc> {
    let local = current.with_timezone(&Local);
    match repeat {
        Repeat::Once => current,
        Repeat::Daily => (local + Duration::days(1)).with_timezone(&Utc),
        Repeat::Weekly => (local + Duration::days(7)).with_timezone(&Utc),
        Repeat::MonthlyOnDay(day) => {
            let anchor = anchor_day.unwrap_or(day);
            let (year, month) = next_month_of(local.year(), local.month());
            monthly_candidate(year, month, anchor, local.time()).unwrap_or(current)
        }
    }
}

/// The clamped day of a given month, at a wall time.
fn monthly_candidate(year: i32, month: u32, anchor: u32, time: NaiveTime) -> Option<DateTime<Utc>> {
    let day = clamp_day(year, month, anchor);
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    local_at_with(date, time)
}

fn next_month_of(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

fn clamp_day(year: i32, month: u32, day: u32) -> u32 {
    let last = days_in_month(year, month);
    day.min(last)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first = NaiveDate::from_ymd_opt(year, month, 1);
    let next_first = NaiveDate::from_ymd_opt(next_year, next_month, 1);
    match (first, next_first) {
        (Some(a), Some(b)) => (b - a).num_days() as u32,
        _ => 28,
    }
}

fn next_weekday(from: NaiveDate, target: u32) -> NaiveDate {
    let current = dates::weekday_from_mon(from);
    let delta = (target + 7 - current) % 7;
    from + Duration::days(delta as i64)
}

fn weekday_index(name: &str) -> Option<u32> {
    let name = name.trim_end_matches('.');
    dates::WEEKDAY_FULL
        .iter()
        .position(|d| d.to_lowercase() == name)
        .or_else(|| {
            dates::WEEKDAY_SHORT
                .iter()
                .position(|d| d.to_lowercase() == name)
        })
        .map(|i| i as u32)
}

/// `1h30m`, `90m`, `2h`, `45s` — the shapes a duration is typed in.
fn duration(raw: &str) -> Option<Duration> {
    let mut total = 0i64;
    let mut rest = raw.trim();
    let mut matched = false;
    while !rest.is_empty() {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        rest = &rest[digits.len()..];
        let unit = rest.chars().next()?;
        rest = &rest[unit.len_utf8()..];
        let value: i64 = digits.parse().ok()?;
        let seconds = match unit {
            's' => 1,
            'm' => 60,
            'h' => 3_600,
            'd' => 86_400,
            _ => return None,
        };
        total += value * seconds;
        matched = true;
        rest = rest.trim_start();
    }
    matched.then(|| Duration::seconds(total))
}

/// `10:30`, `9`, `9:00`, `0930` — a wall time, today unless it has passed.
fn parse_day_and_time(text: &str, now: DateTime<Local>) -> Option<(NaiveDate, NaiveTime)> {
    if let Some(after) = text.strip_prefix("tomorrow ") {
        return Some((now.date_naive() + Duration::days(1), time_of_day(after)?));
    }
    if let Some(after) = text.strip_prefix("today ") {
        return Some((now.date_naive(), time_of_day(after)?));
    }
    // A bare weekday: the next one, at 09:00 unless a time follows.
    if let Some((name, after)) = text.split_once(' ') {
        if let Some(weekday) = weekday_index(name) {
            let time = time_of_day(after.trim_start_matches("at ")).unwrap_or_else(default_time);
            return Some((next_weekday(now.date_naive(), weekday), time));
        }
    }
    if let Some(weekday) = weekday_index(text) {
        return Some((next_weekday(now.date_naive(), weekday), default_time()));
    }
    let time = time_of_day(text)?;
    let today = now.date_naive();
    let candidate = local_at(today, time)?;
    let day = if candidate <= now.with_timezone(&Utc) {
        today + Duration::days(1)
    } else {
        today
    };
    Some((day, time))
}

fn default_time() -> NaiveTime {
    NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default()
}

/// `10:30` · `9:00` · `9` · `0930`
fn time_of_day(raw: &str) -> Option<NaiveTime> {
    let raw = raw.trim().trim_start_matches("at ").trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((h, m)) = raw.split_once(':') {
        let hour: u32 = h.trim().parse().ok()?;
        let minute: u32 = m.trim().parse().ok()?;
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }
    if raw.len() == 4 && raw.chars().all(|c| c.is_ascii_digit()) {
        let hour: u32 = raw[..2].parse().ok()?;
        let minute: u32 = raw[2..].parse().ok()?;
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }
    let hour: u32 = raw.parse().ok()?;
    NaiveTime::from_hms_opt(hour, 0, 0)
}

/// `30th` · `30` · `1st` · `22nd`
fn ordinal(raw: &str) -> Option<u32> {
    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// A local wall time to UTC. A time the clock skipped (the spring-forward hour)
/// lands on the next valid instant rather than disappearing.
fn local_at(day: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    local_at_with(day, time)
}

fn local_at_with(day: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    let naive = day.and_time(time);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        LocalResult::Ambiguous(earlier, _) => Some(earlier.with_timezone(&Utc)),
        LocalResult::None => {
            // Skipped by the clock: take the instant an hour later.
            let shifted = naive + Duration::hours(1);
            Local
                .from_local_datetime(&shifted)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
        }
    }
}

/// Move a reminder forward to the first occurrence strictly after `now`.
/// Returns true when something changed. A one-off is simply marked done.
pub fn advance(reminder: &mut Reminder, now: DateTime<Utc>) -> bool {
    if reminder.done {
        return false;
    }
    let Some(mut at) = reminder.at_utc() else {
        return false;
    };
    if at > now {
        return false;
    }
    reminder.last_fired = Some(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    if reminder.repeat.is_once() {
        reminder.done = true;
        return true;
    }
    // Bounded: a store left unopened for years still advances in a few steps,
    // and a broken repeat can never spin here.
    for _ in 0..1_000 {
        let next = next_after(at, reminder.repeat, reminder.anchor_day);
        if next <= at {
            // A repeat that cannot move forward is broken; mark it done rather
            // than loop or fire forever.
            reminder.done = true;
            return true;
        }
        at = next;
        if at > now {
            reminder.next_at = at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            return true;
        }
    }
    reminder.done = true;
    true
}

/// Snooze to a fixed offset from now, keeping the repeat untouched.
pub fn snooze(reminder: &mut Reminder, now: DateTime<Utc>, by: Duration) {
    reminder.next_at = (now + by).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
}

/// The next occurrence of a repeat, for the snooze menu's "tomorrow 09:00".
pub fn tomorrow_nine(now: DateTime<Local>) -> DateTime<Utc> {
    let day = now.date_naive() + Duration::days(1);
    local_at_with(day, default_time()).unwrap_or_else(|| now.with_timezone(&Utc))
}

/// A short, human phrase for when a reminder lands. This is the only place the
/// strip's wording is decided, so it cannot drift from the row's meaning.
pub fn humanize(at: Option<DateTime<Utc>>, now: DateTime<Local>) -> String {
    let Some(at) = at else {
        return "at an unknown time".into();
    };
    let local = at.with_timezone(&Local);
    let today = now.date_naive();
    let day = local.date_naive();
    let delta = at - now.with_timezone(&Utc);
    let minutes = delta.num_minutes();
    if delta.num_seconds() <= 0 {
        return "now".into();
    }
    if day == today {
        if minutes < 60 {
            return format!("in {}m", minutes.max(1));
        }
        let hours = minutes / 60;
        let rest = minutes % 60;
        if rest == 0 {
            return format!("in {hours}h");
        }
        return format!("in {hours}h {rest}m");
    }
    let time = local.format("%H:%M").to_string();
    if day == today + Duration::days(1) {
        return format!("tomorrow {time}");
    }
    let days = (day - today).num_days();
    if (0..7).contains(&days) {
        return format!("{} {time}", dates::weekday_short(day));
    }
    format!("{} {} {time}", day.day(), dates::month_short(day))
}

/// The repeat's phrase, for the notification body and the strip's rows.
pub fn repeat_phrase(repeat: Repeat, anchor_day: Option<u32>) -> Option<String> {
    match repeat {
        Repeat::Once => None,
        Repeat::Daily => Some("every day".into()),
        Repeat::Weekly => Some("every week".into()),
        Repeat::MonthlyOnDay(day) => {
            let day = anchor_day.unwrap_or(day);
            Some(format!("{} of each month", ordinal_suffix(day)))
        }
    }
}

fn ordinal_suffix(day: u32) -> String {
    let suffix = match day % 100 {
        11..=13 => "th",
        _ => match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{day}{suffix}")
}

/// The text a notification carries: the reminder's words, then its repeat.
pub fn notification_body(reminder: &Reminder) -> String {
    match repeat_phrase(reminder.repeat, reminder.anchor_day) {
        Some(phrase) => format!("{} · {phrase}", reminder.text),
        None => reminder.text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    fn local(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(y, m, d, h, min, 0)
            .single()
            .expect("a wall time that exists")
    }

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        local(y, m, d, h, min).with_timezone(&Utc)
    }

    #[test]
    fn durations_are_typed_the_way_they_are_said() {
        let now = local(2026, 11, 21, 9, 0);
        let w = parse_when("in 1h30m", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 21, 10, 30));
        assert_eq!(w.repeat, Repeat::Once);

        let w = parse_when("in 90m", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 21, 10, 30));

        let w = parse_when("in 2h", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 21, 11, 0));
    }

    #[test]
    fn a_past_time_today_rolls_to_tomorrow() {
        let now = local(2026, 11, 21, 9, 0);
        let w = parse_when("at 10:30", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 21, 10, 30));

        let w = parse_when("at 8:00", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 22, 8, 0));
    }

    #[test]
    fn named_days_and_tomorrow() {
        let now = local(2026, 11, 21, 9, 0); // a Saturday
        assert_eq!(
            parse_when("tomorrow 9:00", now).unwrap().at,
            at(2026, 11, 22, 9, 0)
        );
        // The next Monday, not today.
        assert_eq!(
            parse_when("monday 9:00", now).unwrap().at,
            at(2026, 11, 23, 9, 0)
        );
    }

    #[test]
    fn repeats_carry_their_pattern() {
        let now = local(2026, 11, 21, 9, 0);
        let w = parse_when("every day at 8:00", now).unwrap();
        assert_eq!(w.repeat, Repeat::Daily);
        assert_eq!(w.at, at(2026, 11, 22, 8, 0));

        let w = parse_when("every monday at 9:00", now).unwrap();
        assert_eq!(w.repeat, Repeat::Weekly);
        assert_eq!(w.at, at(2026, 11, 23, 9, 0));

        let w = parse_when("every month on the 30th", now).unwrap();
        assert_eq!(w.repeat, Repeat::MonthlyOnDay(30));
        assert_eq!(w.anchor_day, Some(30));
        assert_eq!(w.at, at(2026, 11, 30, 9, 0));
    }

    #[test]
    fn monthly_clamps_to_the_last_day_and_keeps_the_anchor() {
        let jan31 = at(2026, 1, 31, 9, 0);
        let feb = next_after(jan31, Repeat::MonthlyOnDay(31), Some(31));
        assert_eq!(
            feb.date_naive(),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        let mar = next_after(feb, Repeat::MonthlyOnDay(31), Some(31));
        assert_eq!(
            mar.date_naive(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()
        );
    }

    #[test]
    fn advancing_a_one_off_marks_it_done() {
        let mut r = Reminder {
            text: "Laundry".into(),
            next_at: at(2026, 11, 21, 10, 0).to_rfc3339(),
            ..Reminder::default()
        };
        assert!(advance(&mut r, at(2026, 11, 21, 10, 1)));
        assert!(r.done);
        assert!(r.last_fired.is_some());
    }

    #[test]
    fn advancing_a_daily_repeat_skips_the_gap() {
        let mut r = Reminder {
            text: "Water".into(),
            next_at: at(2026, 11, 1, 8, 0).to_rfc3339(),
            repeat: Repeat::Daily,
            ..Reminder::default()
        };
        let now = at(2026, 11, 21, 9, 0);
        assert!(advance(&mut r, now));
        assert!(!r.done);
        assert!(r.at_utc().unwrap() > now);
        assert_eq!(r.at_utc().unwrap(), at(2026, 11, 22, 8, 0));
    }

    #[test]
    fn due_and_upcoming_are_ordered() {
        let mut doc = crate::model::Doc::empty();
        doc.reminders.push(Reminder {
            text: "later".into(),
            next_at: at(2026, 11, 21, 12, 0).to_rfc3339(),
            ..Reminder::default()
        });
        doc.reminders.push(Reminder {
            text: "sooner".into(),
            next_at: at(2026, 11, 21, 10, 0).to_rfc3339(),
            ..Reminder::default()
        });
        doc.reminders.push(Reminder {
            text: "past".into(),
            next_at: at(2026, 11, 20, 10, 0).to_rfc3339(),
            ..Reminder::default()
        });
        let now = at(2026, 11, 21, 11, 0);
        let due = doc.due_reminders(now);
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].text, "past");
        assert_eq!(due[1].text, "sooner");
        let up = doc.upcoming_reminders();
        assert_eq!(up[0].text, "past");
        assert_eq!(up[2].text, "later");
    }

    #[test]
    fn snooze_keeps_the_repeat() {
        let mut r = Reminder {
            text: "Bill".into(),
            next_at: at(2026, 11, 21, 10, 0).to_rfc3339(),
            repeat: Repeat::MonthlyOnDay(30),
            anchor_day: Some(30),
            ..Reminder::default()
        };
        snooze(&mut r, at(2026, 11, 21, 10, 0), Duration::minutes(10));
        assert_eq!(r.at_utc().unwrap(), at(2026, 11, 21, 10, 10));
        assert_eq!(r.repeat, Repeat::MonthlyOnDay(30));
    }

    #[test]
    fn wording_is_short_and_stable() {
        let now = local(2026, 11, 21, 9, 0);
        assert_eq!(humanize(Some(at(2026, 11, 21, 10, 30)), now), "in 1h 30m");
        assert_eq!(humanize(Some(at(2026, 11, 21, 9, 20)), now), "in 20m");
        assert_eq!(
            humanize(Some(at(2026, 11, 22, 9, 0)), now),
            "tomorrow 09:00"
        );
        assert_eq!(humanize(Some(at(2026, 11, 23, 9, 0)), now), "Mon 09:00");
        assert_eq!(
            repeat_phrase(Repeat::MonthlyOnDay(30), Some(30)).unwrap(),
            "30th of each month"
        );
        assert_eq!(repeat_phrase(Repeat::Once, None), None);
        assert_eq!(ordinal_suffix(1), "1st");
        assert_eq!(ordinal_suffix(22), "22nd");
        assert_eq!(ordinal_suffix(13), "13th");
    }

    #[test]
    fn nonsense_is_refused_with_words() {
        let now = local(2026, 11, 21, 9, 0);
        assert_eq!(parse_when("", now), Err(WhenError::Empty));
        assert_eq!(parse_when("banana", now), Err(WhenError::NotUnderstood));
        assert_eq!(parse_when("in soon", now), Err(WhenError::NotUnderstood));
        assert_eq!(parse_when("at 99:99", now), Err(WhenError::NotUnderstood));
        assert_eq!(parse_when("in 9000h", now), Err(WhenError::OutOfRange));
    }

    #[test]
    fn a_weekday_repeat_lands_on_the_next_one() {
        let now = local(2026, 11, 21, 9, 0); // Saturday
                                             // Today is Saturday and 10:00 has not passed, so today is the first one.
        let w = parse_when("every saturday at 10:00", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 21, 10, 0));
        assert_eq!(w.repeat, Repeat::Weekly);
        // Once the hour has passed, the next one is a week out.
        let w = parse_when("every saturday at 8:00", now).unwrap();
        assert_eq!(w.at, at(2026, 11, 28, 8, 0));
    }

    #[test]
    fn humanize_uses_the_local_clock() {
        let now = local(2026, 11, 21, 9, 0);
        let later = at(2026, 11, 21, 14, 0);
        assert_eq!(humanize(Some(later), now), "in 5h");
        assert!(humanize(None, now).contains("unknown"));
        // Local hours are formatted without a leading zero on the hour.
        let t = at(2026, 11, 22, 9, 5);
        assert!(humanize(Some(t), now).starts_with("tomorrow 09:0"));
        assert_eq!(now.hour(), 9);
    }
}
