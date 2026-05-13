use agent_desktop_core::action::{KeyCombo, Modifier};
use agent_desktop_core::error::AdapterError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_EXTENDEDKEY, VIRTUAL_KEY,
    VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RETURN, VK_ESCAPE, VK_TAB,
    VK_BACK, VK_DELETE, VK_SPACE, VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT,
    VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_F1, VK_F2, VK_F3, VK_F4,
    VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
};

fn modifier_vk(m: &Modifier) -> VIRTUAL_KEY {
    match m {
        Modifier::Ctrl | Modifier::Cmd => VK_CONTROL, // Cmd maps to Ctrl on Windows
        Modifier::Alt => VK_MENU,
        Modifier::Shift => VK_SHIFT,
    }
}

fn key_to_vk(key: &str) -> Option<VIRTUAL_KEY> {
    let lower = key.to_lowercase();
    match lower.as_str() {
        "return" | "enter" => Some(VK_RETURN),
        "escape" | "esc" => Some(VK_ESCAPE),
        "tab" => Some(VK_TAB),
        "backspace" | "back" => Some(VK_BACK),
        "delete" | "del" => Some(VK_DELETE),
        "space" => Some(VK_SPACE),
        "up" => Some(VK_UP),
        "down" => Some(VK_DOWN),
        "left" => Some(VK_LEFT),
        "right" => Some(VK_RIGHT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "pageup" => Some(VK_PRIOR),
        "pagedown" => Some(VK_NEXT),
        "f1" => Some(VK_F1),
        "f2" => Some(VK_F2),
        "f3" => Some(VK_F3),
        "f4" => Some(VK_F4),
        "f5" => Some(VK_F5),
        "f6" => Some(VK_F6),
        "f7" => Some(VK_F7),
        "f8" => Some(VK_F8),
        "f9" => Some(VK_F9),
        "f10" => Some(VK_F10),
        "f11" => Some(VK_F11),
        "f12" => Some(VK_F12),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap().to_ascii_uppercase();
            Some(VIRTUAL_KEY(c as u16))
        }
        _ => None,
    }
}

fn kb_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    // Extended keys: arrows, home, end, insert, delete, page up/down
    let ext_keys = [VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_DELETE];
    if ext_keys.contains(&vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

pub fn press_combo(combo: &KeyCombo) -> Result<(), AdapterError> {
    let vk = key_to_vk(&combo.key).ok_or_else(|| {
        AdapterError::new(
            agent_desktop_core::error::ErrorCode::InvalidArgs,
            format\!("Unknown key: '{}'", combo.key),
        )
    })?;

    let mut inputs = Vec::new();

    // Press modifiers
    for m in &combo.modifiers {
        inputs.push(kb_input(modifier_vk(m), false));
    }
    // Press key
    inputs.push(kb_input(vk, false));
    // Release key
    inputs.push(kb_input(vk, true));
    // Release modifiers (reverse order)
    for m in combo.modifiers.iter().rev() {
        inputs.push(kb_input(modifier_vk(m), true));
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(AdapterError::new(
            agent_desktop_core::error::ErrorCode::ActionFailed,
            "SendInput (keyboard) returned 0",
        ));
    }
    Ok(())
}

pub fn key_down(combo: &KeyCombo) -> Result<(), AdapterError> {
    let vk = key_to_vk(&combo.key).ok_or_else(|| {
        AdapterError::new(
            agent_desktop_core::error::ErrorCode::InvalidArgs,
            format\!("Unknown key: '{}'", combo.key),
        )
    })?;
    let inputs = [kb_input(vk, false)];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
    Ok(())
}

pub fn key_up(combo: &KeyCombo) -> Result<(), AdapterError> {
    let vk = key_to_vk(&combo.key).ok_or_else(|| {
        AdapterError::new(
            agent_desktop_core::error::ErrorCode::InvalidArgs,
            format\!("Unknown key: '{}'", combo.key),
        )
    })?;
    let inputs = [kb_input(vk, true)];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
    Ok(())
}

/// Type a string by sending individual key events for each character.
pub fn type_text(text: &str) -> Result<(), AdapterError> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_UNICODE, KEYBDINPUT};

    let mut inputs = Vec::new();
    for c in text.encode_utf16() {
        // Key down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: c,
                    dwFlags: KEYEVENTF_UNICODE,
                    ..Default::default()
                },
            },
        });
        // Key up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: c,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        });
    }

    if \!inputs.is_empty() {
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            return Err(AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionFailed,
                "SendInput (type_text) returned 0",
            ));
        }
    }
    Ok(())
}
