//! 右键菜单独立 Webview 窗口（父窗 `context-menu`、子窗 `context-submenu`）。
//! 状态存于 `ContextMenuState`，通过 `emit_to` 下发，避免超长 URL 与首帧丢数据。
//! Windows 上同步创建 Webview 可能死锁，打开窗口使用 `async` 命令。
//! 父子菜单视为一组：失焦后延迟检查，仅当两窗均无焦点时关闭。

use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

/// 与前端菜单背景一致，减少首帧黑/灰闪屏。
const MENU_WINDOW_BG: Color = Color(255, 255, 255, 255);

/// 打开/复用菜单窗口时调用：取消尚未执行的「失焦关闭」任务，避免父子切换时误关。
fn bump_menu_focus_debounce() {
    MENU_FOCUS_DEBOUNCE_GEN.fetch_add(1, Ordering::SeqCst);
}

const URL_PARENT: &str = "/?contextMenu=true&role=parent";
const URL_SUBMENU: &str = "/?contextMenu=true&role=submenu";

/// 菜单 payload 缓存，供 `get_context_menu_state` 与 `emit_to` 使用。
pub struct ContextMenuState {
    pub parent: Mutex<Option<Value>>,
    pub submenu: Mutex<Option<Value>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            parent: Mutex::new(None),
            submenu: Mutex::new(None),
        }
    }
}

/// 失焦防抖：每次菜单窗 `Focused(false)` 递增，仅最后一次延迟任务生效。
static MENU_FOCUS_DEBOUNCE_GEN: AtomicU64 = AtomicU64::new(0);

/// 窗口销毁后需重新挂监听；与 `close_context_menu_windows` 配对复位。
static CONTEXT_MENU_FOCUS_LISTENER_ATTACHED: AtomicBool = AtomicBool::new(false);
static CONTEXT_SUBMENU_FOCUS_LISTENER_ATTACHED: AtomicBool = AtomicBool::new(false);

fn reset_menu_focus_listener_flags() {
    CONTEXT_MENU_FOCUS_LISTENER_ATTACHED.store(false, Ordering::SeqCst);
    CONTEXT_SUBMENU_FOCUS_LISTENER_ATTACHED.store(false, Ordering::SeqCst);
}

fn listener_flag_for_label(label: &str) -> Option<&'static AtomicBool> {
    match label {
        "context-menu" => Some(&CONTEXT_MENU_FOCUS_LISTENER_ATTACHED),
        "context-submenu" => Some(&CONTEXT_SUBMENU_FOCUS_LISTENER_ATTACHED),
        _ => None,
    }
}

fn schedule_close_if_menu_group_blurred(app: &AppHandle) {
    let gen = MENU_FOCUS_DEBOUNCE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(280));
        if MENU_FOCUS_DEBOUNCE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let app_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            let parent_focused = app_main
                .get_webview_window("context-menu")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            let sub_focused = app_main
                .get_webview_window("context-submenu")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if !parent_focused && !sub_focused {
                let state = app_main.state::<ContextMenuState>();
                let _ = close_context_menu_windows(app_main.clone(), state);
            }
        });
    });
}

fn attach_menu_focus_listener_if_needed(app: &AppHandle, label: &str, win: &WebviewWindow) {
    let Some(flag) = listener_flag_for_label(label) else {
        return;
    };
    if flag.swap(true, Ordering::SeqCst) {
        return;
    }
    let app_handle = app.clone();
    win.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            schedule_close_if_menu_group_blurred(&app_handle);
        }
    });
}

fn apply_menu_window_appearance(win: &WebviewWindow) {
    apply_menu_window_shadow_and_bg(win);
    apply_platform_menu_window_chrome(win);
}

fn apply_menu_window_shadow_and_bg(win: &WebviewWindow) {
    let _ = win.set_shadow(false);
    let _ = win.set_background_color(Some(MENU_WINDOW_BG));
}

#[cfg(windows)]
fn apply_platform_menu_window_chrome(win: &WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    let Ok(hwnd) = win.hwnd() else {
        return;
    };
    let hwnd: HWND = hwnd.0;
    unsafe {
        let force_disable: i32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED as u32,
            &force_disable as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        let corner: i32 = DWMWCP_DONOTROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

#[cfg(not(windows))]
fn apply_platform_menu_window_chrome(_win: &WebviewWindow) {}

/// 首帧页面加载完成后再显示，避免 WebView 就绪前空白/拉伸被看见。
fn reveal_menu_window_first_paint(win: &WebviewWindow, app: &AppHandle, payload: &Value, is_submenu: bool) {
    let _ = win.set_always_on_top(true);
    let _ = win.show();
    apply_platform_menu_window_chrome(win);
    let _ = win.set_focus();
    if is_submenu {
        emit_submenu_state(app, payload);
    } else {
        emit_parent_state(app, payload);
    }
}

fn close_labeled(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}

fn emit_parent_state(app: &AppHandle, payload: &Value) {
    let _ = app.emit_to("context-menu", "context-menu-state", payload);
}

fn emit_submenu_state(app: &AppHandle, payload: &Value) {
    let _ = app.emit_to("context-submenu", "context-menu-state", payload);
}

/// 关闭子菜单再关闭父菜单，并清空状态。
#[tauri::command]
pub fn close_context_menu_windows(app: AppHandle, state: State<'_, ContextMenuState>) -> Result<(), String> {
    if let Ok(mut g) = state.parent.lock() {
        *g = None;
    }
    if let Ok(mut g) = state.submenu.lock() {
        *g = None;
    }
    close_labeled(&app, "context-submenu");
    close_labeled(&app, "context-menu");
    reset_menu_focus_listener_flags();
    Ok(())
}

#[tauri::command]
pub fn get_context_menu_state(
    state: State<'_, ContextMenuState>,
    role: String,
) -> Result<Option<Value>, String> {
    let v = match role.as_str() {
        "parent" => state.parent.lock().map_err(|e| e.to_string())?.clone(),
        "submenu" => state.submenu.lock().map_err(|e| e.to_string())?.clone(),
        _ => return Err("invalid role".to_string()),
    };
    Ok(v)
}

#[tauri::command]
pub async fn open_context_menu_window(
    app: AppHandle,
    state: State<'_, ContextMenuState>,
    screen_x: f64,
    screen_y: f64,
    width: f64,
    height: f64,
    payload_json: String,
) -> Result<(), String> {
    bump_menu_focus_debounce();
    let payload: Value = serde_json::from_str(&payload_json).map_err(|e| e.to_string())?;
    {
        let mut g = state.parent.lock().map_err(|e| e.to_string())?;
        *g = Some(payload.clone());
    }

    close_labeled(&app, "context-submenu");
    if let Ok(mut g) = state.submenu.lock() {
        *g = None;
    }

    let w = width.max(120.0);
    let h = height.max(80.0);

    if let Some(win) = app.get_webview_window("context-menu") {
        apply_menu_window_appearance(&win);
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        let _ = win.set_position(tauri::LogicalPosition::new(screen_x, screen_y));
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        emit_parent_state(&app, &payload);
        return Ok(());
    }

    let app_for_load = app.clone();
    let payload_for_load = payload.clone();
    let first_paint = Arc::new(AtomicBool::new(false));

    let window = WebviewWindowBuilder::new(
        &app,
        "context-menu",
        WebviewUrl::App(URL_PARENT.into()),
    )
    .title(" ")
    .decorations(false)
    .shadow(false)
    .background_color(MENU_WINDOW_BG)
    .visible(false)
    .resizable(false)
    .skip_taskbar(true)
    .inner_size(w, h)
    .position(screen_x, screen_y)
    .on_page_load({
        let app = app_for_load.clone();
        let payload = payload_for_load.clone();
        let first_paint = first_paint.clone();
        move |_win, load| {
            if load.event() != PageLoadEvent::Finished {
                return;
            }
            if first_paint.swap(true, Ordering::SeqCst) {
                return;
            }
            let handle = app.clone();
            let pl = payload.clone();
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(w) = h.get_webview_window("context-menu") {
                    reveal_menu_window_first_paint(&w, &h, &pl, false);
                }
            });
        }
    })
    .build()
    .map_err(|e| e.to_string())?;

    apply_menu_window_shadow_and_bg(&window);
    attach_menu_focus_listener_if_needed(&app, "context-menu", &window);
    Ok(())
}

#[tauri::command]
pub async fn open_context_submenu_window(
    app: AppHandle,
    state: State<'_, ContextMenuState>,
    screen_x: f64,
    screen_y: f64,
    width: f64,
    height: f64,
    payload_json: String,
) -> Result<(), String> {
    bump_menu_focus_debounce();
    let payload: Value = serde_json::from_str(&payload_json).map_err(|e| e.to_string())?;
    {
        let mut g = state.submenu.lock().map_err(|e| e.to_string())?;
        *g = Some(payload.clone());
    }

    let w = width.max(160.0);
    let h = height.max(80.0);

    if let Some(win) = app.get_webview_window("context-submenu") {
        apply_menu_window_appearance(&win);
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        let _ = win.set_position(tauri::LogicalPosition::new(screen_x, screen_y));
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        emit_submenu_state(&app, &payload);
        return Ok(());
    }

    let app_for_load = app.clone();
    let payload_for_load = payload.clone();
    let first_paint = Arc::new(AtomicBool::new(false));

    let window = WebviewWindowBuilder::new(
        &app,
        "context-submenu",
        WebviewUrl::App(URL_SUBMENU.into()),
    )
    .title(" ")
    .decorations(false)
    .shadow(false)
    .background_color(MENU_WINDOW_BG)
    .visible(false)
    .resizable(false)
    .skip_taskbar(true)
    .inner_size(w, h)
    .position(screen_x, screen_y)
    .on_page_load({
        let app = app_for_load.clone();
        let payload = payload_for_load.clone();
        let first_paint = first_paint.clone();
        move |_win, load| {
            if load.event() != PageLoadEvent::Finished {
                return;
            }
            if first_paint.swap(true, Ordering::SeqCst) {
                return;
            }
            let handle = app.clone();
            let pl = payload.clone();
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(w) = h.get_webview_window("context-submenu") {
                    reveal_menu_window_first_paint(&w, &h, &pl, true);
                }
            });
        }
    })
    .build()
    .map_err(|e| e.to_string())?;

    apply_menu_window_shadow_and_bg(&window);
    attach_menu_focus_listener_if_needed(&app, "context-submenu", &window);
    Ok(())
}
