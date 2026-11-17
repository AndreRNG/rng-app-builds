# rng-app-builds

Cross-platform builds of RNG Media's app family (CutCat, PhotoFalcon, PDFrog, DocDuck, SlideSeal) and the
RNG Studio launcher and its manifest.

- `apps.json`: app id → upstream repo, cargo package, binary.
- `patches/<app>.patch` + `.base`: the `rng` branch from `~/src/<repo>` as one patch on an upstream commit.
- `scripts/sync-patch.sh <app>`: re-export the patch after `update-rng-app`, push, start the build.
- `.github/workflows/build.yml`: builds linux-x86_64, windows-x86_64, macos-universal (unsigned,
  ad-hoc signed) and makes a release `<app>-b<run>` with the packages and `<app>.json`.
- `scripts/publish.sh`: copies the release assets to the public download host and rewrites the manifest.

See TRADEMARKS.md: the app names and icons are not open source.
