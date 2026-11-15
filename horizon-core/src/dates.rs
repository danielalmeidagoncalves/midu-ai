//! Quarter maths. Pure date functions, no document and no DOM.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const SPAN_DAYS: i64 = 90;

pub const WEEKDAY_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
pub const WEEKDAY_FULL: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
pub const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
pub const MONTH_FULL: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Day {
    pub date: NaiveDate,
    pub key: String,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub struct Week {
    pub number: u32,
    /// The month printed in the gutter when it changes.
    pub month: u32,
    /// Seven slots, Monday first. `None` is a cell outside the 90-day window.
    pub slots: [Option<usize>; 7],
    pub first: NaiveDate,
}

#[derive(Clone, Debug)]
pub struct Quarter {
    pub today: NaiveDate,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub days: Vec<Day>,
    pub weeks: Vec<Week>,
}

impl Quarter {
    pub fn new(today: NaiveDate) -> Self {
        let start = today;
        let end = today + Duration::days(SPAN_DAYS - 1);
        let mut days = Vec::with_capacity(SPAN_DAYS as usize);
        for index in 0..SPAN_DAYS as usize {
            let date = start + Duration::days(index as i64);
            days.push(Day {
                date,
                key: key(date),
                index,
            });
        }

        // Monday-first. The first row is padded before today; the last row is padded after
        // day 90. This is the reference behaviour and keeps every weekday in its true column.
        let offset = today.weekday().num_days_from_monday() as usize;
        let mut cells: Vec<Option<usize>> = vec![None; offset];
        cells.extend((0..SPAN_DAYS as usize).map(Some));
        while !cells.len().is_multiple_of(7) {
            cells.push(None);
        }

        let mut weeks = Vec::new();
        for chunk in cells.chunks(7) {
            let mut slots = [None; 7];
            slots.copy_from_slice(chunk);
            let first_index = slots.iter().flatten().next().copied().unwrap_or(0);
            let first = days[first_index].date;
            weeks.push(Week {
                number: first.iso_week().week(),
                month: first.month(),
                slots,
                first,
            });
        }

        Self {
            today,
            start,
            end,
            days,
            weeks,
        }
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start && date <= self.end
    }

    pub fn contains_key(&self, day: &str) -> bool {
        parse_key(day).is_some_and(|d| self.contains(d))
    }

    pub fn index_of(&self, day: &str) -> Option<usize> {
        self.days.iter().position(|d| d.key == day)
    }

    pub fn day(&self, day: &str) -> Option<&Day> {
        self.days.iter().find(|d| d.key == day)
    }

    pub fn range_label(&self) -> String {
        format!("{} — {}", short_day(self.start), short_day(self.end))
    }

    /// The week containing the given day, if it is in the window.
    pub fn week_of(&self, day: &str) -> Option<&Week> {
        self.weeks
            .iter()
            .find(|w| w.slots.iter().flatten().any(|i| self.days[*i].key == day))
    }
}

pub fn key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn parse_key(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    date + Duration::days(days)
}

pub fn weekday_short(date: NaiveDate) -> &'static str {
    WEEKDAY_SHORT[date.weekday().num_days_from_monday() as usize]
}

pub fn weekday_full(date: NaiveDate) -> &'static str {
    WEEKDAY_FULL[date.weekday().num_days_from_monday() as usize]
}

pub fn month_short(date: NaiveDate) -> &'static str {
    MONTH_SHORT[date.month0() as usize]
}

pub fn month_full(date: NaiveDate) -> &'static str {
    MONTH_FULL[date.month0() as usize]
}

pub fn short_day(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        weekday_short(date),
        date.day(),
        month_short(date)
    )
}

pub fn long_date(date: NaiveDate) -> String {
    format!(
        "{} · {} {} {}",
        weekday_full(date),
        date.day(),
        month_full(date),
        date.year()
    )
}

pub fn month_prefix_index(value: &str) -> Option<u32> {
    let value = value.trim().to_lowercase();
    if value.len() < 3 {
        return None;
    }
    MONTH_FULL
        .iter()
        .position(|m| m.to_lowercase().starts_with(&value))
        .map(|i| i as u32 + 1)
}

pub fn weekday_from_mon(date: NaiveDate) -> u32 {
    date.weekday().num_days_from_monday()
}

pub fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn quarter_is_ninety_days_starting_today() {
        let q = Quarter::new(d(2026, 11, 2));
        assert_eq!(q.days.len(), 90);
        assert_eq!(q.start, d(2026, 11, 2));
        assert_eq!(q.end, d(2027, 1, 30));
        assert_eq!(q.days[0].key, "2026-11-02");
        assert_eq!(q.days[89].key, "2027-01-30");
        assert!(q.contains(d(2026, 11, 2)));
        assert!(!q.contains(d(2026, 11, 1)));
        assert!(!q.contains(d(2027, 1, 31)));
    }

    #[test]
    fn weeks_are_monday_first_and_keep_every_day() {
        let q = Quarter::new(d(2026, 11, 2)); // Monday
        assert_eq!(q.weeks.len(), 13);
        assert_eq!(q.weeks[0].slots[0], Some(0));
        assert_eq!(q.weeks[0].first, d(2026, 11, 2));
        assert_eq!(q.weeks[12].slots[5], Some(89));
        assert_eq!(q.weeks[0].number, 45);

        let tuesday = Quarter::new(d(2026, 11, 3));
        assert_eq!(tuesday.weeks.len(), 13);
        assert_eq!(tuesday.weeks[0].slots[0], None);
        assert_eq!(tuesday.weeks[0].slots[1], Some(0));

        let wednesday = Quarter::new(d(2026, 11, 4));
        assert_eq!(wednesday.weeks.len(), 14);
        assert_eq!(wednesday.weeks[0].slots[0], None);
        assert_eq!(wednesday.weeks[0].slots[2], Some(0));
        assert_eq!(wednesday.weeks[13].slots[0], Some(89));
    }

    #[test]
    fn month_labels_come_from_the_first_real_day() {
        let q = Quarter::new(d(2026, 11, 2));
        assert_eq!(q.weeks[0].month, 11);
        let jan = q.weeks.iter().find(|w| w.month == 1).unwrap();
        assert_eq!(jan.first.month(), 1);
    }

    #[test]
    fn iso_week_and_formatting() {
        assert_eq!(Quarter::new(d(2027, 1, 4)).weeks[0].number, 1);
        assert_eq!(short_day(d(2026, 11, 3)), "Tue 3 Nov");
        assert_eq!(long_date(d(2026, 11, 3)), "Tuesday · 3 November 2026");
        assert_eq!(month_prefix_index("dec"), Some(12));
        assert_eq!(month_prefix_index("december"), Some(12));
        assert_eq!(month_prefix_index("de"), None);
        assert_eq!(month_prefix_index("nope"), None);
    }

    #[test]
    fn weekday_helpers() {
        assert!(is_weekend(d(2026, 11, 7)));
        assert!(!is_weekend(d(2026, 11, 6)));
        assert_eq!(weekday_from_mon(d(2026, 11, 2)), 0);
    }
}
