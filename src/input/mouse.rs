use agent_desktop_core::action::{DragParams, MouseButton, MouseEvent, MouseEventKind};
use agent_desktop_core::error::AdapterError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

fn to_absolute(x: f64, y: f64) -> (i32, i32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f64;
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f64;
    let abs_x = ((x / screen_w) * 65535.0) as i32;
    let abs_y = ((y / screen_h) * 65535.0) as i32;
    (abs_x, abs_y)
}

fn mouse_input(x: f64, y: f64, flags: u32) -> INPUT {
    let (ax, ay) = to_absolute(x, y);
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: ax,
                dy: ay,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(flags),
                ..Default::default()
            },
        },
    }
}

fn button_down_flag(btn: &MouseButton) -> u32 {
    match btn {
        MouseButton::Left => MOUSEEVENTF_LEFTDOWN.0,
        MouseButton::Right => MOUSEEVENTF_RIGHTDOWN.0,
        MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN.0,
    }
}

fn button_up_flag(btn: &MouseButton) -> u32 {
    match btn {
        MouseButton::Left => MOUSEEVENTF_LEFTUP.0,
        MouseButton::Right => MOUSEEVENTF_RIGHTUP.0,
        MouseButton::Middle => MOUSEEVENTF_MIDDLEUP.0,
    }
}

pub fn synthesize_mouse(event: MouseEvent) -> Result<(), AdapterError> {
    let mut inputs = Vec::new();

    match &event.kind {
        MouseEventKind::Move => {
            inputs.push(mouse_input(event.point.x, event.point.y, MOUSEEVENTF_MOVE.0));
        }
        MouseEventKind::Down => {
            inputs.push(mouse_input(
                event.point.x,
                event.point.y,
                MOUSEEVENTF_MOVE.0 | button_down_flag(&event.button),
            ));
        }
        MouseEventKind::Up => {
            inputs.push(mouse_input(
                event.point.x,
                event.point.y,
                MOUSEEVENTF_MOVE.0 | button_up_flag(&event.button),
            ));
        }
        MouseEventKind::Click { count } => {
            for _ in 0..*count {
                inputs.push(mouse_input(
                    event.point.x,
                    event.point.y,
                    MOUSEEVENTF_MOVE.0 | button_down_flag(&event.button),
                ));
                inputs.push(mouse_input(
                    event.point.x,
                    event.point.y,
                    button_up_flag(&event.button),
                ));
            }
        }
    }

    if \!inputs.is_empty() {
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            return Err(AdapterError::new(
                agent_desktop_core::error::ErrorCode::ActionFailed,
                "SendInput (mouse) returned 0",
            ));
        }
    }
    Ok(())
}

pub fn synthesize_drag(params: DragParams) -> Result<(), AdapterError> {
    let steps = params.duration_ms.unwrap_or(300) / 10;
    let steps = steps.max(5);

    // Move to start and press
    synthesize_mouse(MouseEvent {
        kind: MouseEventKind::Down,
        point: agent_desktop_core::action::Point {
            x: params.from.x,
            y: params.from.y,
        },
        button: MouseButton::Left,
    })?;

    // Interpolate
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let x = params.from.x + (params.to.x - params.from.x) * t;
        let y = params.from.y + (params.to.y - params.from.y) * t;
        synthesize_mouse(MouseEvent {
            kind: MouseEventKind::Move,
            point: agent_desktop_core::action::Point { x, y },
            button: MouseButton::Left,
        })?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Release
    synthesize_mouse(MouseEvent {
        kind: MouseEventKind::Up,
        point: agent_desktop_core::action::Point {
            x: params.to.x,
            y: params.to.y,
        },
        button: MouseButton::Left,
    })?;

    Ok(())
}
