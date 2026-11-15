#!/usr/bin/env bash
# Build the two things Horizon ships: a desktop package and an Android APK.
#
#   tools/release.sh desktop   -> dist/horizon-app_<version>_amd64.deb
#   tools/release.sh appimage  -> dist/Horizon-<version>-x86_64.AppImage (needs a system libxdo)
#   tools/release.sh android   -> dist/Horizon-<version>.apk          (needs the Android SDK + NDK)
#   tools/release.sh all       -> desktop and android
#
# Nothing here is clever: each recipe is the `dx bundle` invocation that works,
# with the two environment variables this machine needs folded in.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

# libxdo is installed under ~/.local on this machine; the linker needs to find it.
if [ -d "$HOME/.local/lib" ]; then
  export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$HOME/.local/lib"
fi

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="$root/dist"
mkdir -p "$OUT"

desktop() {
  dx bundle --package horizon-app --release --platform linux \
    --package-types deb --out-dir "$OUT"
  echo "desktop: $OUT/horizon-app_${VERSION}_amd64.deb"
}

appimage() {
  # linuxdeploy refuses to bundle while any NEEDED library is missing. On Arch
  # there is no packaged libxdo, so install it or point LD_LIBRARY_PATH at a copy.
  dx bundle --package horizon-app --release --platform linux \
    --package-types appimage --out-dir "$OUT"
  echo "appimage: see $OUT"
}

android() {
  : "${ANDROID_HOME:=${ANDROID_SDK_ROOT:-}}"
  : "${ANDROID_NDK_HOME:=${ANDROID_NDK_ROOT:-$ANDROID_HOME/ndk}}"
  export ANDROID_HOME ANDROID_SDK_ROOT="$ANDROID_HOME" ANDROID_NDK_HOME ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
  # Android has no system OpenSSL, so the git client compiles its own.
  dx bundle --package horizon-app --release --platform android \
    --package-types apk --features mobile,sync-vendored --out-dir "$OUT"
  ls -1 "$OUT"/*.apk
}

case "${1:-}" in
  desktop) desktop ;;
  appimage) appimage ;;
  android) android ;;
  all) desktop; android ;;
  *)
    echo "usage: $0 {desktop|appimage|android|all}" >&2
    exit 2
    ;;
esac
