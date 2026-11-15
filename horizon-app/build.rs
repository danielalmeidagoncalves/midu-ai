//! One build-time flag for the shell this build is for.
//!
//! Horizon has exactly two targets: a desktop window and an Android phone. Most
//! of the code is shared, but a handful of items are desktop-only (the menu,
//! quit, the folder picker) or phone-only (the app-files data dir). Rather than
//! repeat `all(not(target_os = "android"), not(target_os = "ios"))` on every
//! item, the build script emits one cfg so the source says `#[cfg(desktop_shell)]`.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(desktop_shell)");
    let target = std::env::var("TARGET").unwrap_or_default();
    if let Some(os) = target.split('-').nth(2) {
        if os != "android" && os != "ios" {
            println!("cargo::rustc-cfg=desktop_shell");
        }
    }
}
