//! Naming the store for the in-window menu, plus the shortcut line the page
//! prints so nobody hunts for a system menubar this toolkit does not draw.

/// The folder that holds the Markdown store, phrased for a person.
pub fn store_location() -> String {
    match crate::storage::store_path() {
        Some(path) => format!("Markdown lives in {path}"),
        None => "The Markdown store is not open.".to_string(),
    }
}

/// The shortcut to the menu, printed once in the footer so nobody hunts for a
/// system menubar that this toolkit does not draw.
pub const QUIT_HINT: &str = "Alt+H opens the menu";
