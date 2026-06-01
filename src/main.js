const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
let currentAvd = "Andremul";

// ---- mode switch ----
$("mode-easy").onclick = () => switchMode("easy");
$("mode-adv").onclick = () => switchMode("advanced");
function switchMode(m) {
  const easy = m === "easy";
  $("mode-easy").classList.toggle("active", easy);
  $("mode-adv").classList.toggle("active", !easy);
  $("easy").classList.toggle("hidden", !easy);
  $("advanced").classList.toggle("hidden", easy);
}

// ---- logging ----
function appendLog(msg) {
  const el = document.createElement("div");
  el.className = "line";
  el.textContent = `${new Date().toLocaleTimeString()}  ${msg}`;
  const log = $("log");
  log.appendChild(el);
  log.scrollTop = log.scrollHeight;
}

// ---- status ----
function setStatus(s) {
  const icons = { stopped: "■", launching: "⏱", booting: "⏱", running: "✓", error: "!" };
  const labels = { stopped: "Stopped", launching: "Starting…", booting: "Booting…", running: "Running", error: "Error" };
  $("status-icon").className = "status-icon " + s;
  $("status-icon").textContent = icons[s] || "■";
  $("status-label").textContent = labels[s] || s;
  const running = s === "running";
  $("btn-start").classList.toggle("hidden", running);
  $("btn-stop").classList.toggle("hidden", !running);
  const busy = s === "launching" || s === "booting";
  $("btn-start").disabled = busy;
  $("status-sub").textContent = running ? "Emulator host active" : "";
}

// ---- environment ----
async function refresh() {
  const env = await invoke("detect_environment");
  $("os-badge").textContent = env.os;
  if (env.avds && env.avds.length) currentAvd = env.avds[0];
  const checks = [
    ["SDK root", env.sdk_exists, env.sdk_root],
    ["adb / platform-tools", env.has_adb, ""],
    ["Emulator", env.has_emulator, ""],
    ["Command-line tools", env.has_cmdline_tools, ""],
    ["JDK (Java 17+)", env.has_java, env.java_home],
    ["scrcpy (real-time display)", env.scrcpy, env.scrcpy ? "installed" : "not installed — emulator shows its own window"],
  ];
  $("env").innerHTML = checks
    .map(([label, ok, val]) =>
      `<li><span class="dot ${ok ? "ok" : "no"}"></span>${label}<span class="val">${val || (ok ? "found" : "missing")}</span></li>`)
    .join("");
  $("avds").textContent = env.avds && env.avds.length ? env.avds.join(", ") : "none";
  $("setup-hint").classList.toggle("hidden", env.ready);
  $("btn-start").disabled = !env.ready;
}

// ---- target package ----
async function setPackage(pkg) {
  await invoke("set_target_package", { package: pkg || "" });
  localStorage.setItem("andremul.pkg", pkg || "");
}
$("pkg").addEventListener("change", (e) => setPackage(e.target.value.trim()));
$("btn-detect").onclick = async () => {
  const pkgs = await invoke("list_packages");
  $("pkg-list").innerHTML = pkgs.map((p) => `<option value="${p}">`).join("");
  appendLog(pkgs.length ? `Found ${pkgs.length} installed app(s) — pick one in the field.` : "No user apps found (boot a device first).");
};

// ---- actions ----
async function start() {
  try { await invoke("start_emulator", { avd: currentAvd, headless: true }); }
  catch (e) { appendLog("❌ " + e); }
}
async function stop() { await invoke("stop_emulator"); }
async function launch() { await invoke("launch_app"); }

$("btn-start").onclick = start;
$("btn-stop").onclick = stop;
$("btn-start-adv").onclick = start;
$("btn-stop-adv").onclick = stop;
$("btn-launch").onclick = launch;
$("btn-rescan").onclick = refresh;

// ---- events from Rust ----
listen("log", (e) => appendLog(e.payload));
listen("status", (e) => setStatus(e.payload));

// ---- boot ----
(async () => {
  await refresh();
  const saved = localStorage.getItem("andremul.pkg") || "";
  if (saved) { $("pkg").value = saved; await setPackage(saved); }
  await invoke("detect_running");
  appendLog("Andremul ready.");
})();
