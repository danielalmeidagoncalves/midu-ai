#!/bin/sh
# Dioxus desktop pulls in muda, which links libxdo on Linux. When xdotool is not
# installed system-wide, the project keeps a local build in ~/.local/lib.
# Add it to the linker/loader paths when it exists, then run the given command.
set -eu

if [ -n "${HOME:-}" ] && [ -d "$HOME/.local/lib" ]; then
  LIBRARY_PATH="$HOME/.local/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
  LD_LIBRARY_PATH="$HOME/.local/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export LIBRARY_PATH LD_LIBRARY_PATH
fi

exec "$@"
