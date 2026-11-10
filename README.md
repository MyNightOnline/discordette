# Discordette

A small desktop assistant for managing Discord. Built with Tauri 2 + Svelte 5. The UI is in Thai.

Today it posts to **forum** and **text** channels through webhooks, and it is meant to grow into a broader Discord helper.

## Features

- Post to forum channels (title, tags, images/clips) and plain text channels via webhooks
- Markdown toolbar and live preview, drag-and-drop / paste attachments, reorder rooms and images by dragging
- Fetch and create forum tags with an optional bot token (used for tags only, never for posting)
- AI summaries: bring your own provider and model, with an editable prompt template
- Duplicate warning (same title or source link), local post history
- Config import/export; webhook URLs and tokens live in the OS keyring
- Automatic update check (Windows)

> Exported config files contain webhook URLs. Anyone with the file can post to your channels.

## Install

Download the latest `Discordette_*_x64-setup.exe` from [Releases](https://github.com/MyNightOnline/discordette/releases). The installer is not code-signed, so Windows SmartScreen may warn: choose **More info → Run anyway**. After the first install the app updates itself.

## Develop

Requires Node 22+, Rust stable and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev          # run the app
npm test                   # TypeScript tests
npm run check              # svelte-check
cd src-tauri && cargo test # Rust tests
npm run tauri build        # build the installer
```

## Releasing

Releases are built by GitHub Actions and signed with the Tauri updater key.

1. Create a key pair: `npm run tauri signer generate -- -w ~/.tauri/discordette.key`
2. Put the public key in `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`
3. Add the private key as the repo secret `TAURI_SIGNING_PRIVATE_KEY`
4. Tag and push: `git tag v0.1.1 && git push origin v0.1.1`

## License

[MIT](LICENSE)
