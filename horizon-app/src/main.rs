mod agent;
mod app;
mod boot;
mod components;
mod desktop;
#[cfg(feature = "sync")]
mod git;
mod menu;
mod reminders;
mod state;
mod storage;
#[cfg(feature = "sync")]
mod sync;

/// `HORIZON_WINDOW=1500x1000` opens the window at that logical size. Useful when
/// the desktop is scaled and a screenshot needs a wall-shaped viewport.
fn window_size() -> Option<(f64, f64)> {
    let value = std::env::var("HORIZON_WINDOW").ok()?;
    let (w, h) = value.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn main() {
    // The systemd timer's entry point: deliver what is due and exit, before any
    // window or async runtime exists.
    if std::env::args().skip(1).any(|arg| arg == "--fire-due") {
        std::process::exit(reminders::fire_due_cli());
    }
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
