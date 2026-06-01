# 签名与公证

Codex Speak 的 release workflow 支持“有证书就签名，没有证书就跳过”。这样个人开发阶段不会被证书阻塞，正式分发时也不用重写发布流程。

## macOS

需要的 GitHub Secrets：

| Secret | 用途 |
| --- | --- |
| `MACOS_CERTIFICATE_P12_BASE64` | Developer ID Application 证书的 `.p12` 文件，base64 后保存 |
| `MACOS_CERTIFICATE_PASSWORD` | `.p12` 导出密码 |
| `MACOS_CODESIGN_IDENTITY` | `codesign` 使用的证书身份，例如 `Developer ID Application: ...` |
| `MACOS_KEYCHAIN_PASSWORD` | CI 临时 keychain 密码，可用随机长字符串 |
| `APPLE_ID` | Apple ID 邮箱，用于 notarization |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `APPLE_APP_SPECIFIC_PASSWORD` | Apple app-specific password |

workflow 行为：

- 如果没有 `MACOS_CERTIFICATE_P12_BASE64`，跳过证书导入。
- 如果没有 `MACOS_CODESIGN_IDENTITY`，跳过签名。
- 如果有签名证书，会签名：
  - `bin/codex-speak`
  - `bin/codex-speak-pet-macos`
  - `apps/Codex Speak.app`
- 如果同时配置了 Apple notarization 的三个 Secret，会提交 `.app` 公证并 staple。

签名脚本：

```bash
scripts/sign-macos-release.sh dist/codex-speak-macos
```

## Windows

需要的 GitHub Secrets：

| Secret | 用途 |
| --- | --- |
| `WINDOWS_SIGN_CERT_PFX_BASE64` | Windows 代码签名证书 `.pfx` 文件，base64 后保存 |
| `WINDOWS_SIGN_CERT_PASSWORD` | `.pfx` 密码 |

workflow 行为：

- 如果没有 `WINDOWS_SIGN_CERT_PFX_BASE64`，跳过签名。
- 如果有证书，会用 `signtool.exe` 签名：
  - `bin/codex-speak.exe`
  - `apps/codex-speak-control.exe`
- 签名后会执行 `signtool verify /pa`。

签名脚本：

```powershell
.\scripts\sign-windows-release.ps1 -PackageDir dist\codex-speak-windows
```

## 当前限制

- 目前 release 仍然是 `.tar.gz` 和 `.zip`，不是 DMG、PKG、MSI 或 MSIX。
- macOS 公证主要覆盖 `.app`，CLI 和 Pet helper 会 codesign；后续如果要进一步降低 Gatekeeper 摩擦，可以改成签名并公证的 `.pkg`。
- Windows 代码签名可以减少安全提示，但 SmartScreen 信誉仍然需要下载量和证书信誉慢慢建立。
