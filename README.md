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

## Performance and reliability
- **Re-render everything**: Ctrl+Alt+R (rebindable in Shortcuts), also View menu > Re-render everything and the command palette. It stops playback, closes and rebuilds every preview bitmap, clears the render caches (lighting, effects, background, scratch canvases), rebuilds thumbnails and panels, restarts the preview loop, and un-sticks the render queue. If an export is running it asks you to press it again within 3 seconds before cancelling it.
- **The preview can't die any more**: the next animation frame is scheduled before anything can throw. Frame errors are counted, throttled and auto-recovered; a watchdog restarts the loop if the browser drops it; a stuck or suspended audio clock falls back to the wall clock; slow scenes (more than ~0.5 s a frame) get cost-aware redraw spacing instead of locking the UI; motion blur samples scale down during playback if frames run long.
- **Exports can't hang the app**: every call into FFmpeg has a time limit (encoder start 60 s, each frame 60 s, finish 15 min, cancel 8 s) and `busy` is always cleared first. If a hardware encoder fails or stalls the export is retried on the CPU automatically and the CPU is used for the rest of the session. Stored hardware settings are validated on read. Performance and hardware has a **Reset render settings** button.
- **Render queue**: a job that is cancelled, silent for 3 minutes, or whose hidden window fails to load is stopped from outside, so it can never block the queue. Failed jobs keep their project snapshot, so **Retry** / **Retry failed** works. A project that fails to open in the render window is reported instead of rendering an empty video.
- **Rust**: `Enc` now holds `child` and `stdin` behind their own locks and the shared map is only locked briefly, so a frame write blocked on a stalled FFmpeg can no longer block `enc_cancel`, `enc_start` or another job. `enc_finish` polls instead of blocking in `wait()`, so Cancel also works while finishing. `probe_hw` kills a hardware probe that hangs after 12 s. No new crates were added.
- **Sticker outline** is drawn on the graphics card (an SVG alpha-curve filter, no per-frame readbacks or per-pixel JavaScript) and falls back to the original canvas code if the filter is not supported. A checkbox in Performance and hardware switches it off.
- **Memory and I/O**: preview bitmaps are closed when replaced (and failures retried), one project serialisation per interaction instead of one per event, autosave writes the images only when the set of images changes, and interval autosave is skipped while playing.

## Branding assets
- `app-icon.png`: source for `npx tauri icon app-icon.png`.
- `src/assets/` and `docs/assets/` (keep the two copies in sync): `paw-icon.png` (header mark and favicon), `logo.png` (empty-stage logo), `fonts/BaksoSapi.otf`.
- Headings use the `--display` CSS variable in the `pt-brand` style block at the end of `index.html`. Change that one line to swap fonts, or add class `pt-display` to any new element.
