/**
 * 右键独立菜单窗与主窗口之间的事件协议（Tauri `emit` / `listen`）。
 */

/** 主菜单窗 payload（由后端 ContextMenuState 缓存并通过 emit_to 下发） */
export type ContextMenuParentPayload =
  | {
      variant: "tracker";
      trackerId: number;
      inserted: boolean;
      /** 与 Home 中 loading 或 !connected 一致，禁用槽位动作 */
      actionsDisabled: boolean;
    }
  | {
      variant: "receiver";
      canUseReceiverActions: boolean;
    };

/** 接收器「命令」子菜单：仅存标志位，命令列表由前端 `buildReceiverContextMenuGroups()` 生成 */
export type ContextMenuSubmenuPayload = {
  variant: "receiver-commands";
  canUseReceiverActions: boolean;
};

/** 主窗口监听 `context-menu-selection` */
export type ContextMenuSelection =
  | { kind: "tracker-open-console"; trackerId: number }
  | { kind: "tracker-action"; action: string; trackerId: number }
  | { kind: "receiver-open-console" }
  | { kind: "receiver-hid-line"; line: string }
  | { kind: "receiver-param"; cmdKey: string };

/** 主窗口打开父菜单时传入 invoke 的参数 */
export type OpenContextMenuWindowRequest = {
  screenX: number;
  screenY: number;
  width: number;
  height: number;
  payload: ContextMenuParentPayload;
};

/** 打开子菜单窗 */
export type OpenContextSubmenuWindowRequest = {
  screenX: number;
  screenY: number;
  width: number;
  height: number;
  payload: ContextMenuSubmenuPayload;
};
