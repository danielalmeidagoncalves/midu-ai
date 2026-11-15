use chrono::{Datelike, NaiveDate};
use horizon_core::{
    dates::{self, Quarter},
    Doc, Subject,
};

pub fn day_label(doc: &Doc, day: &str, lens: Option<Subject>, window_start: NaiveDate) -> String {
    let Some(date) = dates::parse_key(day) else {
        return day.to_string();
    };
    let all: Vec<String> = doc
        .items_on(day, lens, window_start)
        .into_iter()
        .map(|item| {
            if item.projected {
                format!("{} (projected)", item.text)
            } else {
                item.text
            }
        })
        .collect();
    let content = if all.is_empty() {
        "open, nothing on it".to_string()
    } else {
        all.join("; ")
    };
    let mut label = format!(
        "{} {} {}",
        dates::weekday_full(date),
        date.day(),
        dates::month_full(date)
    );
    label.push_str(&format!(" — {content}"));
    if lens.is_none() && doc.has_page(day) {
        label.push_str(". Day page written.");
    }
    if let Some(lens) = lens {
        format!("{} only — {}", lens.name(), label)
    } else {
        label
    }
}

pub fn cell_label(
    doc: &Doc,
    day: &str,
    lens: Option<Subject>,
    today: &str,
    window_start: NaiveDate,
) -> String {
    let mut label = day_label(doc, day, lens, window_start);
    if day == today {
        label.push_str(" Today.");
    }
    label
}

pub fn standfirst(doc: &Doc, day: &str) -> &'static str {
    let weight = doc.weight(day, None);
    let entries = doc.entries_on(day, None).count();
    let notes = doc.clipped_notes_on(day, None).count();
    if weight >= 3 {
        "This one changes something."
    } else if weight == 2 {
        "A long one — it takes the whole day."
    } else if entries > 1 {
        "A handful of small things, and the day is yours."
    } else if entries == 1 {
        if notes > 0 {
            "One thing, and a detail hanging off it."
        } else {
            "One thing, and the day is otherwise yours."
        }
    } else if notes > 0 {
        "Details only. Nothing here has a time."
    } else {
        "Open. Nothing has to resolve here."
    }
}

pub fn next_thing(doc: &Doc, quarter: &Quarter) -> Option<(String, horizon_core::Mark, String)> {
    for day in quarter.days.iter().skip(1) {
        if let Some(e) = doc.entries_on(&day.key, None).next() {
            return Some((day.key.clone(), e.mark, e.text.clone()));
        }
    }
    None
}

pub fn today_short(quarter: &Quarter) -> String {
    dates::short_day(quarter.today)
}

pub fn today_count(doc: &Doc, quarter: &Quarter) -> usize {
    let key = dates::key(quarter.today);
    doc.entries_on(&key, None).count()
}

pub fn day_page_title(day: &str, today: &str) -> &'static str {
    if day == today {
        "Today's page"
    } else {
        "The day's page"
    }
}
