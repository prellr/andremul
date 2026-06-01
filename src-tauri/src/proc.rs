//! Thin cross-platform command runner.

use std::path::Path;
use std::process::{Command, Stdio};

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub ok: bool,
}

impl Output {
    /// stderr if present, else stdout — handy for error messages.
    pub fn combined(&self) -> String {
        let e = self.stderr.trim();
        if e.is_empty() { self.stdout.trim().to_string() } else { e.to_string() }
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn configure(cmd: &mut Command, envs: &[(String, String)]) {
    for (k, v) in envs {
        cmd.env(k, v);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW); // no console flash on Windows
    }
}

/// Run a command to completion and capture output.
pub fn run(program: &Path, args: &[&str], envs: &[(String, String)]) -> Output {
    let mut cmd = Command::new(program);
    cmd.args(args);
    configure(&mut cmd, envs);
    match cmd.output() {
        Ok(o) => Output {
            code: o.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            ok: o.status.success(),
        },
        Err(e) => Output {
            code: -1,
            stdout: String::new(),
            stderr: e.to_string(),
            ok: false,
        },
    }
}

/// Run a command, feeding `input` to its stdin (e.g. accepting sdkmanager
/// licenses with a stream of "y", or answering avdmanager prompts with "no").
pub fn run_with_input(program: &Path, args: &[&str], envs: &[(String, String)], input: &str) -> Output {
    use std::io::Write;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut cmd, envs);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Output { code: -1, stdout: String::new(), stderr: e.to_string(), ok: false },
    };
    if let Some(mut stdin) = child.stdin.take() {
        let owned = input.to_string();
        // Write on a thread so a full stdout pipe can't deadlock us.
        std::thread::spawn(move || {
            let _ = stdin.write_all(owned.as_bytes());
        });
    }
    match child.wait_with_output() {
        Ok(o) => Output {
            code: o.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            ok: o.status.success(),
        },
        Err(e) => Output { code: -1, stdout: String::new(), stderr: e.to_string(), ok: false },
    }
}

/// Spawn a long-lived process (e.g. the emulator), detached from our stdio.
pub fn spawn(program: &Path, args: &[&str], envs: &[(String, String)]) -> std::io::Result<std::process::Child> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure(&mut cmd, envs);
    cmd.spawn()
}
