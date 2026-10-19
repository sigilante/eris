#!/usr/bin/env bash
# Apply Eris's patch series onto the pinned third_party submodules.
# Usage: patches/apply.sh [vere|nockchain]   (no arg = all)
#
# Idempotent-ish: a patch already applied is detected (git apply --reverse
# --check) and skipped, so re-running is safe. Leaves submodule working trees
# patched; does NOT touch the recorded gitlink.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
targets=("${1:-vere nockchain}")
[ "${1:-}" = "" ] && targets=(vere nockchain)

for name in ${targets[@]}; do
  sub="$root/third_party/$name"
  pdir="$root/patches/$name"
  [ -d "$sub" ] || { echo "!! missing submodule: third_party/$name" >&2; exit 1; }
  shopt -s nullglob
  patches=("$pdir"/*.patch)
  shopt -u nullglob
  if [ ${#patches[@]} -eq 0 ]; then
    echo "== $name: no patches"
    continue
  fi
  for p in "${patches[@]}"; do
    if git -C "$sub" apply --reverse --check "$p" >/dev/null 2>&1; then
      echo "== $name: already applied, skipping $(basename "$p")"
    elif git -C "$sub" apply --check "$p" >/dev/null 2>&1; then
      git -C "$sub" apply "$p"
      echo "== $name: applied $(basename "$p")"
    else
      echo "!! $name: $(basename "$p") does not apply against the current pin." >&2
      echo "   Bump needs a refreshed patch; see patches/README.md + CHANGES.md." >&2
      exit 1
    fi
  done
done
