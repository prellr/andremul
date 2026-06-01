//! Real-time display via scrcpy (cross-platform: scrcpy / scrcpy.exe). The
//! emulator runs headless; scrcpy opens a draggable window with touch input.

use crate::proc;
use crate::sdk::AndroidSdk;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const EXE: &str = "scrcpy.exe";
#[cfg(not(windows))]
const EXE: &str = "scrcpy";

/// Locate the scrcpy binary in common install dirs or on PATH.
pub fn locate() -> Option<PathBuf> {
    let fixed = ["/opt/homebrew/bin/scrcpy", "/usr/local/bin/scrcpy"];
    for c in fixed {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for dir in path.split(sep) {
            let p = Path::new(dir).join(EXE);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn available() -> bool {
    locate().is_some()
}

/// Launch scrcpy against `serial`. Title-barred (draggable) window; full-screen
/// it for a kiosk view.
pub fn start(sdk: &AndroidSdk, serial: &Option<String>) -> Option<std::process::Child> {
    let path = locate()?;
    let mut args: Vec<String> = vec![
        "--window-title=Andremul KDS Display".into(),
        "--stay-awake".into(),
        "--no-audio".into(),
        "--disable-screensaver".into(),
    ];
    if let Some(s) = serial {
        args.push("-s".into());
        args.push(s.clone());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // scrcpy finds adb via the ADB env var; point it at our SDK's adb, and add
    // scrcpy's own dir to PATH so its bundled libs/server resolve.
    let mut env = sdk.tool_env();
    env.push(("ADB".into(), sdk.adb().to_string_lossy().to_string()));
    if let Some(dir) = path.parent() {
        let cur = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ";" } else { ":" };
        env.push(("PATH".into(), format!("{}{}{}", dir.to_string_lossy(), sep, cur)));
    }
    proc::spawn(&path, &refs, &env).ok()
}
