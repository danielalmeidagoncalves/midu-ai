#!/bin/sh
# Serve the repository's local OpenWiki wiki for browsing and link exploration.
# Prefer a globally installed CLI, then the Pi package's bundled binary, then npx.
set -eu

ROOT=$(cd "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

if command -v openwiki >/dev/null 2>&1; then
  exec openwiki visualize "$@"
fi

if [ -n "${HOME:-}" ] && [ -x "$HOME/.pi/agent/npm/node_modules/.bin/openwiki" ]; then
  exec "$HOME/.pi/agent/npm/node_modules/.bin/openwiki" visualize "$@"
fi

if command -v npx >/dev/null 2>&1; then
  exec npx --yes openwiki visualize "$@"
fi

echo "OpenWiki CLI not found. Install it with: npm install -g openwiki" >&2
exit 1
