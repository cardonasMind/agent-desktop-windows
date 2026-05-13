use agent_desktop_core::{
    action::{Action, ActionResult, DragParams, KeyCombo, MouseEvent, WindowOp},
    adapter::{
        ImageBuffer, NativeHandle, PermissionStatus, PlatformAdapter, ScreenshotTarget,
        TreeOptions, WindowFilter,
    },
    error::AdapterError,
    node::{AccessibilityNode, AppInfo, Rect, WindowInfo},
    refs::RefEntry,
};

pub struct WindowsAdapter;

impl WindowsAdapter {
    pub fn new() -> Self {
        // Ensure COM is initialized on the calling thread.
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        Self
    }
}

impl Default for WindowsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformAdapter for WindowsAdapter {
    fn check_permissions(&self) -> PermissionStatus {
        crate::system::permissions::check()
    }

    fn list_windows(&self, filter: &WindowFilter) -> Result<Vec<WindowInfo>, AdapterError> {
        crate::system::app_ops::list_windows_impl(filter)
    }

    fn list_apps(&self) -> Result<Vec<AppInfo>, AdapterError> {
        crate::system::app_ops::list_apps_impl()
    }

    fn focused_window(&self) -> Result<Option<WindowInfo>, AdapterError> {
        let filter = WindowFilter {
            focused_only: true,
            app: None,
        };
        let windows = self.list_windows(&filter)?;
        Ok(windows.into_iter().next())
    }

    fn get_tree(
        &self,
        win: &WindowInfo,
        opts: &TreeOptions,
    ) -> Result<AccessibilityNode, AdapterError> {
        crate::tree::builder::build_tree_for_window(win.pid, &win.title, opts)
            .ok_or_else(|| AdapterError::internal("Empty UIAutomation tree for window"))
    }

    fn get_subtree(
        &self,
        handle: &NativeHandle,
        opts: &TreeOptions,
    ) -> Result<AccessibilityNode, AdapterError> {
        use windows::Win32::UI::Accessibility::IUIAutomationElement;
        use windows::core::Interface;

        let raw = handle.as_raw();
        if raw.is_null() {
            return Err(AdapterError::internal("Null handle in get_subtree"));
        }
        let el: IUIAutomationElement = unsafe {
            IUIAutomationElement::from_raw_borrowed(&raw)
                .ok_or_else(|| AdapterError::internal("Failed to cast handle"))?
                .clone()
        };
        crate::tree::builder::build_subtree(&el, 0, opts)
            .ok_or_else(|| {
                AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ElementNotFound,
                    "Element no longer exists in accessibility tree",
                )
                .with_suggestion("Run 'snapshot' to refresh refs, then retry.")
            })
    }

    fn execute_action(
        &self,
        handle: &NativeHandle,
        action: Action,
    ) -> Result<ActionResult, AdapterError> {
        crate::actions::dispatch::perform_action(handle, &action)
    }

    fn resolve_element(&self, entry: &RefEntry) -> Result<NativeHandle, AdapterError> {
        crate::tree::resolve::resolve_element_impl(entry)
    }

    fn release_handle(&self, handle: &NativeHandle) -> Result<(), AdapterError> {
        let raw = handle.as_raw();
        if raw.is_null() {
            return Ok(());
        }
        // Release the COM reference we added in resolve_element.
        unsafe {
            use windows::Win32::UI::Accessibility::IUIAutomationElement;
            use windows::core::Interface;
            if let Some(el) = IUIAutomationElement::from_raw_borrowed(&raw) {
                el.Release();
            }
        }
        Ok(())
    }

    fn focus_window(&self, win: &WindowInfo) -> Result<(), AdapterError> {
        crate::system::app_ops::focus_window_impl(win)
    }

    fn launch_app(&self, id: &str, timeout_ms: u64) -> Result<WindowInfo, AdapterError> {
        crate::system::app_ops::launch_app_impl(id, timeout_ms)
    }

    fn close_app(&self, id: &str, force: bool) -> Result<(), AdapterError> {
        crate::system::app_ops::close_app_impl(id, force)
    }

    fn screenshot(&self, target: ScreenshotTarget) -> Result<ImageBuffer, AdapterError> {
        match target {
            ScreenshotTarget::Window(pid) => crate::system::screenshot::capture_app(pid),
            ScreenshotTarget::Screen(idx) => crate::system::screenshot::capture_screen(idx),
            ScreenshotTarget::FullScreen => crate::system::screenshot::capture_screen(0),
        }
    }

    fn get_clipboard(&self) -> Result<String, AdapterError> {
        crate::input::clipboard::get()
    }

    fn set_clipboard(&self, text: &str) -> Result<(), AdapterError> {
        crate::input::clipboard::set(text)
    }

    fn clear_clipboard(&self) -> Result<(), AdapterError> {
        crate::input::clipboard::clear()
    }

    fn get_live_value(&self, handle: &NativeHandle) -> Result<Option<String>, AdapterError> {
        use windows::Win32::UI::Accessibility::IUIAutomationElement;
        use windows::core::Interface;

        let raw = handle.as_raw();
        if raw.is_null() {
            return Err(AdapterError::internal("Null handle"));
        }
        let el: IUIAutomationElement = unsafe {
            IUIAutomationElement::from_raw_borrowed(&raw)
                .ok_or_else(|| AdapterError::internal("Failed to cast handle"))?
                .clone()
        };
        Ok(crate::tree::element::element_value(&el))
    }

    fn get_element_bounds(&self, handle: &NativeHandle) -> Result<Option<Rect>, AdapterError> {
        use windows::Win32::UI::Accessibility::IUIAutomationElement;
        use windows::core::Interface;

        let raw = handle.as_raw();
        if raw.is_null() {
            return Err(AdapterError::internal("Null handle"));
        }
        let el: IUIAutomationElement = unsafe {
            IUIAutomationElement::from_raw_borrowed(&raw)
                .ok_or_else(|| AdapterError::internal("Failed to cast handle"))?
                .clone()
        };
        Ok(crate::tree::element::element_bounds(&el))
    }

    fn press_key_for_app(
        &self,
        _app_name: &str,
        combo: &KeyCombo,
    ) -> Result<ActionResult, AdapterError> {
        // On Windows we just send the key globally; app targeting is
        // done by focusing the window first.
        crate::input::keyboard::press_combo(combo)?;
        Ok(ActionResult::new("press"))
    }

    fn window_op(&self, win: &WindowInfo, op: WindowOp) -> Result<(), AdapterError> {
        crate::system::window_ops::execute(win, op)
    }

    fn mouse_event(&self, event: MouseEvent) -> Result<(), AdapterError> {
        crate::input::mouse::synthesize_mouse(event)
    }

    fn drag(&self, params: DragParams) -> Result<(), AdapterError> {
        crate::input::mouse::synthesize_drag(params)
    }
}
