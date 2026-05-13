use agent_desktop_core::adapter::PermissionStatus;
use windows::Win32::UI::Accessibility::UiaRootObjectId;

/// Windows UIAutomation does not require an explicit accessibility permission
/// grant the way macOS does. We verify that the COM subsystem can be
/// initialised and that we can reach the root automation object.
pub fn check() -> PermissionStatus {
    use windows::Win32::UI::Accessibility::IUIAutomation;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::core::Interface;

    unsafe {
        // Ensure COM is initialised (idempotent if already done).
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let hr = windows::Win32::System::Com::CoCreateInstance(
            &windows::Win32::UI::Accessibility::CUIAutomation as *const _ as *const windows::core::GUID,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        );
        match hr {
            Ok(_uia) => PermissionStatus::Granted,
            Err(e) => PermissionStatus::Denied {
                suggestion: format\!(
                    "Failed to create UIAutomation instance: {}. \
                     Ensure you are running on Windows 7+ with COM available.",
                    e
                ),
            },
        }
    }
}
