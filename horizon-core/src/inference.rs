//! The eighteen marks and seventeen subjects, and the inference that never asks.
//!
//! This module is deliberately data-heavy and logic-light. The vocabulary is the product:
//! keyword scoring decides both the mark and the subject, and a wrong guess costs one click.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Mark {
    Travel,
    Health,
    People,
    Money,
    Work,
    Home,
    Ceremony,
    Admin,
    Study,
    Body,
    Food,
    Gift,
    Place,
    Call,
    Deadline,
    Rest,
    Note,
    #[default]
    Thing,
}

impl Mark {
    pub const ALL: [Mark; 18] = [
        Mark::Travel,
        Mark::Health,
        Mark::People,
        Mark::Money,
        Mark::Work,
        Mark::Home,
        Mark::Ceremony,
        Mark::Admin,
        Mark::Study,
        Mark::Body,
        Mark::Food,
        Mark::Gift,
        Mark::Place,
        Mark::Call,
        Mark::Deadline,
        Mark::Rest,
        Mark::Note,
        Mark::Thing,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Mark::Travel => "travel",
            Mark::Health => "health",
            Mark::People => "people",
            Mark::Money => "money",
            Mark::Work => "work",
            Mark::Home => "home",
            Mark::Ceremony => "ceremony",
            Mark::Admin => "admin",
            Mark::Study => "study",
            Mark::Body => "body",
            Mark::Food => "food",
            Mark::Gift => "gift",
            Mark::Place => "place",
            Mark::Call => "call",
            Mark::Deadline => "deadline",
            Mark::Rest => "rest",
            Mark::Note => "note",
            Mark::Thing => "thing",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mark::Travel => "Travel",
            Mark::Health => "Health",
            Mark::People => "People",
            Mark::Money => "Money",
            Mark::Work => "Work",
            Mark::Home => "Home",
            Mark::Ceremony => "Ceremony",
            Mark::Admin => "Admin",
            Mark::Study => "Study",
            Mark::Body => "Body",
            Mark::Food => "Food",
            Mark::Gift => "Gift",
            Mark::Place => "Place",
            Mark::Call => "Call",
            Mark::Deadline => "Deadline",
            Mark::Rest => "Rest",
            Mark::Note => "Note",
            Mark::Thing => "Unsorted",
        }
    }

    pub fn examples(self) -> &'static str {
        match self {
            Mark::Travel => "flight, visa",
            Mark::Health => "dentist, bloods",
            Mark::People => "mum, birthday",
            Mark::Money => "bond, invoice",
            Mark::Work => "offsite, client",
            Mark::Home => "desk, lease",
            Mark::Ceremony => "wedding, lobola",
            Mark::Admin => "passport, form",
            Mark::Study => "course, reading",
            Mark::Body => "gym, run",
            Mark::Food => "dinner, braai",
            Mark::Gift => "present, flowers",
            Mark::Place => "hotel, venue",
            Mark::Call => "call, zoom",
            Mark::Deadline => "due, final",
            Mark::Rest => "leave, quiet",
            Mark::Note => "idea, maybe",
            Mark::Thing => "everything else",
        }
    }

    pub fn from_id(id: &str) -> Option<Mark> {
        Mark::ALL.iter().copied().find(|m| m.id() == id)
    }

    pub fn parse_or_thing(id: &str) -> Mark {
        Mark::from_id(id).unwrap_or(Mark::Thing)
    }
}

impl Serialize for Mark {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Mark {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Mark::parse_or_thing(&s))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Subject {
    Ai,
    Assets,
    Backpack,
    Everyone,
    Bosch,
    Sketches,
    Family,
    Finances,
    House,
    Journalling,
    Kids,
    #[default]
    Me,
    Payments,
    Prompts,
    Retro,
    Tech,
    Travels,
}

impl Subject {
    pub const ALL: [Subject; 17] = [
        Subject::Ai,
        Subject::Assets,
        Subject::Backpack,
        Subject::Everyone,
        Subject::Bosch,
        Subject::Sketches,
        Subject::Family,
        Subject::Finances,
        Subject::House,
        Subject::Journalling,
        Subject::Kids,
        Subject::Me,
        Subject::Payments,
        Subject::Prompts,
        Subject::Retro,
        Subject::Tech,
        Subject::Travels,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Subject::Ai => "ai",
            Subject::Assets => "assets",
            Subject::Backpack => "backpack",
            Subject::Everyone => "everyone",
            Subject::Bosch => "bosch",
            Subject::Sketches => "sketches",
            Subject::Family => "family",
            Subject::Finances => "finances",
            Subject::House => "house",
            Subject::Journalling => "journalling",
            Subject::Kids => "kids",
            Subject::Me => "me",
            Subject::Payments => "payments",
            Subject::Prompts => "prompts",
            Subject::Retro => "retro",
            Subject::Tech => "tech",
            Subject::Travels => "travels",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Subject::Ai => "AI",
            Subject::Assets => "Assets",
            Subject::Backpack => "Backpack",
            Subject::Everyone => "because everyone deserves to be happy",
            Subject::Bosch => "Bosch",
            Subject::Sketches => "Sketches",
            Subject::Family => "Family",
            Subject::Finances => "Finances",
            Subject::House => "House",
            Subject::Journalling => "Journalling",
            Subject::Kids => "Kids",
            Subject::Me => "Me",
            Subject::Payments => "Payments",
            Subject::Prompts => "Prompts",
            Subject::Retro => "Retrogaming",
            Subject::Tech => "Tech",
            Subject::Travels => "Travels",
        }
    }

    pub fn handle(self) -> &'static str {
        match self {
            Subject::Ai => "AI",
            Subject::Assets => "AST",
            Subject::Backpack => "BKP",
            Subject::Everyone => "EVRY",
            Subject::Bosch => "BOS",
            Subject::Sketches => "SKT",
            Subject::Family => "FAM",
            Subject::Finances => "FIN",
            Subject::House => "HSE",
            Subject::Journalling => "JRN",
            Subject::Kids => "KID",
            Subject::Me => "ME",
            Subject::Payments => "PAY",
            Subject::Prompts => "PRM",
            Subject::Retro => "RTG",
            Subject::Tech => "TEC",
            Subject::Travels => "TRV",
        }
    }

    pub fn from_id(id: &str) -> Option<Subject> {
        Subject::ALL.iter().copied().find(|s| s.id() == id)
    }

    pub fn parse_or_me(id: &str) -> Subject {
        Subject::from_id(id).unwrap_or(Subject::Me)
    }

    /// The composer's `/subject` token. Exact id or handle, then a unique prefix of either,
    /// then a unique prefix of the printed name. Ambiguity is refused, never guessed.
    pub fn from_query(q: &str) -> Option<Subject> {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return None;
        }
        if let Some(s) = Subject::from_id(&q) {
            return Some(s);
        }
        if let Some(s) = Subject::ALL
            .iter()
            .copied()
            .find(|s| s.handle().to_lowercase() == q)
        {
            return Some(s);
        }
        let mut hits = Subject::ALL.iter().copied().filter(|s| {
            s.id().starts_with(&q)
                || s.handle().to_lowercase().starts_with(&q)
                || s.name().to_lowercase().starts_with(&q)
        });
        let first = hits.next()?;
        if hits.next().is_some() {
            None
        } else {
            Some(first)
        }
    }
}

impl Serialize for Subject {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Subject {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Subject::parse_or_me(&s))
    }
}

// ---------------------------------------------------------------------------
// Keywords. Order matters: ties go to the earlier row, exactly as in the concept.
// ---------------------------------------------------------------------------

pub const MARK_KEYWORDS: &[(Mark, &[&str])] = &[
    (
        Mark::Travel,
        &[
            "flight",
            "flights",
            "fly",
            "airport",
            "terminal",
            "travel",
            "trip",
            "train",
            "drive up",
            "drive",
            "driving",
            "uber",
            "lift",
            "journey",
            "bus",
            "check-in",
            "visa interview",
            "visa",
        ],
    ),
    (
        Mark::Health,
        &[
            "dentist",
            "doctor",
            "clinic",
            "hospital",
            "cardio",
            "cardiology",
            "physio",
            "vaccin",
            "blood",
            "surgery",
            "optician",
            "therapy",
            "checkup",
            "check-up",
            "scan",
        ],
    ),
    (
        Mark::Ceremony,
        &[
            "lobola meeting",
            "wedding",
            "lobola",
            "funeral",
            "ceremony",
            "graduation",
            "church",
            "baptism",
            "memorial",
            "christmas",
            "fitting",
        ],
    ),
    (
        Mark::Money,
        &[
            "rent",
            "lease",
            "bond",
            "rates",
            "rate",
            "invoice",
            "tax",
            "accountant",
            "salary",
            "payment",
            "pay",
            "bank",
            "insurance",
            "premium",
            "budget",
            "mortgage",
            "loan",
            "quote",
            "contract",
            "sign",
            "renewal",
            "cost",
            "price",
        ],
    ),
    (
        Mark::Work,
        &[
            "work",
            "offsite",
            "team",
            "client",
            "meeting",
            "standup",
            "deck",
            "review",
            "shoot",
            "presentation",
            "office",
            "brief",
            "pitch",
            "role",
            "interview",
        ],
    ),
    (
        Mark::People,
        &[
            "mum",
            "mom",
            "dad",
            "rui",
            "birthday",
            "friend",
            "family",
            "anniversary",
            "visit",
            "call",
        ],
    ),
    (
        Mark::Home,
        &[
            "desk",
            "flat",
            "house",
            "move",
            "clean",
            "plumber",
            "electrician",
            "delivery",
            "arrives",
            "furniture",
            "garden",
            "pack",
        ],
    ),
    (
        Mark::Study,
        &[
            "book",
            "read",
            "reading",
            "study",
            "exam",
            "course",
            "class",
            "lecture",
            "research",
            "chapter",
            "assignment",
            "school",
            "term",
        ],
    ),
    (
        Mark::Body,
        &[
            "gym", "run", "walk", "swim", "yoga", "fitness", "ride", "hike", "pilates", "training",
        ],
    ),
    (
        Mark::Food,
        &[
            "food",
            "market",
            "groceries",
            "grocery",
            "shop",
            "cook",
            "braai",
            "supper",
            "breakfast",
            "restaurant",
            "dinner",
            "lunch",
            "drinks",
            "eat",
        ],
    ),
    (
        Mark::Gift,
        &["gift", "present", "flowers", "card", "surprise"],
    ),
    (
        Mark::Place,
        &[
            "venue",
            "hotel",
            "airbnb",
            "studio",
            "sandton",
            "nelspruit",
            "polokwane",
            "cape town",
            "johannesburg",
        ],
    ),
    (
        Mark::Admin,
        &[
            "form",
            "file",
            "register",
            "renew",
            "passport",
            "licence",
            "license",
            "permit",
            "application",
            "submit",
            "paperwork",
            "passport photos",
        ],
    ),
    (
        Mark::Deadline,
        &["deadline", "due", "final", "cut-off", "cutoff", "close"],
    ),
    (
        Mark::Call,
        &[
            "call", "phone", "whatsapp", "zoom", "ring", "chat", "facetime",
        ],
    ),
    (
        Mark::Rest,
        &[
            "rest", "sleep", "holiday", "leave", "quiet", "break", "sabbath", "away",
        ],
    ),
    (
        Mark::Note,
        &[
            "maybe", "idea", "ask", "check", "think", "remember", "note", "consider", "who", "how",
            "why",
        ],
    ),
];

pub const SUBJECT_KEYWORDS: &[(Subject, &[&str])] = &[
    (
        Subject::Ai,
        &[
            "ai",
            "llm",
            "agent",
            "model",
            "dataset",
            "embedding",
            "fine-tune",
            "neural",
            "inference",
        ],
    ),
    (
        Subject::Assets,
        &[
            "asset",
            "assets",
            "insurance",
            "policy",
            "valuation",
            "portfolio",
            "inventory",
            "deed",
        ],
    ),
    (
        Subject::Backpack,
        &["backpack", "rucksack", "bag", "packing list", "gear list"],
    ),
    (
        Subject::Everyone,
        &[
            "charity",
            "donate",
            "donation",
            "kindness",
            "volunteer",
            "fundraiser",
        ],
    ),
    (
        Subject::Bosch,
        &[
            "bosch", "drill", "tool", "tools", "toolbox", "workshop", "sander", "saw",
        ],
    ),
    (
        Subject::Sketches,
        &[
            "sketch",
            "sketches",
            "draw",
            "drawing",
            "illustration",
            "series",
            "print",
            "ink",
        ],
    ),
    (
        Subject::Family,
        &[
            "family",
            "mum",
            "mom",
            "dad",
            "rui",
            "sipho",
            "naledi",
            "lobola",
            "wedding",
            "christmas",
            "birthday",
            "anniversary",
            "dinner",
            "supper",
        ],
    ),
    (
        Subject::Finances,
        &[
            "finance",
            "finances",
            "tax",
            "accountant",
            "budget",
            "salary",
            "contract",
            "invoice",
            "quote",
            "savings",
        ],
    ),
    (
        Subject::House,
        &[
            "house",
            "flat",
            "lease",
            "desk",
            "move",
            "plumber",
            "electrician",
            "furniture",
            "garden",
            "clean",
            "paint",
        ],
    ),
    (
        Subject::Journalling,
        &[
            "journal",
            "journalling",
            "journaling",
            "diary",
            "review",
            "morning pages",
            "gratitude",
        ],
    ),
    (
        Subject::Kids,
        &[
            "thabo", "kid", "kids", "school", "term", "homework", "teacher", "soccer", "practice",
        ],
    ),
    (
        Subject::Me,
        &[
            "dentist",
            "doctor",
            "clinic",
            "cardiology",
            "blood",
            "physio",
            "health",
            "checkup",
            "check-up",
            "sleep",
            "rest",
            "role",
        ],
    ),
    (
        Subject::Payments,
        &[
            "payment",
            "payments",
            "pay",
            "instalment",
            "installment",
            "bond",
            "rates",
            "rent",
            "due",
            "fees",
        ],
    ),
    (
        Subject::Prompts,
        &["prompt", "prompts", "eval", "evals", "library", "template"],
    ),
    (
        Subject::Retro,
        &[
            "retro",
            "retrogaming",
            "gaming",
            "game",
            "console",
            "arcade",
            "cartridge",
            "emulator",
        ],
    ),
    (
        Subject::Tech,
        &[
            "tech", "laptop", "ssd", "server", "nas", "keyboard", "device", "backup", "firmware",
        ],
    ),
    (
        Subject::Travels,
        &[
            "travel", "travels", "flight", "flights", "fly", "airport", "visa", "passport", "trip",
            "drive", "train", "hotel", "booking", "luggage",
        ],
    ),
];

fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push(' ');
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() || c == '-' {
            out.push(c);
        } else {
            out.push(' ');
        }
    }
    out.push(' ');
    out
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `\b<keyword>s?\b`, with hyphens treated as boundaries exactly as the reference does.
fn contains_word(hay: &str, keyword: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(keyword) {
        let start = from + pos;
        let end = start + keyword.len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let mut after = end;
        if after < bytes.len() && bytes[after] == b's' {
            after += 1;
        }
        let after_ok = after >= bytes.len() || !is_word(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

fn score(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|k| contains_word(text, k))
        .map(|k| k.len())
        .sum()
}

/// Longest total keyword match wins; ties go to the earlier row.
pub fn infer_mark(text: &str) -> Mark {
    let t = normalize(text);
    let mut best = Mark::Thing;
    let mut best_score = 0;
    for (mark, keywords) in MARK_KEYWORDS {
        let s = score(&t, keywords);
        if s > best_score {
            best_score = s;
            best = *mark;
        }
    }
    best
}

/// Same scoring as marks. `Me` is the fallback because the rest is you too.
pub fn infer_subject(text: &str) -> Subject {
    let t = normalize(text);
    let mut best = Subject::Me;
    let mut best_score = 0;
    for (subject, keywords) in SUBJECT_KEYWORDS {
        let s = score(&t, keywords);
        if s > best_score {
            best_score = s;
            best = *subject;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_from_the_spec_examples() {
        assert_eq!(infer_mark("Dentist, 09:20"), Mark::Health);
        assert_eq!(infer_mark("Visa interview"), Mark::Travel);
        assert_eq!(infer_mark("Mum's birthday"), Mark::People);
        assert_eq!(infer_mark("Housing bond first payment"), Mark::Money);
        assert_eq!(infer_mark("Contract ends — decide renewal"), Mark::Money);
        assert_eq!(
            infer_mark("Sketches — ink the Nelspruit series"),
            Mark::Place
        );
        assert_eq!(infer_mark("Retrogaming night with Thabo"), Mark::Thing);
        assert_eq!(infer_mark("Thabo's school fees"), Mark::Study);
    }

    #[test]
    fn subjects_from_the_spec_examples() {
        assert_eq!(infer_subject("Dentist, 09:20"), Subject::Me);
        assert_eq!(
            infer_subject("Housing bond first payment"),
            Subject::Payments
        );
        assert_eq!(
            infer_subject("Sketches — ink the Nelspruit series"),
            Subject::Sketches
        );
        assert_eq!(infer_subject("Visa interview"), Subject::Travels);
        assert_eq!(infer_subject("Thabo's school fees"), Subject::Kids);
        assert_eq!(
            infer_subject("something with no keywords at all"),
            Subject::Me
        );
    }

    #[test]
    fn longest_match_wins_and_plurals_are_quiet() {
        assert_eq!(infer_mark("flights"), Mark::Travel);
        assert_eq!(infer_mark("visas"), Mark::Travel);
        assert_eq!(infer_mark("passport photos"), Mark::Admin);
        assert_eq!(infer_mark("passport renewal"), Mark::Admin);
    }

    #[test]
    fn subject_query_is_exact_then_prefixed_and_refuses_ambiguity() {
        assert_eq!(Subject::from_query("travels"), Some(Subject::Travels));
        assert_eq!(Subject::from_query("TRV"), Some(Subject::Travels));
        assert_eq!(Subject::from_query("sketches"), Some(Subject::Sketches));
        assert_eq!(Subject::from_query("everyone"), Some(Subject::Everyone));
        assert_eq!(Subject::from_query("nonsense"), None);
        // "a" could be AI or Assets; refuse rather than guess.
        assert_eq!(Subject::from_query("a"), None);
    }

    #[test]
    fn all_marks_have_a_name() {
        for mark in Mark::ALL {
            assert!(!mark.name().is_empty());
            assert_eq!(Mark::from_id(mark.id()), Some(mark));
        }
        assert_eq!(Mark::ALL.len(), 18);
        assert_eq!(Subject::ALL.len(), 17);
    }
}
