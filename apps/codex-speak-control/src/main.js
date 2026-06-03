import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const $ = (id) => document.getElementById(id);
const controls = {
  enabled: $("enabled"),
  finalGuideEnabled: $("finalGuideEnabled"),
  progressPromptsEnabled: $("progressPromptsEnabled"),
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
  pronunciationState: $("pronunciationState"),
  pronunciationCount: $("pronunciationCount"),
  pronunciationTerm: $("pronunciationTerm"),
  pronunciationSpoken: $("pronunciationSpoken"),
  pronunciationPreviewText: $("pronunciationPreviewText"),
  pronunciationList: $("pronunciationList"),
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
let pronunciationTerms = {};

function setLog(message) {
  controls.log.textContent = message || "";
}

function setBusy(isBusy) {
  document.body.classList.toggle("busy", isBusy);
}

function renderStatus(status) {
  applying = true;
  controls.enabled.checked = status.enabled;
  controls.finalGuideEnabled.checked = status.final_guide_enabled;
  controls.progressPromptsEnabled.checked = status.progress_prompts_enabled;
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
  const hookOk = status.checks.notify_configured && status.checks.notify_hook_current;
  controls.hookState.textContent = hookOk ? "已连接" : (status.checks.notify_configured ? "需刷新" : "未连接");
  controls.configState.textContent = status.checks.config_exists ? "正常" : "缺失";
  controls.pronunciationState.textContent = status.checks.pronunciation_dictionary_valid
    ? `${status.pronunciation_terms || 0} 条`
    : "需检查";
  controls.controlAppState.textContent = status.checks.control_app_exists ? "已安装" : "未安装";
  const pluginOk = status.checks.plugin_installed
    && status.checks.plugin_current
    && status.checks.plugin_skill_installed
    && status.checks.plugin_skill_current
    && status.checks.plugin_mcp_config_installed
    && status.checks.plugin_mcp_config_current
    && status.checks.plugin_mcp_script_installed
    && status.checks.plugin_mcp_script_current
    && status.checks.codex_skill_installed
    && status.checks.codex_skill_current;
  controls.pluginState.textContent = pluginOk ? "已安装" : "需刷新";
  controls.marketplaceState.textContent = status.checks.marketplace_configured ? "已连接" : "缺失";
  controls.petState.textContent = petStateLabel(status.pet_state?.state, status.checks);
  controls.lastSpoken.textContent = status.last_spoken || "暂无记录";

  const ok = status.checks.config_exists
    && status.checks.cli_exists
    && status.checks.pronunciation_dictionary_valid
    && Boolean(provider?.installed)
    && hookOk
    && status.checks.player_available
    && status.checks.control_app_exists
    && (!status.checks.pet_helper_supported || status.checks.pet_helper_exists)
    && pluginOk
    && status.checks.marketplace_configured;
  controls.health.textContent = ok ? "运行正常" : "需要检查";
  controls.health.dataset.state = ok ? "ok" : "warn";
  applying = false;
}

function renderPronunciation(dictionary) {
  pronunciationTerms = dictionary?.terms || {};
  const entries = Object.entries(pronunciationTerms).sort(([left], [right]) => left.localeCompare(right));
  controls.pronunciationCount.textContent = `${entries.length} 条`;
  if (entries.length === 0) {
    controls.pronunciationList.innerHTML = `<p class="empty">暂无自定义发音</p>`;
    return;
  }
  controls.pronunciationList.replaceChildren(
    ...entries.map(([term, spoken]) => {
      const row = document.createElement("div");
      row.className = "dictionary-row";

      const text = document.createElement("button");
      text.className = "dictionary-term";
      text.type = "button";
      text.dataset.term = term;
      text.innerHTML = `<span>${escapeHtml(term)}</span><small>${escapeHtml(spoken)}</small>`;

      const remove = document.createElement("button");
      remove.className = "dictionary-remove";
      remove.type = "button";
      remove.dataset.removeTerm = term;
      remove.textContent = "删除";

      row.append(text, remove);
      return row;
    })
  );
}

async function refreshPronunciation() {
  const dictionary = await invoke("load_pronunciation");
  renderPronunciation(dictionary);
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
    try {
      const dictionary = await invoke("load_pronunciation");
      renderPronunciation(dictionary);
      setLog("");
    } catch (error) {
      renderPronunciation({ terms: {} });
      setLog(`发音词典需要检查：${String(error)}`);
    }
  } catch (error) {
    setLog(String(error));
    controls.health.textContent = "连接失败";
    controls.health.dataset.state = "warn";
  } finally {
    setBusy(false);
  }
}

function selectedPronunciation() {
  return {
    term: controls.pronunciationTerm.value.trim(),
    spoken: controls.pronunciationSpoken.value.trim()
  };
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

controls.finalGuideEnabled.addEventListener("change", () => {
  savePatch({ finalGuideEnabled: controls.finalGuideEnabled.checked });
});

controls.progressPromptsEnabled.addEventListener("change", () => {
  savePatch({ progressPromptsEnabled: controls.progressPromptsEnabled.checked });
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

$("savePronunciation").addEventListener("click", async () => {
  const { term, spoken } = selectedPronunciation();
  if (!term || !spoken) {
    setLog("请填写原词和读法");
    return;
  }
  setBusy(true);
  try {
    await invoke("set_pronunciation", { term, spoken });
    await refresh();
    controls.pronunciationTerm.value = "";
    controls.pronunciationSpoken.value = "";
    setLog("发音规则已保存");
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

$("previewPronunciation").addEventListener("click", async () => {
  const text = controls.pronunciationPreviewText.value.trim();
  if (!text) {
    setLog("请填写预览文本");
    return;
  }
  setBusy(true);
  try {
    const preview = await invoke("preview_pronunciation", { text });
    setLog(`预览：${preview}`);
  } catch (error) {
    setLog(String(error));
  } finally {
    setBusy(false);
  }
});

controls.pronunciationList.addEventListener("click", async (event) => {
  if (!(event.target instanceof Element)) {
    return;
  }
  const editTerm = event.target.closest("[data-term]")?.dataset.term;
  const removeTerm = event.target.closest("[data-remove-term]")?.dataset.removeTerm;

  if (editTerm) {
    controls.pronunciationTerm.value = editTerm;
    controls.pronunciationSpoken.value = pronunciationTerms[editTerm] || "";
    controls.pronunciationPreviewText.value = `我配置了 ${editTerm}。`;
    return;
  }

  if (!removeTerm) {
    return;
  }

  setBusy(true);
  try {
    await invoke("remove_pronunciation", { term: removeTerm });
    await refresh();
    setLog("发音规则已删除");
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

function escapeHtml(text) {
  return String(text)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

refresh();
