# FoxDock Desktop

FoxSnack 追踪器底座的桌面控制台：基于 **Vue 3 + TypeScript + Vite** 的前端，与 **Tauri 2**（Rust）打包为桌面应用。

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
| `pnpm build` | 同步应用版本号后执行类型检查与前端生产构建 |
| `pnpm tauri dev` | 启动 Tauri 开发模式（会先执行版本同步） |
| `pnpm tauri build` | 打包桌面应用 |
| `pnpm run sync:version` | 将 `package.json` 中的版本同步到 `src-tauri/tauri.conf.json` 与 `Cargo.toml` |
| `pnpm run check:version` | 校验上述三处版本是否一致 |

修改代码后建议执行 `pnpm build` 确认类型与构建通过；涉及桌面端功能时再使用 `pnpm tauri dev` 验证。

## 推荐 IDE

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
