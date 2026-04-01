# Windows USB 串口驱动包（INF / CAT）说明

本仓库在 [`src-tauri/windows-driver/`](../src-tauri/windows-driver/) 下提供两个 **usbser** 类 INF，用于在设备管理器中为指定 VID/PID 的 USB CDC 串口显示固定友好名称：

| 设备 | VID | PID |
|------|-----|-----|
| Slime Smol Tracker | `0x1209` | `0x7692` |
| Slime Smol Receiver | `0x1209` | `0x7690` |

## 开发与 CI

- 默认 INF **未** 包含 `CatalogFile=`，便于在未配置 WDK / 未签名目录包时本地与 CI 仍能完成应用构建。
- 安装程序会在安装结束时调用 `pnputil /add-driver … /install` 将 INF 导入驱动存储。在 **未签名** 或未启用测试签名策略的机器上，`pnputil` 可能失败；此时安装器会提示并中止（见 NSIS hook / MSI 自定义动作行为）。

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
