#!/usr/bin/env bash
# Build an installable Arch package for the current checkout:
#   tools/package.sh             # -> dist/horizon-<version>-1-x86_64.pkg.tar.zst
#   sudo pacman -U dist/horizon-*.pkg.tar.zst
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
recipe="$root/packaging"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)
[ -n "$version" ] || { echo "No workspace version in Cargo.toml" >&2; exit 1; }
mkdir -p "$root/dist" "$recipe/bin"
# Keep the Arch package version aligned with the workspace version.
sed -i "s/^pkgver=.*/pkgver=$version/" "$recipe/PKGBUILD"
cleanup() { rm -rf "${recipe:?}/bin"; }
trap cleanup EXIT

# Build once with Cargo in the checkout, then have makepkg package that exact
# release binary. This keeps Cargo's target cache reusable between releases and
# avoids compiling the vendored libgit2 twice.
if [ -d "$HOME/.local/lib" ]; then
  export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$HOME/.local/lib"
fi
cargo build --release --locked -p horizon-app
install -m755 "$root/target/release/horizon-app" "$recipe/bin/horizon-app"

# --nodeps skips makepkg's local dependency check because libxdo is in ~/.local
# on this host rather than an installed xdotool package. The package still
# declares its runtime dependencies; pacman checks them when you install it.
PKGDEST="$root/dist" makepkg -D "$recipe" -f --nodeps "${@}"
built=$(find "$root/dist" -maxdepth 1 -type f -name "horizon-${version}-*.pkg.tar.*" -print -quit)
[ -n "$built" ] || { echo "makepkg did not produce a package" >&2; exit 1; }
echo "package: $built"
echo "install: sudo pacman -U $built"
