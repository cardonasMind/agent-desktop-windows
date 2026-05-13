use agent_desktop_core::adapter::NativeHandle;
use agent_desktop_core::error::AdapterError;
use agent_desktop_core::refs::RefEntry;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
use windows::Win32::System::Variant::VARIANT;
use super::element;

/// Resolve a RefEntry (from the refmap) back to a live UIAutomation element.
///
/// Strategy: find the window by PID, then walk the tree matching by role,
/// name, and bounds_hash to find the matching element.
pub fn resolve_element_impl(entry: &RefEntry) -> Result<NativeHandle, AdapterError> {
    let win_el = element::window_element_for_pid(entry.pid).ok_or_else(|| {
        AdapterError::new(
            agent_desktop_core::error::ErrorCode::WindowNotFound,
            format\!("No window found for PID {}", entry.pid),
        )
        .with_suggestion("The target application may have closed. Run 'snapshot' to refresh.")
    })?;

    // Try to find the element by walking the tree
    match find_matching_element(&win_el, entry, 0, 15) {
        Some(el) => {
            // Convert IUIAutomationElement to a raw pointer for NativeHandle
            let raw = el.as_raw() as *const std::ffi::c_void;
            // AddRef to keep the element alive
            unsafe {
                use windows::core::Interface;
                el.AddRef();
            }
            Ok(NativeHandle::from_ptr(raw))
        }
        None => Err(AdapterError::element_not_found(&format\!(
            "{}:{}",
            entry.role,
            entry.name.as_deref().unwrap_or("(unnamed)")
        ))),
    }
}

fn find_matching_element(
    el: &IUIAutomationElement,
    entry: &RefEntry,
    depth: u8,
    max_depth: u8,
) -> Option<IUIAutomationElement> {
    if depth > max_depth {
        return None;
    }

    // Check if this element matches the entry
    let ct = element::element_control_type(el);
    let role = super::roles::role_from_control_type(ct);
    let name = element::element_name(el);

    if role == entry.role && name == entry.name {
        // If we have a bounds_hash, verify it matches
        if let Some(expected_hash) = entry.bounds_hash {
            if let Some(bounds) = element::element_bounds(el) {
                if bounds.bounds_hash() == expected_hash {
                    return Some(el.clone());
                }
            }
            // bounds_hash mismatch, continue searching
        } else {
            // No bounds_hash, match on role + name alone
            return Some(el.clone());
        }
    }

    // Recurse into children
    let children = element::children(el);
    for child in &children {
        if let Some(found) = find_matching_element(child, entry, depth + 1, max_depth) {
            return Some(found);
        }
    }

    None
}
