# FoxDock Desktop

Desktop console for the FoxSnack tracker dock: **Vue 3 + TypeScript + Vite** frontend, packaged as a desktop application with **Tauri 2** (Rust).

[中文说明](README-cn.md)

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS recommended)
- [pnpm](https://pnpm.io/) (see `package.json` → `packageManager` for the version pinned by this repository)
- [Rust](https://www.rust-lang.org/) (for Tauri and `src-tauri`)

## Install dependencies

```bash
pnpm install
```

## Common commands

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start the Vite frontend dev server only |
| `pnpm build` | After syncing the derived app version, run type checking and the production frontend build (see “Application version” below) |
| `pnpm tauri dev` | Tauri development mode (runs version sync first) |
| `pnpm tauri build` | Package the desktop application |
| `pnpm run sync:version` | Derive the full version from the current environment and git HEAD, and write it to `src-tauri/tauri.conf.json` and `Cargo.toml` |
| `pnpm run check:version` | Verify that `tauri.conf.json` / `Cargo.toml` match the current derivation rules |

## Recommended IDE

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE).
