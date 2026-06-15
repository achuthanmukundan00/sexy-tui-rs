use crate::tui::Component;
use crate::editor_component::EditorComponent;

use crate::theme::Theme;

pub struct EditorTheme {
    pub border_color: Box<dyn Fn(&str) -> String>,
}

impl EditorTheme {
    pub fn new(theme: &Theme) -> Self {
        let t = theme.clone();
        EditorTheme {
            border_color: Box::new(move |s| t.fg("accent", s)),
        }
    }
}

#[derive(Default)]
pub struct EditorOptions {
    pub padding_x: u16,
}


use crate::tui::Focusable;

/// Multi-line text editor widget.
pub struct Editor {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    focused: bool,
    theme: EditorTheme,
    options: EditorOptions,
}

impl Editor {
    pub fn new(theme: EditorTheme, options: EditorOptions) -> Self {
        Editor {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            focused: false,
            theme,
            options,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() { self.lines.push(String::new()); }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }
}

impl Component for Editor {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.options.padding_x * 2 + 2); // 2 for border
        let border = (self.theme.border_color)("─");
        let top = format!("┌{}┐", border.repeat(inner as usize));
        let bottom = format!("└{}┘", border.repeat(inner as usize));

        let mut lines = vec![top];
        let spacer = " ".repeat(self.options.padding_x as usize);

        for (i, line) in self.lines.iter().enumerate() {
            let truncated = crate::utils::truncate_to_width(line, inner as usize, None);
            let padded = format!("│{}{}{}│", spacer, truncated,
                " ".repeat((inner as usize).saturating_sub(crate::utils::visible_width(&truncated) + self.options.padding_x as usize + 2)));
            lines.push(padded);
            if i < self.lines.len().saturating_sub(1) { lines.push(format!("│{}{}│", spacer, " ".repeat(inner as usize))); }
        }
        lines.push(bottom);
        lines
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};
        if matches_key(data, Key::enter) {
            let rest = self.lines[self.cursor_row][self.cursor_col..].to_string();
            self.lines[self.cursor_row].truncate(self.cursor_col);
            self.lines.insert(self.cursor_row + 1, rest);
            self.cursor_row += 1;
            self.cursor_col = 0;
        } else if matches_key(data, Key::backspace) {
            if self.cursor_col > 0 {
                let prev = self.lines[self.cursor_row][..self.cursor_col].chars().last().unwrap();
                self.lines[self.cursor_row].remove(self.cursor_col - prev.len_utf8());
                self.cursor_col -= prev.len_utf8();
            } else if self.cursor_row > 0 {
                let rest = self.lines.remove(self.cursor_row);
                self.cursor_row -= 1;
                self.cursor_col = self.lines[self.cursor_row].len();
                self.lines[self.cursor_row].push_str(&rest);
            }
        } else if matches_key(data, Key::up) && self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        } else if matches_key(data, Key::down) && self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        } else if matches_key(data, Key::left) && self.cursor_col > 0 {
            let prev = self.lines[self.cursor_row][..self.cursor_col].chars().last().unwrap();
            self.cursor_col -= prev.len_utf8();
        } else if matches_key(data, Key::right) && self.cursor_col < self.lines[self.cursor_row].len() {
            let next = self.lines[self.cursor_row][self.cursor_col..].chars().next().unwrap();
            self.cursor_col += next.len_utf8();
        } else if data.len() == 1 && !data.starts_with('\x1b') && data.chars().all(|c| !c.is_control()) {
            self.lines[self.cursor_row].insert_str(self.cursor_col, data);
            self.cursor_col += data.len();
        }
        self.invalidate();
    }

    fn invalidate(&mut self) {}
}

impl Focusable for Editor {
    fn set_focused(&mut self, focused: bool) { self.focused = focused; }
    fn is_focused(&self) -> bool { self.focused }
}

impl EditorComponent for Editor {
    fn get_text(&self) -> String {
        self.get_text()
    }

    fn set_text(&mut self, text: &str) {
        self.set_text(text);
    }

    fn on_submit(&mut self, _text: &str) {
        // Default: no-op. Consumers should override via composition.
    }

    fn on_change(&mut self, _text: &str) {
        // Default: no-op. Consumers should override via composition.
    }
}
