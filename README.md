# agent-desktop-windows

Windows Platform Adapter for [agent-desktop](https://github.com/lahfir/agent-desktop) — native desktop automation CLI for AI agents.

## What is this?

This is a complete implementation of the Windows `PlatformAdapter` for agent-desktop, replacing the empty stub in `crates/windows/`. It enables agent-desktop to work on Windows using the UIAutomation COM API.

## Features implemented

| Module | What it does | Lines |
|---|---|---|
| `tree/element.rs` | UIAutomation COM wrapper, property queries | 309 |
| `tree/builder.rs` | Recursive AccessibilityNode tree (skeleton, compact, interactive_only) | 117 |
| `tree/resolve.rs` | Resolve RefEntry back to live IUIAutomationElement | 80 |
| `tree/roles.rs` | Map ControlTypeId to agent-desktop role strings | 70 |
| `actions/dispatch.rs` | Execute actions via UIA patterns (Invoke, Value, Toggle, ExpandCollapse, Scroll, SelectionItem) with mouse fallback | 373 |
| `input/mouse.rs` | Win32 SendInput for mouse (click, move, drag) | 136 |
| `input/keyboard.rs` | Win32 SendInput for keyboard + Unicode type_text | 176 |
| `input/clipboard.rs` | Win32 Clipboard API (get, set, clear) | 79 |
| `system/app_ops.rs` | EnumWindows, list_apps, launch, close, focus | 290 |
| `system/screenshot.rs` | GDI BitBlt capture + minimal PNG encoder (zero external deps) | 187 |
| `system/window_ops.rs` | Resize, move, minimize, maximize, restore | 47 |
| `system/permissions.rs` | UIAutomation COM check (Windows has no gate like macOS) | 32 |
| `adapter.rs` | WindowsAdapter implementing full PlatformAdapter trait | 203 |
| **Total** | **20 files** | **2,181 lines** |

## How to use

1. Clone the original repo: `git clone https://github.com/lahfir/agent-desktop.git`
2. Replace `crates/windows/` with the contents of this repo
3. `cargo build --release`
4. Use:

```bash
agent-desktop.exe snapshot -i          # see interactive elements (@e1, @e2...)
agent-desktop.exe click @e3            # click a button by ref
agent-desktop.exe type @e5 "hello"     # type into a text field
agent-desktop.exe press ctrl+s         # keyboard shortcut
agent-desktop.exe screenshot           # capture screen
agent-desktop.exe list-apps            # list running apps
agent-desktop.exe list-windows         # list visible windows
```

## Windows APIs used

- **UIAutomation COM** (`IUIAutomation`, `IUIAutomationElement`) - accessibility tree traversal and element interaction
- **Win32 SendInput** - mouse and keyboard input simulation
- **Win32 Clipboard API** - clipboard read/write
- **Win32 GDI** (BitBlt) - screen capture
- **Win32 EnumWindows** - window enumeration
- **Win32 Process API** - app management

## Requirements

- Windows 10/11
- Rust toolchain (rustup.rs)
- Visual Studio Build Tools with C++ workload

## License

Same as agent-desktop: Apache-2.0
