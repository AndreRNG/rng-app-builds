#!/usr/bin/env python3
"""Writes launcher/assets/THIRD-PARTY.txt: every crate RNG Studio links on any platform, with
its licence and the licence/notice files it ships. Run after changing launcher dependencies."""
import glob, json, os, subprocess, sys
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "launcher")
meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1"], cwd=root))
# Only crates that end up in a shipped binary (normal deps) on the platforms we build.
shipped = set()
for t in ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "aarch64-apple-darwin", "x86_64-apple-darwin"):
    tree = subprocess.check_output(["cargo", "tree", "-e", "normal", "--target", t, "--prefix", "none", "-f", "{p}"], cwd=root, text=True)
    shipped |= {" ".join(l.split()[:2]) for l in tree.splitlines() if l.strip()}
texts = {}
bad, out = [], []
for p in sorted(meta["packages"], key=lambda p: p["name"]):
    if p["name"] == "rng-studio" or f"{p['name']} v{p['version']}" not in shipped:
        continue
    lic = p.get("license") or "SEE LICENSE FILE"
    if any(x in lic for x in ("GPL", "AGPL", "NC")) and "OR" not in lic:
        bad.append(f"{p['name']} {lic}")
    d = os.path.dirname(p["manifest_path"])
    files = sorted(f for f in glob.glob(os.path.join(d, "*"))
                   if os.path.basename(f).upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")) and os.path.isfile(f))
    out.append("=" * 78 + f"\n{p['name']} {p['version']}  ({lic})\n" + (p.get("repository") or "") + "\n")
    seen = set()
    for f in files:
        t = open(f, encoding="utf-8", errors="replace").read().strip()
        if t in seen:
            continue
        seen.add(t)
        if t in texts:
            out.append(f"--- {os.path.basename(f)}: same text as {texts[t]} above\n")
        else:
            texts[t] = f"{p['name']} {os.path.basename(f)}"
            out.append(f"--- {os.path.basename(f)}\n{t}\n")
head = ("RNG Studio (c) RNG Media S.L.\n\nRNG Studio is built with the open-source components below. "
        "Each is used under the licence shown; their copyright and licence texts follow.\n\n")
open(os.path.join(root, "assets", "THIRD-PARTY.txt"), "w").write(head + "\n".join(out))
print(len(out), "components;", "flags:", bad or "none")
sys.exit(1 if bad else 0)
