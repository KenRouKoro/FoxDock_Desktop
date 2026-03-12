# 项目规则

## 技术栈

- 前端：Vue 3 + TypeScript + Vite
- 桌面端：Tauri 2（Rust）

## 开发约定

- 修改代码后先运行 `npm run build`，确保类型检查与构建通过
- 涉及桌面端功能时，同时验证 `npm run tauri dev`
- 新增功能优先复用现有目录与命名风格

## 项目任务

- 前端开发任务：根据需求文档，使用 Vue 3 + TypeScript 实现前端功能。
- 桌面端开发任务：根据需求文档，使用 Tauri 2（Rust）实现桌面端功能。

## 额外说明

需求文档位于项目更目录下 'project.md' 文件中。