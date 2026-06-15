use crate::tui::Component;

/// Text widget — displays text with word wrapping and padding.
pub struct Text {
    content: String,
    padding_x: u16,
    padding_y: u16,
    bg_fn: Option<Box<dyn Fn(&str) -> String>>,
}

impl Text {
    pub fn new(content: &str, padding_x: u16, padding_y: u16, bg_fn: Option<Box<dyn Fn(&str) -> String>>) -> Self {
        Text { content: content.to_string(), padding_x, padding_y, bg_fn }
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
    }
}

impl Component for Text {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.padding_x * 2);
        let spacer = " ".repeat(self.padding_x as usize);
        let mut lines = vec!["".to_string(); self.padding_y as usize];
        for line in crate::utils::wrap_text_with_ansi(&self.content, inner as usize) {
            let padded = format!("{}{}", spacer, line);
            lines.push(if let Some(ref bg) = self.bg_fn { bg(&padded) } else { padded });
        }
        lines.extend(vec!["".to_string(); self.padding_y as usize]);
        lines
    }
    fn invalidate(&mut self) {}
}

/// TruncatedText widget — single-line text that truncates to fit width.
pub struct TruncatedText {
    content: String,
    padding_x: u16,
    padding_y: u16,
}

impl TruncatedText {
    pub fn new(content: &str, padding_x: u16, padding_y: u16) -> Self {
        TruncatedText { content: content.to_string(), padding_x, padding_y }
    }
}

impl Component for TruncatedText {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.padding_x * 2) as usize;
        let truncated = crate::utils::truncate_to_width(&self.content, inner, None);
        let mut lines = vec!["".to_string(); self.padding_y as usize];
        lines.push(format!("{}{}", " ".repeat(self.padding_x as usize), truncated));
        lines.extend(vec!["".to_string(); self.padding_y as usize]);
        lines
    }
    fn invalidate(&mut self) {}
}

/// Spacer widget — empty vertical space.
pub struct Spacer { lines: u16 }

impl Spacer {
    pub fn new(lines: u16) -> Self { Spacer { lines } }
}

impl Component for Spacer {
    fn render(&self, _width: u16) -> Vec<String> {
        vec!["".to_string(); self.lines as usize]
    }
    fn invalidate(&mut self) {}
}
