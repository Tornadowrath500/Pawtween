# Pawtween

A small, friendly animation tool for making motion graphics, stickers and overlays.

- **Web version (free):** https://YOUR_NAME.github.io/YOUR_REPO/
- **Desktop version (pay what you want):** https://YOUR_NAME.itch.io/pawtween
  Adds FFmpeg export (H.265, ProRes 4444 with transparency, APNG, PNG sequence), hardware encoding, a render queue, batch export and a projects library.

## About AI
**Pawtween has no AI features.** It does not generate, enhance, or train on anything. Every piece of artwork in the app and on its pages was drawn by hand. The code was written with AI coding assistance, and I test and maintain it myself.

## Licenses
Code: MIT (see LICENSE). Name, logo and artwork: all rights reserved (see ART-LICENSE). FFmpeg in the desktop build: GPL, see THIRD-PARTY.md.

## Windows note
The installer is unsigned, so SmartScreen may warn. Click "More info", then "Run anyway".

---

# Developer notes (desktop build)

# Pawtween Desktop (Tauri)

Untested starter project. Expect to fix small things on the first build.

## Build it
1. Install Node.js 20+ and Rust (rustup.rs). Windows: also the "Desktop development with C++" build tools. Ubuntu/Debian: `sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev`.
2. Download a static FFmpeg build and put `ffmpeg.exe` (Windows) or `ffmpeg` (Linux, `chmod +x`) in `src-tauri/binaries/`. Builds: https://github.com/BtbN/FFmpeg-Builds/releases
3. `npm install`
4. `npx tauri icon app-icon.png` (makes every icon size; swap in your own 1024x1024 logo any time)
5. `npm run dev` to test, `npm run build` to package. Installers appear in `src-tauri/target/release/bundle/`.

## Release with GitHub Actions
Push a tag like `v0.1.0`. The workflow builds Windows and Linux installers into a draft GitHub Release. Upload the same files to itch.io.

## Notes
- Exports use FFmpeg: fast, universally playable MP4, and WebM with transparency.
- The bundled FFmpeg builds are GPL. Ship their license text and link https://github.com/BtbN/FFmpeg-Builds in your README.
- Windows uses Chromium (WebView2), so every feature works. Linux uses WebKitGTK, which lacks canvas filters: blur, sticker outline and Adjust may not show. Linux users can use the web version in Chrome for those.
- Unsigned installers show a Windows SmartScreen warning ("More info", then "Run anyway").

## Installer and auto-updates
The build makes a one-click Windows installer (NSIS) plus Linux AppImage/deb. Installed copies check GitHub for updates when the app starts.
1. Make a signing key once: `npx tauri signer generate -w ~/.tauri/pawtween.key` (keep the private key safe; never commit it).
2. Paste the PUBLIC key into `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) and set your GitHub name/repo in `endpoints`.
3. In your GitHub repo: Settings > Secrets > Actions, add `TAURI_SIGNING_PRIVATE_KEY` (the private key text) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. Raise `version` in `tauri.conf.json` and `Cargo.toml`, push a tag like `v0.2.0`. The workflow builds installers, signs the update files and publishes `latest.json`. Publish the draft release; users get the update prompt.

## Desktop-only extras
- **Streaming export.** Frames go straight from the canvas into FFmpeg as raw RGBA (no temporary PNG files). On first export the app checks for a hardware H.264 encoder (NVIDIA NVENC, AMD AMF, Intel QuickSync) and uses it for MP4; otherwise libx264 `veryfast`. Formats: MP4, H.265, WebM (alpha), MOV ProRes 4444, GIF, APNG, PNG sequence.
- **Projects library.** Save project writes into the app data folder (`<app data>/projects/NAME.pawtween` plus a thumbnail). The app lists them on start and preloads the newest one so it opens in one click. "Save a copy to a file..." still saves anywhere.
- **Batch export** asks for an output folder once, then writes every selected size without more dialogs.
- Native save dialogs, auto-update.

## Rust commands
`probe_hw`, `enc_start` / `enc_frame` (raw body, job id in the `x-job` header) / `enc_finish` / `enc_cancel` (several jobs can run at once, which is how the render queue works), `reveal` (show a file in Explorer, Finder or the file manager), `get_open_file` and `read_file` (open a .pawtween or .pawtweenpack by double-clicking it). None of these have been compiled by the person who wrote them, so expect to fix small typos on the first `npm run dev`.

## Other desktop changes
- `tauri.conf.json`: `.pawtween` and `.pawtweenpack` file associations, and the webview's own Ctrl+scroll page zoom is turned off so it stops fighting the canvas and timeline zoom.
- Render queue: each job renders from a snapshot of the project inside a hidden copy of the app, so you can keep editing. Performance and hardware (File menu) picks the video encoder, CPU threads and low priority.
- macOS: `bundle.targets` only lists nsis, appimage and deb. Add `dmg` and a macOS runner in the release workflow if you want a Mac build.
