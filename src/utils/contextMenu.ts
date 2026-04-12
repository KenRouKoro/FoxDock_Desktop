import { getCurrentWindow } from "@tauri-apps/api/window";

type ScreenWithAvail = Screen & { availLeft?: number; availTop?: number };

/**
 * 将当前 WebView 视口内坐标（与 getBoundingClientRect 一致，逻辑像素）转为屏幕逻辑坐标。
 * 独立菜单窗内勿使用 `window.screenX`/`screenY`，在 WebView2 中不可靠。
 */
export async function clientPointToScreenLogical(
  clientX: number,
  clientY: number,
): Promise<{ x: number; y: number }> {
  const win = getCurrentWindow();
  const inner = await win.innerPosition();
  const scale = await win.scaleFactor();
  const origin = inner.toLogical(scale);
  return { x: origin.x + clientX, y: origin.y + clientY };
}

/** 将独立菜单窗定位在可用屏幕区域内（逻辑像素，与 screenX/screenY 一致）。 */
export function clampWindowPosition(
  screenX: number,
  screenY: number,
  width: number,
  height: number,
  margin = 8,
): { x: number; y: number } {
  const scr = window.screen as ScreenWithAvail;
  const ax = scr.availLeft ?? 0;
  const ay = scr.availTop ?? 0;
  const aw = window.screen.availWidth;
  const ah = window.screen.availHeight;
  let x = screenX;
  let y = screenY;
  if (x + width + margin > ax + aw) {
    x = ax + aw - width - margin;
  }
  if (y + height + margin > ay + ah) {
    y = ay + ah - height - margin;
  }
  x = Math.max(ax + margin, x);
  y = Math.max(ay + margin, y);
  return { x, y };
}

/**
 * 与 ContextMenuShell 中 `.menu-header`（padding 上下各 6px + 约 12px 字行高）对齐。
 */
const HEADER_H = 32;
/**
 * 与 `.menu-item`（padding 上下各 8px + 14px 字默认行高约 20px）对齐，避免窗高不足把末行压扁。
 */
const ROW_H = 38;
/** 根容器上下 2px 边框、取整与余量 */
const MENU_SIZE_TAIL = 0;

export function estimateTrackerParentMenuSize(actionCount: number): { width: number; height: number } {
  const rows = 1 + actionCount;
  return { width: 200, height: HEADER_H + rows * ROW_H + MENU_SIZE_TAIL };
}

export function estimateReceiverParentMenuSize(): { width: number; height: number } {
  const rows = 2;
  return { width: 220, height: HEADER_H + rows * ROW_H + MENU_SIZE_TAIL };
}

export function estimateReceiverSubmenuSize(itemCount: number): { width: number; height: number } {
  const groupHeaders = 4;
  const h = Math.min(480, HEADER_H + (itemCount + groupHeaders) * ROW_H + MENU_SIZE_TAIL);
  return { width: 280, height: h };
}
