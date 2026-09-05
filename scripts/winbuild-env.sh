#!/usr/bin/env bash
# Prepares the Git Bash environment for Rust/MSVC builds on this machine.
#
# Two machine quirks (see docs/DECISIONS.md D4/D5):
# 1. MSYS2's GNU `link` shadows MSVC link.exe  -> prepend the MSVC bin dir.
# 2. The Windows SDK physically lives at G:\WindowsSDK\10 behind a junction;
#    tooling discovers it through the standard path, so nothing extra is
#    needed here, but we verify it exists to fail fast with a clear message.
#
# Usage:  source scripts/winbuild-env.sh   (then run cargo normally)

set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

MSVC_ROOT="/c/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/VC/Tools/MSVC"
if [[ -d "$MSVC_ROOT" ]]; then
  MSVC_VER=$(ls "$MSVC_ROOT" | sort -V | tail -1)
  export PATH="$MSVC_ROOT/$MSVC_VER/bin/Hostx64/x64:$PATH"
else
  echo "warning: MSVC BuildTools not found at $MSVC_ROOT" >&2
fi

if [[ ! -f "/c/Program Files (x86)/Windows Kits/10/Lib" && ! -d "/c/Program Files (x86)/Windows Kits/10/Lib" ]]; then
  echo "warning: Windows SDK Lib dir not reachable through the junction" >&2
fi

command -v link.exe >/dev/null 2>&1 || {
  echo "error: link.exe still not resolvable; MSVC environment broken" >&2
  exit 1
}
command -v cargo >/dev/null 2>&1 || {
  echo "error: cargo not found; is ~/.cargo/bin present?" >&2
  exit 1
}
