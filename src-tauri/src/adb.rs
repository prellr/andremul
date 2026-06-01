//! adb operations, scoped to the emulator Andremul controls.

use crate::proc;
use crate::sdk::AndroidSdk;

fn base<'a>(serial: &'a Option<String>, args: &'a [&'a str]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let Some(s) = serial {
        v.push("-s".into());
        v.push(s.clone());
    }
    v.extend(args.iter().map(|a| a.to_string()));
    v
}

fn run(sdk: &AndroidSdk, serial: &Option<String>, args: &[&str]) -> proc::Output {
    let owned = base(serial, args);
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    proc::run(&sdk.adb(), &refs, &sdk.tool_env())
}

/// (serial, state) for each attached device.
pub fn devices(sdk: &AndroidSdk) -> Vec<(String, String)> {
    let out = proc::run(&sdk.adb(), &["devices"], &sdk.tool_env());
    out.stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn online_emulator(sdk: &AndroidSdk) -> Option<String> {
    devices(sdk)
        .into_iter()
        .find(|(serial, state)| serial.starts_with("emulator-") && state == "device")
        .map(|(s, _)| s)
}

pub fn boot_completed(sdk: &AndroidSdk, serial: &Option<String>) -> bool {
    run(sdk, serial, &["shell", "getprop", "sys.boot_completed"])
        .stdout
        .trim()
        == "1"
}

pub fn user_packages(sdk: &AndroidSdk, serial: &Option<String>) -> Vec<String> {
    run(sdk, serial, &["shell", "pm", "list", "packages", "-3"])
        .stdout
        .lines()
        .map(|l| l.replace("package:", "").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn is_installed(sdk: &AndroidSdk, serial: &Option<String>, package: &str) -> bool {
    user_packages(sdk, serial).iter().any(|p| p == package)
}

pub fn launch(sdk: &AndroidSdk, serial: &Option<String>, package: &str) {
    let _ = run(
        sdk,
        serial,
        &["shell", "monkey", "-p", package, "-c", "android.intent.category.LAUNCHER", "1"],
    );
}

pub fn apply_kiosk(sdk: &AndroidSdk, serial: &Option<String>) {
    let cmds: [&[&str]; 5] = [
        &["shell", "settings", "put", "system", "screen_off_timeout", "2147483647"],
        &["shell", "svc", "power", "stayon", "true"],
        &["shell", "settings", "put", "secure", "screensaver_enabled", "0"],
        &["shell", "settings", "put", "secure", "sleep_timeout", "-1"],
        &["shell", "locksettings", "set-disabled", "true"],
    ];
    for c in cmds {
        let _ = run(sdk, serial, c);
    }
}

pub fn kill_emulator(sdk: &AndroidSdk, serial: &Option<String>) {
    let _ = run(sdk, serial, &["emu", "kill"]);
}
