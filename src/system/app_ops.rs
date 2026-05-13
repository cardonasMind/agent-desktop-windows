use agent_desktop_core::error::AdapterError;
use agent_desktop_core::node::{AppInfo, WindowInfo};
use std::collections::HashMap;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE, CloseHandle};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_TERMINATE, PROCESS_QUERY_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowTextLengthW, GetWindowThreadProcessId,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    GetClassNameW,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

struct EnumCtx {
    windows: Vec<WindowInfo>,
    app_filter: Option<String>,
    focused_only: bool,
}

/// Enumerate visible top-level windows.
pub fn list_windows_impl(
    filter: &agent_desktop_core::adapter::WindowFilter,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let mut ctx = EnumCtx {
        windows: Vec::new(),
        app_filter: filter.app.as_ref().map(|s| s.to_lowercase()),
        focused_only: filter.focused_only,
    };

    let focused_hwnd = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };

    unsafe {
        let ctx_ptr = &mut ctx as *mut EnumCtx as isize;
        let _ = EnumWindows(Some(enum_window_proc), LPARAM(ctx_ptr));
    }

    // Mark focused window
    if \!ctx.windows.is_empty() {
        let focused_pid = if \!focused_hwnd.is_invalid() {
            let mut pid: u32 = 0;
            unsafe { GetWindowThreadProcessId(focused_hwnd, Some(&mut pid)); }
            pid as i32
        } else {
            0
        };
        for w in &mut ctx.windows {
            w.is_focused = w.pid == focused_pid;
        }
        if ctx.focused_only {
            ctx.windows.retain(|w| w.is_focused);
        }
    }

    Ok(ctx.windows)
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumCtx);

    if \!IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return TRUE;
    }

    // Skip certain system classes
    let mut class_buf = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buf) as usize;
    if class_len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..class_len]);
        if class_name == "Progman" || class_name == "WorkerW" || class_name == "Shell_TrayWnd" {
            return TRUE;
        }
    }

    let mut title_buf = vec\![0u16; (title_len + 1) as usize];
    GetWindowTextW(hwnd, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf)
        .trim_end_matches('\0')
        .to_string();

    if title.is_empty() {
        return TRUE;
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    let app_name = get_process_name(pid).unwrap_or_else(|| "Unknown".into());

    if let Some(ref filter) = ctx.app_filter {
        if \!app_name.to_lowercase().contains(filter) {
            return TRUE;
        }
    }

    let id = format\!("w-{:x}", hwnd.0 as usize & 0xFFFFFF);

    ctx.windows.push(WindowInfo {
        id,
        title,
        app: app_name,
        pid: pid as i32,
        bounds: None,
        is_focused: false,
    });

    TRUE
}

fn get_process_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let len = GetModuleFileNameExW(Some(handle.0), None, &mut buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit('\\')
            .next()
            .map(|s| s.trim_end_matches(".exe").to_string())
    }
}

/// List running GUI applications (deduplicated by PID).
pub fn list_apps_impl() -> Result<Vec<AppInfo>, AdapterError> {
    let filter = agent_desktop_core::adapter::WindowFilter {
        focused_only: false,
        app: None,
    };
    let windows = list_windows_impl(&filter)?;
    let mut seen = HashMap::new();
    for w in &windows {
        seen.entry(w.pid).or_insert_with(|| AppInfo {
            name: w.app.clone(),
            pid: w.pid,
            bundle_id: None, // Windows has no bundle IDs
        });
    }
    Ok(seen.into_values().collect())
}

/// Bring a window to the foreground.
pub fn focus_window_impl(win: &WindowInfo) -> Result<(), AdapterError> {
    let hwnd = find_hwnd_for_window(win)?;
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd)
            .ok()
            .map_err(|_| AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionFailed,
                "SetForegroundWindow failed",
            ))?;
    }
    Ok(())
}

/// Launch an application by name or path.
pub fn launch_app_impl(id: &str, timeout_ms: u64) -> Result<WindowInfo, AdapterError> {
    use std::process::Command;
    use std::time::{Duration, Instant};

    // Try to launch via cmd /c start
    let _ = Command::new("cmd")
        .args(["/c", "start", "", id])
        .spawn()
        .map_err(|e| AdapterError::new(
            agent_desktop_core::error::ErrorCode::AppNotFound,
            format\!("Failed to launch '{}': {}", id, e),
        ))?;

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let search = id.to_lowercase();
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let filter = agent_desktop_core::adapter::WindowFilter {
            focused_only: false,
            app: Some(search.clone()),
        };
        if let Ok(wins) = list_windows_impl(&filter) {
            if let Some(w) = wins.into_iter().next() {
                return Ok(w);
            }
        }
        if Instant::now() >= deadline {
            return Err(AdapterError::timeout(format\!(
                "App '{}' did not open a window within {}ms",
                id, timeout_ms
            )));
        }
    }
}

/// Close an application by name.
pub fn close_app_impl(id: &str, force: bool) -> Result<(), AdapterError> {
    let filter = agent_desktop_core::adapter::WindowFilter {
        focused_only: false,
        app: Some(id.to_string()),
    };
    let windows = list_windows_impl(&filter)?;
    if windows.is_empty() {
        return Err(AdapterError::new(
            agent_desktop_core::error::ErrorCode::AppNotFound,
            format\!("No running app matching '{}'", id),
        ));
    }

    for w in &windows {
        if force {
            unsafe {
                if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, w.pid as u32) {
                    let _ = TerminateProcess(handle, 1);
                    let _ = CloseHandle(handle);
                }
            }
        } else {
            let hwnd = find_hwnd_for_window(w).ok();
            if let Some(hwnd) = hwnd {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                        windows::Win32::Foundation::WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
        }
    }
    Ok(())
}

fn find_hwnd_for_window(win: &WindowInfo) -> Result<HWND, AdapterError> {
    let filter = agent_desktop_core::adapter::WindowFilter {
        focused_only: false,
        app: Some(win.app.clone()),
    };
    // Re-enumerate to get a fresh HWND via the callback
    struct FindCtx {
        target_pid: i32,
        target_title: String,
        found: Option<HWND>,
    }

    let mut ctx = FindCtx {
        target_pid: win.pid,
        target_title: win.title.clone(),
        found: None,
    };

    unsafe extern "system" fn find_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut FindCtx);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid as i32 \!= ctx.target_pid {
            return TRUE;
        }
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len > 0 {
            let mut buf = vec\![0u16; (title_len + 1) as usize];
            GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string();
            if title == ctx.target_title {
                ctx.found = Some(hwnd);
                return BOOL(0); // stop enumerating
            }
        }
        TRUE
    }

    unsafe {
        let _ = EnumWindows(Some(find_proc), LPARAM(&mut ctx as *mut FindCtx as isize));
    }

    ctx.found.ok_or_else(|| AdapterError::new(
        agent_desktop_core::error::ErrorCode::WindowNotFound,
        format\!("Window '{}' not found", win.title),
    ))
}

/// Public wrapper for find_hwnd_for_window, used by window_ops.
pub fn find_hwnd_for_window_pub(win: &WindowInfo) -> Result<HWND, AdapterError> {
    find_hwnd_for_window(win)
}
