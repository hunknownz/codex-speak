#!/usr/bin/env bash
set -euo pipefail

CLI_PATH="$HOME/.codex/codex-speak/bin/codex-speak"
OUTPUT_DIR=""
ALLOW_MISSING_MODELS=0
NON_INTERACTIVE=0
SKIP_APP_OPEN=0
SKIP_SPEAK=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PACKAGE_CLI="$PACKAGE_DIR/bin/codex-speak"

usage() {
  cat <<'USAGE'
Usage: manual-qa-macos.sh [options]

Options:
  --cli-path <path>          Installed codex-speak CLI path.
  --output-dir <path>        Directory for QA logs and qa-report.json.
  --allow-missing-models     Allow default model files to be missing.
  --non-interactive          Skip manual questions; useful for CI smoke.
  --skip-app-open            Do not open the control app.
  --skip-speak               Do not run speech playback checks.
  --help                     Show this help.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --cli-path)
      CLI_PATH="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --allow-missing-models)
      ALLOW_MISSING_MODELS=1
      shift
      ;;
    --non-interactive)
      NON_INTERACTIVE=1
      shift
      ;;
    --skip-app-open)
      SKIP_APP_OPEN=1
      shift
      ;;
    --skip-speak)
      SKIP_SPEAK=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$OUTPUT_DIR" ]; then
  OUTPUT_DIR="${TMPDIR:-/tmp}/codex-speak-macos-qa-$(date +%Y%m%d-%H%M%S)"
fi

mkdir -p "$OUTPUT_DIR"
CHECKS_FILE="$OUTPUT_DIR/checks.jsonl"
: >"$CHECKS_FILE"
HAD_FAILURE=0

json_escape() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\t'/\\t}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  printf '%s' "$value"
}

json_bool() {
  if [ "$1" -eq 1 ]; then
    printf 'true'
  else
    printf 'false'
  fi
}

add_qa_check() {
  local name="$1"
  local status="$2"
  local detail="$3"
  local exit_code="${4:-null}"
  local stdout="${5:-}"
  local stderr="${6:-}"

  if [ "$exit_code" != "null" ]; then
    exit_code="${exit_code//[^0-9-]/}"
    [ -n "$exit_code" ] || exit_code="null"
  fi

  printf '{"name":"%s","status":"%s","detail":"%s","exitCode":%s,"stdout":"%s","stderr":"%s"}\n' \
    "$(json_escape "$name")" \
    "$(json_escape "$status")" \
    "$(json_escape "$detail")" \
    "$exit_code" \
    "$(json_escape "$stdout")" \
    "$(json_escape "$stderr")" \
    >>"$CHECKS_FILE"

  if [ "$status" = "fail" ]; then
    HAD_FAILURE=1
  fi
}

safe_name() {
  printf '%s' "$1" | tr -c 'A-Za-z0-9_.-' '-'
}

invoke_qa_executable() {
  local name="$1"
  local allow_failure="$2"
  local executable="$3"
  shift 3

  local safe
  safe="$(safe_name "$name")"
  local stdout="$OUTPUT_DIR/$safe.stdout.txt"
  local stderr="$OUTPUT_DIR/$safe.stderr.txt"

  set +e
  "$executable" "$@" >"$stdout" 2>"$stderr"
  local exit_code=$?
  set -e

  local status="pass"
  if [ "$exit_code" -ne 0 ] && [ "$allow_failure" -ne 1 ]; then
    status="fail"
  fi
  add_qa_check "$name" "$status" "exit $exit_code" "$exit_code" "$stdout" "$stderr"
  return 0
}

invoke_qa_command() {
  local name="$1"
  local allow_failure="$2"
  shift 2
  invoke_qa_executable "$name" "$allow_failure" "$CLI_PATH" "$@"
}

add_manual_check() {
  local name="$1"
  local question="$2"

  if [ "$NON_INTERACTIVE" -eq 1 ]; then
    add_qa_check "$name" "skip" "skipped by --non-interactive"
    return
  fi

  local answer
  while true; do
    read -r -p "$question [y/n/s] " answer
    case "$(printf '%s' "$answer" | tr '[:upper:]' '[:lower:]')" in
      y|yes)
        add_qa_check "$name" "pass" "tester confirmed"
        return
        ;;
      n|no)
        add_qa_check "$name" "fail" "tester reported failure"
        return
        ;;
      s|skip)
        add_qa_check "$name" "skip" "tester skipped"
        return
        ;;
      *)
        echo "Please answer y, n, or s."
        ;;
    esac
  done
}

write_qa_report() {
  local checks_json="$OUTPUT_DIR/checks.json"
  awk 'BEGIN { print "[" } { print sep $0; sep="," } END { print "]" }' "$CHECKS_FILE" >"$checks_json"

  local report_path="$OUTPUT_DIR/qa-report.json"
  local os_name os_version arch user
  os_name="$(sw_vers -productName 2>/dev/null || printf 'macOS')"
  os_version="$(sw_vers -productVersion 2>/dev/null || uname -r)"
  arch="$(uname -m)"
  user="$(id -un 2>/dev/null || printf '')"

  {
    printf '{\n'
    printf '  "generatedAt": "%s",\n' "$(json_escape "$(date -u +%Y-%m-%dT%H:%M:%SZ)")"
    printf '  "cliPath": "%s",\n' "$(json_escape "$CLI_PATH")"
    printf '  "outputDir": "%s",\n' "$(json_escape "$OUTPUT_DIR")"
    printf '  "nonInteractive": %s,\n' "$(json_bool "$NON_INTERACTIVE")"
    printf '  "allowMissingModels": %s,\n' "$(json_bool "$ALLOW_MISSING_MODELS")"
    printf '  "machine": {\n'
    printf '    "os": "%s",\n' "$(json_escape "$os_name")"
    printf '    "osVersion": "%s",\n' "$(json_escape "$os_version")"
    printf '    "architecture": "%s",\n' "$(json_escape "$arch")"
    printf '    "user": "%s"\n' "$(json_escape "$user")"
    printf '  },\n'
    printf '  "checks": '
    cat "$checks_json"
    printf '\n}\n'
  } >"$report_path"

  echo "QA report: $report_path"
}

if [ ! -x "$CLI_PATH" ]; then
  add_qa_check "cli exists" "fail" "missing executable CLI at $CLI_PATH"
  write_qa_report
  exit 1
fi

add_qa_check "cli exists" "pass" "$CLI_PATH"

invoke_qa_command "cli version" 0 --version >/dev/null
if [ -f "$PACKAGE_DIR/release-manifest.json" ] && [ -x "$PACKAGE_CLI" ]; then
  invoke_qa_executable "release package manifest" 0 "$PACKAGE_CLI" verify-package --package-dir "$PACKAGE_DIR" >/dev/null
else
  add_qa_check "release package manifest" "fail" "missing release package manifest or package CLI near $SCRIPT_DIR"
fi

verify_args=(verify-install)
if [ "$ALLOW_MISSING_MODELS" -eq 1 ]; then
  verify_args+=(--allow-missing-models)
fi
invoke_qa_command "verify install" 0 "${verify_args[@]}" >/dev/null

invoke_qa_command "doctor json" 1 doctor --json >/dev/null || true
doctor_stdout="$OUTPUT_DIR/doctor-json.stdout.txt"
if ruby -rjson -e 'JSON.parse(File.read(ARGV[0]))' "$doctor_stdout" >/dev/null 2>&1; then
  add_qa_check "doctor json parse" "pass" "doctor JSON parsed"
else
  add_qa_check "doctor json parse" "fail" "doctor JSON did not parse"
fi

invoke_qa_command "status" 0 status >/dev/null
invoke_qa_command "models list" 0 models list >/dev/null
invoke_qa_command "verify codex integration" 0 verify-codex >/dev/null
invoke_qa_command "verify controls" 0 verify-controls >/dev/null

mixed_text="我运行 hello world，并检查 README.md、codex_speak_prepare、--provider sherpa_melo、OpenRouter、OAuth、M C P、J.S.O.N、CLI、API、CPU、XYZ、build failed because timeout 和 ProjectAlpha42。"
invoke_qa_command "mixed english extract" 0 extract --text "$mixed_text" >/dev/null
mixed_stdout="$OUTPUT_DIR/mixed-english-extract.stdout.txt"
if grep -q "你好世界示例" "$mixed_stdout" \
  && grep -q "说明文件" "$mixed_stdout" \
  && grep -q "准备朗读导览的插件工具" "$mixed_stdout" \
  && grep -q "命令参数" "$mixed_stdout" \
  && grep -q "默认中文朗读引擎" "$mixed_stdout" \
  && grep -q "开放路由平台" "$mixed_stdout" \
  && grep -q "授权登录协议" "$mixed_stdout" \
  && grep -q "插件通道" "$mixed_stdout" \
  && grep -q "数据格式" "$mixed_stdout" \
  && grep -q "命令行工具" "$mixed_stdout" \
  && grep -q "处理器" "$mixed_stdout" \
  && grep -q "英文缩写" "$mixed_stdout" \
  && grep -q "构建失败，因为超时" "$mixed_stdout" \
  && grep -q "英文编号" "$mixed_stdout" \
  && ! grep -q "README.md" "$mixed_stdout" \
  && ! grep -q "codex_speak_prepare" "$mixed_stdout" \
  && ! grep -q -- "--provider" "$mixed_stdout" \
  && ! grep -q "sherpa_melo" "$mixed_stdout" \
  && ! grep -q "OpenRouter" "$mixed_stdout" \
  && ! grep -q "OAuth" "$mixed_stdout" \
  && ! grep -q "M C P" "$mixed_stdout" \
  && ! grep -q "J.S.O.N" "$mixed_stdout" \
  && ! grep -q "hello world" "$mixed_stdout" \
  && ! grep -q "MCP" "$mixed_stdout" \
  && ! grep -q "JSON" "$mixed_stdout" \
  && ! grep -q "XYZ" "$mixed_stdout" \
  && ! grep -q "build" "$mixed_stdout" \
  && ! grep -q "failed" "$mixed_stdout" \
  && ! grep -q "timeout" "$mixed_stdout" \
  && ! grep -q "ProjectAlpha42" "$mixed_stdout"; then
  add_qa_check "mixed english normalization" "pass" "technical English terms normalized for speech"
else
  add_qa_check "mixed english normalization" "fail" "expected technical English terms to be normalized in $mixed_stdout"
fi

support_dir="$OUTPUT_DIR/support-bundle"
invoke_qa_command "support bundle" 0 support-bundle --output "$support_dir" >/dev/null
for file in doctor.json status.json models.json pronunciation-dictionary.json support-bundle-metadata.json; do
  path="$support_dir/$file"
  if [ -f "$path" ]; then
    add_qa_check "support $file" "pass" "$path"
  else
    add_qa_check "support $file" "fail" "missing $path"
  fi
done
if [ -f "$support_dir/release-manifest.json" ]; then
  add_qa_check "support release manifest" "pass" "$support_dir/release-manifest.json"
elif [ -f "$support_dir/release-manifest-missing.txt" ]; then
  add_qa_check "support release manifest" "pass" "$support_dir/release-manifest-missing.txt"
else
  add_qa_check "support release manifest" "fail" "missing release manifest or missing marker"
fi

invoke_qa_command "app path" 0 app path >/dev/null

if [ "$SKIP_APP_OPEN" -ne 1 ] && [ "$NON_INTERACTIVE" -ne 1 ]; then
  invoke_qa_command "app open" 0 app open >/dev/null
  add_manual_check "control app visible" "Did the Codex Speak control panel open?"
  add_manual_check "control settings adjustable" "Can you toggle child mode and change speed, voice profile, and TTS provider in the control panel?"
  add_manual_check "pronunciation dictionary adjustable" "Can you add, preview, and delete one pronunciation dictionary rule in the control panel?"
  add_manual_check "desktop pet transparent" "Is the desktop pet visible without a white or beige background block?"
fi

if [ "$SKIP_SPEAK" -ne 1 ] && [ "$NON_INTERACTIVE" -ne 1 ]; then
  invoke_qa_command "speak sample" 1 speak --text "你好，这是 Codex Speak 的 macOS 真机朗读验收。" >/dev/null || true
  add_manual_check "speech audible" "Did you hear the test voice clearly?"

  invoke_qa_command "speak mixed english sample" 1 speak --text "$mixed_text" >/dev/null || true
  add_manual_check "mixed english speech clear" "Did the mixed Chinese and English sample avoid spelling technical words letter by letter?"

  long_text="这是 Codex Speak 的停止按钮测试。我会读得稍微久一点，方便你确认停止命令有没有打断朗读。"
  "$CLI_PATH" speak --text "$long_text" >"$OUTPUT_DIR/stop-background-speak.stdout.txt" 2>"$OUTPUT_DIR/stop-background-speak.stderr.txt" &
  speak_pid=$!
  sleep 2
  invoke_qa_command "stop speech" 0 stop >/dev/null
  wait "$speak_pid" >/dev/null 2>&1 || true
  add_qa_check "background speak process" "pass" "started pid $speak_pid" "null" "$OUTPUT_DIR/stop-background-speak.stdout.txt" "$OUTPUT_DIR/stop-background-speak.stderr.txt"
  add_manual_check "speech stopped" "Did the stop command interrupt the voice?"
fi

write_qa_report

if [ "$HAD_FAILURE" -eq 1 ]; then
  echo "macOS manual QA found failures."
  exit 1
fi

echo "macOS manual QA completed."
