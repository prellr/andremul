//! Emulator launch + AVD listing. Picks platform-appropriate GPU modes.

use crate::proc;
use crate::sdk::AndroidSdk;

pub fn list_avds(sdk: &AndroidSdk) -> Vec<String> {
    if !sdk.has_emulator() {
        return vec![];
    }
    proc::run(&sdk.emulator(), &["-list-avds"], &sdk.tool_env())
        .stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// KDS-tuned launch args. Headless → software GPU (no display attached); windowed
/// → "auto" so the emulator picks the best host accelerator on each OS.
pub fn launch_args(avd: &str, headless: bool) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-avd".into(),
        avd.into(),
        "-no-snapshot".into(),
        "-no-boot-anim".into(),
        "-netdelay".into(),
        "none".into(),
        "-netspeed".into(),
        "full".into(),
        "-no-audio".into(),
    ];
    if headless {
        a.push("-no-window".into());
        a.push("-gpu".into());
        a.push("swiftshader_indirect".into());
    } else {
        a.push("-gpu".into());
        a.push("auto".into());
    }
    a
}

pub fn start(sdk: &AndroidSdk, avd: &str, headless: bool) -> std::io::Result<std::process::Child> {
    let args = launch_args(avd, headless);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    proc::spawn(&sdk.emulator(), &refs, &sdk.tool_env())
}
