#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adb;
mod bootstrap;
mod emulator;
mod proc;
mod scrcpy;
mod sdk;

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
struct Core {
    child: Option<std::process::Child>,
    scrcpy_child: Option<std::process::Child>,
    serial: Option<String>,
    /// Optional Android package id to auto-launch on boot (e.g. a kiosk app).
    target_package: Option<String>,
}
type SharedCore = Mutex<Core>;

#[derive(Serialize, Clone)]
struct EnvInfo {
    os: String,
    sdk_root: String,
    sdk_exists: bool,
    has_adb: bool,
    has_emulator: bool,
    has_cmdline_tools: bool,
    has_java: bool,
    java_home: String,
    ready: bool,
    avds: Vec<String>,
    scrcpy: bool,
}

#[tauri::command]
fn detect_environment() -> EnvInfo {
    let s = sdk::AndroidSdk::resolve();
    EnvInfo {
        os: std::env::consts::OS.to_string(),
        sdk_root: s.root.to_string_lossy().to_string(),
        sdk_exists: s.exists(),
        has_adb: s.has_adb(),
        has_emulator: s.has_emulator(),
        has_cmdline_tools: s.has_cmdline_tools(),
        has_java: s.has_java(),
        java_home: s.java_home().unwrap_or_default(),
        ready: s.is_ready(),
        avds: emulator::list_avds(&s),
        scrcpy: scrcpy::available(),
    }
}

fn log(app: &AppHandle, msg: impl Into<String>) {
    let _ = app.emit("log", msg.into());
}
fn status(app: &AppHandle, s: &str) {
    let _ = app.emit("status", s.to_string());
}

#[tauri::command]
fn start_emulator(app: AppHandle, state: State<SharedCore>, avd: String, _headless: bool) -> Result<(), String> {
    let s = sdk::AndroidSdk::resolve();
    if !s.is_ready() {
        return Err("SDK not ready — install components and create the AVD first.".into());
    }
    // With scrcpy available, run the emulator headless (scrcpy is the display);
    // otherwise run it windowed so its own screen is at least visible.
    let headless = scrcpy::available();
    // Adopt an already-running emulator instead of launching a duplicate.
    if let Some(serial) = adb::online_emulator(&s) {
        log(&app, format!("Emulator already running ({serial}) — adopting it."));
        state.lock().unwrap().serial = Some(serial);
        spawn_boot_watch(app.clone());
        return Ok(());
    }
    status(&app, "launching");
    log(&app, format!("Starting emulator {avd} (headless={headless})…"));
    match emulator::start(&s, &avd, headless) {
        Ok(child) => state.lock().unwrap().child = Some(child),
        Err(e) => {
            status(&app, "error");
            return Err(e.to_string());
        }
    }
    spawn_boot_watch(app.clone());
    Ok(())
}

fn spawn_boot_watch(app: AppHandle) {
    std::thread::spawn(move || {
        let s = sdk::AndroidSdk::resolve();
        status(&app, "booting");

        let mut serial = None;
        for _ in 0..60 {
            if let Some(sn) = adb::online_emulator(&s) {
                serial = Some(sn);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        let serial = match serial {
            Some(s) => s,
            None => {
                log(&app, "No emulator device appeared.");
                status(&app, "error");
                return;
            }
        };
        app.state::<SharedCore>().lock().unwrap().serial = Some(serial.clone());
        log(&app, format!("Device {serial}. Waiting for boot…"));

        let so = Some(serial);
        let mut booted = false;
        for _ in 0..90 {
            if adb::boot_completed(&s, &so) {
                booted = true;
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        if !booted {
            log(&app, "Boot did not complete in time.");
            status(&app, "error");
            return;
        }
        log(&app, "✅ Boot complete.");
        adb::apply_kiosk(&s, &so);
        let target = app.state::<SharedCore>().lock().unwrap().target_package.clone();
        match target {
            Some(pkg) if adb::is_installed(&s, &so, &pkg) => {
                adb::launch(&s, &so, &pkg);
                log(&app, format!("Launched {pkg}."));
            }
            Some(pkg) => log(&app, format!("App {pkg} not installed (install via Play Store or sideload).")),
            None => log(&app, "No auto-launch app set — emulator is up; pick one in Advanced if you want."),
        }
        // Real-time display via scrcpy (the emulator runs headless when it's present).
        if scrcpy::available() {
            if let Some(child) = scrcpy::start(&s, &so) {
                app.state::<SharedCore>().lock().unwrap().scrcpy_child = Some(child);
                log(&app, "🖥️ Real-time display started (scrcpy).");
            }
        } else {
            log(&app, "scrcpy not installed — install it (`brew install scrcpy`) for a clean display window.");
        }
        status(&app, "running");
    });
}

#[tauri::command]
fn stop_emulator(app: AppHandle, state: State<SharedCore>) {
    let s = sdk::AndroidSdk::resolve();
    let serial = state.lock().unwrap().serial.clone();
    if let Some(mut c) = state.lock().unwrap().scrcpy_child.take() {
        let _ = c.kill();
    }
    adb::kill_emulator(&s, &serial);
    if let Some(mut c) = state.lock().unwrap().child.take() {
        let _ = c.kill();
    }
    state.lock().unwrap().serial = None;
    status(&app, "stopped");
    log(&app, "Stopped emulator.");
}

#[tauri::command]
fn set_target_package(state: State<SharedCore>, package: String) {
    let trimmed = package.trim().to_string();
    state.lock().unwrap().target_package = if trimmed.is_empty() { None } else { Some(trimmed) };
}

#[tauri::command]
fn list_packages(state: State<SharedCore>) -> Vec<String> {
    let s = sdk::AndroidSdk::resolve();
    let serial = state.lock().unwrap().serial.clone();
    adb::user_packages(&s, &serial)
}

#[tauri::command]
fn launch_app(app: AppHandle, state: State<SharedCore>) {
    let s = sdk::AndroidSdk::resolve();
    let serial = state.lock().unwrap().serial.clone();
    let pkg = state.lock().unwrap().target_package.clone();
    match pkg {
        Some(p) => {
            adb::launch(&s, &serial, &p);
            log(&app, format!("Launched {p}."));
        }
        None => log(&app, "No app selected to launch."),
    }
}

#[tauri::command]
fn setup_android(app: AppHandle) {
    std::thread::spawn(move || bootstrap::run_setup(app));
}

#[tauri::command]
fn detect_running(app: AppHandle, state: State<SharedCore>) {
    let s = sdk::AndroidSdk::resolve();
    if let Some(serial) = adb::online_emulator(&s) {
        let so = Some(serial.clone());
        if adb::boot_completed(&s, &so) {
            state.lock().unwrap().serial = Some(serial.clone());
            log(&app, format!("Detected running emulator {serial}."));
            status(&app, "running");
        }
    }
}

fn main() {
    // Headless bootstrap test: download JDK/cmdline-tools into the SDK root and exit.
    if std::env::args().any(|a| a == "--bootstrap-test") {
        bootstrap::cli_test();
        std::process::exit(0);
    }

    // Headless self-test: print the cross-platform environment detection and exit.
    if std::env::args().any(|a| a == "--selftest") {
        let info = detect_environment();
        println!("Andremul (Tauri) self-test");
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
        std::process::exit(0);
    }

    tauri::Builder::default()
        .manage(SharedCore::default())
        .invoke_handler(tauri::generate_handler![
            detect_environment,
            start_emulator,
            stop_emulator,
            set_target_package,
            list_packages,
            launch_app,
            setup_android,
            detect_running
        ])
        // Stop the emulator + scrcpy when the window is closed.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                cleanup(&app);
                app.exit(0); // closing the window fully quits the app
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Andremul")
        // …and on app quit (Cmd+Q / last window), as a backstop.
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                cleanup(app);
            }
        });
}

/// Shut down the emulator and scrcpy that Andremul started (or adopted), so they
/// don't linger after the app is closed. Idempotent.
fn cleanup(app: &AppHandle) {
    let sdk = sdk::AndroidSdk::resolve();
    let (serial, scrcpy_child, emu_child) = {
        let state = app.state::<SharedCore>();
        let mut core = state.lock().unwrap();
        (core.serial.clone(), core.scrcpy_child.take(), core.child.take())
    };
    if let Some(mut c) = scrcpy_child {
        let _ = c.kill();
    }
    adb::kill_emulator(&sdk, &serial); // graceful: works for spawned or adopted
    if let Some(mut c) = emu_child {
        let _ = c.kill(); // hard fallback
    }
}
