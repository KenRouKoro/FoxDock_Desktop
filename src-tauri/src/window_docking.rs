use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as std_mpsc, Mutex, OnceLock};
use std::thread::JoinHandle;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const FOXDOCK_DEFAULT_WIDTH: i32 = 480;
const I18N_ERROR_PREFIX: &str = "i18n:";

fn i18n_error(key: &str) -> String {
    format!("{I18N_ERROR_PREFIX}{key}")
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowDockingConfig {
    pub auto_dock_on_startup: bool,
    pub dock_always_on_top: bool,
    pub follow_slime_vr_window: bool,
    pub snap_style_approximation: bool,
}

impl Default for WindowDockingConfig {
    fn default() -> Self {
        Self {
            auto_dock_on_startup: true,
            dock_always_on_top: false,
            follow_slime_vr_window: false,
            snap_style_approximation: false,
        }
    }
}

impl WindowDockingConfig {
    pub fn from_system_settings(settings: &crate::system_settings::SystemSettings) -> Self {
        Self {
            auto_dock_on_startup: settings.auto_dock_on_startup,
            dock_always_on_top: settings.dock_always_on_top,
            follow_slime_vr_window: settings.follow_slime_vr_window,
            snap_style_approximation: settings.snap_style_approximation,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl WorkArea {
    fn width(self) -> i32 {
        self.right - self.left
    }

    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RectI32 {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl RectI32 {
    fn width(self) -> i32 {
        self.right - self.left
    }

    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

type NativeHwnd = isize;

#[derive(Debug, Clone, Copy)]
struct TargetWindowInfo {
    hwnd: NativeHwnd,
    process_id: u32,
    thread_id: u32,
}

#[cfg(target_os = "windows")]
fn native_from_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> NativeHwnd {
    hwnd as NativeHwnd
}

#[cfg(target_os = "windows")]
fn hwnd_from_native(hwnd: NativeHwnd) -> windows_sys::Win32::Foundation::HWND {
    hwnd as windows_sys::Win32::Foundation::HWND
}

struct FollowRuntime {
    listener_thread_id: u32,
    join_handle: JoinHandle<()>,
}

pub struct WindowDockingState {
    locked_y: Mutex<Option<i32>>,
    active_work_area: Mutex<Option<WorkArea>>,
    config: Mutex<WindowDockingConfig>,
    target_window: Mutex<Option<TargetWindowInfo>>,
    styled_window: Mutex<Option<NativeHwnd>>,
    follow_runtime: Mutex<Option<FollowRuntime>>,
    correction_in_progress: AtomicBool,
    follow_apply_in_progress: AtomicBool,
    layout_active: AtomicBool,
}

impl Default for WindowDockingState {
    fn default() -> Self {
        Self {
            locked_y: Mutex::new(None),
            active_work_area: Mutex::new(None),
            config: Mutex::new(WindowDockingConfig::default()),
            target_window: Mutex::new(None),
            styled_window: Mutex::new(None),
            follow_runtime: Mutex::new(None),
            correction_in_progress: AtomicBool::new(false),
            follow_apply_in_progress: AtomicBool::new(false),
            layout_active: AtomicBool::new(false),
        }
    }
}

impl WindowDockingState {
    fn remember_work_area(&self, work_area: WorkArea) {
        if let Ok(mut guard) = self.locked_y.lock() {
            *guard = Some(work_area.top);
        }
        if let Ok(mut guard) = self.active_work_area.lock() {
            *guard = Some(work_area);
        }
    }

    fn current_work_area(&self) -> Option<WorkArea> {
        self.active_work_area.lock().ok().and_then(|guard| *guard)
    }

    fn locked_y(&self) -> Option<i32> {
        self.locked_y.lock().ok().and_then(|guard| *guard)
    }

    fn set_config(&self, config: WindowDockingConfig) {
        if let Ok(mut guard) = self.config.lock() {
            *guard = config;
        }
    }

    fn config(&self) -> WindowDockingConfig {
        self.config.lock().map(|guard| *guard).unwrap_or_default()
    }

    fn set_target_window(&self, target: Option<TargetWindowInfo>) {
        if let Ok(mut guard) = self.target_window.lock() {
            *guard = target;
        }
    }

    fn target_window(&self) -> Option<TargetWindowInfo> {
        self.target_window.lock().ok().and_then(|guard| *guard)
    }

    fn set_styled_window(&self, hwnd: Option<NativeHwnd>) {
        if let Ok(mut guard) = self.styled_window.lock() {
            *guard = hwnd;
        }
    }

    fn styled_window(&self) -> Option<NativeHwnd> {
        self.styled_window.lock().ok().and_then(|guard| *guard)
    }

    fn set_layout_active(&self, active: bool) {
        self.layout_active.store(active, Ordering::Release);
    }

    fn is_layout_active(&self) -> bool {
        self.layout_active.load(Ordering::Acquire)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockWindowsResult {
    pub slimevr_found: bool,
    pub slimevr_docked: bool,
}

struct FoxDockLayout {
    work_area: WorkArea,
    x: i32,
}

pub fn initialize_runtime_config(
    window: &WebviewWindow,
    state: &WindowDockingState,
    config: WindowDockingConfig,
) -> Result<(), String> {
    state.set_config(config);
    if config.auto_dock_on_startup {
        align_main_window_to_primary_work_area(window, state)?;
    } else {
        state.set_layout_active(false);
        let _ = window.set_always_on_top(false);
    }
    Ok(())
}

pub fn sync_runtime_config(
    app_handle: &AppHandle,
    state: &WindowDockingState,
    config: WindowDockingConfig,
) -> Result<(), String> {
    state.set_config(config);

    if let Some(main_window) = app_handle.get_webview_window("main") {
        apply_foxdock_topmost(&main_window, state)?;
    }

    if !config.follow_slime_vr_window {
        stop_follow_runtime(state)?;
    } else if let Some(target) = state.target_window() {
        start_follow_runtime(app_handle, state, target)?;
    }

    if !config.snap_style_approximation {
        restore_snap_style(state);
    } else if let Some(target) = state.target_window() {
        let _ = apply_snap_style_to_target(state, Some(target.hwnd));
    }

    Ok(())
}

pub fn align_main_window_to_primary_work_area(
    window: &WebviewWindow,
    state: &WindowDockingState,
) -> Result<(), String> {
    let work_area = primary_work_area()?;
    let _ = apply_foxdock_layout(window, state, work_area)?;
    state.set_target_window(None);
    stop_follow_runtime(state)?;
    let _ = apply_snap_style_to_target(state, None);
    apply_foxdock_topmost(window, state)?;
    Ok(())
}

pub fn handle_main_window_moved(
    window: &WebviewWindow,
    state: &WindowDockingState,
) -> Result<(), String> {
    if !state.is_layout_active() {
        return Ok(());
    }

    if state.correction_in_progress.swap(false, Ordering::AcqRel) {
        return Ok(());
    }

    let work_area = state
        .current_work_area()
        .or_else(|| primary_work_area().ok())
        .ok_or_else(|| i18n_error("backend_errors.primary_work_area_unavailable"))?;
    state.remember_work_area(work_area);

    let locked_y = state.locked_y().unwrap_or(work_area.top);
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let width = current_window_width(window);
    let max_x = (work_area.right - width).max(work_area.left);
    let clamped_x = position.x.clamp(work_area.left, max_x);
    if position.y == locked_y && position.x == clamped_x {
        return Ok(());
    }

    state.correction_in_progress.store(true, Ordering::Release);
    if let Err(error) = window.set_position(PhysicalPosition::new(clamped_x, locked_y)) {
        state.correction_in_progress.store(false, Ordering::Release);
        return Err(error.to_string());
    }

    Ok(())
}

pub fn dock_main_window_with_slimevr(
    app_handle: &AppHandle,
    window: &WebviewWindow,
    state: &WindowDockingState,
) -> Result<DockWindowsResult, String> {
    let config = state.config();
    let Some(target) = find_slimevr_main_window()? else {
        align_main_window_to_primary_work_area(window, state)?;
        return Ok(DockWindowsResult {
            slimevr_found: false,
            slimevr_docked: false,
        });
    };

    dock_main_window_with_target(window, state, target)?;
    if config.follow_slime_vr_window {
        start_follow_runtime(app_handle, state, target)?;
    } else {
        stop_follow_runtime(state)?;
    }

    Ok(DockWindowsResult {
        slimevr_found: true,
        slimevr_docked: true,
    })
}

fn dock_main_window_with_target(
    window: &WebviewWindow,
    state: &WindowDockingState,
    target: TargetWindowInfo,
) -> Result<(), String> {
    let work_area = work_area_for_window(target.hwnd).or_else(|_| primary_work_area())?;
    let foxdock_layout = apply_foxdock_layout(window, state, work_area)?;
    let slimevr_rect = RectI32 {
        left: foxdock_layout.work_area.left,
        top: foxdock_layout.work_area.top,
        right: foxdock_layout.x,
        bottom: foxdock_layout.work_area.bottom,
    };

    if slimevr_rect.width() <= 0 || foxdock_layout.work_area.width() <= current_window_width(window) {
        return Err(i18n_error("backend_errors.primary_work_area_too_narrow"));
    }

    set_external_window_rect(target.hwnd, slimevr_rect)?;
    focus_external_window(target.hwnd);
    state.set_target_window(Some(target));
    let _ = apply_snap_style_to_target(state, Some(target.hwnd));
    apply_foxdock_topmost(window, state)?;
    Ok(())
}

fn follow_main_window_with_target(
    window: &WebviewWindow,
    state: &WindowDockingState,
    target: TargetWindowInfo,
) -> Result<(), String> {
    let work_area = work_area_for_window(target.hwnd).or_else(|_| primary_work_area())?;
    let target_rect = current_window_rect(target.hwnd)?;
    let _ = apply_foxdock_layout_next_to_target(window, state, work_area, target_rect)?;
    state.set_target_window(Some(target));
    let _ = apply_snap_style_to_target(state, Some(target.hwnd));
    apply_foxdock_topmost(window, state)?;
    Ok(())
}

fn apply_foxdock_layout(
    window: &WebviewWindow,
    state: &WindowDockingState,
    work_area: WorkArea,
) -> Result<FoxDockLayout, String> {
    let width = current_window_width(window);
    let height = work_area.height();
    let x = (work_area.right - width).max(work_area.left);

    window
        .set_size(PhysicalSize::new(width as u32, height as u32))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, work_area.top))
        .map_err(|error| error.to_string())?;

    state.remember_work_area(work_area);
    state.set_layout_active(true);

    Ok(FoxDockLayout { work_area, x })
}

fn apply_foxdock_layout_next_to_target(
    window: &WebviewWindow,
    state: &WindowDockingState,
    work_area: WorkArea,
    target_rect: RectI32,
) -> Result<FoxDockLayout, String> {
    let width = current_window_width(window);
    let height = work_area.height();
    let max_x = (work_area.right - width).max(work_area.left);
    let x = target_rect.right.clamp(work_area.left, max_x);

    window
        .set_size(PhysicalSize::new(width as u32, height as u32))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, work_area.top))
        .map_err(|error| error.to_string())?;

    state.remember_work_area(work_area);
    state.set_layout_active(true);

    Ok(FoxDockLayout { work_area, x })
}

fn apply_foxdock_topmost(window: &WebviewWindow, state: &WindowDockingState) -> Result<(), String> {
    let should_topmost = state.is_layout_active() && state.config().dock_always_on_top;
    window
        .set_always_on_top(should_topmost)
        .map_err(|error| error.to_string())
}

fn current_window_width(window: &WebviewWindow) -> i32 {
    window
        .outer_size()
        .ok()
        .map(|size| size.width as i32)
        .filter(|width| *width > 0)
        .unwrap_or(FOXDOCK_DEFAULT_WIDTH)
}

fn apply_snap_style_to_target(
    state: &WindowDockingState,
    target: Option<NativeHwnd>,
) -> Result<(), String> {
    let previous = state.styled_window();
    if previous != target {
        if let Some(hwnd) = previous {
            let _ = set_window_corner_preference(hwnd, false);
        }
    }

    if state.config().snap_style_approximation {
        if let Some(hwnd) = target {
            let _ = set_window_corner_preference(hwnd, true);
            state.set_styled_window(Some(hwnd));
            return Ok(());
        }
    }

    state.set_styled_window(None);
    Ok(())
}

fn restore_snap_style(state: &WindowDockingState) {
    if let Some(hwnd) = state.styled_window() {
        let _ = set_window_corner_preference(hwnd, false);
    }
    state.set_styled_window(None);
}

fn start_follow_runtime(
    app_handle: &AppHandle,
    state: &WindowDockingState,
    target: TargetWindowInfo,
) -> Result<(), String> {
    if !state.config().follow_slime_vr_window {
        return Ok(());
    }

    let existing_target = state.target_window();
    let has_same_target = existing_target
        .map(|current| current.hwnd == target.hwnd)
        .unwrap_or(false);
    let runtime_exists = state
        .follow_runtime
        .lock()
        .map_err(|error| error.to_string())?
        .is_some();
    if runtime_exists && has_same_target {
        return Ok(());
    }

    stop_follow_runtime(state)?;
    state.set_target_window(Some(target));
    install_follow_controller(app_handle.clone(), target.hwnd);

    let (ready_tx, ready_rx) = std_mpsc::channel::<Result<u32, String>>();
    let listener = std::thread::spawn(move || follow_thread_main(target, ready_tx));
    let listener_thread_id = ready_rx
        .recv()
        .map_err(|_| i18n_error("backend_errors.window_follow_thread_failed"))??;

    let mut guard = state.follow_runtime.lock().map_err(|error| error.to_string())?;
    *guard = Some(FollowRuntime {
        listener_thread_id,
        join_handle: listener,
    });
    Ok(())
}

fn stop_follow_runtime(state: &WindowDockingState) -> Result<(), String> {
    let runtime = {
        let mut guard = state.follow_runtime.lock().map_err(|error| error.to_string())?;
        guard.take()
    };

    if let Some(runtime) = runtime {
        stop_follow_listener(runtime)?;
    }

    clear_follow_controller();
    Ok(())
}

#[cfg(target_os = "windows")]
fn follow_thread_main(target: TargetWindowInfo, ready_tx: std_mpsc::Sender<Result<u32, String>>) {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMessageW, PeekMessageW, MSG, PM_NOREMOVE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE,
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MINIMIZEEND, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS,
    };

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);

        let location_hook = SetWinEventHook(
            EVENT_OBJECT_LOCATIONCHANGE,
            EVENT_OBJECT_LOCATIONCHANGE,
            std::ptr::null_mut(),
            Some(slimevr_win_event_proc),
            target.process_id,
            target.thread_id,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let destroy_hook = SetWinEventHook(
            EVENT_OBJECT_DESTROY,
            EVENT_OBJECT_DESTROY,
            std::ptr::null_mut(),
            Some(slimevr_win_event_proc),
            target.process_id,
            target.thread_id,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let hide_hook = SetWinEventHook(
            EVENT_OBJECT_HIDE,
            EVENT_OBJECT_HIDE,
            std::ptr::null_mut(),
            Some(slimevr_win_event_proc),
            target.process_id,
            target.thread_id,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let minimize_hook = SetWinEventHook(
            EVENT_SYSTEM_MINIMIZEEND,
            EVENT_SYSTEM_MINIMIZEEND,
            std::ptr::null_mut(),
            Some(slimevr_win_event_proc),
            target.process_id,
            target.thread_id,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        if location_hook.is_null()
            && destroy_hook.is_null()
            && hide_hook.is_null()
            && minimize_hook.is_null()
        {
            let _ = ready_tx.send(Err(i18n_error("backend_errors.window_follow_hook_failed")));
            return;
        }

        let _ = ready_tx.send(Ok(GetCurrentThreadId()));

        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}

        if !location_hook.is_null() {
            UnhookWinEvent(location_hook);
        }
        if !destroy_hook.is_null() {
            UnhookWinEvent(destroy_hook);
        }
        if !hide_hook.is_null() {
            UnhookWinEvent(hide_hook);
        }
        if !minimize_hook.is_null() {
            UnhookWinEvent(minimize_hook);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn follow_thread_main(_target: TargetWindowInfo, ready_tx: std_mpsc::Sender<Result<u32, String>>) {
    let _ = ready_tx.send(Err(i18n_error("backend_errors.windows_only")));
}

fn stop_follow_listener(runtime: FollowRuntime) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

        let ok = PostThreadMessageW(runtime.listener_thread_id, WM_QUIT, 0, 0);
        if ok == 0 {
            return Err(i18n_error("backend_errors.window_follow_stop_failed"));
        }
    }

    runtime
        .join_handle
        .join()
        .map_err(|_| i18n_error("backend_errors.window_follow_join_failed"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct FollowController {
    app_handle: AppHandle,
    target_hwnd: NativeHwnd,
}

#[cfg(target_os = "windows")]
static FOLLOW_CONTROLLER: OnceLock<Mutex<Option<FollowController>>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn follow_controller() -> &'static Mutex<Option<FollowController>> {
    FOLLOW_CONTROLLER.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn install_follow_controller(app_handle: AppHandle, target_hwnd: NativeHwnd) {
    if let Ok(mut guard) = follow_controller().lock() {
        *guard = Some(FollowController {
            app_handle,
            target_hwnd,
        });
    }
}

#[cfg(target_os = "windows")]
fn clear_follow_controller() {
    if let Ok(mut guard) = follow_controller().lock() {
        *guard = None;
    }
}

#[cfg(not(target_os = "windows"))]
fn install_follow_controller(_app_handle: AppHandle, _target_hwnd: NativeHwnd) {}

#[cfg(not(target_os = "windows"))]
fn clear_follow_controller() {}

#[cfg(target_os = "windows")]
unsafe extern "system" fn slimevr_win_event_proc(
    _hook: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
    event: u32,
    hwnd: windows_sys::Win32::Foundation::HWND,
    idobject: i32,
    idchild: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if idobject != 0 || idchild != 0 {
        return;
    }

    let Some(controller) = follow_controller()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().cloned())
    else {
        return;
    };

    let native_hwnd = native_from_hwnd(hwnd);
    if native_hwnd != controller.target_hwnd {
        return;
    }

    let docking_state = controller.app_handle.state::<WindowDockingState>();
    let state = docking_state.inner();
    if state.follow_apply_in_progress.load(Ordering::Acquire) {
        return;
    }

    let Some(main_window) = controller.app_handle.get_webview_window("main") else {
        return;
    };

    if matches!(
        event,
        windows_sys::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_DESTROY
            | windows_sys::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_HIDE
    ) {
        restore_snap_style(state);
        state.set_target_window(None);
        let _ = apply_foxdock_topmost(&main_window, state);
        return;
    }

    state.follow_apply_in_progress.store(true, Ordering::Release);
    let target = state
        .target_window()
        .filter(|target| target.hwnd == native_hwnd)
        .or_else(|| find_slimevr_main_window().ok().flatten());
    if let Some(target) = target {
        let _ = follow_main_window_with_target(&main_window, state, target);
    }
    state.follow_apply_in_progress.store(false, Ordering::Release);
}

#[cfg(target_os = "windows")]
fn primary_work_area() -> Result<WorkArea, String> {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut rect as *mut RECT as *mut c_void,
            0,
        )
    };
    if ok == 0 {
        return Err(i18n_error("backend_errors.primary_work_area_unavailable"));
    }

    Ok(WorkArea {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

#[cfg(not(target_os = "windows"))]
fn primary_work_area() -> Result<WorkArea, String> {
    Err(i18n_error("backend_errors.windows_only"))
}

#[cfg(target_os = "windows")]
fn work_area_for_window(hwnd: NativeHwnd) -> Result<WorkArea, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    unsafe {
        let monitor = MonitorFromWindow(hwnd_from_native(hwnd), MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return Err(i18n_error("backend_errors.monitor_lookup_failed"));
        }

        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info as *mut MONITORINFO) == 0 {
            return Err(i18n_error("backend_errors.monitor_lookup_failed"));
        }

        Ok(WorkArea {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        })
    }
}

#[cfg(target_os = "windows")]
fn find_slimevr_main_window() -> Result<Option<TargetWindowInfo>, String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowLongW, GetWindowRect, GetWindowTextLengthW,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GWL_EXSTYLE, GW_OWNER,
        WS_EX_TOOLWINDOW,
    };

    let hwnds = enumerate_top_level_windows()?;
    let mut best_match: Option<(TargetWindowInfo, bool, i64)> = None;

    for hwnd in hwnds {
        let hwnd_ptr = hwnd_from_native(hwnd);
        let valid = unsafe { IsWindow(hwnd_ptr) != 0 };
        if !valid {
            continue;
        }

        let visible = unsafe { IsWindowVisible(hwnd_ptr) != 0 };
        let iconic = unsafe { IsIconic(hwnd_ptr) != 0 };
        if !visible && !iconic {
            continue;
        }

        let owner = unsafe { GetWindow(hwnd_ptr, GW_OWNER) };
        if !owner.is_null() {
            continue;
        }

        let ex_style = unsafe { GetWindowLongW(hwnd_ptr, GWL_EXSTYLE) as u32 };
        if ex_style & WS_EX_TOOLWINDOW != 0 {
            continue;
        }

        let title_len = unsafe { GetWindowTextLengthW(hwnd_ptr) };
        if title_len <= 0 {
            continue;
        }

        let mut process_id = 0u32;
        let thread_id = unsafe { GetWindowThreadProcessId(hwnd_ptr, &mut process_id) };
        if process_id == 0 || thread_id == 0 {
            continue;
        }

        let Some(process_name) = process_name_from_pid(process_id)? else {
            continue;
        };
        if !process_name.eq_ignore_ascii_case("SlimeVR.exe") {
            continue;
        }

        let mut rect: RECT = unsafe { std::mem::zeroed() };
        let rect_ok = unsafe { GetWindowRect(hwnd_ptr, &mut rect) != 0 };
        if !rect_ok {
            continue;
        }
        let area = rect_area(&rect);
        let candidate = (
            TargetWindowInfo {
                hwnd,
                process_id,
                thread_id,
            },
            !iconic,
            area,
        );
        if best_match
            .as_ref()
            .map(|current| (candidate.1, candidate.2) > (current.1, current.2))
            .unwrap_or(true)
        {
            best_match = Some(candidate);
        }
    }

    Ok(best_match.map(|(target, _, _)| target))
}

#[cfg(not(target_os = "windows"))]
fn find_slimevr_main_window() -> Result<Option<TargetWindowInfo>, String> {
    Err(i18n_error("backend_errors.windows_only"))
}

#[cfg(target_os = "windows")]
fn enumerate_top_level_windows() -> Result<Vec<NativeHwnd>, String> {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let hwnds = unsafe { &mut *(lparam as *mut Vec<NativeHwnd>) };
        hwnds.push(native_from_hwnd(hwnd));
        1
    }

    let mut hwnds = Vec::new();
    let ok = unsafe { EnumWindows(Some(callback), &mut hwnds as *mut Vec<NativeHwnd> as isize) };
    if ok == 0 {
        return Err(i18n_error("backend_errors.window_enumeration_failed"));
    }
    Ok(hwnds)
}

#[cfg(target_os = "windows")]
fn process_name_from_pid(process_id: u32) -> Result<Option<String>, String> {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(i18n_error("backend_errors.process_snapshot_failed"));
    }

    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    let mut found_name = None;
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) != 0 };
    while has_entry {
        if entry.th32ProcessID == process_id {
            found_name = Some(wide_slice_to_string(&entry.szExeFile));
            break;
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) != 0 };
    }

    unsafe {
        CloseHandle(snapshot);
    }

    Ok(found_name)
}

#[cfg(target_os = "windows")]
fn wide_slice_to_string(raw: &[u16]) -> String {
    let len = raw.iter().position(|value| *value == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

#[cfg(target_os = "windows")]
fn rect_area(rect: &windows_sys::Win32::Foundation::RECT) -> i64 {
    let width = (rect.right - rect.left).max(0) as i64;
    let height = (rect.bottom - rect.top).max(0) as i64;
    width * height
}

#[cfg(target_os = "windows")]
fn set_external_window_rect(hwnd: NativeHwnd, desired: RectI32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, IsIconic, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER,
        SW_RESTORE,
    };

    unsafe {
        let hwnd = hwnd_from_native(hwnd);
        let mut current: RECT = std::mem::zeroed();
        let has_rect = GetWindowRect(hwnd, &mut current) != 0;
        let already_positioned = has_rect
            && current.left == desired.left
            && current.top == desired.top
            && current.right == desired.right
            && current.bottom == desired.bottom;

        if already_positioned && IsIconic(hwnd) == 0 {
            return Ok(());
        }

        ShowWindow(hwnd, SW_RESTORE);
        let ok = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            desired.left,
            desired.top,
            desired.width(),
            desired.height(),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        if ok == 0 {
            return Err(i18n_error("backend_errors.window_set_rect_failed"));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn current_window_rect(hwnd: NativeHwnd) -> Result<RectI32, String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd_from_native(hwnd), &mut rect) == 0 {
            return Err(i18n_error("backend_errors.window_rect_read_failed"));
        }
        Ok(RectI32 {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        })
    }
}

#[cfg(not(target_os = "windows"))]
fn current_window_rect(_hwnd: NativeHwnd) -> Result<RectI32, String> {
    Err(i18n_error("backend_errors.windows_only"))
}

#[cfg(target_os = "windows")]
fn focus_external_window(hwnd: NativeHwnd) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{BringWindowToTop, SetForegroundWindow};

    unsafe {
        let hwnd = hwnd_from_native(hwnd);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
fn focus_external_window(_hwnd: NativeHwnd) {}

#[cfg(target_os = "windows")]
fn set_window_corner_preference(hwnd: NativeHwnd, snapped_style: bool) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;

    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_DEFAULT: u32 = 0;
    const DWMWCP_DONOTROUND: u32 = 1;

    let preference = if snapped_style {
        DWMWCP_DONOTROUND
    } else {
        DWMWCP_DEFAULT
    };

    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd_from_native(hwnd),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    };
    if hr < 0 {
        return Err(i18n_error("backend_errors.window_corner_preference_failed"));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_window_corner_preference(_hwnd: NativeHwnd, _snapped_style: bool) -> Result<(), String> {
    Err(i18n_error("backend_errors.windows_only"))
}
