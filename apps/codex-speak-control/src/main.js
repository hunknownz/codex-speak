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
  providerHint: $("providerHint"),
  modelState: $("modelState"),
  hookState: $("hookState"),
  configState: $("configState"),
  controlAppState: $("controlAppState"),
  pluginState: $("pluginState"),
  marketplaceState: $("marketplaceState"),
  petState: $("petState"),
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
  controls.providerHint.textContent = provider
    ? `${provider.role} · ${provider.languages} · ${provider.footprint}`
    : "-";
  controls.modelState.textContent = provider?.installed ? "正常" : (provider?.reason || "缺失");
  controls.hookState.textContent = status.checks.notify_configured ? "已连接" : "未连接";
  controls.configState.textContent = status.checks.config_exists ? "正常" : "缺失";
  controls.controlAppState.textContent = status.checks.control_app_exists ? "已安装" : "未安装";
  const pluginOk = status.checks.plugin_installed
    && status.checks.plugin_skill_installed
    && status.checks.plugin_mcp_config_installed
    && status.checks.plugin_mcp_script_installed;
  controls.pluginState.textContent = pluginOk ? "已安装" : "需修复";
  controls.marketplaceState.textContent = status.checks.marketplace_configured ? "已连接" : "缺失";
  controls.petState.textContent = petStateLabel(status.pet_state?.state, status.checks);
  controls.lastSpoken.textContent = status.last_spoken || "暂无记录";

  const ok = status.checks.config_exists
    && status.checks.cli_exists
    && Boolean(provider?.installed)
    && status.checks.notify_configured
    && status.checks.player_available
    && status.checks.control_app_exists
    && (!status.checks.pet_helper_supported || status.checks.pet_helper_exists)
    && pluginOk
    && status.checks.marketplace_configured;
  controls.health.textContent = ok ? "运行正常" : "需要检查";
  controls.health.dataset.state = ok ? "ok" : "warn";
  applying = false;
}

function petStateLabel(state, checks = {}) {
  if (!checks.pet_helper_supported) {
    return "本机状态";
  }
  if (!checks.pet_helper_exists) {
    return "未安装";
  }
  switch (state) {
    case "ready":
      return "待朗读";
    case "speaking":
      return "朗读中";
    case "done":
      return "刚完成";
    case "error":
      return "有问题";
    default:
      return "待命";
  }
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
    setLog("试听完成");
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

$("installModel").addEventListener("click", async () => {
  setBusy(true);
  setLog("正在下载并安装当前朗读引擎的模型...");
  try {
    const status = await invoke("install_current_model");
    renderStatus(status);
    setLog("模型已安装");
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

$("supportBundle").addEventListener("click", async () => {
  setBusy(true);
  try {
    const path = await invoke("write_support_bundle");
    setLog(`支持包已生成：${path}`);
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

refresh();
