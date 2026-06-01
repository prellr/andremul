# Andremul

Run Android apps on your desktop. Andremul is a lightweight cross-platform host
(**macOS · Windows · Linux**) that boots an Android emulator, optionally
auto-launches an app of your choice, and mirrors it in a clean real-time window —
ideal for running a single Android app as a desktop **kiosk** (POS, kitchen
display, signage, line-of-business clients, etc.).

Built with [Tauri](https://tauri.app) (Rust core + native webview) — small
binaries, no Electron.

> **You bring your own apps.** Andremul does not ship or download any Android
> app; install whatever you need from the Play Store or sideload your own APK.
> Andremul is not affiliated with Google or any app vendor.

## What it does
- **One-click bring-up** — detects your Android SDK, boots a dedicated AVD, and
  (optionally) launches a chosen app package on every start.
- **Real-time display** via [scrcpy](https://github.com/Genymobile/scrcpy) — the
  emulator runs headless and scrcpy shows a draggable, full-screenable window
  with touch input. (No scrcpy? It falls back to the emulator's own window.)
- **Kiosk hardening** — stay-awake, no screensaver, no lock screen, so an
  unattended display survives reboots.
- **Adopts a running emulator** instead of failing on a duplicate.

## Requirements
- **Android SDK** — `adb`, `emulator`, command-line tools, and a system image,
  plus a created AVD. (Easiest: install Android Studio, or the command-line tools.)
- **JDK 17+** — for the SDK's Java tools (Android Studio bundles one).
- **scrcpy** *(optional, recommended)* — for the real-time display:
  - macOS: `brew install scrcpy`
  - Windows: `winget install Genymobile.scrcpy` (or scoop/choco)
  - Linux: your package manager

Andremul resolves SDK/JDK locations per-OS automatically (e.g.
`~/Library/Android/sdk` on macOS, `%LOCALAPPDATA%\Android\Sdk` on Windows).

## Run from source
Requires [Rust](https://rustup.rs). Node is **not** needed (static frontend).

```sh
cd src-tauri
cargo run                 # build + launch
cargo run -- --selftest   # print detected environment and exit
```

## Download / install
- **Windows**: built automatically by CI. Go to the **Actions** tab → latest
  "Build Windows" run → download the `andremul-windows-installers` artifact, or
  grab the installer from a tagged **Release**. Recipients need the WebView2
  runtime (preinstalled on Windows 10/11) and scrcpy for the display.
- **macOS**: `./scripts/package-mac.sh` builds a signed/notarized `.app` (set
  `DEV_ID` + `NOTARY_PROFILE` to notarize; otherwise it's ad-hoc signed).

## Usage
1. **Advanced → Environment** should show green checks (SDK, adb, emulator,
   command-line tools, JDK). If not, set up the Android SDK first.
2. *(Optional)* **Advanced → Auto-launch app on boot** — type a package id, or
   click **Detect installed** to pick from apps installed on the device.
3. **Easy → Start** — boots/adopts the emulator, applies kiosk settings, launches
   your app (if set), and opens the real-time display.

## Roadmap
- In-app SDK bootstrap + AVD creation (currently set up the SDK yourself)
- Built-in screencap mirror as a no-scrcpy fallback
- Watchdog / auto-recovery, embedded display window
- macOS/Linux CI in addition to Windows

## Project layout
```
src/                 static frontend (index.html, main.js, styles.css)
src-tauri/
  src/
    main.rs          Tauri commands, state, boot-watch, --selftest
    sdk.rs           cross-platform Android SDK + JDK locator
    proc.rs          command runner (Windows no-console-flash)
    adb.rs           devices, boot, packages, launch, kiosk settings
    emulator.rs      list AVDs, launch (headless/windowed)
    scrcpy.rs        real-time display
  tauri.conf.json, capabilities/, icons/
scripts/package-mac.sh   macOS .app packaging
.github/workflows/       Windows installer CI
```

## License
MIT — see [LICENSE](LICENSE).
