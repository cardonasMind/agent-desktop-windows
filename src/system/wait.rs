use agent_desktop_core::error::AdapterError;

/// Wait for a menu to appear or disappear for a given PID.
/// Placeholder — full implementation needs UIAutomation menu detection.
pub fn wait_for_menu(_pid: i32, _open: bool, _timeout_ms: u64) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("wait_for_menu"))
}
