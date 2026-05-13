use agent_desktop_core::node::Rect;
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationElement, IUIAutomationCondition,
    CUIAutomation, TreeScope_Children, TreeScope_Element,
    UIA_NamePropertyId, UIA_ControlTypePropertyId, UIA_BoundingRectanglePropertyId,
    UIA_IsEnabledPropertyId, UIA_HasKeyboardFocusPropertyId,
    UIA_ValueValuePropertyId, UIA_IsOffscreenPropertyId,
    UIA_HelpTextPropertyId,
};
use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, COINIT_MULTITHREADED, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Variant::VARIANT;
use windows::core::Interface;
use std::sync::OnceLock;

static UIA: OnceLock<IUIAutomation> = OnceLock::new();

/// Get or create the singleton IUIAutomation COM instance.
pub fn uia() -> &'static IUIAutomation {
    UIA.get_or_init(|| {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .expect("Failed to create IUIAutomation instance")
        }
    })
}

/// Get the root element of the desktop.
pub fn root_element() -> Option<IUIAutomationElement> {
    unsafe { uia().GetRootElement().ok() }
}

/// Find the first top-level window element matching a PID.
pub fn window_element_for_pid(pid: i32) -> Option<IUIAutomationElement> {
    unsafe {
        let root = root_element()?;
        let uia = uia();

        // Create a condition: ProcessId == pid
        let pid_cond = uia.CreatePropertyCondition(
            windows::Win32::UI::Accessibility::UIA_ProcessIdPropertyId,
            &VARIANT::from(pid),
        ).ok()?;

        // Create condition: ControlType == Window
        let win_type_id: i32 = 50032; // UIA_WindowControlTypeId
        let type_cond = uia.CreatePropertyCondition(
            UIA_ControlTypePropertyId,
            &VARIANT::from(win_type_id),
        ).ok()?;

        let and_cond = uia.CreateAndCondition(&pid_cond, &type_cond).ok()?;

        root.FindFirst(TreeScope_Children, &and_cond).ok()
    }
}

/// Find ALL top-level windows for a PID (some apps have multiple).
pub fn window_elements_for_pid(pid: i32) -> Vec<IUIAutomationElement> {
    unsafe {
        let Some(root) = root_element() else { return vec\![]; };
        let uia_instance = uia();

        let pid_cond = match uia_instance.CreatePropertyCondition(
            windows::Win32::UI::Accessibility::UIA_ProcessIdPropertyId,
            &VARIANT::from(pid),
        ) {
            Ok(c) => c,
            Err(_) => return vec\![],
        };

        let win_type_id: i32 = 50032;
        let type_cond = match uia_instance.CreatePropertyCondition(
            UIA_ControlTypePropertyId,
            &VARIANT::from(win_type_id),
        ) {
            Ok(c) => c,
            Err(_) => return vec\![],
        };

        let and_cond = match uia_instance.CreateAndCondition(&pid_cond, &type_cond) {
            Ok(c) => c,
            Err(_) => return vec\![],
        };

        match root.FindAll(TreeScope_Children, &and_cond) {
            Ok(arr) => {
                let mut results = Vec::new();
                if let Ok(len) = arr.Length() {
                    for i in 0..len {
                        if let Ok(el) = arr.GetElement(i) {
                            results.push(el);
                        }
                    }
                }
                results
            }
            Err(_) => vec\![],
        }
    }
}

/// Get the element name.
pub fn element_name(el: &IUIAutomationElement) -> Option<String> {
    unsafe {
        el.GetCurrentPropertyValue(UIA_NamePropertyId)
            .ok()
            .and_then(|v| bstr_from_variant(&v))
    }
}

/// Get the element value (for text fields, etc.).
pub fn element_value(el: &IUIAutomationElement) -> Option<String> {
    unsafe {
        el.GetCurrentPropertyValue(UIA_ValueValuePropertyId)
            .ok()
            .and_then(|v| bstr_from_variant(&v))
    }
}

/// Get the help text / description.
pub fn element_help_text(el: &IUIAutomationElement) -> Option<String> {
    unsafe {
        el.GetCurrentPropertyValue(UIA_HelpTextPropertyId)
            .ok()
            .and_then(|v| bstr_from_variant(&v))
    }
}

/// Get the control type ID.
pub fn element_control_type(el: &IUIAutomationElement) -> i32 {
    unsafe {
        el.GetCurrentPropertyValue(UIA_ControlTypePropertyId)
            .ok()
            .and_then(|v| i32_from_variant(&v))
            .unwrap_or(0)
    }
}

/// Check if the element is enabled.
pub fn is_enabled(el: &IUIAutomationElement) -> bool {
    unsafe {
        el.GetCurrentPropertyValue(UIA_IsEnabledPropertyId)
            .ok()
            .and_then(|v| bool_from_variant(&v))
            .unwrap_or(false)
    }
}

/// Check if element has keyboard focus.
pub fn has_focus(el: &IUIAutomationElement) -> bool {
    unsafe {
        el.GetCurrentPropertyValue(UIA_HasKeyboardFocusPropertyId)
            .ok()
            .and_then(|v| bool_from_variant(&v))
            .unwrap_or(false)
    }
}

/// Check if the element is offscreen.
pub fn is_offscreen(el: &IUIAutomationElement) -> bool {
    unsafe {
        el.GetCurrentPropertyValue(UIA_IsOffscreenPropertyId)
            .ok()
            .and_then(|v| bool_from_variant(&v))
            .unwrap_or(false)
    }
}

/// Get the bounding rectangle.
pub fn element_bounds(el: &IUIAutomationElement) -> Option<Rect> {
    unsafe {
        let v = el.GetCurrentPropertyValue(UIA_BoundingRectanglePropertyId).ok()?;
        rect_from_variant(&v)
    }
}

/// Get the child elements (direct children).
pub fn children(el: &IUIAutomationElement) -> Vec<IUIAutomationElement> {
    unsafe {
        let uia_instance = uia();
        let true_cond = match uia_instance.CreateTrueCondition() {
            Ok(c) => c,
            Err(_) => return vec\![],
        };
        match el.FindAll(TreeScope_Children, &true_cond) {
            Ok(arr) => {
                let mut results = Vec::new();
                if let Ok(len) = arr.Length() {
                    for i in 0..len {
                        if let Ok(child) = arr.GetElement(i) {
                            results.push(child);
                        }
                    }
                }
                results
            }
            Err(_) => vec\![],
        }
    }
}

/// Get available UIAutomation patterns (actions) for an element.
pub fn available_actions(el: &IUIAutomationElement) -> Vec<String> {
    // Check for common patterns
    let mut actions = Vec::new();
    unsafe {
        // InvokePattern (click)
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_InvokePatternId).is_ok() {
            actions.push("Click".into());
        }
        // ValuePattern (set value)
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_ValuePatternId).is_ok() {
            actions.push("SetValue".into());
        }
        // TogglePattern
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_TogglePatternId).is_ok() {
            actions.push("Toggle".into());
        }
        // ExpandCollapsePattern
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_ExpandCollapsePatternId).is_ok() {
            actions.push("Expand".into());
            actions.push("Collapse".into());
        }
        // SelectionItemPattern
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_SelectionItemPatternId).is_ok() {
            actions.push("Select".into());
        }
        // ScrollPattern
        if el.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_ScrollPatternId).is_ok() {
            actions.push("Scroll".into());
        }
    }
    actions
}

/// Get the process ID of an element.
pub fn element_pid(el: &IUIAutomationElement) -> i32 {
    unsafe {
        el.GetCurrentPropertyValue(windows::Win32::UI::Accessibility::UIA_ProcessIdPropertyId)
            .ok()
            .and_then(|v| i32_from_variant(&v))
            .unwrap_or(0)
    }
}

// ---- VARIANT helpers ----

fn bstr_from_variant(v: &VARIANT) -> Option<String> {
    unsafe {
        // VT_BSTR = 8
        if v.Anonymous.Anonymous.vt == 8 {
            let bstr = &v.Anonymous.Anonymous.Anonymous.bstrVal;
            if bstr.is_empty() {
                None
            } else {
                Some(bstr.to_string())
            }
        } else {
            None
        }
    }
}

fn i32_from_variant(v: &VARIANT) -> Option<i32> {
    unsafe {
        match v.Anonymous.Anonymous.vt {
            3 => Some(v.Anonymous.Anonymous.Anonymous.lVal),   // VT_I4
            2 => Some(v.Anonymous.Anonymous.Anonymous.iVal as i32), // VT_I2
            _ => None,
        }
    }
}

fn bool_from_variant(v: &VARIANT) -> Option<bool> {
    unsafe {
        match v.Anonymous.Anonymous.vt {
            11 => Some(v.Anonymous.Anonymous.Anonymous.boolVal.as_bool()), // VT_BOOL
            3 => Some(v.Anonymous.Anonymous.Anonymous.lVal \!= 0),
            _ => None,
        }
    }
}

fn rect_from_variant(v: &VARIANT) -> Option<Rect> {
    unsafe {
        // BoundingRectangle is returned as a VT_R8 | VT_ARRAY (SAFEARRAY of doubles)
        // with 4 elements: [left, top, width, height]
        if v.Anonymous.Anonymous.vt == (0x2000 | 5) {
            // VT_ARRAY | VT_R8
            let psa = v.Anonymous.Anonymous.Anonymous.parray;
            if psa.is_null() {
                return None;
            }
            let data = (*psa).pvData as *const f64;
            if data.is_null() {
                return None;
            }
            Some(Rect {
                x: *data,
                y: *data.add(1),
                width: *data.add(2),
                height: *data.add(3),
            })
        } else {
            None
        }
    }
}
