#!/usr/bin/env bash
# Export an app's local `rng` branch as patches/<app>.patch on top of patches/<app>.base,
# commit, push, and start the cross-platform build.   sync-patch.sh cutcat [--no-build]
set -euo pipefail
cd "$(dirname "$0")/.."
app=$1
src=$(python3 -c "import json,os,sys;print(os.path.expanduser(json.load(open('apps.json'))[sys.argv[1]]['src']))" "$app")
[ "$(git -C "$src" branch --show-current)" = rng ] || { echo "$src is not on rng" >&2; exit 1; }
[ -z "$(git -C "$src" status --porcelain)" ] || { echo "$src has uncommitted changes" >&2; exit 1; }
base=$(git -C "$src" merge-base origin/main rng)
git -C "$src" format-patch --binary --stdout "$base..rng" > "patches/$app.patch"
echo "$base" > "patches/$app.base"
git add "patches/$app.patch" "patches/$app.base"
git diff --cached --quiet || git commit -qm "$app: patch on upstream ${base:0:10}"
git push -q origin main
[ "${2:-}" = --no-build ] || gh workflow run build.yml -f app="$app"
echo "synced $app (base ${base:0:10})"
