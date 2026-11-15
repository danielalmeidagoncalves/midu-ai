pub mod agenda;
pub mod brief;
pub mod chrome;
pub mod fold;
pub mod index;
pub mod labels;
pub mod margin;
pub mod markdown;
pub mod marks;
#[cfg(desktop_shell)]
pub mod menu;

#[cfg(desktop_shell)]
pub use menu::MenuStrip;
pub mod notes;
#[cfg(feature = "sync")]
pub mod sync;
pub mod wall;
