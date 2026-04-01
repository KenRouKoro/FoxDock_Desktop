# Windows USB 串口驱动包（INF / CAT）说明

本仓库在 [`src-tauri/windows-driver/`](../src-tauri/windows-driver/) 下提供两个 **usbser** 类 INF，用于在设备管理器中为指定 VID/PID 的 USB CDC 串口显示固定友好名称：

| 设备 | VID | PID |
|------|-----|-----|
| Slime Smol Tracker | `0x1209` | `0x7692` |
| Slime Smol Receiver | `0x1209` | `0x7690` |

## 开发与 CI

- 默认 INF **未** 包含 `CatalogFile=`，便于在未配置 WDK / 未签名目录包时本地与 CI 仍能完成应用构建。
- **在 GitHub Actions 中自动签名**（安装包 + 可选驱动 `.cat`）的配置说明见 [CI_SIGNING.md](CI_SIGNING.md)。
- 安装程序会在安装结束时调用 `pnputil /add-driver … /install` 将 INF 导入驱动存储。在 **无签名目录包** 时 `pnputil` 常会失败：安装器会**提示但继续完成应用安装**，INF 仍保留在安装目录的 `resources\windows-driver\` 下。

### 管理员权限与「此时安装 INF」

- **NSIS**：`bundle.windows.nsis.installMode` 为 **`perMachine`** 时，Tauri 生成的安装程序使用 **`RequestExecutionLevel admin`**，用户通过 UAC 同意后，**整个安装（含 `POSTINSTALL` 钩子）在同一管理员上下文中执行**。钩子内直接调用 `pnputil`，**不需要**再 `RunAs` 提权（否则可能触发第二次 UAC）。安装脚本会优先使用 **64 位** `pnputil`（`System32`，在 32 位安装程序环境下必要时经 `Sysnative`），避免 WoW64 误用 32 位工具。
- **MSI**：`pnputil` 放在 **deferred** 且 **`Impersonate="no"`** 的自定义动作中执行，通常以 **LocalSystem** 运行，权限高于普通管理员，同样满足向驱动存储添加包的要求。

若仍失败，原因多为 **未签名的目录包策略**，而不是「没有管理员权限」。

## 无商业 CA 证书时，还能怎么做？

在拿不到 **公开 CA 代码签名证书** 的前提下，系统级「设备管理器里的友好名称」与「静默 `pnputil` 成功」通常**无法同时**满足，可按下述优先级取舍：

### 1. 应用内名称（不依赖 INF，推荐）

本应用已在 Rust 侧对 `0x1209:0x7692` / `0x1209:0x7690` 做 **VID/PID → 固定显示名** 映射（并可通过 `list_slime_smol_serial_ports` 枚举）。**不装 INF、不签名**，下拉列表与逻辑里仍可显示 **Slime Smol Tracker / Receiver**。这是无 CA 时最稳妥的产品体验。

### 2. 保留 INF 文件，由用户或管理员手动安装

安装包始终附带 `resources\windows-driver\*.inf`。自动 `pnputil` 失败时，可：

- 在**设备管理器**中选中对应「USB 串行设备」→ 更新驱动程序 → **浏览我的电脑以查找驱动程序** → 选该目录下的 `.inf`；或  
- 在**管理员**命令提示符中尝试：  
  `pnputil /add-driver "路径\slime_smol_xxx.inf" /install`  
  未签名时 Windows 可能弹出安全提示，用户需确认。

部分环境还可配合策略或测试用途使用 **`pnputil` 的 `/acceptUnsigned`**（视 Windows 版本与安全策略而定），仅建议在明确知情的前提下使用。

### 3. 自签名 + 测试签名模式（仅开发 / 内部机）

用 **自签名证书** 对目录包签名，并在目标机上启用 **测试签名**（`bcdedit /set testsigning on`，需管理员且重启）。仅适合开发机或可控内网，**不适合**向普通最终用户分发。

### 4. 以后有预算再上 CA

取得标准 **代码签名证书** 后，按下文「正式发布」生成 `.cat` 并签名，即可恢复「安装时静默注册 INF」的体验。

## 正式发布（推荐）

对 64 位 Windows 10/11 的 **标准** 用户环境，第三方驱动目录包应包含：

1. **Catalog（.cat）**  
   使用 Windows Driver Kit 中的 **Inf2Cat** 为包含 INF 的目录生成目录文件。
2. **Authenticode 签名**  
   使用有效的**代码签名证书**对 `.cat`（及需要时的 `.inf`）签名。
3. **在 INF 的 `[Version]` 段** 增加一行，例如：  
   `CatalogFile=slime_smol.cat`  
   并将生成的 `slime_smol.cat` 与两个 INF 一并放入 `src-tauri/windows-driver/`，再执行发布构建。

详细步骤见 Microsoft 文档：

- [Creating a Catalog File for a PnP Driver Package](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/creating-a-catalog-file-for-a-pnp-driver-package)
- [Catalog Files and Digital Signatures](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/catalog-files)

## 验证

安装完成后可在管理员命令提示符中执行：

```bat
pnputil /enum-drivers | findstr /i slime
```

并插入设备后在设备管理器中确认友好名称与串口枚举一致。
