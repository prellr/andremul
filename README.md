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

## Setting up the Android SDK

If you don't already have the SDK, you have three options.

### Option A — Android Studio (easiest)
Install [Android Studio](https://developer.android.com/studio); it bundles the
command-line tools, a JDK, and an SDK at the conventional location. Then create an
AVD in **Device Manager** (a landscape tablet image is ideal for kiosk use).

### Option B — Let Claude do it
Paste the prompt below into **Claude Code** (it needs shell access). It detects
your OS/CPU, installs a JDK + the command-line tools, downloads the right SDK
packages, creates an AVD, installs scrcpy, and verifies a boot:

```text
Set up everything needed to run Andremul (https://github.com/prellr/andremul) on this machine.

1. Detect my OS and CPU architecture (arm64 vs x86_64).
2. Ensure a JDK 17+ is available. Use the system JDK if present; otherwise install
   Temurin 17 (Homebrew/winget/apt, or a no-sudo tarball into ~/.andremul/jdk). Export JAVA_HOME.
3. Install the Android SDK "command line tools only" (latest, from
   https://developer.android.com/studio#command-tools) into the conventional SDK root:
     - macOS:   ~/Library/Android/sdk
     - Windows: %LOCALAPPDATA%\Android\Sdk
     - Linux:   ~/Android/Sdk
   Unzip so sdkmanager ends up at <SDK>/cmdline-tools/latest/bin/.
4. With sdkmanager (accept all licenses non-interactively), install:
   platform-tools, emulator, platforms;android-34, and the Play Store system image
   matching my CPU:
     - arm64:  system-images;android-34;google_apis_playstore;arm64-v8a
     - x86_64: system-images;android-34;google_apis_playstore;x86_64
5. Create a landscape tablet AVD named "Andremul":
   avdmanager create avd -n Andremul -k "<that system image>" --device "10.1in WXGA (Tablet)"
   Then set hw.initialOrientation=landscape and disable the lock screen in its config.ini.
6. Install scrcpy for the real-time display (brew/winget/apt).
7. Set ANDROID_SDK_ROOT and add platform-tools + emulator to my PATH (persist in my shell profile).
8. Verify: print `adb version` and `emulator -list-avds`, then boot the AVD headless,
   poll until sys.boot_completed=1, and shut it down. Report what you installed and any manual steps left.
```

### Option C — Command-line tools by hand

**macOS / Linux**
```sh
SDK="$HOME/Library/Android/sdk"      # Linux: $HOME/Android/Sdk
mkdir -p "$SDK/cmdline-tools"
# Download "command line tools only" (mac/linux) from the link above, then:
unzip commandlinetools-*.zip -d "$SDK/cmdline-tools"
mv "$SDK/cmdline-tools/cmdline-tools" "$SDK/cmdline-tools/latest"
SM="$SDK/cmdline-tools/latest/bin/sdkmanager"
yes | "$SM" --licenses
ABI=arm64-v8a   # use x86_64 on Intel Macs / Linux x86
"$SM" "platform-tools" "emulator" "platforms;android-34" "system-images;android-34;google_apis_playstore;$ABI"
"$SDK/cmdline-tools/latest/bin/avdmanager" create avd -n Andremul \
  -k "system-images;android-34;google_apis_playstore;$ABI" --device "10.1in WXGA (Tablet)"
export ANDROID_SDK_ROOT="$SDK"; export PATH="$SDK/platform-tools:$SDK/emulator:$PATH"
```

**Windows (PowerShell)** — requires JDK 17 (set `JAVA_HOME`) and the *Windows
Hypervisor Platform* feature enabled for acceleration:
```powershell
$SDK = "$env:LOCALAPPDATA\Android\Sdk"
New-Item -ItemType Directory -Force "$SDK\cmdline-tools" | Out-Null
# Download "command line tools only" (windows) from the link above, then:
Expand-Archive commandlinetools-win-*.zip "$SDK\cmdline-tools"
Rename-Item "$SDK\cmdline-tools\cmdline-tools" latest
$SM = "$SDK\cmdline-tools\latest\bin\sdkmanager.bat"
& $SM --licenses                       # accept each prompt
& $SM "platform-tools" "emulator" "platforms;android-34" "system-images;android-34;google_apis_playstore;x86_64"
& "$SDK\cmdline-tools\latest\bin\avdmanager.bat" create avd -n Andremul `
  -k "system-images;android-34;google_apis_playstore;x86_64" --device "10.1in WXGA (Tablet)"
setx ANDROID_SDK_ROOT "$SDK"
```

> **ABI matters:** use `arm64-v8a` on Apple Silicon / Windows-on-ARM, `x86_64` on
> Intel/AMD. ARM-only apps on an x86_64 image rely on the emulator's ARM
> translation, which may or may not work — test your specific app.

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
