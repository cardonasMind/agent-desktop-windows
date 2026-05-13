use agent_desktop_core::action::WindowOp;
use agent_desktop_core::error::AdapterError;
use agent_desktop_core::node::WindowInfo;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    MoveWindow, ShowWindow, SW_MINIMIZE, SW_MAXIMIZE, SW_RESTORE,
    GetWindowRect,
};

pub fn execute(win: &WindowInfo, op: WindowOp) -> Result<(), AdapterError> {
    let hwnd = crate::system::app_ops::find_hwnd_for_window_pub(win)?;

    unsafe {
        match op {
            WindowOp::Resize { width, height } => {
                let mut rect = windows::Win32::Foundation::RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                MoveWindow(hwnd, rect.left, rect.top, width as i32, height as i32, true)
                    .map_err(|e| AdapterError::new(
                        agent_desktop_core::error::ErrorCode::ActionFailed,
                        format\!("MoveWindow (resize) failed: {}", e),
                    ))?;
            }
            WindowOp::Move { x, y } => {
                let mut rect = windows::Win32::Foundation::RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                MoveWindow(hwnd, x as i32, y as i32, w, h, true)
                    .map_err(|e| AdapterError::new(
                        agent_desktop_core::error::ErrorCode::ActionFailed,
                        format\!("MoveWindow failed: {}", e),
                    ))?;
            }
            WindowOp::Minimize => {
                ShowWindow(hwnd, SW_MINIMIZE);
            }
            WindowOp::Maximize => {
                ShowWindow(hwnd, SW_MAXIMIZE);
            }
            WindowOp::Restore => {
                ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }
    Ok(())
}
