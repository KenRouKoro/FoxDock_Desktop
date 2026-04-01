# CI 中自动签名（GitHub Actions）

在 [`.github/workflows/windows-ci.yml`](../.github/workflows/windows-ci.yml) 的 **Build Tauri (Windows)** 步骤中，会通过 [`scripts/ci/run-tauri-windows-build.ps1`](../scripts/ci/run-tauri-windows-build.ps1) 决定：

- **未配置**签名密钥：行为与原先一致，执行 `pnpm tauri build`（不对安装包做 Authenticode 签名）。
- **已配置**下面两个 **Repository secrets** 时：
  1. 将 Base64 编码的 **PFX** 导入本机构建机的 `LocalMachine\My` 证书存储；
  2. 若机器上能找到 **Inf2Cat**（例如已安装 WDK，或通过变量 `INF2CAT_EXE` 指定路径），则为 `src-tauri/windows-driver` 生成并签名 `slime_smol.cat`（详见 [`DRIVER_SIGNING.md`](DRIVER_SIGNING.md)）；
  3. 使用证书 **指纹** 调用 `pnpm tauri build --config …`，为 **可执行文件与安装包** 启用 Tauri 的 Windows 代码签名（SHA256 + 时间戳）。

## 需要在 GitHub 配置的 Secrets

| Secret | 说明 |
|--------|------|
| `WINDOWS_CODE_SIGNING_PFX_BASE64` | 代码签名证书 **PFX** 文件经 Base64 编码后的整串内容（不含换行）。 |
| `WINDOWS_CODE_SIGNING_PFX_PASSWORD` | 该 PFX 的密码。 |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater 用私钥内容（`tauri signer generate` 生成）。 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 上述 updater 私钥密码（若生成时设置）。 |

生成本地 Base64（PowerShell 示例）：

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\path\to\cert.pfx")) | Set-Clipboard
```

将输出粘贴到 Secret 的值中即可。

## Updater 签名与 Release 资产

- `TAURI_SIGNING_PRIVATE_KEY` 配置后，Tauri 构建会生成 updater 所需签名文件（`.sig`）与 `latest.json`。
- Windows CI 现已上传 MSI/NSIS 及其 `.sig`，并尝试上传 `latest.json` 到 workflow artifacts 与 GitHub Release。
- 自动更新依赖 `src-tauri/tauri.conf.json` 中的 `plugins.updater.pubkey` 与 `endpoints`，请确保公钥和 Release 资产匹配。

本地生成 updater 密钥（PowerShell）示例：

```powershell
pnpm tauri signer generate -w "$HOME/.tauri/foxdock-updater.key"
```

## 可选：指定 Inf2Cat 路径（驱动目录包）

GitHub 托管的 `windows-latest` 镜像**默认不包含** WDK，因此通常**找不到** `Inf2Cat.exe`。此时仍会完成 **应用/安装包签名**，仅跳过驱动 `.cat` 的生成与签名（脚本会打印警告）。

若需要在 CI 中一并生成已签名的驱动目录包，可任选其一：

- 在 **自托管 Runner** 上安装 **Windows Driver Kit (WDK)**，使 `Inf2Cat` 位于常见搜索路径下；或
- 在仓库 **Variables** 中设置 `INF2CAT_EXE` 为 `Inf2Cat.exe` 的完整路径（需该路径在 Runner 上存在）。

## 证书要求

- 使用面向 **Authenticode** 的代码签名证书（商业 CA 或符合组织策略的私用 CA）。
- 驱动目录包 `.cat` 的签名与安装包签名可使用**同一** PFX（常见做法）。

## 相关脚本

| 脚本 | 作用 |
|------|------|
| [`scripts/ci/import-signing-certificate.ps1`](../scripts/ci/import-signing-certificate.ps1) | 解码并导入 PFX，设置 `CERTIFICATE_THUMBPRINT`。 |
| [`scripts/ci/build-and-sign-driver-catalog.ps1`](../scripts/ci/build-and-sign-driver-catalog.ps1) | Inf2Cat + `signtool` 签名 `slime_smol.cat`（若 Inf2Cat 不可用则跳过）。 |
| [`scripts/ci/run-tauri-windows-build.ps1`](../scripts/ci/run-tauri-windows-build.ps1) | 编排上述步骤并执行 `pnpm tauri build`。 |
