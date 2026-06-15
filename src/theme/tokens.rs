/// Design token vocabulary — default values for all tokens.

use std::collections::HashMap;

/// Apply all built-in default tokens to the values map.
pub fn apply_defaults(values: &mut HashMap<String, String>) {
    // Color tokens
    let colors = [
        ("surface", "#0b0d10"),
        ("overlay", "#141820"),
        ("raised", "#1a1f2b"),
        ("foreground", "#d7d0c2"),
        ("muted", "#51565f"),
        ("dim", "#3a3f47"),
        ("accent", "#ff9d3b"),
        ("success", "#94b982"),
        ("error", "#d66f6f"),
        ("warning", "#d2a85f"),
        ("info", "#7da3c8"),
        ("border", "#51565f"),
        ("border_focused", "accent"),
        ("border_idle", "muted"),
        ("user_msg_text", "#f5ead8"),
        ("user_msg_bg", "#1a1f2b"),
        ("assistant_msg_text", "#d7d0c2"),
        ("assistant_msg_bg", "#0b0d10"),
        ("tool_title", "#b2bec3"),
        ("tool_output", "#636e72"),
        ("tool_pending_bg", "#1a1f2b"),
        ("tool_success_bg", "#141820"),
        ("tool_error_bg", "#1a1010"),
        ("diff_added", "#94b982"),
        ("diff_removed", "#d66f6f"),
        ("diff_context", "#51565f"),
        ("md_heading", "accent"),
        ("md_link", "info"),
        ("md_code", "success"),
        ("md_code_border", "muted"),
        ("md_quote", "muted"),
        ("md_quote_border", "dim"),
        ("md_hr", "dim"),
        ("md_list_bullet", "muted"),
        ("syntax_comment", "#51565f"),
        ("syntax_keyword", "#b28ac8"),
        ("syntax_function", "#7da3c8"),
        ("syntax_variable", "#d7d0c2"),
        ("syntax_string", "#94b982"),
        ("syntax_number", "#ff9d3b"),
        ("syntax_type", "#d2a85f"),
        ("syntax_operator", "#ff9d3b"),
        ("syntax_punctuation", "#b2bec3"),
    ];
    for (k, v) in &colors {
        values.entry(k.to_string()).or_insert(v.to_string());
    }

    // Border tokens
    values.entry("border_style".into()).or_insert("rounded".into());

    // Spacing tokens
    let spacings = [
        ("spacing_none", "0"), ("spacing_xs", "1"), ("spacing_sm", "2"),
        ("spacing_md", "4"), ("spacing_lg", "8"), ("spacing_xl", "12"),
    ];
    for (k, v) in &spacings {
        values.entry(k.to_string()).or_insert(v.to_string());
    }

    // Icon tokens
    let icons = [
        ("icon_branch", "\u{e0a0}"),     // 
        ("icon_success", "\u{f00c}"),     // 
        ("icon_error", "\u{f00d}"),       // 
        ("icon_warning", "\u{f071}"),     // 
        ("icon_folder", "\u{f07c}"),      // 
        ("icon_search", "\u{f002}"),      // 
        ("icon_prompt", "❯"),
        ("icon_rust", "\u{e7a8}"),        // 
        ("icon_python", "\u{e73c}"),      // 
        ("icon_go", "\u{e627}"),          // 
        ("icon_js", "\u{e74e}"),          // 
    ];
    for (k, v) in &icons {
        values.entry(k.to_string()).or_insert(v.to_string());
    }

    // Editor tokens
    values.entry("editor_cursor_style".into()).or_insert("bar".into());
    values.entry("editor_padding".into()).or_insert("sm".into());
    values.entry("editor_border".into()).or_insert("rounded".into());

    // Select list tokens
    values.entry("select_list_prefix".into()).or_insert("❯ ".into());
    values.entry("select_list_scroll_indicator".into()).or_insert("true".into());
    values.entry("select_list_max_visible".into()).or_insert("10".into());

    // Loader tokens
    values.entry("loader_spinner_frames".into()).or_insert("⠋,⠙,⠹,⠸,⠼,⠴,⠦,⠧,⠇,⠏".into());
    values.entry("loader_interval_ms".into()).or_insert("80".into());
}

/// ASCII fallback for icon tokens when Nerd Font is unavailable.
pub fn ascii_fallback(token: &str) -> &str {
    match token {
        "icon_branch" => "git:",
        "icon_success" => "√",
        "icon_error" => "×",
        "icon_warning" => "!",
        "icon_folder" => "./",
        "icon_search" => "?",
        "icon_prompt" => ">",
        "icon_rust" => "rs",
        "icon_python" => "py",
        "icon_go" => "go",
        "icon_js" => "js",
        _ => "•",
    }
}
