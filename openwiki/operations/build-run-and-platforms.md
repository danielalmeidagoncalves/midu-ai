---
type: operations-guide
title: Build, run, debug and platform operations
description: Commands and feature gates for Horizon desktop and Android, the Linux libxdo workaround, the Markdown store locations, the Markdown copy destinations, Zed tasks and CodeLLDB profiles, and JSON import/export.
tags: [operations, build, desktop, android, zed, debugging]
sources:
  - id: openwiki-source-81811552667c74c81f90582e
    resource: repo://.zed/debug.json
  - id: openwiki-source-ff8e704eea33edb3b019211a
    resource: repo://.zed/tasks.json
  - id: openwiki-source-651d1fb6c9e49916a916ab51
    resource: repo://Cargo.toml
  - id: openwiki-source-d5e1da136cea57fb52b1c9e5
    resource: repo://Dioxus.toml
  - id: openwiki-source-d1eead8d200b68e1165d8f17
    resource: repo://HANDOFF.md
  - id: openwiki-source-fa908abff259e47afe8cb7fd
    resource: repo://horizon-app/Cargo.toml
  - id: openwiki-source-d42883e743c5f7cafd49a2bd
    resource: repo://horizon-app/src/state.rs
  - id: openwiki-source-e965f5c1e6b44694ef40854f
    resource: repo://horizon-app/src/storage.rs
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-1edec297169af95a93474027
    resource: repo://tools/with-libxdo.sh
generated: { by: "pi", at: "2026-11-04T22:52:59.779Z" }
verified:
  - by: openwiki/0.7.3
    at: 2026-11-04T22:52:59.779Z
---

# Build, run, debug and platform operations

Horizon is one workspace and one UI crate. The platform choice is a Cargo feature, not a fork.

```text
Cargo.toml                 workspace: horizon-core + horizon-app
horizon-app/Cargo.toml     default = ["desktop", "sync"]; mobile; sync; sync-vendored
Dioxus.toml                app name, identifier, default_platform = "desktop"
```

`horizon-app`'s platform features map directly to Dioxus backends: `desktop = ["dioxus/desktop"]` and `mobile = ["dioxus/mobile"]`. The desktop feature is the default, so `cargo run -p horizon-app` is the native build; Android uses `--no-default-features --features mobile`. `sync` (on by default) adds `git2` and the git client; `sync-vendored` adds `git2/vendored-openssl` for targets without a system OpenSSL. libgit2 is the engine because gitoxide cannot push (`gix` 0.90 has no receive-pack client). The NDK is required to link any Android build, not because of git; sync adds the bundled C sources and vendored OpenSSL on top. A `build.rs` sets one `desktop_shell` cfg for every non-Android, non-iOS target, so desktop-only items read `#[cfg(desktop_shell)]`.

## Desktop

```sh
cargo run -p horizon-app
# or through the Dioxus CLI
dx serve --package horizon-app --desktop
```

Linux needs WebKitGTK and GTK. Dioxus desktop also pulls in `muda`, which links `libxdo` on Linux. If `xdotool` is installed system-wide, nothing else is needed. If `libxdo` lives in a local prefix, the linker and loader need it:

```sh
LIBRARY_PATH=$HOME/.local/lib LD_LIBRARY_PATH=$HOME/.local/lib \
  cargo run -p horizon-app
```

The same pair is needed at *runtime*, so `cargo test -p horizon-app` takes the identical prefix.

The desktop window draws its own menu strip (see [Surfaces](../surfaces/ui-and-accessibility.md)). The system menubar from `muda` shows submenu titles but no entries on this GTK build, so nothing depends on it: Quit lives in the in-window menu and on `Ctrl+Q`.

The repository wraps that in [`tools/with-libxdo.sh`](../../tools/with-libxdo.sh). The script checks whether `$HOME/.local/lib` exists, prepends it to `LIBRARY_PATH` and `LD_LIBRARY_PATH`, and then `exec`s the requested command. When the directory does not exist it is a no-op. The Zed desktop tasks and the debug build task call this wrapper, so the same commands work with or without a system `libxdo`.

The first launch is empty. `HORIZON_DEMO=1` opts into the sample quarter only when the store is empty:

```sh
HORIZON_DEMO=1 cargo run -p horizon-app
```

## No web target

Horizon has no browser build. It is a desktop and Android app with no server component and nothing to deploy; the only platform split in the code is desktop-versus-Android. The release profile is tuned for size (`opt-level = "z"`, LTO, one codegen unit, debuginfo stripped), and `App` injects the stylesheet into the webview's document head at startup.

## Android

The same crate and the same components switch to the vertical week-by-week agenda below 700px. The documented build is:

```sh
rustup target add aarch64-linux-android
# without sync (no NDK needed beyond the linker)
cargo build -p horizon-app --no-default-features --features mobile --target aarch64-linux-android

# with sync: libgit2 is C and Android has no system OpenSSL
cargo build -p horizon-app --no-default-features --features mobile,sync-vendored \
  --target aarch64-linux-android

# with an Android SDK/NDK:
dx build --package horizon-app --platform android
```

Android writes the same Markdown tree as the desktop build. The live store is app-private Markdown, addressed through `sysdirs` with the `android-auto` feature. The Markdown copy goes to the app's external files directory (`getExternalFilesDir`, called over JNI in `horizon-app/src/android.rs`) so it is reachable over USB and from a file manager, with the app-private folder as the fallback. The recorded environment had no Android SDK/NDK: the target was cargo-checked, but the app was not built or device-verified, and the `mobile,sync-vendored` build has not been run.

## Zed tasks and debug profiles

`.zed/tasks.json` contains the project tasks:

- run desktop with demo or without; serve desktop with hot reload;
- build desktop;
- test core; test app; check workspace; check android; clippy; fmt check/write; contrast; serve wiki.

The `check android` task uses `--no-default-features --features mobile`, so it does not need the NDK and does not compile the git client. A sync build is the longer `mobile,sync-vendored` command above.

The desktop run/serve/build tasks call the libxdo wrapper. Long-running tasks reuse a terminal and allow rerun to cancel the previous process. One-shot tasks cover core tests, app tests, the workspace check, an Android `cargo check`, clippy, fmt and the contrast script.

`.zed/debug.json` contains two CodeLLDB launch profiles: demo and no-demo. Both build through the `Horizon: build desktop` task, launch `target/debug/horizon-app`, set `sourceLanguages = ["rust"]`, and set `RUST_BACKTRACE=1`. `LD_LIBRARY_PATH` is `${env:HOME}/.local/lib` so the debuggee can find a local `libxdo`. The first debug launch may download Zed's CodeLLDB adapter.

## Data and store locations

The live store on desktop and Android is a folder of plain Markdown files: one per entry, one per note, one per day page, plus `state.md`. The default desktop path is `dirs::data_dir()/horizon` (usually `~/.local/share/horizon`). Android uses the `sysdirs` app files directory. `HORIZON_STORE` overrides the folder for tests or an explicit location.

The first load reads the Markdown folder. An empty folder means a first launch: quiet by default, or the demo quarter under `HORIZON_DEMO=1`.

The brief exposes the portable paths:

- **JSON download** of the current document, and **Restore from JSON** which replaces everything through `export::from_json` (and then writes the Markdown folder).
- **Write a copy of the Markdown store**: `store::to_files` minus `state.md`, plus the same `README.md` the live folder carries. Desktop writes a timestamped folder under Downloads; Android writes to the app's external files directory and falls back to the app-private folder. The copy and the store are the same shape, so a folder moves between devices without conversion.

The store folder is also a git repository when `sync` is on. `data_dir()/horizon-sync/token` holds the personal access token, mode 0600 on Unix, beside the store so it cannot be committed. There is no account, telemetry or settings screen. JSON is the whole-document portability boundary; the Markdown copy is the same store the app already keeps.

## Recorded verification

`cargo test -p horizon-core` passed 63 tests and `cargo test -p horizon-app` passed 6 tests (store write/read, stale-file cleanup, v1→v2 name migration, and three git tests; the app tests need the local `libxdo` path). The git tests drive a real bare repository as the remote: two device folders, a fast-forward, a disjoint merge, a same-file conflict whose aside file is identical on both sides, and no `state.md` in any commit. `cargo check`, `cargo clippy`, `cargo fmt --check`, the desktop build and the Android `cargo check` all completed in the recorded environment. A native end-to-end run with `HORIZON_STORE=/tmp/horizon-e2e HORIZON_DEMO=1` produced 31 entry files, 4 note files, 6 day files and `state.md`; a second launch loaded the same folder without duplicating files. A sync run against a bare remote committed `Wrote 31 entries, 4 notes, 6 day pages, 1 file` after twenty quiet seconds and pushed 43 files; a second empty store pulled all 31 entries, merged the unrelated histories and pushed the merge. `dx build --desktop` initially required a locally built `libxdo`; a normal machine can install `xdotool` instead. See [Tests, invariants and verification](../quality/testing-and-invariants.md) for the behavior those checks protect.
