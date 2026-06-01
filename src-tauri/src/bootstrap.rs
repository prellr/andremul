//! In-app "Set up Android" bootstrap. Downloads a JDK + Google's command-line
//! tools, installs the SDK packages via sdkmanager (accepting Google's licenses),
//! and creates an AVD — cross-platform, streaming progress to the UI.
//!
//! Nothing is redistributed: everything is fetched from official sources
//! (Adoptium for the JDK, dl.google.com for the SDK) at the user's request.

use crate::proc;
use crate::sdk::AndroidSdk;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// Log to the UI when running under Tauri, or to stderr for the headless CLI test.
fn log(app: Option<&AppHandle>, m: impl Into<String>) {
    let m = m.into();
    match app {
        Some(a) => {
            let _ = a.emit("log", m);
        }
        None => eprintln!("[setup] {m}"),
    }
}

fn abi() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64-v8a",
        _ => "x86_64",
    }
}

fn google_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        "linux"
    }
}

fn system_image() -> String {
    format!("system-images;android-34;google_apis_playstore;{}", abi())
}

// ---- download with progress ----

fn download(app: Option<&AppHandle>, url: &str, dest: &Path, label: &str) -> Result<(), String> {
    log(app, format!("Downloading {label}…"));
    let resp = ureq::get(url).call().map_err(|e| format!("{label}: {e}"))?;
    let total_len: u64 = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 65536];
    let (mut total, mut last) = (0u64, 0u64);
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        total += n as u64;
        if total - last > 25_000_000 {
            last = total;
            let pct = if total_len > 0 {
                format!(" ({}%)", total * 100 / total_len)
            } else {
                String::new()
            };
            log(app, format!("  {label}: {} MB{pct}", total / 1_048_576));
        }
    }
    log(app, format!("  {label}: {} MB ✓", total / 1_048_576));
    Ok(())
}

fn extract_zip(zip: &Path, dest: &Path) -> Result<(), String> {
    let f = fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut ar = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    ar.extract(dest).map_err(|e| e.to_string())
}

fn extract_tar_gz(tgz: &Path, dest: &Path) -> Result<(), String> {
    let f = fs::File::open(tgz).map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    ar.unpack(dest).map_err(|e| e.to_string())
}

#[cfg(unix)]
fn chmod_x(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(md) = fs::metadata(e.path()) {
                let mut p = md.permissions();
                p.set_mode(0o755);
                let _ = fs::set_permissions(e.path(), p);
            }
        }
    }
}

// ---- JDK ----

fn temurin_url() -> String {
    let arch = if std::env::consts::ARCH == "aarch64" { "aarch64" } else { "x64" };
    let os = if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };
    format!("https://api.adoptium.net/v3/binary/latest/17/ga/{os}/{arch}/jdk/hotspot/normal/eclipse")
}

fn ensure_jdk(app: Option<&AppHandle>, sdk: &AndroidSdk) -> Result<(), String> {
    if sdk.has_java() {
        log(app, "JDK 17 already present — skipping.");
        return Ok(());
    }
    let base = dirs::home_dir().ok_or("no home dir")?.join(".andremul/jdk");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let url = temurin_url();
    if cfg!(target_os = "windows") {
        let zip = base.join("temurin.zip");
        download(app, &url, &zip, "JDK 17 (Temurin)")?;
        extract_zip(&zip, &base)?;
        let _ = fs::remove_file(&zip);
    } else {
        let tgz = base.join("temurin.tar.gz");
        download(app, &url, &tgz, "JDK 17 (Temurin)")?;
        extract_tar_gz(&tgz, &base)?;
        let _ = fs::remove_file(&tgz);
    }
    log(app, "JDK 17 installed.");
    Ok(())
}

// ---- command-line tools ----

fn discover_clt_url() -> Result<String, String> {
    let os = google_os();
    let xml = ureq::get("https://dl.google.com/android/repository/repository2-1.xml")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let needle = format!("commandlinetools-{os}-");
    let mut best: u64 = 0;
    let mut idx = 0;
    while let Some(pos) = xml[idx..].find(&needle) {
        let start = idx + pos + needle.len();
        let digits: String = xml[start..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u64>() {
            if n > best {
                best = n;
            }
        }
        idx = start;
    }
    if best == 0 {
        return Err("could not find command-line tools download URL".into());
    }
    Ok(format!(
        "https://dl.google.com/android/repository/commandlinetools-{os}-{best}_latest.zip"
    ))
}

fn ensure_cmdline_tools(app: Option<&AppHandle>, sdk: &AndroidSdk) -> Result<(), String> {
    if sdk.has_cmdline_tools() {
        log(app, "Command-line tools already present — skipping.");
        return Ok(());
    }
    let clt_dir = sdk.root.join("cmdline-tools");
    fs::create_dir_all(&clt_dir).map_err(|e| e.to_string())?;
    let url = discover_clt_url()?;
    let zip = clt_dir.join("clt.zip");
    download(app, &url, &zip, "Android command-line tools")?;

    let tmp = clt_dir.join("_unzip");
    let _ = fs::remove_dir_all(&tmp);
    extract_zip(&zip, &tmp)?;
    let src = tmp.join("cmdline-tools"); // archive contains a top-level "cmdline-tools"
    let dest = clt_dir.join("latest");
    let _ = fs::remove_dir_all(&dest);
    fs::rename(&src, &dest).map_err(|e| format!("place cmdline-tools: {e}"))?;
    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_file(&zip);
    #[cfg(unix)]
    chmod_x(&dest.join("bin"));
    log(app, "Command-line tools installed.");
    Ok(())
}

// ---- packages + AVD ----

fn install_packages(app: Option<&AppHandle>, sdk: &AndroidSdk) -> Result<(), String> {
    log(app, "Accepting Google SDK licenses…");
    let _ = proc::run_with_input(&sdk.sdkmanager(), &["--licenses"], &sdk.tool_env(), &"y\n".repeat(60));

    let image = system_image();
    log(app, format!("Installing platform-tools, emulator, platform 34, and {image}… (downloads ~1.5 GB — several minutes)"));
    let pkgs = ["platform-tools", "emulator", "platforms;android-34", image.as_str()];
    let out = proc::run(&sdk.sdkmanager(), &pkgs, &sdk.tool_env());
    if !out.ok {
        return Err(format!("sdkmanager failed: {}", out.combined()));
    }
    log(app, "SDK packages installed.");
    Ok(())
}

fn tweak_avd_config(name: &str) {
    let cfg = dirs::home_dir()
        .unwrap_or_default()
        .join(format!(".android/avd/{name}.avd/config.ini"));
    if let Ok(mut s) = fs::read_to_string(&cfg) {
        for (k, v) in [
            ("hw.initialOrientation", "landscape"),
            ("showDeviceFrame", "no"),
            ("hw.keyboard", "yes"),
        ] {
            if !s.contains(&format!("{k}=")) {
                s.push_str(&format!("\n{k}={v}"));
            }
        }
        let _ = fs::write(&cfg, s);
    }
}

fn create_avd(app: Option<&AppHandle>, sdk: &AndroidSdk) -> Result<(), String> {
    let name = "Andremul";
    if crate::emulator::list_avds(sdk).iter().any(|a| a == name) {
        log(app, "AVD 'Andremul' already exists — skipping.");
        return Ok(());
    }
    let image = system_image();
    log(app, "Creating AVD 'Andremul' (landscape tablet)…");
    let out = proc::run_with_input(
        &sdk.avdmanager(),
        &["create", "avd", "-n", name, "-k", &image, "--device", "10.1in WXGA (Tablet)", "--force"],
        &sdk.tool_env(),
        "no\n",
    );
    if !out.ok {
        return Err(format!("avdmanager failed: {}", out.combined()));
    }
    tweak_avd_config(name);
    log(app, "AVD 'Andremul' created.");
    Ok(())
}

/// The setup sequence. `app` is None for the headless CLI test.
fn setup(app: Option<&AppHandle>) -> Result<(), String> {
    log(app, "▶ Setting up Android — downloads from Google/Adoptium; you are accepting Google's SDK licenses.");
    let sdk = AndroidSdk::resolve();
    ensure_jdk(app, &sdk)?;
    // Re-resolve after each step so JAVA_HOME / tool paths pick up new installs.
    let sdk = AndroidSdk::resolve();
    ensure_cmdline_tools(app, &sdk)?;
    let sdk = AndroidSdk::resolve();
    install_packages(app, &sdk)?;
    create_avd(app, &sdk)?;
    Ok(())
}

/// Full setup, run on a background thread. Emits "setup" status + "log" lines.
pub fn run_setup(app: AppHandle) {
    let _ = app.emit("setup", "running");
    match setup(Some(&app)) {
        Ok(()) => {
            log(Some(&app), "✅ Android setup complete — click Start.");
            let _ = app.emit("setup", "done");
        }
        Err(e) => {
            log(Some(&app), format!("❌ Setup failed: {e}"));
            let _ = app.emit("setup", "error");
        }
    }
}

/// Headless test of the download/extract chain (skips already-present pieces).
/// Run with `andremul --bootstrap-test` (set ANDROID_SDK_ROOT to a temp dir).
pub fn cli_test() {
    let sdk = AndroidSdk::resolve();
    eprintln!("[setup] SDK root: {}", sdk.root.display());
    if let Err(e) = ensure_jdk(None, &sdk) {
        eprintln!("[setup] JDK step failed: {e}");
        return;
    }
    let sdk = AndroidSdk::resolve();
    if let Err(e) = ensure_cmdline_tools(None, &sdk) {
        eprintln!("[setup] cmdline-tools step failed: {e}");
        return;
    }
    let sdk = AndroidSdk::resolve();
    let out = proc::run(&sdk.sdkmanager(), &["--version"], &sdk.tool_env());
    eprintln!("[setup] sdkmanager --version => ok={} : {}", out.ok, out.stdout.trim());
    eprintln!("[setup] cmdline-tools present: {}", sdk.has_cmdline_tools());
}
