use agent_desktop_core::action::{Action, ActionResult, Direction, ElementState};
use agent_desktop_core::adapter::NativeHandle;
use agent_desktop_core::error::AdapterError;
use windows::Win32::UI::Accessibility::{
    IUIAutomationElement, IUIAutomationInvokePattern, IUIAutomationValuePattern,
    IUIAutomationTogglePattern, IUIAutomationExpandCollapsePattern,
    IUIAutomationSelectionItemPattern, IUIAutomationScrollPattern,
    UIA_InvokePatternId, UIA_ValuePatternId, UIA_TogglePatternId,
    UIA_ExpandCollapsePatternId, UIA_SelectionItemPatternId, UIA_ScrollPatternId,
    ScrollAmount_SmallIncrement, ScrollAmount_SmallDecrement, ScrollAmount_NoAmount,
    ToggleState_On, ToggleState_Off,
    ExpandCollapseState_Collapsed, ExpandCollapseState_Expanded,
};
use windows::core::Interface;

use crate::tree::element;
use crate::tree::roles;

/// Execute an action on a UIAutomation element referenced by NativeHandle.
pub fn perform_action(
    handle: &NativeHandle,
    action: &Action,
) -> Result<ActionResult, AdapterError> {
    let raw = handle.as_raw();
    if raw.is_null() {
        return Err(AdapterError::new(
            agent_desktop_core::error::ErrorCode::ElementNotFound,
            "Null element handle",
        ));
    }

    let el: IUIAutomationElement = unsafe {
        IUIAutomationElement::from_raw_borrowed(&raw)
            .ok_or_else(|| AdapterError::internal("Failed to cast NativeHandle to IUIAutomationElement"))?
            .clone()
    };

    match action {
        Action::Click => invoke_click(&el),
        Action::DoubleClick => {
            // UIAutomation doesn't have a native double-click; invoke + mouse fallback
            invoke_click(&el)
        }
        Action::RightClick => {
            // Right-click via mouse event at element center
            if let Some(bounds) = element::element_bounds(&el) {
                let cx = bounds.x + bounds.width / 2.0;
                let cy = bounds.y + bounds.height / 2.0;
                crate::input::mouse::synthesize_mouse(agent_desktop_core::MouseEvent {
                    kind: agent_desktop_core::MouseEventKind::Click { count: 1 },
                    point: agent_desktop_core::Point { x: cx, y: cy },
                    button: agent_desktop_core::MouseButton::Right,
                })?;
                Ok(ActionResult::new("right_click"))
            } else {
                Err(AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionFailed,
                    "Cannot right-click: element has no bounds",
                ))
            }
        }
        Action::TripleClick => {
            // Triple-click via mouse at element center
            if let Some(bounds) = element::element_bounds(&el) {
                let cx = bounds.x + bounds.width / 2.0;
                let cy = bounds.y + bounds.height / 2.0;
                crate::input::mouse::synthesize_mouse(agent_desktop_core::MouseEvent {
                    kind: agent_desktop_core::MouseEventKind::Click { count: 3 },
                    point: agent_desktop_core::Point { x: cx, y: cy },
                    button: agent_desktop_core::MouseButton::Left,
                })?;
                Ok(ActionResult::new("triple_click"))
            } else {
                Err(AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionFailed,
                    "Cannot triple-click: element has no bounds",
                ))
            }
        }
        Action::SetValue(val) => set_value(&el, val),
        Action::SetFocus => {
            unsafe {
                el.SetFocus().map_err(|e| AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionFailed,
                    format\!("SetFocus failed: {}", e),
                ))?;
            }
            Ok(ActionResult::new("focus"))
        }
        Action::TypeText(text) => {
            // Focus the element first, then type via SendInput
            let _ = unsafe { el.SetFocus() };
            std::thread::sleep(std::time::Duration::from_millis(50));
            crate::input::keyboard::type_text(text)?;
            Ok(ActionResult::new("type"))
        }
        Action::Clear => {
            // Try ValuePattern.SetValue("") first, else select-all + delete
            if let Ok(result) = set_value(&el, "") {
                return Ok(result);
            }
            let _ = unsafe { el.SetFocus() };
            std::thread::sleep(std::time::Duration::from_millis(50));
            crate::input::keyboard::press_combo(&agent_desktop_core::KeyCombo {
                key: "a".into(),
                modifiers: vec\![agent_desktop_core::Modifier::Ctrl],
            })?;
            std::thread::sleep(std::time::Duration::from_millis(30));
            crate::input::keyboard::press_combo(&agent_desktop_core::KeyCombo {
                key: "delete".into(),
                modifiers: vec\![],
            })?;
            Ok(ActionResult::new("clear"))
        }
        Action::Toggle => toggle(&el),
        Action::Check => {
            // Ensure toggle state is ON
            let state = get_toggle_state(&el);
            if state \!= Some(ToggleState_On) {
                toggle(&el)
            } else {
                Ok(ActionResult::new("check").with_state(element_state(&el)))
            }
        }
        Action::Uncheck => {
            let state = get_toggle_state(&el);
            if state \!= Some(ToggleState_Off) {
                toggle(&el)
            } else {
                Ok(ActionResult::new("uncheck").with_state(element_state(&el)))
            }
        }
        Action::Expand => expand_collapse(&el, true),
        Action::Collapse => expand_collapse(&el, false),
        Action::Select(option) => select_item(&el, option),
        Action::Scroll(direction, amount) => scroll(&el, direction, *amount),
        Action::ScrollTo => {
            // Try to scroll element into view via ScrollItemPattern
            unsafe {
                if let Ok(pattern) = el.GetCurrentPattern(
                    windows::Win32::UI::Accessibility::UIA_ScrollItemPatternId,
                ) {
                    let scroll_item: windows::Win32::UI::Accessibility::IUIAutomationScrollItemPattern =
                        pattern.cast().map_err(|e| AdapterError::internal(format\!("Cast failed: {}", e)))?;
                    scroll_item.ScrollIntoView().map_err(|e| AdapterError::new(
                        agent_desktop_core::error::ErrorCode::ActionFailed,
                        format\!("ScrollIntoView failed: {}", e),
                    ))?;
                    return Ok(ActionResult::new("scroll_to"));
                }
            }
            Err(AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionNotSupported,
                "Element does not support ScrollItemPattern",
            ))
        }
        Action::PressKey(combo) => {
            crate::input::keyboard::press_combo(combo)?;
            Ok(ActionResult::new("press"))
        }
        Action::KeyDown(combo) => {
            crate::input::keyboard::key_down(combo)?;
            Ok(ActionResult::new("key_down"))
        }
        Action::KeyUp(combo) => {
            crate::input::keyboard::key_up(combo)?;
            Ok(ActionResult::new("key_up"))
        }
        Action::Hover => {
            if let Some(bounds) = element::element_bounds(&el) {
                let cx = bounds.x + bounds.width / 2.0;
                let cy = bounds.y + bounds.height / 2.0;
                crate::input::mouse::synthesize_mouse(agent_desktop_core::MouseEvent {
                    kind: agent_desktop_core::MouseEventKind::Move,
                    point: agent_desktop_core::Point { x: cx, y: cy },
                    button: agent_desktop_core::MouseButton::Left,
                })?;
                Ok(ActionResult::new("hover"))
            } else {
                Err(AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionFailed,
                    "Cannot hover: element has no bounds",
                ))
            }
        }
        Action::Drag(params) => {
            crate::input::mouse::synthesize_drag(params.clone())?;
            Ok(ActionResult::new("drag"))
        }
    }
}

fn invoke_click(el: &IUIAutomationElement) -> Result<ActionResult, AdapterError> {
    unsafe {
        // Try InvokePattern first (cleanest)
        if let Ok(pattern) = el.GetCurrentPattern(UIA_InvokePatternId) {
            let invoke: IUIAutomationInvokePattern = pattern
                .cast()
                .map_err(|e| AdapterError::internal(format\!("Cast to InvokePattern failed: {}", e)))?;
            invoke.Invoke().map_err(|e| AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionFailed,
                format\!("Invoke failed: {}", e),
            ))?;
            return Ok(ActionResult::new("click").with_state(element_state(el)));
        }

        // Fallback: click at element center via mouse
        if let Some(bounds) = element::element_bounds(el) {
            let cx = bounds.x + bounds.width / 2.0;
            let cy = bounds.y + bounds.height / 2.0;
            crate::input::mouse::synthesize_mouse(agent_desktop_core::MouseEvent {
                kind: agent_desktop_core::MouseEventKind::Click { count: 1 },
                point: agent_desktop_core::Point { x: cx, y: cy },
                button: agent_desktop_core::MouseButton::Left,
            })?;
            return Ok(ActionResult::new("click").with_state(element_state(el)));
        }

        Err(AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            "Element does not support InvokePattern and has no bounds for mouse click",
        ))
    }
}

fn set_value(el: &IUIAutomationElement, val: &str) -> Result<ActionResult, AdapterError> {
    unsafe {
        let pattern = el.GetCurrentPattern(UIA_ValuePatternId).map_err(|_| {
            AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionNotSupported,
                "Element does not support ValuePattern",
            )
        })?;
        let value_pattern: IUIAutomationValuePattern = pattern
            .cast()
            .map_err(|e| AdapterError::internal(format\!("Cast to ValuePattern failed: {}", e)))?;

        let bstr = windows::core::BSTR::from(val);
        value_pattern.SetValue(&bstr).map_err(|e| AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            format\!("SetValue failed: {}", e),
        ))?;
        Ok(ActionResult::new("set_value").with_state(element_state(el)))
    }
}

fn toggle(el: &IUIAutomationElement) -> Result<ActionResult, AdapterError> {
    unsafe {
        let pattern = el.GetCurrentPattern(UIA_TogglePatternId).map_err(|_| {
            AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionNotSupported,
                "Element does not support TogglePattern",
            )
        })?;
        let toggle_pattern: IUIAutomationTogglePattern = pattern
            .cast()
            .map_err(|e| AdapterError::internal(format\!("Cast to TogglePattern failed: {}", e)))?;
        toggle_pattern.Toggle().map_err(|e| AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            format\!("Toggle failed: {}", e),
        ))?;
        Ok(ActionResult::new("toggle").with_state(element_state(el)))
    }
}

fn get_toggle_state(
    el: &IUIAutomationElement,
) -> Option<windows::Win32::UI::Accessibility::ToggleState> {
    unsafe {
        let pattern = el.GetCurrentPattern(UIA_TogglePatternId).ok()?;
        let toggle_pattern: IUIAutomationTogglePattern = pattern.cast().ok()?;
        toggle_pattern.CurrentToggleState().ok()
    }
}

fn expand_collapse(el: &IUIAutomationElement, expand: bool) -> Result<ActionResult, AdapterError> {
    unsafe {
        let pattern = el
            .GetCurrentPattern(UIA_ExpandCollapsePatternId)
            .map_err(|_| {
                AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionNotSupported,
                    "Element does not support ExpandCollapsePattern",
                )
            })?;
        let ec: IUIAutomationExpandCollapsePattern = pattern
            .cast()
            .map_err(|e| AdapterError::internal(format\!("Cast failed: {}", e)))?;
        if expand {
            ec.Expand()
        } else {
            ec.Collapse()
        }
        .map_err(|e| AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            format\!("{} failed: {}", if expand { "Expand" } else { "Collapse" }, e),
        ))?;
        Ok(ActionResult::new(if expand { "expand" } else { "collapse" })
            .with_state(element_state(el)))
    }
}

fn select_item(el: &IUIAutomationElement, _option: &str) -> Result<ActionResult, AdapterError> {
    unsafe {
        let pattern = el
            .GetCurrentPattern(UIA_SelectionItemPatternId)
            .map_err(|_| {
                AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionNotSupported,
                    "Element does not support SelectionItemPattern",
                )
            })?;
        let sel: IUIAutomationSelectionItemPattern = pattern
            .cast()
            .map_err(|e| AdapterError::internal(format\!("Cast failed: {}", e)))?;
        sel.Select().map_err(|e| AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            format\!("Select failed: {}", e),
        ))?;
        Ok(ActionResult::new("select").with_state(element_state(el)))
    }
}

fn scroll(
    el: &IUIAutomationElement,
    direction: &Direction,
    amount: u32,
) -> Result<ActionResult, AdapterError> {
    unsafe {
        let pattern = el.GetCurrentPattern(UIA_ScrollPatternId).map_err(|_| {
            AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionNotSupported,
                "Element does not support ScrollPattern",
            )
        })?;
        let scroll_pattern: IUIAutomationScrollPattern = pattern
            .cast()
            .map_err(|e| AdapterError::internal(format\!("Cast failed: {}", e)))?;

        for _ in 0..amount {
            let (h_amount, v_amount) = match direction {
                Direction::Up => (ScrollAmount_NoAmount, ScrollAmount_SmallDecrement),
                Direction::Down => (ScrollAmount_NoAmount, ScrollAmount_SmallIncrement),
                Direction::Left => (ScrollAmount_SmallDecrement, ScrollAmount_NoAmount),
                Direction::Right => (ScrollAmount_SmallIncrement, ScrollAmount_NoAmount),
            };
            scroll_pattern.Scroll(h_amount, v_amount).map_err(|e| {
                AdapterError::new(
                    agent_desktop_core::error::ErrorCode::ActionFailed,
                    format\!("Scroll failed: {}", e),
                )
            })?;
        }
        Ok(ActionResult::new("scroll"))
    }
}

fn element_state(el: &IUIAutomationElement) -> ElementState {
    let ct = element::element_control_type(el);
    let role = roles::role_from_control_type(ct).to_string();
    let mut states = Vec::new();
    if element::is_enabled(el) {
        states.push("enabled".into());
    }
    if element::has_focus(el) {
        states.push("focused".into());
    }
    ElementState {
        role,
        states,
        value: element::element_value(el),
    }
}
