// `desktop_shell` is set by a build script for every target except Android and
// iOS, so conditional code reads `#[cfg(desktop_shell)]` instead of repeating a
// two-target list on every item.
#[cfg(not(desktop_shell))]
mod android;
mod app;
mod boot;
mod components;
#[cfg(desktop_shell)]
mod desktop;
#[cfg(feature = "sync")]
mod git;
#[cfg(desktop_shell)]
mod menu;
mod state;
mod storage;
#[cfg(feature = "sync")]
mod sync;

/// `HORIZON_WINDOW=1500x1000` opens the window at that logical size. Useful when
/// the desktop is scaled and a screenshot needs a wall-shaped viewport.
#[cfg(desktop_shell)]
fn window_size() -> Option<(f64, f64)> {
    let value = std::env::var("HORIZON_WINDOW").ok()?;
    let (w, h) = value.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn main() {
    #[cfg(all(desktop_shell, feature = "desktop"))]
    {
        let mut window = dioxus::desktop::WindowBuilder::new().with_title("Horizon");
        if let Some((w, h)) = window_size() {
            use dioxus::desktop::tao::dpi::LogicalSize;
            window = window.with_inner_size(LogicalSize::new(w, h));
        }
        let cfg = dioxus::desktop::Config::new().with_window(window);
        dioxus::LaunchBuilder::desktop()
            .with_cfg(cfg)
            .launch(app::App);
    }
    #[cfg(not(all(desktop_shell, feature = "desktop")))]
    dioxus::launch(app::App);
}
