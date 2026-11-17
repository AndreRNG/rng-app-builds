#!/bin/bash
# Copy the shared RNG Design tokens (design/rng_design.rs) into every app checkout and the launcher.
# usage: scripts/sync-design.sh [app-dir ...]   (default: all known checkouts)
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
src=$here/design/rng_design.rs
targets=("$@")
[ ${#targets[@]} -eq 0 ] && targets=(~/src/filmcraft-design ~/src/photocraft ~/src/pdfcraft ~/src/wordcraft ~/src/deckcraft)
for d in "${targets[@]}"; do
  dst=$d/crates/ui-egui/src/rng_design.rs
  [ -d "$d/crates/ui-egui/src" ] || { echo "skip $d"; continue; }
  [ -f "$dst" ] || { echo "skip $d (not adopted yet)"; continue; }
  cp "$src" "$dst" && echo "synced $dst"
done
[ -f "$here/launcher/src/rng_design.rs" ] && cp "$src" "$here/launcher/src/rng_design.rs" && echo "synced launcher"
