#!/usr/bin/env bash
# Build what Horizon ships: a Linux desktop package.
#
#   tools/release.sh desktop   -> dist/horizon-app_<version>_amd64.deb
#   tools/release.sh appimage  -> dist/Horizon-<version>-x86_64.AppImage (needs a system libxdo)
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

case "${1:-}" in
  desktop) desktop ;;
  appimage) appimage ;;
  all) desktop; appimage ;;
  *)
    echo "usage: $0 {desktop|appimage|all}" >&2
    exit 2
    ;;
esac
