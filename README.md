# Pawtween

A small, friendly animation tool for creating motion graphics, stickers and overlays.

- **Web version (free):** https://Tornadowrath500.github.io/Pawtween/
- **Desktop version (early access only) (pay what you want):** https://tornadowrath500.itch.io/pawtween
  Adds FFmpeg export (H.265, ProRes 4444 with transparency, APNG, PNG sequence), hardware encoding, a render queue, batch export and a projects library.

## About AI
Pawtween has absolutely no AI features. It doesn't generate anything, it doesn't enhance anything and it doesn't train on anything. All the artwork in the app and on the site is hand drawn. The code was written with the aid of AI coding assistance and I test and maintain it myself.

## Licenses
Code: MIT (see LICENSE). Name, logo and artwork: all rights reserved (see ART-LICENSE). FFmpeg in the desktop build: GPL, see THIRD-PARTY.md.

## Windows note
The installer is unsigned so SmartScreen may give you a warning. Click "More info" then "Run anyway".

---

# Developer notes (desktop build)

# Pawtween Desktop (Tauri)

Untested starter project. Expect to fix the little things on the first build.

## Build it
1. Install Node.js 20 or higher and Rust from rustup.rs. On Windows, also install the "Desktop development with C++" build tools. On Ubuntu or Debian, run `sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev`.
2. Download a static FFmpeg build and put ffmpeg.exe (Windows) or ffmpeg (Linux, chmod +x) in src-tauri/binaries/. Builds available at: https://github.com/BtbN/FFmpeg-Builds/releases
3. `npm install`
4. `npx tauri icon app-icon.png` (creates all the icon sizes; you can swap in your own 1024x1024 logo at any time)
5. 'npm run dev' to test, 'npm run build' to package. The installers will be in 'src-tauri/target/release/bundle/'.

## Release with GitHub Actions
Push a tag (v0.1.0). The workflow will build the Windows and Linux installers and put them into a draft GitHub Release. The same files are then uploaded to itch.io.

## Notes
- Exports use FFmpeg: fast, universally playable MP4 and WebM with transparency.
- The bundled FFmpeg builds are GPL. Include the licence text and link to https://github.com/BtbN/FFmpeg-Builds in your README.
- Windows uses Chromium (WebView2) so everything works. Linux uses WebKitGTK which doesn't have canvas filters (blur, sticker outline and Adjust may not appear). Linux users can use the web version in Chrome for those features.
- Unsigned installers will trigger a Windows SmartScreen warning ("More info", then "Run anyway").

## Installer and auto-updates
The build creates a one-click Windows installer (NSIS) as well as a Linux AppImage/deb. Installed copies will check GitHub for updates upon launch.
1. Generate a signing key once: `npx tauri signer generate -w ~/.tauri/pawtween.key` (do keep your private key safe, don't commit it).
2. Paste your PUBLIC key into src-tauri/tauri.conf.json (in the plugins.updater.pubkey field) and set your GitHub username and repo in the endpoints section.
In your GitHub repo: Settings > Secrets > Actions, add `TAURI_SIGNING_PRIVATE_KEY` (the private key text) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. Increment `version` in both `tauri.conf.json` and `Cargo.toml` and then push a tag (e.g. `v0.2.0`). The workflow will then build the installers, sign the update files and publish the `latest.json`. Publish the draft release and users will then get the update prompt.

## Desktop-only extras
- Streaming export. Frames go directly from the canvas into FFmpeg as raw RGBA (no intermediate PNG files). On the first export the app will check for a hardware H.264 encoder (NVIDIA NVENC, AMD AMF, Intel QuickSync) and use that for MP4 if available, otherwise libx264 veryfast. Formats: MP4, H.265, WebM (alpha), MOV ProRes 4444, GIF, APNG, PNG sequence.
- Projects library. Project writes are saved into the app data folder (<app data>/projects/NAME.pawtween) along with a thumbnail. They're all listed on start and the most recent one is preloaded so it can be opened with a single click. "Save a copy to a file..." still works as before and will save to wherever you want.
- Batch export asks for an output folder once and then writes out every selected size without any further dialogs.
- Native save dialogs. Auto-update.

## Rust commands
probe_hw, enc_start / enc_frame (raw body, job id in the x-job header) / enc_finish / enc_cancel (multiple jobs can run at the same time, that's how the render queue works), reveal (will open a file in Explorer/Finder/the file manager), get_open_file and read_file (open a .pawtween or .pawtweenpack by double clicking it). None of those have been compiled by the person who wrote them so expect to be fixing minor typos on the first npm run dev.

## Other desktop changes
- tauri.conf.json: .pawtween and .pawtweenpack file associations, webview's own Ctrl+scroll page zoom is turned off so it doesn't fight with the canvas/timeline zoom.
- Render queue: each job renders from a snapshot of the project within a hidden copy of the app so you can continue editing. Performance and hardware (File menu) selects the video encoder, CPU threads and low priority.
- macOS: bundle.targets only lists nsis, appimage and deb. If you want a Mac build you'll need to add dmg and a macOS runner to the release workflow.
