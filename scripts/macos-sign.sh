#!/usr/bin/env bash
# macos-sign.sh <App.app> : Developer ID signs + notarizes + staples when the Apple secrets are set
# (APPLE_CERTIFICATE = base64 .p12, APPLE_CERTIFICATE_PASSWORD, APPLE_API_KEY = base64 .p8,
# APPLE_API_KEY_ID, APPLE_API_ISSUER); otherwise ad-hoc signs (users right-click → Open once).
set -euo pipefail
app=$1
if [ -z "${APPLE_CERTIFICATE:-}" ] || [ -z "${APPLE_API_KEY:-}" ]; then
  echo "::warning::Apple signing secrets not set: ad-hoc signing $app"
  codesign --force --deep -s - "$app"
  exit 0
fi
kc="$RUNNER_TEMP/sign.keychain-db"; kcpw=$(openssl rand -hex 16)
echo "$APPLE_CERTIFICATE" | base64 --decode > "$RUNNER_TEMP/cert.p12"
security create-keychain -p "$kcpw" "$kc"
security set-keychain-settings -lut 3600 "$kc"
security unlock-keychain -p "$kcpw" "$kc"
security import "$RUNNER_TEMP/cert.p12" -P "$APPLE_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$kc"
security set-key-partition-list -S apple-tool:,apple: -k "$kcpw" "$kc" >/dev/null
security list-keychains -d user -s "$kc" $(security list-keychains -d user | tr -d '"')
id=$(security find-identity -v -p codesigning "$kc" | grep -m1 "Developer ID Application" | awk '{print $2}')
[ -n "$id" ] || { echo "::error::no Developer ID Application identity in APPLE_CERTIFICATE"; exit 1; }
codesign --force --deep --options runtime --timestamp -s "$id" "$app"
codesign --verify --strict --deep "$app"
echo "$APPLE_API_KEY" | base64 --decode > "$RUNNER_TEMP/key.p8"
ditto -c -k --keepParent "$app" "$RUNNER_TEMP/notarize.zip"
xcrun notarytool submit "$RUNNER_TEMP/notarize.zip" --key "$RUNNER_TEMP/key.p8" \
  --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER" --wait --timeout 30m
xcrun stapler staple "$app"
rm -f "$RUNNER_TEMP/cert.p12" "$RUNNER_TEMP/key.p8" "$RUNNER_TEMP/notarize.zip"
security delete-keychain "$kc" || true
