#!/usr/bin/env bash
# publish.sh <tag>... : copy finished builds from the private repo's releases to the PUBLIC
# AndreRNG/launchllama releases and rewrite its manifest.json (what LaunchLlama reads through
# www.rng.media/apps/manifest.json). Tags look like cutcat-b12 or launchllama-b7; "latest" = the
# newest release of every app and the launcher.
set -euo pipefail
PRIV=AndreRNG/rng-app-builds  # public now; name kept
PUB=AndreRNG/launchllama
here=$(cd "$(dirname "$0")/.." && pwd)
work=~/RNG/work/launchllama-publish
pubdir=~/src/launchllama-public
mkdir -p "$work"
[ -d "$pubdir/.git" ] || gh repo clone "$PUB" "$pubdir"
git -C "$pubdir" pull -q --ff-only

tags=("$@")
if [ "${tags[0]:-}" = latest ]; then
  mapfile -t tags < <(gh release list -R "$PRIV" --limit 200 --json tagName --jq '.[].tagName' \
    | python3 -c 'import sys
seen={}
for t in sys.stdin.read().split():
    app=t.rsplit("-b",1)[0]
    seen.setdefault(app,t)
print("\n".join(seen.values()))')
fi

for tag in "${tags[@]}"; do
  id=${tag%-b*}
  d="$work/$tag"
  rm -rf "$d"; mkdir -p "$d"
  echo "== $tag"
  gh release download "$tag" -R "$PRIV" -D "$d"
  python3 - "$d" "$id" <<'PY'
import hashlib, json, os, sys
d, id = sys.argv[1], sys.argv[2]
meta = json.load(open(os.path.join(d, f"{id}.json")))
for p, a in meta["platforms"].items():
    h = hashlib.sha256(open(os.path.join(d, a["file"]), "rb").read()).hexdigest()
    assert h == a["sha256"], f"{a['file']}: checksum mismatch"
print("checksums ok:", ", ".join(meta["platforms"]))
PY
  latest=false; [ "$id" = launchllama ] && latest=true
  files=$(cd "$d" && ls | grep -v '\.json$' | sed "s|^|$d/|")
  if [ "$id" = launchllama ]; then
    # Stable names so www.rng.media/launchllama links never change.
    for f in $files; do
      case $f in
        *-linux-x86_64.tar.gz) cp "$f" "$d/LaunchLlama-linux-x86_64.tar.gz" ;;
        *-windows-x86_64.exe)  cp "$f" "$d/LaunchLlama-windows.exe" ;;
        *-macos.dmg)           cp "$f" "$d/LaunchLlama-mac.dmg" ;;
      esac
    done
    files=$(cd "$d" && ls | grep -v '\.json$' | sed "s|^|$d/|")
  fi
  if gh release view "$tag" -R "$PUB" >/dev/null 2>&1; then
    gh release upload "$tag" -R "$PUB" --clobber $files
  else
    title=$(python3 -c 'import json,sys;m=json.load(open(sys.argv[1]));print(m["name"], m["version"], "(build %s)" % m["build"])' "$d/$id.json")
    gh release create "$tag" -R "$PUB" --title "$title" --latest=$latest \
      --notes "Install and update with LaunchLlama: https://www.rng.media/launchllama" $files
  fi
  python3 - "$d/$id.json" "$pubdir/manifest.json" "$tag" "$PUB" "$here/apps.json" <<'PY'
import json, os, sys
entry_path, man_path, tag, pub, apps_path = sys.argv[1:]
e = json.load(open(entry_path))
order = list(json.load(open(apps_path)))
m = json.load(open(man_path)) if os.path.exists(man_path) else {"schema": 1, "launcher": None, "apps": []}
for a in e["platforms"].values():
    a["url"] = f"https://github.com/{pub}/releases/download/{tag}/{a.pop('file')}"
e.pop("upstream_commit", None)
if e["id"] == "launchllama":
    m["launcher"] = e
else:
    m["apps"] = [x for x in m["apps"] if x["id"] != e["id"]] + [e]
    m["apps"].sort(key=lambda x: order.index(x["id"]) if x["id"] in order else 99)
json.dump(m, open(man_path, "w"), indent=2)
print("manifest:", e["id"], e["version"], "build", e["build"])
PY
done

# Licences, notices and our change list for every published app (linked from the download page).
for tag in "${tags[@]}"; do
  id=${tag%-b*}
  dst="$pubdir/licences/$id"; rm -rf "$dst"; mkdir -p "$dst"
  if [ "$id" = launchllama ]; then
    cp "$here/launcher/assets/THIRD-PARTY.txt" "$dst/"
  else
    src=$(python3 -c "import json,os,sys;print(os.path.expanduser(json.load(open(sys.argv[1]))[sys.argv[2]]['src']))" "$here/apps.json" "$id")
    for f in LICENSE-MIT LICENSE-APACHE NOTICE MODIFICATIONS.md; do git -C "$src" show "rng:$f" > "$dst/$f" 2>/dev/null || rm -f "$dst/$f"; done
    git -C "$src" archive rng licenses 2>/dev/null | tar -x -C "$dst" || true
  fi
done

cd "$pubdir"
git add manifest.json licences
git diff --cached --quiet || { git commit -qm "manifest: ${tags[*]}"; git push -q; }
echo "published: ${tags[*]}"
