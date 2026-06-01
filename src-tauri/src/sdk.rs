//! Cross-platform Android SDK + JDK locator. This is the heart of portability:
//! it resolves tool paths with the right executable extensions and default
//! install locations for macOS, Windows, and Linux.

use std::env;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const EXE: &str = ".exe";
#[cfg(not(windows))]
const EXE: &str = "";

// cmdline-tools scripts: .bat on Windows, no extension elsewhere.
#[cfg(windows)]
const BAT: &str = ".bat";
#[cfg(not(windows))]
const BAT: &str = "";

pub struct AndroidSdk {
    pub root: PathBuf,
}

impl AndroidSdk {
    pub fn resolve() -> Self {
        for key in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
            if let Ok(p) = env::var(key) {
                if !p.is_empty() {
                    return Self { root: PathBuf::from(p) };
                }
            }
        }
        Self { root: Self::default_root() }
    }

    fn default_root() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_default();
        #[cfg(target_os = "macos")]
        {
            home.join("Library/Android/sdk")
        }
        #[cfg(target_os = "windows")]
        {
            let local = env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData/Local"));
            local.join("Android/Sdk")
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            home.join("Android/Sdk")
        }
    }

    pub fn exists(&self) -> bool {
        self.root.exists()
    }

    pub fn adb(&self) -> PathBuf {
        self.root.join("platform-tools").join(format!("adb{EXE}"))
    }

    pub fn emulator(&self) -> PathBuf {
        self.root.join("emulator").join(format!("emulator{EXE}"))
    }

    fn cmdline_tool(&self, name: &str) -> PathBuf {
        let base = self.root.join("cmdline-tools");
        let latest = base.join("latest").join("bin").join(format!("{name}{BAT}"));
        if latest.exists() {
            return latest;
        }
        // Fall back to the newest versioned dir that contains the tool.
        if let Ok(read) = std::fs::read_dir(&base) {
            let mut dirs: Vec<PathBuf> = read.flatten().map(|e| e.path()).collect();
            dirs.sort();
            for d in dirs.into_iter().rev() {
                let cand = d.join("bin").join(format!("{name}{BAT}"));
                if cand.exists() {
                    return cand;
                }
            }
        }
        latest
    }

    pub fn sdkmanager(&self) -> PathBuf {
        self.cmdline_tool("sdkmanager")
    }
    pub fn avdmanager(&self) -> PathBuf {
        self.cmdline_tool("avdmanager")
    }

    pub fn has_adb(&self) -> bool {
        self.adb().exists()
    }
    pub fn has_emulator(&self) -> bool {
        self.emulator().exists()
    }
    pub fn has_cmdline_tools(&self) -> bool {
        self.sdkmanager().exists()
    }

    /// Resolve a JDK 17+ home for the Java-based tools. Order: JAVA_HOME, a local
    /// JDK we may have unpacked, then Android Studio's bundled runtime (JBR).
    pub fn java_home(&self) -> Option<String> {
        if let Ok(jh) = env::var("JAVA_HOME") {
            if Path::new(&jh).exists() {
                return Some(jh);
            }
        }
        let home = dirs::home_dir().unwrap_or_default();
        // A locally unpacked JDK, e.g. ~/Library/Android/jdk17/<v>/Contents/Home (mac)
        // or ~/.andremul/jdk on other platforms.
        let java_bin = format!("bin/java{EXE}");
        let local_candidates = [
            home.join("Library/Android/jdk17"),
            home.join(".andremul/jdk"),
        ];
        for base in local_candidates {
            if let Some(found) = find_java_home(&base, &java_bin) {
                return Some(found);
            }
        }
        // Android Studio bundled JBR.
        let studio = studio_jbr_candidates();
        for c in studio {
            if Path::new(&c).join(&java_bin).exists() {
                return Some(c);
            }
        }
        None
    }

    pub fn has_java(&self) -> bool {
        self.java_home().is_some()
    }

    pub fn is_ready(&self) -> bool {
        self.has_adb() && self.has_emulator() && self.has_cmdline_tools() && self.has_java()
    }

    /// Environment overrides handed to SDK tools so they agree on locations and
    /// can find a usable JDK.
    pub fn tool_env(&self) -> Vec<(String, String)> {
        let mut v = vec![
            ("ANDROID_SDK_ROOT".into(), self.root.to_string_lossy().into()),
            ("ANDROID_HOME".into(), self.root.to_string_lossy().into()),
        ];
        if let Some(jh) = self.java_home() {
            v.push(("JAVA_HOME".into(), jh));
        }
        v
    }
}

/// Search `base` for a directory whose `<dir>/Contents/Home/bin/java` (mac bundle)
/// or `<dir>/bin/java` (plain) exists, returning the Java home.
fn find_java_home(base: &Path, java_bin: &str) -> Option<String> {
    if !base.exists() {
        return None;
    }
    let read = std::fs::read_dir(base).ok()?;
    let mut dirs: Vec<PathBuf> = read.flatten().map(|e| e.path()).collect();
    dirs.sort();
    for d in dirs.into_iter().rev() {
        // macOS bundle layout
        let mac_home = d.join("Contents/Home");
        if mac_home.join(java_bin).exists() {
            return Some(mac_home.to_string_lossy().into());
        }
        // plain layout
        if d.join(java_bin).exists() {
            return Some(d.to_string_lossy().into());
        }
    }
    None
}

fn studio_jbr_candidates() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec!["/Applications/Android Studio.app/Contents/jbr/Contents/Home".into()]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            "C:\\Program Files\\Android\\Android Studio\\jbr".into(),
            "C:\\Program Files\\Android\\Android Studio\\jre".into(),
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            "/opt/android-studio/jbr".into(),
            home.join("android-studio/jbr").to_string_lossy().into(),
        ]
    }
}
