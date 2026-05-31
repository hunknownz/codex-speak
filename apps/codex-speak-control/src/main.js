import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const $ = (id) => document.getElementById(id);
const controls = {
  enabled: $("enabled"),
  childMode: $("childMode"),
  provider: $("provider"),
  voiceProfile: $("voiceProfile"),
  speed: $("speed"),
  maxChars: $("maxChars"),
  speedValue: $("speedValue"),
  maxCharsValue: $("maxCharsValue"),
  providerState: $("providerState"),
  modelState: $("modelState"),
  hookState: $("hookState"),
  configState: $("configState"),
  lastSpoken: $("lastSpoken"),
  health: $("health"),
  log: $("log")
};

let applying = false;
let saveTimer = null;

function setLog(message) {
  controls.log.textContent = message || "";
}

function setBusy(isBusy) {
  document.body.classList.toggle("busy", isBusy);
}

function renderStatus(status) {
  applying = true;
  controls.enabled.checked = status.enabled;
  controls.childMode.checked = status.child_mode;
  controls.provider.value = status.provider || "sherpa_melo";
  controls.voiceProfile.value = status.voice_profile || "clear_bright";
  controls.speed.value = status.speed;
  controls.maxChars.value = status.max_read_chars;
  controls.speedValue.value = Number(status.speed).toFixed(2);
  controls.maxCharsValue.value = status.max_read_chars;
  const provider = (status.providers || []).find((item) => item.id === status.provider);
  controls.providerState.textContent = provider?.label || status.provider || "-";
  controls.modelState.textContent = provider?.installed ? "正常" : (provider?.reason || "缺失");
  controls.hookState.textContent = status.checks.notify_configured ? "已连接" : "未连接";
  controls.configState.textContent = status.checks.config_exists ? "正常" : "缺失";
  controls.lastSpoken.textContent = status.last_spoken || "暂无记录";

  const ok = status.checks.config_exists
    && status.checks.cli_exists
    && Boolean(provider?.installed)
    && status.checks.notify_configured;
  controls.health.textContent = ok ? "运行正常" : "需要检查";
  controls.health.dataset.state = ok ? "ok" : "warn";
  applying = false;
}

async function refresh() {
  setBusy(true);
  try {
    const status = await invoke("load_status");
    renderStatus(status);
    setLog("");
  } catch (error) {
    setLog(String(error));
    controls.health.textContent = "连接失败";
    controls.health.dataset.state = "warn";
  } finally {
    setBusy(false);
  }
}

async function savePatch(patch) {
  if (applying) return;
  setBusy(true);
  try {
    const status = await invoke("update_settings", { patch });
    renderStatus(status);
    setLog("已保存");
  } catch (error) {
    setLog(String(error));
    await refresh();
  } finally {
    setBusy(false);
  }
}

function schedulePatch(patch) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => savePatch(patch), 180);
}

controls.enabled.addEventListener("change", () => {
  savePatch({ enabled: controls.enabled.checked });
});

controls.childMode.addEventListener("change", () => {
  savePatch({ childMode: controls.childMode.checked });
});

controls.provider.addEventListener("change", () => {
  savePatch({ provider: controls.provider.value });
});

controls.voiceProfile.addEventListener("change", () => {
  savePatch({ voiceProfile: controls.voiceProfile.value });
});

controls.speed.addEventListener("input", () => {
  controls.speedValue.value = Number(controls.speed.value).toFixed(2);
  schedulePatch({ speed: Number(controls.speed.value) });
});

controls.maxChars.addEventListener("input", () => {
  controls.maxCharsValue.value = controls.maxChars.value;
  schedulePatch({ maxReadChars: Number(controls.maxChars.value) });
});

$("testSpeak").addEventListener("click", async () => {
  setBusy(true);
  try {
    await invoke("speak_sample");
    setLog("正在试听");
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

$("stopSpeak").addEventListener("click", async () => {
  setBusy(true);
  try {
    await invoke("stop_speech");
    setLog("已停止");
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

$("refresh").addEventListener("click", refresh);

$("doctor").addEventListener("click", async () => {
  setBusy(true);
  try {
    setLog(await invoke("run_doctor"));
    await refresh();
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

refresh();
