use crate::tui::{Component, Focusable};

/// Single-line text input widget.
pub struct Input {
    text: String,
    cursor: usize,
    focused: bool,
    cached_width: Option<u16>,
    cached_lines: Option<Vec<String>>,
}

impl Input {
    pub fn new() -> Self {
        Input { text: String::new(), cursor: 0, focused: false,
            cached_width: None, cached_lines: None }
    }

    pub fn set_value(&mut self, value: &str) {
        self.text = value.to_string();
        self.cursor = self.text.len();
        self.invalidate();
    }

    pub fn get_value(&self) -> &str { &self.text }
}

impl Component for Input {
    fn render(&self, width: u16) -> Vec<String> {
        if let (Some(w), Some(lines)) = (&self.cached_width, &self.cached_lines) {
            if *w == width { return lines.clone(); }
        }
        let marker = if self.focused { crate::tui::CURSOR_MARKER } else { "" };
        let before = &self.text[..self.cursor];
        let at = self.text[self.cursor..].chars().next().unwrap_or(' ');
        let after = &self.text[self.cursor + at.len_utf8()..];
        let line = format!("> {}{}\x1b[7m{}\x1b[27m{}",
            before, marker, at, after);
        let truncated = crate::utils::truncate_to_width(&line, width as usize, None);
        vec![truncated]
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};
        if matches_key(data, Key::backspace) && self.cursor > 0 {
            let prev = self.text[..self.cursor].chars().last().unwrap();
            self.text.replace_range(self.cursor - prev.len_utf8()..self.cursor, "");
            self.cursor -= prev.len_utf8();
        } else if matches_key(data, Key::left) && self.cursor > 0 {
            let prev = self.text[..self.cursor].chars().last().unwrap();
            self.cursor -= prev.len_utf8();
        } else if matches_key(data, Key::right) && self.cursor < self.text.len() {
            let next = self.text[self.cursor..].chars().next().unwrap();
            self.cursor += next.len_utf8();
        } else if data.len() == 1 && !data.starts_with('\x1b') {
            self.text.insert_str(self.cursor, data);
            self.cursor += data.len();
        }
        self.invalidate();
    }

    fn invalidate(&mut self) {
        self.cached_width = None;
        self.cached_lines = None;
    }
}

impl Focusable for Input {
    fn set_focused(&mut self, focused: bool) { self.focused = focused; }
    fn is_focused(&self) -> bool { self.focused }
}

impl Default for Input {
    fn default() -> Self { Self::new() }
}
