#!/usr/bin/env bash
# release-local.sh <run-id> <app> : do a build.yml run's "release" step from this PC (when only that job
# failed, e.g. blocked by GitHub billing). Downloads the run's packages, writes <app>.json and makes
# the private release <app>-b<run_number>, exactly as the workflow would.
set -euo pipefail
R=AndreRNG/rng-app-builds
run=$1
here=$(cd "$(dirname "$0")/.." && pwd)
num=$(gh api repos/$R/actions/runs/$run --jq .run_number)
sha=$(gh api repos/$R/actions/runs/$run --jq .head_sha)
app=$2
work=~/RNG/work/release-local/$run
rm -rf "$work"; mkdir -p "$work"
gh run download "$run" -R $R -D "$work/a"
mkdir -p "$work/dist"; find "$work/a" -type f -exec mv {} "$work/dist/" \;
git -C "$here" show "$sha:apps.json" > "$work/apps.json"
git -C "$here" show "$sha:patches/$app.base" > "$work/base"
python3 - "$work" "$app" "$num" <<'PY'
import hashlib, json, os, re, subprocess, sys
work, app, num = sys.argv[1], sys.argv[2], int(sys.argv[3])
meta = json.load(open(f"{work}/apps.json"))[app]
base = open(f"{work}/base").read().strip()
src = os.path.expanduser(meta["src"])
toml = subprocess.check_output(["git", "-C", src, "show", f"{base}:Cargo.toml"], text=True)
version = re.search(r'(?m)^version\s*=\s*"([^"]+)"', toml).group(1)
d = f"{work}/dist"
plat = {}
for f in sorted(os.listdir(d)):
    m = re.match(r".*-b\d+-(.+)\.(tar\.gz|zip)$", f)
    if not m:
        continue
    p = m.group(1)
    exe = open(f"{d}/{p}.exe.txt").read().strip().split("=", 1)[1]
    data = open(f"{d}/{f}", "rb").read()
    plat[p] = {"file": f, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data), "exe": exe}
out = {"id": app, "name": meta["name"], "tagline": meta["tagline"], "wm_class": meta.get("wm_class", ""),
       "version": version, "build": num, "upstream_commit": base, "platforms": plat}
json.dump(out, open(f"{d}/{app}.json", "w"), indent=2)
print(json.dumps(out, indent=2))
assert plat, "no packages"
PY
cd "$work/dist"
name=$(python3 -c "import json;m=json.load(open('$app.json'));print(m['name'],m['version'])")
gh release create "$app-b$num" -R $R --title "$name (build $num)" \
  --notes "Platforms: $(python3 -c "import json;print(', '.join(json.load(open('$app.json'))['platforms']))") (released from run $run on the local PC)" \
  $(ls *.tar.gz *.zip) "$app.json"
