//! Horizon's pure core.
//!
//! Every product opinion lives here: the mark and subject vocabulary, the inference, the tier
//! derivation, the 90-day quarter maths, the composer grammar, the markdown parser, and the
//! export shapes. There is no DOM and no I/O.

pub mod composer;
pub mod dates;
pub mod export;
pub mod inference;
pub mod markdown;
pub mod model;
pub mod reminders;
pub mod seed;
pub mod store;

pub use composer::{
    parse as parse_composer, split_subject, ComposerError, ComposerPreview, ComposerResult,
};
pub use dates::{Quarter, SPAN_DAYS};
pub use inference::{infer_mark, infer_subject, Mark, Subject};
pub use markdown::{parse as parse_markdown, toggle_task_line, Block, Inline, ListItem};
pub use model::{
    note_key_entry, note_key_loose, note_key_page, Doc, DrawMode, Entry, Item, LooseNote,
    NoteTarget, PageMeta, Reminder, ReminderSource, Repeat, Tier, UiState, DOC_VERSION,
};
pub use reminders::{
    advance as advance_reminder, humanize, parse_when, repeat_phrase, When, WhenError,
};
pub use store::{from_files, to_files, StoreError, STORE_VERSION};
