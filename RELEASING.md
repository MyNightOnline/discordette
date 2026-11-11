# Releasing Discordette

Releases are built by GitHub Actions (`.github/workflows/release.yml`) when a `v*` tag is pushed.

## One-time setup
Repo → Settings → Secrets and variables → Actions → add secret `TAURI_SIGNING_PRIVATE_KEY`
(contents of the updater private key matching the `pubkey` in `src-tauri/tauri.conf.json`).
Without it the build fails at the signing step.

## Each release
1. Bump the version (must be higher than the last release, or installed apps won't see the update) in:
   - `package.json` and `package-lock.json` (lines 3 and 9)
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml` and the `discordette` entry in `src-tauri/Cargo.lock`
2. Commit and push to `main`.
3. Tag and push:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
4. Watch the **Actions** tab (~5–10 min). It publishes a Release with the `-setup.exe`, its `.sig`, and `latest.json` (used by the in-app updater).

## Local build
`bun run tauri build` produces installers in `src-tauri/target/release/bundle/`.
It exits 1 at the end without `TAURI_SIGNING_PRIVATE_KEY`; the installers are still fine.
To skip that: `bun run tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`
