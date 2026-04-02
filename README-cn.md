# FoxDock Desktop

FoxSnack 追踪器底座的桌面控制台：基于 **Vue 3 + TypeScript + Vite** 的前端，与 **Tauri 2**（Rust）打包为桌面应用。

[English README](README.md)

## 环境要求

- [Node.js](https://nodejs.org/)（建议 LTS）
- [pnpm](https://pnpm.io/)（仓库指定版本见 `package.json` 的 `packageManager`）
- [Rust](https://www.rust-lang.org/)（用于 Tauri 与 `src-tauri`）

## 安装依赖

```bash
pnpm install
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `pnpm dev` | 仅启动 Vite 前端开发服务器 |
| `pnpm build` | 同步派生应用版本后执行类型检查与前端生产构建（见下方「应用版本号」） |
| `pnpm tauri dev` | 启动 Tauri 开发模式（会先执行版本同步） |
| `pnpm tauri build` | 打包桌面应用 |
| `pnpm run sync:version` | 按当前环境与 git HEAD 派生完整版本并写入 `src-tauri/tauri.conf.json` 与 `Cargo.toml` |
| `pnpm run check:version` | 校验 `tauri.conf.json` / `Cargo.toml` 是否与当前派生规则一致 |

## 推荐 IDE

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 许可证

本项目以 [GNU Affero General Public License v3.0](LICENSE) 授权。
