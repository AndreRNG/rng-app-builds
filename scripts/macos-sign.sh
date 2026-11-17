#!/usr/bin/env bash
# macos-sign.sh <App.app> <out-dir> <file-stem> [entitlements.plist]
# Makes <out-dir>/<stem>.zip (for RNG Studio) and <out-dir>/<stem>.dmg (drag-to-Applications window).
# With the Apple secrets set (APPLE_CERTIFICATE = base64 .p12, APPLE_CERTIFICATE_PASSWORD,
# APPLE_API_KEY = base64 .p8, APPLE_API_KEY_ID, APPLE_API_ISSUER) the app and the disk image are
# Developer ID signed (hardened runtime), notarized, stapled and checked with Gatekeeper, so they
# open with no warning. Without them the app is ad-hoc signed (users right-click → Open once).
# Public repo: secrets only ever live in env vars and a throwaway keychain/files in $RUNNER_TEMP.
set -euo pipefail
app=$1 out=$2 stem=$3 ent=${4:-}
here=$(cd "$(dirname "$0")/.." && pwd)
[ -n "$ent" ] || ent="$here/mac/entitlements/default.plist"
vol=$(basename "$app" .app)
tmp="$RUNNER_TEMP/macsign"; rm -rf "$tmp"; mkdir -p "$tmp" "$out"
signed=false
[ -n "${APPLE_CERTIFICATE:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && signed=true

cleanup() {
  security delete-keychain "$tmp/sign.keychain-db" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

notarize() {  # notarize <file> : submit, wait, fail loudly with Apple's log if not Accepted
  local res status id
  res=$(xcrun notarytool submit "$1" --key "$tmp/key.p8" --key-id "$APPLE_API_KEY_ID" \
    --issuer "$APPLE_API_ISSUER" --wait --timeout 45m --output-format json) || true
  status=$(echo "$res" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("status",""))' 2>/dev/null || true)
  id=$(echo "$res" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))' 2>/dev/null || true)
  echo "notarization of $(basename "$1"): ${status:-failed} ($id)"
  if [ "$status" != Accepted ]; then
    [ -n "$id" ] && xcrun notarytool log "$id" --key "$tmp/key.p8" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER" || echo "$res"
    echo "::error::Apple did not accept $(basename "$1")"; exit 1
  fi
}

# Python libraries and similar must not write into the signed bundle at run time.
find "$app" \( -name __pycache__ -o -name .DS_Store \) -prune -exec rm -rf {} +
xattr -cr "$app"

if $signed; then
  kc="$tmp/sign.keychain-db"; kcpw=$(openssl rand -hex 24)
  echo "$APPLE_CERTIFICATE" | base64 --decode > "$tmp/cert.p12"
  echo "$APPLE_API_KEY" | base64 --decode > "$tmp/key.p8"
  security create-keychain -p "$kcpw" "$kc"
  security set-keychain-settings -lut 3600 "$kc"
  security unlock-keychain -p "$kcpw" "$kc"
  security import "$tmp/cert.p12" -P "$APPLE_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$kc" >/dev/null
  rm -f "$tmp/cert.p12"
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$kcpw" "$kc" >/dev/null
  security list-keychains -d user -s "$kc" $(security list-keychains -d user | tr -d '"')
  id=$(security find-identity -v -p codesigning "$kc" | grep -m1 '"Developer ID Application' | awk '{print $2}')
  [ -n "$id" ] || { echo "::error::APPLE_CERTIFICATE holds no Developer ID Application certificate"; exit 1; }
  sign=(codesign --force --timestamp --options runtime --keychain "$kc" -s "$id")

  # Inside out: every library/helper first, the app bundle last (no --deep, Apple's advice).
  while IFS= read -r -d '' f; do
    file -b "$f" | grep -q Mach-O || continue
    if file -b "$f" | grep -q executable; then "${sign[@]}" --entitlements "$ent" "$f"; else "${sign[@]}" "$f"; fi
  done < <(find "$app/Contents" -type f -print0)
  while IFS= read -r -d '' b; do "${sign[@]}" "$b"; done \
    < <(find "$app/Contents" -depth -mindepth 2 \( -name '*.framework' -o -name '*.app' -o -name '*.bundle' \) -type d -print0)
  "${sign[@]}" --entitlements "$ent" "$app"
  codesign --verify --deep --strict --verbose=2 "$app"

  ditto -c -k --keepParent "$app" "$tmp/notarize.zip"
  notarize "$tmp/notarize.zip"
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  spctl -a -vvv -t exec "$app"
else
  echo "::warning::Apple signing secrets not set: ad-hoc signing $app (macOS will warn on first open)"
  codesign --force --deep -s - "$app"
fi

ditto -c -k --keepParent "$app" "$out/$stem.zip"

# Disk image: app + Applications shortcut over the arrow background.
python3 -m venv "$tmp/venv" && "$tmp/venv/bin/pip" install -q 'dmgbuild>=1.6,<2'
tiffutil -cathidpicheck "$here/mac/dmg/background.png" "$here/mac/dmg/background@2x.png" -out "$tmp/bg.tiff" >/dev/null
"$tmp/venv/bin/dmgbuild" -s "$here/mac/dmg/settings.py" -D app="$app" -D bg="$tmp/bg.tiff" "$vol" "$out/$stem.dmg"

if $signed; then
  codesign --force --timestamp --keychain "$kc" -s "$id" "$out/$stem.dmg"
  notarize "$out/$stem.dmg"
  xcrun stapler staple "$out/$stem.dmg"
  xcrun stapler validate "$out/$stem.dmg"
  spctl -a -vvv -t open --context context:primary-signature "$out/$stem.dmg"
  # The app as a user gets it: mount, check the copy inside with Gatekeeper.
  mnt=$(hdiutil attach -nobrowse -readonly "$out/$stem.dmg" | awk -F'\t' '/\/Volumes\//{print $NF}')
  spctl -a -vvv -t exec "$mnt/$(basename "$app")"
  hdiutil detach "$mnt" -quiet
  echo "macOS: $vol signed, notarized and stapled (app + dmg)"
fi
