use agent_desktop_core::node::AccessibilityNode;
use agent_desktop_core::adapter::TreeOptions;
use windows::Win32::UI::Accessibility::IUIAutomationElement;
use super::element;
use super::roles;

/// Build a subtree rooted at `el`, respecting max_depth and options.
pub fn build_subtree(
    el: &IUIAutomationElement,
    depth: u8,
    opts: &TreeOptions,
) -> Option<AccessibilityNode> {
    if depth > opts.max_depth {
        return None;
    }

    // Skip offscreen elements in compact mode
    if opts.compact && element::is_offscreen(el) {
        return None;
    }

    let ct = element::element_control_type(el);
    let role = roles::role_from_control_type(ct).to_string();
    let name = element::element_name(el);
    let value = element::element_value(el);
    let description = element::element_help_text(el);
    let bounds = if opts.include_bounds {
        element::element_bounds(el)
    } else {
        None
    };

    // Gather states
    let mut states = Vec::new();
    if element::is_enabled(el) {
        states.push("enabled".into());
    } else {
        states.push("disabled".into());
    }
    if element::has_focus(el) {
        states.push("focused".into());
    }

    let is_interactive = roles::is_interactive(&role);

    // Handle skeleton mode: only go 3 levels deep, show children_count for truncated containers
    if opts.skeleton && depth >= 3 && \!is_interactive {
        let child_elements = element::children(el);
        let cc = child_elements.len() as u32;
        return Some(AccessibilityNode {
            ref_id: None,
            role,
            name,
            value,
            description,
            hint: None,
            states,
            bounds,
            children_count: if cc > 0 { Some(cc) } else { None },
            children: vec\![],
        });
    }

    // Build children recursively
    let child_elements = element::children(el);
    let mut children = Vec::new();
    for child in &child_elements {
        if let Some(child_node) = build_subtree(child, depth + 1, opts) {
            // In interactive_only mode, skip non-interactive leaf nodes
            if opts.interactive_only
                && \!roles::is_interactive(&child_node.role)
                && child_node.children.is_empty()
            {
                continue;
            }
            // In compact mode, skip empty groups
            if opts.compact
                && \!roles::is_interactive(&child_node.role)
                && child_node.children.is_empty()
                && child_node.name.is_none()
                && child_node.value.is_none()
            {
                continue;
            }
            children.push(child_node);
        }
    }

    // If interactive_only and this node is not interactive and has no
    // interactive descendants, skip it (unless it's the root)
    if opts.interactive_only && \!is_interactive && children.is_empty() && depth > 0 {
        return None;
    }

    Some(AccessibilityNode {
        ref_id: None, // Refs are assigned later by the core ref_alloc pass
        role,
        name,
        value,
        description,
        hint: None,
        states,
        bounds,
        children_count: None,
        children,
    })
}

/// Build a tree for a specific window given PID and optional title filter.
pub fn build_tree_for_window(
    pid: i32,
    _title: &str,
    opts: &TreeOptions,
) -> Option<AccessibilityNode> {
    let win_el = element::window_element_for_pid(pid)?;
    build_subtree(&win_el, 0, opts)
}
