/// Maps a UIA ControlTypeId (i32) to the role string used by agent-desktop core.
pub fn role_from_control_type(ct: i32) -> &'static str {
    match ct {
        50000 => "button",
        50001 => "calendar",
        50002 => "checkbox",
        50003 => "combobox",
        50004 => "edit",          // textfield
        50005 => "hyperlink",     // link
        50006 => "image",
        50007 => "listitem",
        50008 => "list",
        50009 => "menu",
        50010 => "menubar",
        50011 => "menuitem",
        50012 => "progressbar",
        50013 => "radiobutton",
        50014 => "scrollbar",
        50015 => "slider",
        50016 => "spinner",       // incrementor
        50017 => "statusbar",
        50018 => "tab",
        50019 => "tabitem",
        50020 => "text",
        50021 => "toolbar",
        50022 => "tooltip",
        50023 => "tree",
        50024 => "treeitem",
        50025 => "custom",
        50026 => "group",
        50027 => "thumb",
        50028 => "datagrid",
        50029 => "dataitem",
        50030 => "document",
        50031 => "splitbutton",
        50032 => "window",
        50033 => "pane",
        50034 => "header",
        50035 => "headeritem",
        50036 => "table",
        50037 => "titlebar",
        50038 => "separator",
        50039 => "semanticzoom",
        50040 => "appbar",
        _ => "unknown",
    }
}

/// Roles considered interactive (receive @e refs in snapshots).
pub fn is_interactive(role: &str) -> bool {
    matches\!(
        role,
        "button"
            | "checkbox"
            | "combobox"
            | "edit"
            | "hyperlink"
            | "listitem"
            | "menuitem"
            | "radiobutton"
            | "slider"
            | "spinner"
            | "tab"
            | "tabitem"
            | "treeitem"
            | "splitbutton"
            | "dataitem"
            | "menubar"
    )
}
