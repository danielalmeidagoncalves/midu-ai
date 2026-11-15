//! The empty first launch, and an opt-in demo quarter for tests and screenshots.

use chrono::{NaiveDate, TimeZone, Utc};

use crate::dates::{self, add_days};
use crate::inference::Subject;
use crate::model::{Doc, Entry, LooseNote};

/// A quiet, legal first launch: nothing to tidy, nothing already late.
pub fn empty() -> Doc {
    Doc::empty()
}

/// The concept's sample quarter, shifted to whatever "today" is. Used by tests and by
/// `HORIZON_DEMO=1` when the store is empty. It is never loaded silently.
pub fn demo(today: NaiveDate) -> Doc {
    let mut doc = Doc::empty();
    let at = |offset: i64| dates::key(add_days(today, offset));
    let now = Utc.with_ymd_and_hms(2026, 11, 2, 8, 0, 0).unwrap();

    let mut add = |offset: i64, text: &str, weight: u8, subject: Option<Subject>| {
        let mut e = Entry::new_with(&at(offset), text, subject, Some(now));
        e.weight = weight;
        doc.entries.push(e);
    };

    add(1, "Dentist, 09:20", 1, None);
    add(3, "Standing desk arrives", 1, None);
    add(5, "Mum's birthday", 2, None);
    add(5, "Call Rui about the flat", 1, None);
    add(7, "Bosch — service the drill", 1, None);
    add(9, "Lease renewal — sign", 2, None);
    add(12, "Lobola meeting, Nelspruit", 3, None);
    add(12, "Book the flights down", 1, None);
    add(17, "Cardiology follow-up", 2, None);
    add(22, "Tax pack to the accountant", 1, None);
    add(26, "Sipho & Naledi's wedding", 3, None);
    add(28, "Backpack — replace the strap", 1, None);
    add(30, "Sipho's suit fitting", 1, None);
    add(32, "Visa interview", 2, None);
    add(32, "Passport photos", 1, None);
    add(36, "Sketches — ink the Nelspruit series", 1, None);
    add(39, "Contract ends — decide renewal", 3, None);
    add(42, "Retrogaming night with Thabo", 1, None);
    add(46, "Drive up to Polokwane", 1, None);
    add(50, "Thabo's school fees", 2, None);
    add(53, "Christmas at home", 2, None);
    add(55, "Prompts — rebuild the eval set", 1, None);
    add(59, "Year-end dinner", 1, None);
    add(64, "Start the new role", 3, None);
    add(67, "Thabo's school term starts", 1, None);
    add(71, "AI — ship the reading agent", 1, None);
    add(73, "Journalling: the ninety-day review", 1, None);
    add(80, "Housing bond first payment", 2, None);
    add(86, "Thabo's birthday", 1, None);
    add(89, "Assets — annual insurance check", 1, None);

    // One standing thing, to show recurrence as outline texture rather than ink.
    let mut gym = Entry::new_with(&at(1), "Gym", None, Some(now));
    gym.repeat_weekly = true;
    doc.entries.push(gym);

    for entry in &mut doc.entries {
        match entry.text.as_str() {
            "Lobola meeting, Nelspruit" => entry.set_note(
                "# What we agreed in June\n\n- **Nine head of cattle**, of which six are already arranged.\n- The envelope goes to the eldest uncle, not the father.\n- We arrive at *the house*, 09:00 — `not the venue`.\n\n- [x] flights\n- [ ] who drives back Sunday\n- [ ] photographs\n\nSee ^[2026-11-07^Mum's birthday] — she wants to come but the drive is long."
                    .replace("2026-11-07", &at(5)),
            ),
            "Contract ends — decide renewal" => entry.set_note(
                "## The three things that actually decide it\n\n1. the notice period, in writing\n2. take-home after tax vs. the new role\n3. whether four days a week is on the table\n\n> Everything else is noise this month.\n\n`notice` says 30 days — **check the annexure**."
                    .into(),
            ),
            "Visa interview" => entry.set_note(
                "- folder: passport, bank statements, invitation letter\n- [x] photos\n- [ ] print two copies of the itinerary\n- arrive 20 minutes early, phones off"
                    .into(),
            ),
            _ => {}
        }
    }

    doc.notes.push(LooseNote::new(
        "Ask the bond consultant about January rates",
        None,
    ));
    doc.notes.push(LooseNote {
        day: Some(at(12)),
        ..LooseNote::new("Nelspruit — who is driving back on the Sunday?", None)
    });
    doc.notes.push(LooseNote {
        day: Some(at(39)),
        ..LooseNote::new("Renewal: get the notice period in writing", None)
    });
    doc.notes
        .push(LooseNote::new("Passport expires 2029 — no rush", None));

    let today_page = "## Today\n\nThree things matter this week, and only one has a time on it.\n\n# Where the quarter turns\n\n- **14 Nov** — Lobola in Nelspruit. Everything before it is travel admin.\n- **11 Dec** — the contract answers itself one way or the other.\n- **5 Jan** — the new role starts, whether or not the flat is sorted.\n\n# Before the Nelspruit trip\n\n- [x] book the flights down\n- [ ] who is driving back on Sunday\n- [ ] confirm with Sipho what the family expects us to bring\n\n> Ask the bond consultant about January rates — the answer changes which month we move.\n\n| what | when | cost |\n| --- | --- | --- |\n| flights | 14 Nov | ~R4 200 |\n| car back | 15 Nov | still asking |";
    doc.set_page(&at(0), today_page, today);
    doc.set_page(
        &at(12),
        "# Nelspruit — lobola\n\nThe day the whole quarter was arranged around.\n\n- 09:00 — meet the family at the house, not the venue.\n- Bring the envelope; the uncles will ask twice.\n- @Sipho has the list of what was agreed in June.\n\n## After\n\n- [ ] settle the driving back on Sunday ^[2026-11-15^drive home]\n- [ ] photographs from Naledi's brother"
            .replace("2026-11-15", &at(13))
            .as_str(),
        today,
    );
    doc.set_page(
        &at(22),
        "Tax pack.\n\n- invoices for the contract, Aug–Nov\n- the bond statement (they need the interest figure)\n- **nothing else** — the accountant asked for exactly these three",
        today,
    );
    doc.set_page(
        &at(39),
        "# Contract ends\n\nDecide before the 11th, not on it.\n\n- [ ] read the notice period again — *in writing, from them*\n- [ ] the new role: what does it actually pay after tax\n- [ ] if we stay, ask for the four-day week instead of the raise\n\n> The renewal is not the question. The question is whether the work still fits the life we are building.",
        today,
    );
    doc.set_page(
        &at(55),
        "## Prompts — rebuild the eval set\n\nKeep it small enough to run in an evening.\n\n1. twenty real questions from the notebook\n2. five that used to fail\n3. one that should be refused\n\n- [ ] score the old set for comparison",
        today,
    );
    doc.set_page(
        &at(59),
        "Year-end.\n\nWrite down what this year was, in four sentences, before midnight.",
        today,
    );

    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_has_a_little_of_everything() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let doc = demo(today);
        assert!(doc.entries.len() > 20);
        assert!(doc.notes.len() >= 4);
        assert!(doc.pages.len() >= 6);
        assert!(doc.entries.iter().any(|e| e.weight == 3));
        assert!(doc.entries.iter().any(|e| e.repeat_weekly));
        assert!(doc.entries.iter().any(|e| e.has_note()));
        assert!(doc.loose_notes().count() >= 2);
    }
}
